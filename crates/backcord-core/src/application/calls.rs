//! Joining and switching calls, and answering them, and the sounds they make.

use super::*;
use crate::chat::MemberChange;
use crate::sounds::Sound;

/// The sound a change in the call makes: this account joining or leaving it,
/// or, while it's there, someone else connecting or going, or starting or
/// stopping a stream. People arriving together make one sound. Reconnecting makes none,
/// and nor do those a DM call is still ringing, until they answer.
pub(super) fn call_sounds(before: &voice::State, after: &voice::State) -> Vec<Effect> {
    let here = |state: &voice::State| matches!(state.phase, Phase::Connected | Phase::Reconnecting);
    let others = |state: &voice::State| -> HashSet<String> {
        state
            .participants
            .iter()
            .filter(|person| !person.local && person.connected)
            .map(|person| person.id.clone())
            .collect()
    };
    let sound =
        if after.phase == Phase::Connected && (!here(before) || before.room != after.room) {
            Some(Sound::Join)
        } else if here(before) && !here(after) {
            Some(Sound::Leave)
        } else if before.phase == Phase::Connected && after.phase == Phase::Connected {
            let (before, after) = (others(before), others(after));
            if !after.is_subset(&before) {
                Some(Sound::Join)
            } else if !before.is_subset(&after) {
                Some(Sound::Leave)
            } else {
                None
            }
        } else {
            None
        }
        .or_else(|| stream_sound(before, after))
        .or_else(|| viewer_sound(before, after));
    sound.map(Effect::Sound).into_iter().collect()
}

/// Someone else already in the call going live, or ending their stream.
/// Those streaming as this account arrives, or as they arrive, make only the
/// sound of joining, and those leaving mid-stream the sound of leaving, which
/// come first.
fn stream_sound(before: &voice::State, after: &voice::State) -> Option<Sound> {
    if before.phase != Phase::Connected
        || after.phase != Phase::Connected
        || before.room != after.room
    {
        return None;
    }
    fn streaming(state: &voice::State) -> HashSet<&str> {
        state
            .participants
            .iter()
            .filter(|person| !person.local && person.connected && person.streaming)
            .map(|person| person.id.as_str())
            .collect()
    }
    let (before, after) = (streaming(before), streaming(after));
    if !after.is_subset(&before) {
        Some(Sound::StreamStart)
    } else if !before.is_subset(&after) {
        Some(Sound::StreamStop)
    } else {
        None
    }
}

/// Someone else starting or stopping watching this account's stream, or the
/// one it watches, while that stream goes on.
fn viewer_sound(before: &voice::State, after: &voice::State) -> Option<Sound> {
    let me = after
        .participants
        .iter()
        .find(|person| person.local)
        .map(|person| person.id.as_str());
    // Each viewer by the stream they watch: this account's, as none, or the
    // streamer's id.
    let viewers = |state: &voice::State| -> HashSet<(Option<String>, String)> {
        let mine = state
            .sharing
            .iter()
            .flat_map(|sharing| sharing.viewers.iter().map(|viewer| (None, viewer)));
        let watched = state.watching.iter().flat_map(|watching| {
            watching
                .viewers
                .iter()
                .map(|viewer| (Some(watching.user.clone()), viewer))
        });
        mine.chain(watched)
            .filter(|(_, viewer)| Some(viewer.as_str()) != me)
            .map(|(stream, viewer)| (stream, viewer.clone()))
            .collect()
    };
    let streams = |state: &voice::State| -> HashSet<Option<String>> {
        let mine = state.sharing.as_ref().map(|_| None);
        let watched = state
            .watching
            .iter()
            .map(|watching| Some(watching.user.clone()));
        mine.into_iter().chain(watched).collect()
    };
    let lasting = streams(before);
    let lasting: HashSet<_> = lasting.intersection(&streams(after)).cloned().collect();
    let (before, after) = (viewers(before), viewers(after));
    let changed = |from: &HashSet<(Option<String>, String)>, to: &HashSet<_>| {
        from.difference(to)
            .any(|(stream, _)| lasting.contains(stream))
    };
    if changed(&after, &before) {
        Some(Sound::ViewerJoin)
    } else if changed(&before, &after) {
        Some(Sound::ViewerLeave)
    } else {
        None
    }
}

impl Application {
    pub(super) fn request_join_voice(&mut self, id: String) -> Vec<Effect> {
        if !self.state.can_join_voice(&id) {
            return vec![];
        }
        if self.state.voice.active() && self.state.voice.room.as_ref() == Some(&id) {
            return vec![];
        }
        if self.state.voice_full(&id) {
            self.state.notice = Some("This voice channel is full.".into());
            return vec![];
        }
        if self.state.voice.active() {
            self.state.switch_voice = Some(id);
            if self.state.skip_switch_prompt {
                return self.confirm_voice_switch();
            }
            vec![]
        } else {
            self.voice_command(voice::Command::Join(id))
        }
    }

    /// Joins a room's call and, once it is connected, turns the camera on,
    /// as Discord's video call button does; in a call already, only the
    /// camera goes on.
    pub(super) fn request_video_call(&mut self, id: String) -> Vec<Effect> {
        if !self.state.can_join_voice(&id) {
            return vec![];
        }
        if self.state.voice.active() && self.state.voice.room.as_ref() == Some(&id) {
            if self.state.voice.camera_phase == voice::CameraPhase::Off {
                return self.voice_command(voice::Command::Video(true));
            }
            return vec![];
        }
        self.state.pending_camera = Some(id.clone());
        self.request_join_voice(id)
    }

    /// Turns the camera on for the call [`AppState::pending_camera`] waits
    /// for, once it connects.
    pub(super) fn film_when_connected(&mut self) -> Vec<Effect> {
        let voice = &self.state.voice;
        let ready = self.state.pending_camera.as_ref().is_some_and(|room| {
            voice.phase == Phase::Connected && voice.room.as_ref() == Some(room)
        });
        if ready {
            self.state.pending_camera = None;
            return self.voice_command(voice::Command::Video(true));
        }
        vec![]
    }

    pub(super) fn accept_call(&mut self, id: String) -> Vec<Effect> {
        if !self.state.can_join_voice(&id) {
            return vec![];
        }
        // What's on screen stays, as in Discord's client: the call shows in
        // the voice dock, and the DM opens from there or from its row.
        let mut effects = vec![];
        if self.state.voice.room.as_ref() != Some(&id) || !self.state.voice.active() {
            if self.state.voice.active() {
                effects.extend(self.voice_command(voice::Command::Leave));
            }
            effects.extend(self.voice_command(voice::Command::Join(id)));
        }
        effects
    }

    /// Anti-Disconnect: joins the Discord server voice channel `room` again,
    /// which the call in it just ended in, while it's still there and this
    /// account may join it; not after the fifth time in a minute.
    pub(super) fn rejoin_voice(&mut self, room: String) -> Vec<Effect> {
        const TIMES: usize = 5;
        const MINUTE: u64 = 60_000;
        let server_channel = self.state.rooms.iter().any(|known| {
            known.id == room && known.voice && !known.direct && known.network == Network::Discord
        });
        if !self.state.anti_disconnect
            || !server_channel
            || !self.state.can_join_voice(&room)
            || self.state.voice_full(&room)
        {
            return vec![];
        }
        let now = crate::clock::now_ms();
        self.rejoins.retain(|at| now.saturating_sub(*at) < MINUTE);
        if self.rejoins.len() >= TIMES {
            return vec![];
        }
        self.rejoins.push(now);
        tracing::info!("Anti-Disconnect: joining the voice channel again");
        self.state.voice.error = None;
        self.voice_command(voice::Command::Join(room))
    }

    /// Auto Unmute: undoes a server mute or deafen of this account in the
    /// Discord server voice channel the lists show it in, where it may mute
    /// or deafen members, once each time it changes; not after the fifth
    /// time in a minute, so a bot that keeps muting it doesn't start a fight.
    pub(super) fn unmute_myself(&mut self) -> Vec<Effect> {
        const TIMES: usize = 5;
        const MINUTE: u64 = 60_000;
        if !self.state.auto_unmute {
            return vec![];
        }
        let Some(me) = self.state.discord_user.as_deref() else {
            return vec![];
        };
        let seen = self
            .state
            .rooms
            .iter()
            .filter(|room| room.voice && !room.direct && room.network == Network::Discord)
            .find_map(|room| {
                let person = room
                    .people
                    .iter()
                    .find(|person| super::rooms::network_id(&person.id) == me)?;
                Some((
                    room.id.clone(),
                    person.id.clone(),
                    person.server_muted,
                    person.server_deafened,
                ))
            });
        let Some((room, user, muted, deafened)) = seen else {
            self.unmute_seen = None;
            return vec![];
        };
        let now_seen = (room.clone(), muted, deafened);
        if self.unmute_seen.as_ref() == Some(&now_seen) {
            return vec![];
        }
        self.unmute_seen = Some(now_seen);
        if !muted && !deafened {
            return vec![];
        }
        let now = crate::clock::now_ms();
        self.unmutes.retain(|at| now.saturating_sub(*at) < MINUTE);
        if self.unmutes.len() >= TIMES {
            return vec![];
        }
        let mut effects = vec![];
        if muted {
            effects.extend(self.change_member(
                room.clone(),
                user.clone(),
                MemberChange::ServerMute(false),
            ));
        }
        if deafened {
            effects.extend(self.change_member(room, user, MemberChange::ServerDeafen(false)));
        }
        if !effects.is_empty() {
            self.unmutes.push(now);
            tracing::info!("Auto Unmute: undoing a server mute or deafen");
        }
        effects
    }

    /// Anti Group Call: a Discord group DM with a call going on, which a
    /// ringing call is too, is left at once: its call, if this account is in
    /// it here, then the group itself, silently, as Leave Group's dialog can.
    /// Each group once; a ring doesn't show, as `discord.rs` leaves it out.
    pub(super) fn leave_group_calls(&mut self) -> Vec<Effect> {
        if !self.state.anti_group_call || !self.state.discord_online {
            return vec![];
        }
        let groups: Vec<String> = self
            .state
            .rooms
            .iter()
            .filter(|room| {
                room.network == Network::Discord
                    && room.direct
                    && room.person.is_none()
                    && !room.request
                    && !room.people.is_empty()
                    && !self.groups_left.contains(&room.id)
            })
            .map(|room| room.id.clone())
            .collect();
        let mut effects = vec![];
        for room in groups {
            if self.state.voice.active() && self.state.voice.room.as_ref() == Some(&room) {
                effects.extend(self.voice_command(voice::Command::Leave));
            }
            tracing::info!("Anti Group Call: leaving a group DM with a call");
            self.groups_left.insert(room.clone());
            effects.push(Effect::Backend(Command::Discord(
                DiscordCommand::LeaveGroup { room, silent: true },
            )));
        }
        effects
    }

    /// Follows `user` into voice, or stops with none; joins where they are
    /// now at once.
    pub(super) fn follow_voice(&mut self, user: Option<String>) -> Vec<Effect> {
        self.state.following = user;
        self.follow_seen = None;
        self.follow_into_voice()
    }

    /// Follow: joins the voice channel the person followed went to, once each
    /// time they go to another: leaving theirs by hand stays left until they
    /// move. Only Discord servers' channels this account may join, which
    /// are those of servers it shares with them.
    pub(super) fn follow_into_voice(&mut self) -> Vec<Effect> {
        let Some(user) = self.state.following.as_deref() else {
            return vec![];
        };
        let user = super::rooms::network_id(user);
        let found = self
            .state
            .rooms
            .iter()
            .find(|room| {
                room.voice
                    && !room.direct
                    && room.network == Network::Discord
                    && room
                        .people
                        .iter()
                        .any(|person| super::rooms::network_id(&person.id) == user)
            })
            .map(|room| room.id.clone());
        if found == self.follow_seen {
            return vec![];
        }
        let Some(room) = found else {
            self.follow_seen = None;
            return vec![];
        };
        let here = self.state.voice.active() && self.state.voice.room.as_ref() == Some(&room);
        if here {
            self.follow_seen = Some(room);
            return vec![];
        }
        // Tried again with the next list while a call starts or ends.
        if self.state.voice_busy()
            || !self.state.can_join_voice(&room)
            || self.state.voice_full(&room)
        {
            return vec![];
        }
        self.follow_seen = Some(room.clone());
        tracing::info!("Follow: joining the voice channel of the person followed");
        let mut effects = if self.state.voice.active() {
            self.voice_command(voice::Command::Leave)
        } else {
            vec![]
        };
        effects.extend(self.voice_command(voice::Command::Join(room)));
        effects
    }

    pub(super) fn confirm_voice_switch(&mut self) -> Vec<Effect> {
        let Some(id) = self.state.switch_voice.take() else {
            return vec![];
        };
        if !self.state.can_join_voice(&id) {
            self.state.pending_watch = None;
            return vec![];
        }
        if self.state.voice_full(&id) {
            self.state.pending_watch = None;
            self.state.notice = Some("This voice channel is full.".into());
            return vec![];
        }
        // Leaving the call it's in keeps the stream it switches to watch.
        let watch = self.state.pending_watch.take();
        let mut effects = if self.state.voice.active() {
            self.voice_command(voice::Command::Leave)
        } else {
            vec![]
        };
        self.state.pending_watch = watch;
        effects.extend(self.voice_command(voice::Command::Join(id)));
        effects.extend(self.watch_when_connected());
        effects
    }

    /// Someone in a voice channel's list was clicked: shows the channel,
    /// and watches them if they share their screen. Watching from outside
    /// the call joins it once asked, as [`AppState::switch_voice`] does, and
    /// [`Self::watch_when_connected`] then watches.
    pub(super) fn show_voice_person(&mut self, room: String, user: String) -> Vec<Effect> {
        let voice = &self.state.voice;
        let here = voice.active() && voice.room.as_ref() == Some(&room);
        let call = voice
            .participants
            .iter()
            .find(|person| here && person.id == user);
        let listed = self
            .state
            .rooms
            .iter()
            .find(|listed| listed.id == room)
            .and_then(|listed| listed.people.iter().find(|person| person.id == user));
        let me = call.is_some_and(|person| person.local)
            || self.state.discord_user.as_ref() == Some(&user);
        let streaming = call.is_some_and(|person| person.streaming)
            || listed.is_some_and(|person| person.streaming);
        let mut effects = self.dispatch(Action::Open(room.clone()));
        if me || !streaming || self.state.selected.as_ref() != Some(&room) {
            return effects;
        }
        if here {
            self.state.pending_watch = Some((room, user));
            effects.extend(self.watch_when_connected());
        } else if self.state.can_join_voice(&room) {
            self.state.switch_voice = Some(room.clone());
            self.state.pending_watch = Some((room, user));
            // From another call, a switch that no longer asks goes at once.
            if self.state.skip_switch_prompt && self.state.voice.active() {
                effects.extend(self.confirm_voice_switch());
            }
        }
        effects
    }

    /// Watches the stream [`AppState::pending_watch`] waits for, once its
    /// call is connected and they show sharing in it.
    pub(super) fn watch_when_connected(&mut self) -> Vec<Effect> {
        let voice = &self.state.voice;
        let ready = self
            .state
            .pending_watch
            .as_ref()
            .is_some_and(|(room, user)| {
                voice.phase == Phase::Connected
                    && voice.room.as_ref() == Some(room)
                    && voice
                        .participants
                        .iter()
                        .any(|person| &person.id == user && person.streaming)
            });
        match self.state.pending_watch.take_if(|_| ready) {
            Some((_, user)) => self.voice_command(voice::Command::Watch(user)),
            None => vec![],
        }
    }

    /// A call command, with the sound of muting or deafening in a call.
    pub(super) fn voice_command(&mut self, command: voice::Command) -> Vec<Effect> {
        let connected = self.state.voice.phase == Phase::Connected;
        let sound = match &command {
            _ if !connected => None,
            voice::Command::Deafen(true) | voice::Command::DeafenUnmuted => Some(Sound::Deafen),
            // Unmuting undeafens too.
            voice::Command::Deafen(false) | voice::Command::Mute(false)
                if self.state.voice.deafened =>
            {
                Some(Sound::Undeafen)
            }
            voice::Command::Mute(true) => Some(Sound::Mute),
            voice::Command::Mute(false) => Some(Sound::Unmute),
            _ => None,
        };
        let mut effects = self.voice_command_quietly(command);
        effects.extend(sound.map(Effect::Sound));
        effects
    }

    pub(super) fn voice_command_quietly(&mut self, command: voice::Command) -> Vec<Effect> {
        if matches!(command, voice::Command::Leave) {
            self.state.switch_voice = None;
            self.state.pending_watch = None;
            self.state.pending_camera = None;
        }
        if !self.state.preview {
            return vec![Effect::Backend(Command::Voice(command))];
        }
        let before = self.state.voice.clone();
        match command {
            voice::Command::Join(id) => {
                if let Some(room) = self
                    .state
                    .rooms
                    .iter()
                    .find(|room| room.id == id && room.voice)
                {
                    self.state.voice = voice::State {
                        phase: Phase::Connected,
                        room: Some(id),
                        name: room.name.clone(),
                        participants: vec![voice::Participant {
                            id: "preview-you".into(),
                            user: "You".into(),
                            person: "preview-you".into(),
                            local: true,
                            muted: true,
                            deafened: false,
                            server_muted: false,
                            server_deafened: false,
                            speaking: false,
                            connected: true,
                            problem: None,
                            avatar: None,
                            streaming: false,
                            video: None,
                        }],
                        can_film: true,
                        ..self.preview_voice()
                    };
                }
            }
            voice::Command::Leave => {
                let filming = self.state.voice.camera_phase != voice::CameraPhase::Off;
                self.state.voice = self.preview_voice();
                if filming {
                    let mut effects = call_sounds(&before, &self.state.voice);
                    effects.push(Effect::PreviewCamera(None));
                    return effects;
                }
            }
            voice::Command::Video(on) if self.state.voice.active() => {
                let feed = on.then(crate::picture::Feed::new);
                for person in &mut self.state.voice.participants {
                    if person.local {
                        person.video.clone_from(&feed);
                    }
                }
                self.state.voice.camera_phase = if on {
                    voice::CameraPhase::On
                } else {
                    voice::CameraPhase::Off
                };
                let camera = self.state.voice.camera.clone();
                return vec![Effect::PreviewCamera(feed.map(|feed| (camera, feed)))];
            }
            voice::Command::Devices => return vec![Effect::ListDevices],
            voice::Command::Microphone(id) => self.state.voice.microphone = Some(id),
            voice::Command::Speaker(id) => self.state.voice.speaker = Some(id),
            voice::Command::Camera(id) => self.state.voice.camera = Some(id),
            voice::Command::CameraQuality(quality) => self.state.voice.camera_quality = quality,
            voice::Command::Mute(muted) => {
                if !muted {
                    self.state.voice.deafened = false;
                }
                self.state.voice.muted = muted;
                self.state.voice.unmute_on_undeafen = false;
            }
            voice::Command::Deafen(false) => {
                let voice = &mut self.state.voice;
                voice.deafened = false;
                // The user's own mute is as it was before deafening.
                if std::mem::take(&mut voice.unmute_on_undeafen) {
                    voice.muted = false;
                }
            }
            command @ (voice::Command::Deafen(true) | voice::Command::DeafenUnmuted) => {
                let voice = &mut self.state.voice;
                voice.unmute_on_undeafen |=
                    !voice.muted || matches!(command, voice::Command::DeafenUnmuted);
                voice.deafened = true;
                voice.muted = true;
            }
            _ => {}
        }
        for participant in &mut self.state.voice.participants {
            participant.muted = self.state.voice.muted;
        }
        call_sounds(&before, &self.state.voice)
    }
}
