//! The gateway session: connecting, then answering BACKCORD's requests and
//! taking in Discord's events until the connection ends, and reconnecting.

use std::{
    pin::Pin,
    sync::{Arc, RwLock},
};

use anyhow::{Result, anyhow};
use serde_json::Value;
use tokio::{
    sync::{broadcast, mpsc, watch},
    time::{Duration, Instant, Sleep},
};

use super::{
    Live, Request, Snapshot, Status, live::live_events, member_lists::MemberLists,
    member_search::MemberSearches, now_ms, publisher::Publisher, read_acks::ReadAcks, server,
    stream_openings::StreamOpenings, tracker::Tracker, voice_join::VoiceJoin,
};
use crate::{
    api::Api,
    cache,
    event::Event,
    gateway::{Closed, Dispatch, Gateway, Reconnects, TooManyIdentifies},
    member_list::Presence,
    message::{self, Names},
    ready,
    settings::Away,
    token,
};

/// What outlasts a session: the handle's channels and what BACKCORD has on screen.
pub(super) struct Shared {
    pub(super) snapshots: watch::Sender<Arc<Snapshot>>,
    pub(super) live: broadcast::Sender<Live>,
    pub(super) names: Arc<RwLock<Names>>,
    /// The notes this account has read: USER_NOTE_UPDATE changes them, and a
    /// new session, which may have missed some, forgets them.
    pub(super) notes: super::Notes,
    pub(super) api: Arc<Api>,
    pub(super) lists: MemberLists,
    /// The presence this session last sent, which it identifies with again.
    pub(super) presence: Value,
    /// The game this account plays, as Rich Presence or detection says.
    pub(super) activity: watch::Receiver<Option<Arc<Value>>>,
    /// Whether the account shares it, which runs or stops what finds it.
    pub(super) show_game: watch::Sender<bool>,
    /// The song this account plays, from what follows its linked Spotify,
    /// and those links, which READY and USER_CONNECTIONS_UPDATE give.
    pub(super) music: watch::Receiver<Option<Arc<Value>>>,
    pub(super) spotify: watch::Sender<crate::spotify::Accounts>,
    /// Nobody has used BACKCORD since then, in Unix milliseconds, and whether
    /// it may show idle; see [`settings::Away`].
    pub(super) inactive: Option<(u64, bool)>,
    /// The server and channel or thread on screen, and a thread beside it,
    /// kept through reconnecting.
    pub(super) viewing: Option<(String, String)>,
    pub(super) beside: Option<(String, String)>,
}

pub(super) async fn run(
    mut token: String,
    mut requests: mpsc::UnboundedReceiver<Request>,
    mut shared: Shared,
) {
    let snapshots = shared.snapshots.clone();
    let mut reconnects = Reconnects::default();
    // Auto Delete's deleter, for as long as the service runs.
    let _deleter = AbortOnDrop(tokio::spawn(super::auto_delete::run(shared.api.clone())));
    // With the build number kept from the last start, the web client's page
    // loads beside connecting, rather than before it.
    if let Ok(Some(kept)) = tokio::task::spawn_blocking(crate::cache::build_number).await {
        crate::client::recall_build_number(kept);
    }
    if crate::client::has_build_number() {
        let api = shared.api.clone();
        tokio::spawn(async move { api.learn_build_number().await });
    }
    loop {
        // A login changed while connected, such as by a new password, is the
        // one to connect with.
        let current = shared.api.login();
        if !current.is_empty() && current != token {
            token = current;
        }
        snapshots.send_modify(|snapshot| Arc::make_mut(snapshot).status = Status::Connecting);
        let connect = async {
            // Without one kept, the page goes first, so the first identify
            // says the current build.
            if !crate::client::has_build_number() {
                shared.api.learn_build_number().await;
            }
            let playing = *shared.show_game.borrow();
            let music = shared.music.borrow().clone();
            let music = music.filter(|_| shared.presence["status"] != "invisible");
            crate::own_activity::merge(
                &mut shared.presence,
                shared.activity.borrow().as_deref().filter(|_| playing),
                music.as_deref(),
            );
            Gateway::connect(&token, shared.presence.clone(), &mut reconnects).await
        };
        let connected = tokio::select! {
            connected = connect => connected,
            () = snapshots.closed() => return,
        };
        let (error, up) = match connected {
            Ok((gateway, ready)) => {
                if let Some(refreshed) = ready::auth_token(&ready) {
                    token = refreshed.to_owned();
                    relogin(&shared.api, &token).await;
                }
                let started = tokio::time::Instant::now();
                let ended = connected_session(gateway, ready, &mut requests, &mut shared).await;
                shared.lists.disconnected();
                match ended {
                    Ended::Dropped => return,
                    Ended::Failed(error) => (error, Some(started.elapsed())),
                }
            }
            Err(error) => (error, None),
        };
        tracing::warn!(%error, "Discord connection ended");
        // Discord may end the session of a login it replaced, such as the
        // one a new password replaced; the new one connects again.
        let replaced = {
            let current = shared.api.login();
            !current.is_empty() && current != token
        };
        let stopped = if replaced {
            None
        } else if error.downcast_ref::<Closed>().is_some_and(Closed::is_final) {
            Some("Discord refused the saved login. Link your Discord account again.")
        } else if error.downcast_ref::<TooManyIdentifies>().is_some() {
            Some(
                "Discord keeps ending the connection, so BACKCORD stopped reconnecting. Restart BACKCORD to try again.",
            )
        } else {
            None
        };
        if let Some(reason) = stopped {
            snapshots.send_modify(|snapshot| {
                let snapshot = Arc::make_mut(snapshot);
                snapshot.status = Status::Offline(reason.into());
                snapshot.voice.clear();
            });
            // Keep answering until BACKCORD drops the handles.
            while let Some(request) = requests.recv().await {
                refuse(request, &mut shared);
            }
            return;
        }
        snapshots.send_modify(|snapshot| {
            let snapshot = Arc::make_mut(snapshot);
            snapshot.status = Status::Offline("Can't reach Discord. Reconnecting...".into());
            snapshot.voice.clear();
        });
        let wait = tokio::time::sleep(reconnects.wait(&error, up));
        tokio::pin!(wait);
        loop {
            tokio::select! {
                () = &mut wait => break,
                request = requests.recv() => match request {
                    Some(request) => refuse(request, &mut shared),
                    None => return,
                },
            }
        }
    }
}

/// Stops a task when it goes out of scope.
struct AbortOnDrop(tokio::task::JoinHandle<()>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Discord refreshed the login in READY, or gave a new one for a new
/// password or two-factor authentication, and the one BACKCORD identified
/// with is to go: requests, identifying again, the credential store and the
/// files kept all take the new one. The gateway takes it from READY itself,
/// or connects again with it.
pub(super) async fn relogin(api: &Api, token: &str) {
    tracing::info!("Discord refreshed the login");
    api.relogin(token);
    let token = token.to_owned();
    let stored = tokio::task::spawn_blocking(move || {
        cache::relogin(&token);
        token::store(&token)
    })
    .await;
    if let Ok(Err(error)) = stored {
        tracing::warn!(%error, "could not save Discord's refreshed login");
    }
}

/// Answers a request while disconnected. Viewing a channel, being idle,
/// and sharing the game played are remembered.
fn refuse(request: Request, shared: &mut Shared) {
    match request {
        Request::Join { reply, .. } => {
            let _ = reply.send(Err(anyhow!(
                "Discord isn't connected right now. Try again in a moment."
            )));
        }
        Request::View { guild, channel } => {
            shared.viewing = channel.clone().map(|channel| (guild.clone(), channel));
            shared.lists.view_offline(&shared.snapshots, guild, channel);
        }
        // Pointed at again once connected.
        Request::Peek { .. } => {}
        Request::Beside { guild, thread } => {
            shared.beside = thread.map(|thread| (guild, thread));
        }
        // The next session shows it once it connects.
        Request::Inactive(since, show_idle) => {
            shared.inactive = since.map(|since| (since, show_idle));
        }
        // The settings changed already; what finds the game stops or
        // runs now, and the next READY says the same.
        Request::ShowGame(shown) => {
            shared.show_game.send_replace(shown);
            shared.snapshots.send_modify(|snapshot| {
                let snapshot = Arc::make_mut(snapshot);
                snapshot.show_game = shown;
                snapshot.playing = None;
            });
        }
        _ => {}
    }
}

enum Ended {
    /// Every handle was dropped.
    Dropped,
    Failed(anyhow::Error),
}

async fn connected_session(
    gateway: Gateway,
    ready: Value,
    requests: &mut mpsc::UnboundedReceiver<Request>,
    shared: &mut Shared,
) -> Ended {
    let Some(me) = ready::user_id(&ready).map(str::to_owned) else {
        return Ended::Failed(anyhow!("Discord's READY had no user"));
    };
    let mut tracker = Tracker::from_ready(me, &ready, shared.names.clone());
    shared
        .spotify
        .send_if_modified(|accounts| accounts.keep(&tracker.me, &ready["connected_accounts"]));
    shared
        .notes
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clear();
    // READY can contain tens of megabytes of JSON. Only the parsed account
    // state is needed while connected; release the payload before waiting.
    drop(ready);
    trim_startup_heap();
    tracker.keep_unavailable(&shared.snapshots.borrow().servers);
    // Only what's kept of the earlier servers stays: held to the end of
    // this function, they would last as long as the session.
    let earlier = shared.snapshots.borrow().servers.clone();
    for (guild, channel) in shared.viewing.iter().chain(&shared.beside) {
        tracker.keep_thread(&earlier, guild, channel);
    }
    drop(earlier);
    if let Some((guild, channel)) = &shared.viewing {
        tracker.view(guild, Some(channel));
    }
    tracker.beside(shared.beside.as_ref().map(|(_, thread)| thread.as_str()));
    let mut snapshot = tracker.snapshot();
    snapshot.keep_spotify(&shared.snapshots.borrow());
    shared.snapshots.send_replace(Arc::new(snapshot));
    // Discord's client counts every command index out of date in a new
    // session.
    let _ = shared
        .live
        .send(Live::CommandIndexStale(crate::commands::Stale::All));
    let quiet = tracker.next_quiet(now_ms());
    let mut session = Session {
        gateway,
        publisher: Publisher::new(shared.snapshots.clone()),
        acks: ReadAcks::new(shared.api.clone()),
        status_ends: Box::pin(tokio::time::sleep(status_left(&tracker))),
        mute_end: tracker.unreads.next_mute_end(),
        mutes_end: Box::pin(tokio::time::sleep(until(tracker.unreads.next_mute_end()))),
        quiet,
        quiets: Box::pin(tokio::time::sleep(until(quiet))),
        away_change: None,
        away_changes: Box::pin(tokio::time::sleep(until(None))),
        tracker,
        shared,
        join: VoiceJoin::new(),
        streams: StreamOpenings::new(),
        searches: MemberSearches::default(),
        previews: mpsc::unbounded_channel(),
        asked_in_call: std::collections::HashSet::new(),
    };
    if let Err(error) = session.connect().await {
        return Ended::Failed(error);
    }
    session.run(requests).await
}

/// glibc otherwise holds freed READY allocations for the entire session.
/// Trim once after parsing, before the session starts handling requests.
fn trim_startup_heap() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    {
        unsafe extern "C" {
            fn malloc_trim(pad: usize) -> std::ffi::c_int;
        }
        // SAFETY: malloc_trim only returns unused allocator pages to the OS.
        unsafe { malloc_trim(0) };
    }
}

/// A connected session, and what it waits on.
struct Session<'a> {
    gateway: Gateway,
    tracker: Tracker,
    shared: &'a mut Shared,
    publisher: Publisher,
    join: VoiceJoin,
    streams: StreamOpenings,
    searches: MemberSearches,
    acks: ReadAcks,
    /// When the custom status ends by itself.
    status_ends: Pin<Box<Sleep>>,
    /// When the soonest mute in force ends, in Unix milliseconds, and the
    /// wait for it, after which what shows muted changes by itself.
    mute_end: Option<u64>,
    mutes_end: Pin<Box<Sleep>>,
    /// When the next thread this account is in goes quiet, or its next
    /// timeout ends, in Unix milliseconds, and the wait for it, after which
    /// the thread stops listing or the timeout's limits lift.
    quiet: Option<u64>,
    quiets: Pin<Box<Sleep>>,
    /// When being away next changes the presence, in Unix milliseconds,
    /// and the wait for it.
    away_change: Option<u64>,
    away_changes: Pin<Box<Sleep>>,
    /// Discord's answers when asked for message requests' latest messages.
    previews: (mpsc::UnboundedSender<Value>, mpsc::UnboundedReceiver<Value>),
    /// Those in this account's call whose server profile it asked for, by
    /// server and user, to tell bots: each asked once a session.
    asked_in_call: std::collections::HashSet<(String, String)>,
}

impl Session<'_> {
    async fn connect(&mut self) -> Result<()> {
        // This session shows the status the settings hold, as Discord's own apps do.
        self.show_presence().await?;
        self.shared.lists.publish_group(&self.tracker);
        self.ask_previews();
        self.shared
            .lists
            .connect(&self.tracker, &mut self.gateway)
            .await?;
        // Every server with a call going, so its statuses show as it opens.
        self.shared
            .lists
            .ask_all_busy(&self.tracker, &mut self.gateway)
            .await
    }

    async fn run(mut self, requests: &mut mpsc::UnboundedReceiver<Request>) -> Ended {
        loop {
            let done = tokio::select! {
                () = self.publisher.due(), if self.publisher.stale() => {
                    self.publisher.publish(&mut self.tracker, &self.shared.lists);
                    self.quiets_changed();
                    Ok(())
                }
                request = requests.recv() => match request {
                    Some(request) => self.request(request).await,
                    None => {
                        self.gateway.close().await;
                        return Ended::Dropped;
                    }
                },
                () = self.status_ends.as_mut(), if self.tracker.custom_status.as_ref().is_some_and(|custom| custom.expires_at_ms.is_some()) => {
                    self.tracker.custom_status = None;
                    self.publisher.changed();
                    self.show_presence().await
                }
                () = activity_changed(&mut self.shared.activity) => self.show_presence().await,
                () = activity_changed(&mut self.shared.music) => self.show_presence().await,
                () = self.away_changes.as_mut(), if self.away_change.is_some() => {
                    self.show_presence().await
                }
                () = self.quiets.as_mut(), if self.quiet.is_some() => {
                    // Published again, it waits for the next.
                    self.quiet = None;
                    self.publisher.changed();
                    Ok(())
                }
                () = self.mutes_end.as_mut(), if self.mute_end.is_some() => {
                    self.publisher.changed();
                    self.mutes_changed();
                    Ok(())
                }
                () = self.shared.lists.went_stale(), if self.shared.lists.unsent() => {
                    // The list on screen went stale without Discord sending it.
                    self.shared.lists.refresh(&mut self.gateway).await
                }
                Some(found) = self.previews.1.recv() => {
                    if self.tracker.put_previews(&found) {
                        self.publisher.changed();
                    }
                    Ok(())
                }
                () = self.acks.send_when_due(), if self.acks.waiting() => Ok(()),
                () = self.streams.timed_out(), if self.streams.waiting() => Ok(()),
                () = self.join.timed_out(), if self.join.waiting() => Ok(()),
                event = self.gateway.next() => match event {
                    Ok(event) => self.event(event).await,
                    Err(error) => {
                        self.join.fail("Discord disconnected while joining voice.");
                        self.streams.fail("Discord disconnected while opening the stream.");
                        Err(error)
                    }
                },
            };
            if let Err(error) = done {
                return Ended::Failed(error);
            }
        }
    }

    /// Acts on a request from the handle; an error ends the session.
    async fn request(&mut self, request: Request) -> Result<()> {
        let gateway = &mut self.gateway;
        match request {
            Request::Join {
                guild,
                channel,
                reply,
            } => {
                let sent = gateway
                    .update_voice_state(server(&guild), Some(&channel), true, false, false)
                    .await;
                self.join.start(guild, channel, reply);
                sent
            }
            Request::Leave { guild } => {
                // A call there on another client stays: no channel would
                // take it out of that one too.
                if !self.join.leave(&guild) && !self.tracker.in_voice_here(&guild) {
                    return Ok(());
                }
                gateway
                    .update_voice_state(server(&guild), None, false, false, false)
                    .await
            }
            Request::SelfState {
                guild,
                channel,
                mute,
                deaf,
                video,
            } => {
                let (mute, deaf) = crate::client::shown_voice(mute, deaf);
                gateway
                    .update_voice_state(server(&guild), Some(&channel), mute, deaf, video)
                    .await
            }
            // It opens once it lists, so it lists as on screen already.
            Request::OpenThread {
                guild,
                id,
                thread,
                beside,
            } => {
                if let Some(thread) = &thread {
                    self.tracker.put_thread(&guild, thread);
                }
                if beside {
                    self.tracker.beside(Some(&id));
                    self.shared.beside = Some((guild, id));
                } else {
                    self.tracker.view(&guild, Some(&id));
                    self.shared.viewing = Some((guild, id));
                }
                self.publisher.changed();
                Ok(())
            }
            Request::Events { guild, events } => {
                if self.tracker.put_events(&guild, events) {
                    self.publisher.changed();
                }
                Ok(())
            }
            Request::Beside { guild, thread } => {
                if self.tracker.beside(thread.as_deref()) {
                    self.publisher.changed();
                }
                self.shared.beside = thread.map(|thread| (guild, thread));
                Ok(())
            }
            Request::Peek { guild } => self.shared.lists.peek(gateway, guild).await,
            Request::View { guild, channel } => {
                self.shared.viewing = channel.clone().map(|channel| (guild.clone(), channel));
                let threads = self.tracker.view(&guild, channel.as_deref());
                let channel = channel.map(|channel| self.tracker.listed_channel(&guild, channel));
                let listed = self
                    .shared
                    .lists
                    .view(&self.tracker, gateway, guild, channel)
                    .await?;
                if listed || threads {
                    self.publisher.changed();
                }
                Ok(())
            }
            Request::MarkRead {
                guild,
                channel,
                sync,
            } => {
                if let Some(message) = self.tracker.unreads.mark_read(&guild, &channel) {
                    self.publisher.changed();
                    if sync {
                        self.acks.read(channel, message);
                    }
                }
                Ok(())
            }
            Request::MarkUnread {
                channel,
                last,
                mentions,
                sync,
            } => {
                self.tracker.unreads.mark_unread(&channel, last, mentions);
                self.publisher.changed();
                // A read on its way would read it again.
                self.acks.forget(&channel);
                if sync {
                    let api = self.shared.api.clone();
                    tokio::spawn(async move {
                        let last = last.to_string();
                        if let Err(error) = api.ack_manual(&channel, &last, mentions).await {
                            tracing::warn!(%error, "could not mark a Discord channel unread");
                        }
                    });
                }
                Ok(())
            }
            Request::GoLive {
                guild,
                channel,
                reply,
            } => {
                self.streams
                    .go_live(gateway, &self.tracker, guild, channel, reply)
                    .await
            }
            Request::Watch { stream_key, reply } => {
                self.streams
                    .watch(gateway, &self.tracker, stream_key, reply)
                    .await
            }
            Request::EndStream { stream_key } => self.streams.end(gateway, &stream_key).await,
            Request::PauseStream { stream_key, paused } => {
                gateway.pause_stream(&stream_key, paused).await
            }
            Request::CustomStatus(custom) => {
                // The settings update that follows says the same; this
                // shows it without waiting.
                self.tracker.custom_status = custom;
                self.publisher.changed();
                self.custom_status_changed();
                self.show_presence().await
            }
            Request::Place {
                guild,
                placements,
                reply,
            } => {
                let was = self.tracker.place(&guild, &placements);
                if let Some(reply) = reply {
                    let _ = reply.send(was);
                }
                self.publisher.changed();
                Ok(())
            }
            Request::Status(status) => {
                self.tracker.status = status;
                self.publisher.changed();
                self.show_presence().await
            }
            Request::Inactive(since, show_idle) => {
                self.shared.inactive = since.map(|since| (since, show_idle));
                self.show_presence().await
            }
            Request::ShowGame(shown) => {
                // As with the status, the settings update that follows agrees.
                self.tracker.show_game = shown;
                self.publisher.changed();
                self.show_presence().await
            }
            Request::ServerList(entries) => {
                self.tracker.set_server_list(&entries);
                self.publisher.changed();
                Ok(())
            }
            Request::Session(reply) => {
                let _ = reply.send(self.tracker.session.clone());
                Ok(())
            }
            Request::Soundboard { guilds } => gateway.request_soundboard_sounds(&guilds).await,
            Request::Search {
                guild,
                query,
                reply,
            } => self.searches.start(gateway, &guild, &query, reply).await,
            Request::SearchPeople { query, reply } => {
                let guilds: Vec<String> = self
                    .tracker
                    .servers
                    .iter()
                    .map(|server| server.id.clone())
                    .filter(|guild| !gateway.members_held(guild))
                    .collect();
                self.searches.people(gateway, &guilds, &query, reply).await
            }
            Request::Members {
                guild,
                users,
                reply,
            } => {
                // Discord answers at most 100 users per request; a reply
                // waits for the first hundred.
                let mut chunks = users.chunks(100);
                if let (Some(reply), Some(users)) = (reply, chunks.next()) {
                    self.searches.members(gateway, &guild, users, reply).await?;
                }
                for users in chunks {
                    gateway.request_members(&guild, users, None).await?;
                }
                Ok(())
            }
        }
    }

    /// Gives an answer only this account sees the server of its channel,
    /// which Discord may leave off it, so it isn't taken for a DM's.
    fn place_ephemeral(&self, dispatch: &mut Dispatch) {
        let data = &mut dispatch.data;
        if !matches!(dispatch.event, Event::MessageCreate | Event::MessageUpdate)
            || !data["guild_id"].is_null()
            || data["flags"].as_u64().unwrap_or_default() & message::EPHEMERAL == 0
        {
            return;
        }
        let channel = data["channel_id"].as_str().unwrap_or_default();
        let guild = self
            .tracker
            .servers
            .iter()
            .find(|server| server.channel(channel).is_some())
            .map(|server| Value::from(server.id.as_str()));
        if let (Some(guild), Some(data)) = (guild, data.as_object_mut()) {
            data.insert("guild_id".into(), guild);
        }
    }

    /// This account's nickname in a server whose GUILD_MEMBER_UPDATE for it
    /// this is, before the update takes it in.
    fn nick_before(&self, event: &Event, data: &Value) -> Option<Option<String>> {
        let me = self.tracker.me.as_str();
        if *event != Event::GuildMemberUpdate
            || me.is_empty()
            || data["user"]["id"].as_str() != Some(me)
        {
            return None;
        }
        let guild = data["guild_id"].as_str()?;
        let names = self
            .shared
            .names
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        Some(names.nickname(guild, me).map(str::to_owned))
    }

    /// Anti Nickname Change: sets this account's nickname back to `before`
    /// when someone else changed it, where it may change its own.
    fn keep_nickname(&self, data: &Value, before: Option<String>) {
        let Some(guild) = data["guild_id"].as_str() else {
            return;
        };
        let roles: Vec<String> = data["roles"]
            .as_array()
            .map(|roles| {
                roles
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        let wanted = ready::permission::CHANGE_NICKNAME | ready::permission::MANAGE_NICKNAMES;
        let may = self
            .tracker
            .servers
            .iter()
            .find(|server| server.id == guild)
            .is_some_and(|server| {
                server.server_permissions(&self.tracker.me, &roles) & wanted != 0
            });
        let after = data["nick"].as_str();
        let Some(nick) = super::keep_nick::restore(guild, before.as_deref(), after, may) else {
            return;
        };
        tracing::info!("Anti Nickname Change: setting this account's nickname back");
        let (api, guild) = (self.shared.api.clone(), guild.to_owned());
        tokio::spawn(async move {
            let nick = nick.unwrap_or_default();
            if let Err(error) = api.set_member_nick(&guild, None, &nick).await {
                super::keep_nick::forget(&guild);
                tracing::warn!(%error, "Anti Nickname Change: could not set the nickname back");
            }
        });
    }

    /// Takes in a dispatched event; an error ends the session.
    async fn event(&mut self, mut dispatch: Dispatch) -> Result<()> {
        self.place_ephemeral(&mut dispatch);
        let (event, data) = (&dispatch.event, &dispatch.data);
        // A server update that no join or stream waits for moves one
        // under way.
        let awaited = match event {
            Event::VoiceServerUpdate => self.join.awaits(data),
            Event::StreamServerUpdate => self.streams.awaits(data),
            _ => true,
        };
        self.join.apply(&self.tracker, &dispatch);
        // Anti Nickname Change: this account's nickname in the server before
        // a change to its member takes it in.
        let nick_before = self.nick_before(event, data);
        // Whether the tracker already took this event in, and changed.
        let mut applied = None;
        match event {
            Event::UserNoteUpdate => {
                if let Some(user) = data["id"].as_str() {
                    super::keep_note(
                        &self.shared.notes,
                        user,
                        data["note"].as_str().unwrap_or_default(),
                    );
                }
            }
            Event::ContentInventoryInboxStale => {
                let _ = self.shared.live.send(Live::ActivityFeedStale {
                    refresh_after_ms: data["refresh_after_ms"].as_u64(),
                });
            }
            Event::PresenceUpdate => self.shared.lists.presence(&self.tracker, data),
            Event::GuildMemberListUpdate => {
                if self.shared.lists.update(&self.tracker, data) {
                    self.publisher.changed();
                }
            }
            Event::GuildMembersChunk | Event::RateLimited => {
                applied = self.searches.answer(&mut self.tracker, &dispatch);
            }
            _ => {}
        }
        // Parsed once, for the timeline and for a notification.
        let message = matches!(event, Event::MessageCreate | Event::MessageUpdate)
            .then(|| message::parse(data))
            .flatten();
        let (me, session) = (&self.tracker.me, self.tracker.session.as_deref());
        for live in live_events(
            me,
            session,
            &self.shared.names,
            event,
            data,
            message.as_ref(),
        ) {
            let _ = self.shared.live.send(live);
        }
        // A message request shows its latest message.
        let previewed = match (event, &message) {
            (Event::MessageCreate | Event::MessageUpdate, Some(message)) => {
                self.tracker.preview(message)
            }
            (Event::MessageDelete, _) => self.tracker.unpreview(
                data["channel_id"].as_str().unwrap_or_default(),
                data["id"].as_str().unwrap_or_default(),
            ),
            _ => false,
        };
        if previewed {
            self.publisher.changed();
        }
        if *event == Event::MessageCreate
            && self.tracker.notifies(data)
            && let Some(message) = message
        {
            let guild = data["guild_id"].as_str().map(str::to_owned);
            let _ = self.shared.live.send(Live::Notify { guild, message });
        }
        if applied.unwrap_or_else(|| self.tracker.apply(event, data)) {
            self.publisher.changed();
            self.ask_previews();
        }
        // A server on screen, back from an outage or joined again, says
        // afresh when its busy voice channels' calls started.
        if *event == Event::GuildCreate
            && data["unavailable"].as_bool() != Some(true)
            && let Some(guild) = data["id"].as_str()
        {
            self.shared
                .lists
                .server_back(&mut self.gateway, guild)
                .await?;
            self.shared
                .lists
                .ask_busy(&self.tracker, &mut self.gateway, guild)
                .await?;
        }
        // Those in this account's call not known in its server: their
        // member tells whether they're bots, which the call's voice
        // effects leave alone.
        if *event == Event::VoiceStateUpdate
            && let Some((guild, users)) = self.tracker.unknown_in_call()
        {
            let users: Vec<String> = users
                .into_iter()
                .filter(|user| self.asked_in_call.insert((guild.clone(), user.clone())))
                .collect();
            for users in users.chunks(100) {
                self.gateway.request_members(&guild, users, None).await?;
            }
        }
        // A call starting in a server not asked yet: its statuses ahead.
        if *event == Event::VoiceStateUpdate
            && let Some(guild) = data["guild_id"].as_str()
        {
            self.shared
                .lists
                .ask_new_call(&self.tracker, &mut self.gateway, guild)
                .await?;
        }
        // Names took the change in, so what shows the member follows; and
        // this account's messages its new display name and avatar.
        if matches!(event, Event::GuildMemberUpdate | Event::GuildMemberRemove) {
            self.shared.lists.member_changed(&self.tracker, data);
            let _ = self.shared.live.send(Live::Names);
        }
        if let Some(before) = nick_before {
            self.keep_nickname(data, before);
        }
        if *event == Event::MessageCreate {
            super::auto_delete::note(data, &self.tracker.me);
        }
        if *event == Event::UserUpdate {
            self.shared.lists.user_changed(&self.tracker, data);
            let _ = self.shared.live.send(Live::Names);
        }
        if !awaited && let Some(moved) = self.tracker.moved(event, data) {
            let _ = self.shared.live.send(moved);
        }
        if *event == Event::UserGuildSettingsUpdate {
            self.mutes_changed();
        }
        if *event == Event::UserSettingsProtoUpdate {
            self.custom_status_changed();
            self.show_presence().await?;
        }
        if *event == Event::UserConnectionsUpdate {
            self.ask_connections();
        }
        self.shared.lists.group_event(&self.tracker, &dispatch);
        if self.streams.waiting() {
            if event.is_stream() {
                self.streams.apply(&self.tracker, &dispatch);
            } else if *event == Event::VoiceStateUpdate {
                // This account's voice session may be what a stream waited for.
                self.streams.voice_changed(&self.tracker);
            }
        }
        Ok(())
    }

    /// Sends the gateway this session's presence when it changed, and
    /// waits for the next time being away changes it.
    /// Asks Discord for the latest message of each message request that
    /// shows none, 25 at a time, as its requests list does.
    fn ask_previews(&mut self) {
        let wanted = self.tracker.previews_wanted();
        if wanted.is_empty() {
            return;
        }
        let (api, found) = (self.shared.api.clone(), self.previews.0.clone());
        tokio::spawn(async move {
            for channels in wanted.chunks(25) {
                match api.request_previews(channels).await {
                    Ok(answer) => {
                        let _ = found.send(answer);
                    }
                    Err(error) => {
                        tracing::warn!(%error, "could not load message requests' messages");
                    }
                }
            }
        });
    }

    async fn show_presence(&mut self) -> Result<()> {
        let (away, next) = Away::after(
            self.shared.inactive.map(|(since, _)| since),
            self.tracker.afk_timeout,
            self.shared.inactive.is_some_and(|(_, show_idle)| show_idle),
            now_ms(),
        );
        self.away_change = next;
        self.away_changes
            .as_mut()
            .reset(Instant::now() + until(next));
        let mut presence = self.tracker.own_presence(away);
        let activity = self.shared.activity.borrow_and_update().clone();
        let activity = activity.filter(|_| self.tracker.show_game);
        self.show_playing(activity.as_deref());
        // An invisible account shares no music, as in Discord's client.
        let music = self.shared.music.borrow_and_update().clone();
        let music = music.filter(|_| self.tracker.status != "invisible");
        crate::own_activity::merge(&mut presence, activity.as_deref(), music.as_deref());
        if self.tracker.shows_here(&presence["activities"]) {
            self.publisher.changed();
        }
        if presence != self.shared.presence {
            self.gateway.update_presence(presence.clone()).await?;
            self.shared
                .lists
                .own_presence(&self.tracker, Presence::parse(&presence));
            self.shared.presence = presence;
        }
        Ok(())
    }

    /// Runs or stops what finds the game as the settings say, and shows the
    /// game played.
    fn show_playing(&mut self, activity: Option<&Value>) {
        let shown = self.tracker.show_game;
        self.shared.show_game.send_if_modified(|old| {
            let changed = *old != shown;
            *old = shown;
            changed
        });
        let playing = activity.and_then(crate::own_activity::Playing::of);
        if self.tracker.playing.as_deref() != playing.as_ref() {
            self.tracker.playing = playing.map(Arc::new);
            self.publisher.changed();
        }
    }

    /// Asks Discord for the account's linked services, which changed. Until
    /// it answers, or if it can't, the links known stay.
    fn ask_connections(&self) {
        let (api, spotify) = (self.shared.api.clone(), self.shared.spotify.clone());
        let me = self.tracker.me.clone();
        tokio::spawn(async move {
            match api.connections().await {
                Ok(connections) => {
                    spotify.send_if_modified(|accounts| accounts.keep(&me, &connections));
                }
                Err(error) => tracing::warn!(%error, "could not load the account's connections"),
            }
        });
    }

    /// Waits for the custom status as it is now to end by itself.
    fn custom_status_changed(&mut self) {
        self.status_ends
            .as_mut()
            .reset(Instant::now() + status_left(&self.tracker));
    }
}

/// A stopped activity bridge must not spin the session loop.
async fn activity_changed(activity: &mut watch::Receiver<Option<Arc<Value>>>) {
    if activity.changed().await.is_ok() {
        return;
    }
    std::future::pending::<()>().await;
}

impl Session<'_> {
    /// Waits for the soonest mute in force now to end. The wait is set
    /// again even for the same end, since one that elapsed would otherwise
    /// end at once each time it's polled.
    fn mutes_changed(&mut self) {
        self.mute_end = self.tracker.unreads.next_mute_end();
        self.mutes_end
            .as_mut()
            .reset(Instant::now() + until(self.mute_end));
    }
}

impl Session<'_> {
    /// Waits for the next thread to go quiet or timeout to end, once a
    /// snapshot is out. The wait is set only when its end moves.
    fn quiets_changed(&mut self) {
        let quiet = self.tracker.next_quiet(now_ms());
        if quiet != self.quiet {
            self.quiet = quiet;
            self.quiets.as_mut().reset(Instant::now() + until(quiet));
        }
    }
}

/// How long until `at`, in Unix milliseconds; long when there's no `at`.
fn until(at: Option<u64>) -> Duration {
    at.map_or(Duration::from_secs(86_400 * 365), |at| {
        Duration::from_millis(at.saturating_sub(now_ms()))
    })
}

/// How long until the custom status ends by itself; long when it doesn't.
fn status_left(tracker: &Tracker) -> Duration {
    tracker
        .custom_status
        .as_ref()
        .and_then(|custom| custom.expires_at_ms)
        .map_or(Duration::from_secs(86_400 * 365), |at| {
            Duration::from_millis(at.saturating_sub(now_ms()))
        })
}

#[cfg(test)]
mod tests {
    use futures_util::{SinkExt, StreamExt};
    use serde_json::json;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
        time::timeout,
    };
    use tokio_tungstenite::{WebSocketStream, accept_async, tungstenite::Message};

    use super::*;
    use crate::spotify::Accounts;

    const WAIT: Duration = Duration::from_secs(10);

    /// The activities of the next presence this session sends Discord.
    async fn sent_activities(peer: &mut WebSocketStream<TcpStream>) -> Value {
        loop {
            let message = timeout(WAIT, peer.next()).await.unwrap().unwrap().unwrap();
            if let Message::Text(text) = message {
                let mut command: Value = serde_json::from_str(text.as_str()).unwrap();
                if command["op"] == 3 {
                    return command["d"]["activities"].take();
                }
            }
        }
    }

    async fn dispatch(
        peer: &mut WebSocketStream<TcpStream>,
        sequence: u64,
        name: &str,
        data: Value,
    ) {
        let dispatch = json!({"op": 0, "s": sequence, "t": name, "d": data});
        peer.send(Message::text(dispatch.to_string()))
            .await
            .unwrap();
    }

    /// Over sockets, from READY's links and the song to what Discord hears.
    #[tokio::test]
    async fn the_song_joins_the_presence_and_the_links_follow_discord() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("ws://{}", listener.local_addr().unwrap());
        let presence = json!({"status": "online", "since": 0, "activities": [], "afk": false});
        let ready = json!({
            "session_id": "mine",
            "user": {"id": "7", "username": "me"},
            "connected_accounts": [{"type": "spotify", "id": "x", "show_activity": true}],
        });
        let (connected, mut peer) =
            tokio::join!(Gateway::connect_at(&url, presence.clone()), async {
                let (stream, _) = listener.accept().await.unwrap();
                let mut peer = accept_async(stream).await.unwrap();
                let hello = json!({"op": 10, "d": {"heartbeat_interval": 45_000}});
                peer.send(Message::text(hello.to_string())).await.unwrap();
                // The identify.
                peer.next().await.unwrap().unwrap();
                dispatch(&mut peer, 1, "READY", ready).await;
                peer
            });
        let (gateway, ready) = connected.unwrap();
        // Where the session asks for the links again.
        let http = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut api = Api::anonymous().unwrap();
        api.base = format!("http://{}", http.local_addr().unwrap());

        let (snapshots, _watcher) = watch::channel(Arc::new(Snapshot::default()));
        let (live, _) = broadcast::channel(16);
        let names = Arc::new(RwLock::new(Names::default()));
        let (_game, activity) = watch::channel(None);
        let (show_game, _shown) = watch::channel(true);
        let (music, music_seen) = watch::channel(None);
        let (spotify, mut accounts) = watch::channel(Accounts::default());
        let mut shared = Shared {
            snapshots,
            live: live.clone(),
            names: names.clone(),
            notes: super::super::Notes::default(),
            api: Arc::new(api),
            lists: MemberLists::new(vec![], live, names, Presence::Online),
            presence,
            activity,
            show_game,
            music: music_seen,
            spotify,
            inactive: None,
            viewing: None,
            beside: None,
        };
        let (requests, mut received) = mpsc::unbounded_channel();
        let session = tokio::spawn(async move {
            connected_session(gateway, ready, &mut received, &mut shared).await;
        });
        timeout(WAIT, accounts.changed()).await.unwrap().unwrap();

        let song = json!({"type": 2, "name": "Spotify", "details": "A Song"});
        music.send_replace(Some(Arc::new(song.clone())));
        let mut sent = sent_activities(&mut peer).await;
        while sent != json!([song]) {
            sent = sent_activities(&mut peer).await;
        }
        // Invisible shares none of it, as Discord's client doesn't.
        requests.send(Request::Status("invisible".into())).unwrap();
        assert_eq!(sent_activities(&mut peer).await, json!([]));
        requests.send(Request::Status("online".into())).unwrap();
        assert_eq!(sent_activities(&mut peer).await, json!([song]));
        // A pause takes the song out at once.
        music.send_replace(None);
        assert_eq!(sent_activities(&mut peer).await, json!([]));

        // The links changed, so Discord is asked which they are: none now.
        dispatch(&mut peer, 2, "USER_CONNECTIONS_UPDATE", json!({})).await;
        let (mut asked, _) = timeout(WAIT, http.accept()).await.unwrap().unwrap();
        let mut line = [0; 4096];
        let got = asked.read(&mut line).await.unwrap();
        assert!(String::from_utf8_lossy(&line[..got]).starts_with("GET /users/@me/connections "));
        asked
            .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\nconnection: close\r\n\r\n[]")
            .await
            .unwrap();
        timeout(WAIT, accounts.changed()).await.unwrap().unwrap();

        session.abort();
    }
}
