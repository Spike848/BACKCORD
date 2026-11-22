//! The call this account is in, wherever the window shows it:
//!
//! - `stage/`, a call's stage: its tiles, the controls under it, a voice
//!   channel's Join;
//! - `dock.rs`, the dock above the account's card: how the call is, its
//!   channel, sharing, and the soundboard, camera and share buttons;
//! - `popups/`, what opens from the controls: the device menus with the
//!   microphone's level, the share menu, the soundboard, someone's volumes;
//! - `details.rs`, the connection in detail from the signal: the ping, its
//!   graph, the loss, the privacy code and the stream's encoding;
//! - `dm.rs`, a DM's call: its invitation above the messages, a call
//!   ringing elsewhere, and the question before switching calls;
//! - `corner.rs`, the stream or camera kept in a corner while the call is
//!   off screen, and the large tile full screen.
//!
//! The tiles are the kit's [`crate::tile`]; a frame is drawn by
//! [`crate::video`].

pub mod corner;
pub mod details;
pub mod dm;
pub mod dock;
pub(crate) mod popups;
pub mod stage;

use std::{cell::Cell, sync::Arc};

use backcord_core::{
    application::Action,
    chat::VoicePerson,
    meter::InputMeter,
    sounds::Sound,
    voice::{self, Command, Phase},
};
use iced::{Point, Rectangle, Task, Vector};

#[cfg(test)]
pub use details::LinkEvent;
pub use details::Tab as LinkTab;
pub use popups::Volumes;
pub use popups::device_rows;
#[cfg(test)]
pub use popups::{Listing, ShareEvent};
pub use stage::{Focus, speaking};

use crate::app::{App, Message};

/// The call's own state in the window: what the stage shows large, its
/// scroll, what's open, the corner picture, the ring.
#[derive(Default)]
pub struct Call {
    pub focus: Option<Focus>,
    /// Whose streams were watched as the window last looked, so a new one
    /// starts large.
    pub watched: Vec<String>,
    /// When each stream's preview was last asked for, by its streamer.
    pub previews_asked: std::collections::HashMap<String, std::time::Instant>,
    /// The large tile over the whole window.
    pub full: bool,
    /// How far the stage is scrolled, and the tile the pointer is on, by its
    /// place.
    pub scrolled: f32,
    pub pointed: Option<usize>,
    /// The popup open from a control, and where that control is.
    pub popup: Option<Popup>,
    pub spot: Cell<Rectangle>,
    /// The device list open in a device menu.
    pub listing: bool,
    /// The share menu's choices, once its sources are listed; while
    /// they're being listed, which listing the menu waits for, by its
    /// count (`listings`), so one answering after a cancel opens nothing.
    pub share: Option<popups::Share>,
    pub share_listing: Option<u64>,
    pub listings: u64,
    /// The corner picture's corner, and how far it's dragged from it.
    pub left: bool,
    pub bottom: bool,
    pub drag: Option<Vector>,
    /// The corner picture is held, and where the pointer first moved.
    pub grab: Option<Option<Point>>,
    /// The call was connected as the window last looked: it unmutes or
    /// deafens on joining once.
    pub ready: bool,
    /// The microphone's level while the input menu shows it, read as
    /// its meter is drawn.
    pub meter: Option<InputMeter>,
    /// A call ringing this account, while it rings.
    pub ringer: Option<backcord_core::ring::Ringer>,
    /// Since when the call has been connected, by its room, and the
    /// dock's channel line under the pointer.
    pub since: Option<(String, std::time::Instant)>,
    pub detail_lit: bool,
    /// The noise suppression the dock's toggle turns back on.
    pub noise_last: backcord_core::noise::Suppressor,
    /// The connection's details, while they or the signal's tooltip show.
    pub details: details::Details,
    /// The window the large tile's full screen put full screen, and
    /// whether the desktop had it so already, in which case leaving keeps
    /// it so.
    pub screen: Option<(iced::window::Id, bool)>,
}

/// What opens from a control, above it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Popup {
    Input,
    Output,
    Share,
    /// What the share button opens while live: another window, the
    /// stream's quality, stopping.
    Live,
    Soundboard,
    /// The connection's details, from the signal.
    Link,
}

/// What's done to the call, or in it.
#[derive(Clone, Debug)]
pub enum Event {
    /// A voice channel's Join, or a DM's call joined.
    Join(String),
    Leave,
    Mute,
    Deafen,
    Camera,
    /// The dock's noise suppression, off or back on.
    Noise,
    StopSharing,
    Watch(String),
    /// Stop watching this person's stream, or every one.
    StopWatching(Option<String>),
    /// Watch this person's stream alone, as Watch Stream does.
    WatchOnly(String),
    /// A watched stream's sound muted or back, by its streamer.
    MuteStream(String),
    /// A watched stream's menu, by its streamer, at the press.
    StreamMenu(String),
    /// A tile shown large, by its place, or none.
    Focus(Option<usize>),
    Full(bool),
    Scrolled(f32),
    Point(usize, bool),
    /// A control's popup, opened above it, or the open one closed.
    Open(Popup),
    Close,
    Share(popups::ShareEvent),
    /// The live stream's quality changed in its menu.
    LiveQuality(backcord_core::voice::StreamQuality),
    /// The device list in a device menu, opened or closed; a device chosen,
    /// the microphone's or the speakers', by its row.
    List,
    Device(bool, usize),
    /// A device menu's volume, the microphone's or the speakers', and
    /// whether it's the drag's last.
    DeviceVolume(bool, u32, bool),
    Suppressor(backcord_core::noise::Suppressor),
    /// A soundboard sound played for everyone, heard here first, or starred.
    Play(String),
    Preview(String),
    Favorite(String, bool),
    /// Someone's voice or stream volume in their menu, and whether it's the
    /// drag's last.
    Volume(bool, u32, bool),
    /// Someone in a call right clicked, by the call's room and their id,
    /// where in the window.
    Person(String, String, Point),
    /// Someone's stream from their LIVE in the sidebar.
    ShowPerson(String, String),
    /// Someone's row in a voice channel clicked: their card, at the
    /// pointer.
    Profile(String),
    /// The call's channel or DM shown, from the dock or the corner; the
    /// dock's line to it under the pointer.
    OpenCall,
    DetailLit(bool),
    /// A second of the call's clock.
    Second,
    /// In the connection's details, or on the signal.
    Link(details::LinkEvent),
    DismissError,
    /// A DM's call answered, or declined.
    Answer(String, bool),
    /// The open DM's call started, with the camera or without.
    Start(bool),
    /// Switching calls confirmed, or not.
    Switch(bool),
    /// The corner picture pressed, the pointer moved to here in the window
    /// while it's held, and let go.
    Grab,
    DragTo(Point),
    Drop,
}

pub fn update(app: &mut App, event: Event) -> Task<Message> {
    let voice = &app.client.state().voice;
    let action = match event {
        Event::Join(room) => Action::RequestJoinVoice(room),
        Event::Leave => Action::Voice(Command::Leave),
        Event::Mute => Action::Voice(Command::Mute(!voice.muted)),
        Event::Deafen => Action::Voice(Command::Deafen(!voice.deafened)),
        // A click while the camera starts cancels it.
        Event::Camera => Action::Voice(Command::Video(
            voice.camera_phase == voice::CameraPhase::Off,
        )),
        Event::Noise => {
            toggle_noise(app);
            return Task::none();
        }
        Event::StopSharing => {
            crate::call::popups::close_popup(app);
            Action::Voice(Command::StopLive)
        }
        Event::LiveQuality(quality) => Action::Voice(Command::StreamQuality(quality)),
        Event::Watch(user) => Action::Voice(Command::Watch(user)),
        Event::StopWatching(user) => Action::Voice(Command::StopWatching(user)),
        Event::Focus(place) => {
            crate::call::stage::focus_tile(app, place);
            return Task::none();
        }
        Event::Full(full) => {
            app.call.full = full && app.call.focus.is_some();
            app.call.pointed = None;
            crate::call::stage::place_videos(app);
            return Task::none();
        }
        Event::Scrolled(scrolled) => {
            app.call.scrolled = scrolled;
            crate::call::stage::place_videos(app);
            return Task::none();
        }
        Event::Point(place, entered) => {
            if entered {
                app.call.pointed = Some(place);
            } else if app.call.pointed == Some(place) {
                app.call.pointed = None;
            }
            return Task::none();
        }
        Event::Open(popup) => return crate::call::popups::open_popup(app, popup),
        Event::Close => {
            crate::call::popups::close_popup(app);
            return Task::none();
        }
        Event::Share(event) => return crate::call::popups::share::share_event(app, event),
        Event::List => {
            app.call.listing = !app.call.listing;
            return Task::none();
        }
        Event::Device(input, index) => {
            app.call.listing = false;
            let devices = if input {
                &voice.microphones
            } else {
                &voice.speakers
            };
            let Some(device) = devices.get(index) else {
                return Task::none();
            };
            let id = device.id.clone();
            if input {
                // The level follows the microphone chosen.
                app.call.meter = Some(InputMeter::start(Some(&id)));
            }
            Action::Voice(if input {
                Command::Microphone(id)
            } else {
                Command::Speaker(id)
            })
        }
        Event::DeviceVolume(input, value, done) => {
            crate::call::popups::device::device_volume(app, input, value, done);
            return Task::none();
        }
        Event::Suppressor(suppressor) => {
            choose_suppressor(app, suppressor);
            return Task::none();
        }
        Event::Play(sound) => Action::Voice(Command::Soundboard(sound)),
        Event::Preview(sound) => Action::Voice(Command::PreviewSound(sound)),
        Event::Favorite(sound, on) => Action::Voice(Command::FavoriteSound(sound, on)),
        Event::Volume(stream, value, done) => {
            crate::call::popups::person::person_volume(app, stream, value, done);
            return Task::none();
        }
        Event::Person(room, user, at) => {
            return crate::call::popups::person::open_voice_menu(app, room, user, at);
        }
        Event::ShowPerson(room, user) => Action::ShowVoicePerson { room, user },
        Event::Profile(user) => {
            let at = app.pressed.get();
            return app.open_card(user, at);
        }
        Event::OpenCall => {
            // The corner's tile shows large in the call it returns to.
            if let Some(place) = crate::call::stage::corner_place(app) {
                crate::call::stage::focus_tile(app, Some(place));
            }
            Action::GoToCall
        }
        Event::DetailLit(lit) => {
            app.call.detail_lit = lit;
            return Task::none();
        }
        Event::Second => {
            crate::call::details::read_link(app);
            return Task::none();
        }
        Event::Link(event) => return crate::call::details::link_event(app, event),
        Event::DismissError => Action::DismissVoiceError,
        Event::Answer(room, true) => Action::AcceptCall(room),
        Event::Answer(room, false) => Action::DeclineCall(room),
        Event::Start(video) => {
            let Some(room) = app.client.state().selected.clone() else {
                return Task::none();
            };
            if video {
                Action::StartVideoCall(room)
            } else {
                Action::RequestJoinVoice(room)
            }
        }
        Event::Switch(true) => Action::ConfirmVoiceSwitch,
        Event::Switch(false) => Action::CancelVoiceSwitch,
        Event::WatchOnly(user) => {
            app.client
                .dispatch(Action::Voice(Command::StopWatching(None)));
            Action::Voice(Command::Watch(user))
        }
        Event::MuteStream(user) => {
            let key = stream_key(app, &user);
            let muted = !backcord_core::volume::stream_muted(&key);
            mute_stream(app, &key, muted);
            return Task::none();
        }
        Event::StreamMenu(user) => return open_stream_menu(app, user),
        Event::Grab => {
            app.call.grab = Some(None);
            return Task::none();
        }
        Event::DragTo(at) => {
            match app.call.grab {
                Some(None) => app.call.grab = Some(Some(at)),
                Some(Some(from)) => {
                    let by = at - from;
                    // A drag starts past 4 px, else it's a click.
                    if app.call.drag.is_some() || by.x.abs() > 4.0 || by.y.abs() > 4.0 {
                        app.call.drag = Some(by);
                    }
                }
                None => {}
            }
            return Task::none();
        }
        Event::Drop => {
            let dragged = app.call.drag.is_some();
            app.call.grab = None;
            if !dragged {
                return update(app, Event::OpenCall);
            }
            crate::call::corner::drop_corner(app);
            return Task::none();
        }
    };
    app.client.dispatch(action);
    app.sync()
}

/// After the core's state changed: a new stream starts large, the call
/// unmutes or deafens as it connects, as the next one should, a call
/// rings while one does, and the frames follow what shows.
pub fn call_sync(app: &mut App) {
    for sound in app.client.take_sounds() {
        play(app, sound);
    }
    ask_previews(app);
    let voice = &app.client.state().voice;
    let watching: Vec<String> = voice
        .watching
        .iter()
        .map(|watching| watching.user.clone())
        .collect();
    if watching != app.call.watched {
        // A stream just begun starts large; one shown large that ended
        // gives the grid back.
        if let Some(user) = watching
            .iter()
            .find(|user| !app.call.watched.contains(user))
        {
            app.call.focus = Some(Focus::Stream(user.clone()));
        } else if matches!(&app.call.focus, Some(Focus::Stream(user)) if !watching.contains(user)) {
            app.call.focus = None;
        }
        app.call.watched = watching;
    }
    // BACKCORD joins calls muted and hearing; these preferences deafen,
    // or turn the microphone on, once connected. Deafened, the microphone
    // waits for the undeafening.
    let ready = voice_ready(voice);
    if ready && !app.call.ready {
        let unmuted = app.layout.unmute_on_join.unwrap_or(true);
        if app.layout.deafen_on_join {
            if !voice.deafened {
                app.client.dispatch(Action::DeafenOnJoin { unmuted });
            }
        } else if unmuted && voice.muted {
            app.client.dispatch(Action::UnmuteOnJoin);
        }
    }
    app.call.ready = ready;
    // The signal goes as the call stops measuring, its touch telling
    // nothing of the pointer leaving it.
    if !crate::call::details::link_measured(app.ctx()) {
        app.call.details.pointed = false;
    }
    let voice = &app.client.state().voice;
    if !voice.active() {
        app.call.full = false;
        if matches!(app.call.popup, Some(Popup::Share | Popup::Soundboard))
            || app.call.share_listing.is_some()
        {
            crate::call::popups::close_popup(app);
        }
    }
    crate::call::stage::keep_focus(app);
    crate::call::dock::time_call(app);
    ring(app);
    crate::call::stage::place_videos(app);
}

/// One of BACKCORD's sounds, as loud as Voice's settings say, unless a
/// call is deafened.
pub(crate) fn play(app: &App, sound: Sound) {
    let voice = &app.client.state().voice;
    if voice.active() && voice.deafened && !matches!(sound, Sound::Deafen | Sound::Undeafen) {
        return;
    }
    backcord_core::sounds::play(
        app.layout.voice.sound_set,
        sound,
        app.layout.voice.sound_gain(sound),
        voice.speaker.as_deref(),
    );
}

/// Out of a call, the account's toggles mute or deafen the next one, or
/// stop doing so, with the sound each makes in a call. Unmuting
/// undeafens, as in a call.
pub fn toggle_on_join(app: &mut App, deafen: bool) {
    let layout = &mut app.layout;
    let sound = if deafen {
        layout.deafen_on_join = !layout.deafen_on_join;
        if layout.deafen_on_join {
            Sound::Deafen
        } else {
            Sound::Undeafen
        }
    } else if layout.deafen_on_join {
        (layout.deafen_on_join, layout.unmute_on_join) = (false, Some(true));
        Sound::Undeafen
    } else {
        let unmute = !layout.unmute_on_join.unwrap_or(true);
        layout.unmute_on_join = Some(unmute);
        if unmute { Sound::Unmute } else { Sound::Mute }
    };
    let root = app.client.root();
    crate::layout::store(root, "deafen_on_join", app.layout.deafen_on_join);
    crate::layout::store(root, "unmute_on_join", app.layout.unmute_on_join);
    play(app, sound);
}

/// The large tile full screen takes the whole screen: the window goes
/// full screen as it shows and back as it goes, by whichever path (the
/// button, a double click, Escape, the tile or the call gone). A window
/// the desktop made full screen before stays so. Run after every update;
/// it asks nothing while neither changed.
pub fn fit_screen(app: &mut App) -> Task<Message> {
    use iced::window;
    let window = app.shell.window;
    let wanted = crate::call::stage::full_shown(app)
        .then_some(window)
        .flatten();
    match (app.call.screen, wanted) {
        (Some((made, _)), Some(id)) if made == id => Task::none(),
        (None, None) => Task::none(),
        (_, Some(id)) => {
            let already = app.shell.full_screen;
            app.call.screen = Some((id, already));
            if already {
                Task::none()
            } else {
                window::set_mode(id, window::Mode::Fullscreen)
            }
        }
        (Some((made, already)), None) => {
            app.call.screen = None;
            if already || window != Some(made) {
                Task::none()
            } else {
                window::set_mode(made, window::Mode::Windowed)
            }
        }
    }
}

/// Noise suppression off, or back on to the kind last used before Off
/// (DeepFilterNet unless another was), saved, as the dock's toggle.
fn toggle_noise(app: &mut App) {
    use backcord_core::noise::Suppressor;
    let voice = &app.layout.voice;
    let next = if voice.noise_suppression == Suppressor::Off {
        app.call.noise_last
    } else {
        Suppressor::Off
    };
    choose_suppressor(app, next);
}

pub(crate) fn choose_suppressor(app: &mut App, suppressor: backcord_core::noise::Suppressor) {
    if suppressor != backcord_core::noise::Suppressor::Off {
        app.call.noise_last = suppressor;
    }
    app.layout.voice.noise_suppression = suppressor;
    backcord_core::noise::set(suppressor);
    crate::layout::store(app.client.root(), "noise_suppression", suppressor);
}

/// A call ringing this account rings until it's answered or stops.
fn ring(app: &mut App) {
    let state = app.client.state();
    if ringing(state).next().is_none() || state.do_not_disturb() {
        app.call.ringer = None;
    } else if app.call.ringer.is_none() {
        app.call.ringer = backcord_core::ring::Ringer::start(
            backcord_core::ring::Tone::Incoming,
            state.voice.speaker.as_deref(),
        )
        .inspect_err(|error| tracing::warn!(%error, "could not ring"))
        .ok();
    }
}

/// The microphone's level is being read.
#[cfg(test)]
pub fn metering(app: &App) -> bool {
    app.call.meter.is_some()
}

/// The call can be spoken in: connected, or getting back to it.
pub fn voice_ready(voice: &voice::State) -> bool {
    matches!(voice.phase, Phase::Connected | Phase::Reconnecting)
}

/// The DMs whose calls ring this account, outside the call it's in.
pub fn ringing(
    state: &backcord_core::application::AppState,
) -> impl Iterator<Item = &backcord_core::chat::ChatRoom> {
    let voice = &state.voice;
    state
        .rooms
        .iter()
        .map(Arc::as_ref)
        .filter(|room| room.incoming && !(voice.active() && voice.room.as_ref() == Some(&room.id)))
}

/// Who is in a call, as a sentence's start: "Ann is", "Ann and Bo are".
pub fn who_is_here(people: &[VoicePerson]) -> String {
    let names: Vec<&str> = people.iter().map(|person| person.name.as_str()).collect();
    match names.as_slice() {
        [one] => format!("{one} is"),
        [first, second] => format!("{first} and {second} are"),
        [first, second, third] => format!("{first}, {second} and {third} are"),
        [first, second, rest @ ..] => format!("{first}, {second} and {} others are", rest.len()),
        [] => String::new(),
    }
}

/// The signal's bars by how good the connection is, as Discord grades it.
pub fn signal_icon(quality: voice::Quality) -> crate::pictures::Icon {
    use crate::pictures::Icon;
    match quality {
        voice::Quality::Bad => Icon::SignalLow,
        voice::Quality::Average => Icon::SignalMedium,
        voice::Quality::Fine | voice::Quality::Unknown => Icon::SignalHigh,
    }
}

/// The signal's colour: grey while unknown.
pub fn signal_ink(theme: &crate::theme::Theme, quality: voice::Quality) -> iced::Color {
    match quality {
        voice::Quality::Bad => theme.danger,
        voice::Quality::Average => theme.accent,
        voice::Quality::Fine => theme.live,
        voice::Quality::Unknown => theme.muted,
    }
}

/// How often a stream's preview is asked for again while it isn't
/// watched, as Discord's client refreshes it.
const PREVIEW_AGAIN: std::time::Duration = std::time::Duration::from_secs(4 * 60);

/// Asks for the previews of the streams in the call not watched, which
/// their tiles show behind Watch Stream.
fn ask_previews(app: &mut App) {
    let voice = &app.client.state().voice;
    let Some(room) = voice.room.clone() else {
        return;
    };
    let due: Vec<String> = voice
        .participants
        .iter()
        .filter(|person| person.streaming && !person.local)
        .filter(|person| {
            !voice
                .watching
                .iter()
                .any(|watching| watching.user == person.id)
        })
        .filter(|person| {
            app.call
                .previews_asked
                .get(&person.id)
                .is_none_or(|at| at.elapsed() >= PREVIEW_AGAIN)
        })
        .map(|person| person.id.clone())
        .collect();
    for user in due {
        app.call
            .previews_asked
            .insert(user.clone(), std::time::Instant::now());
        app.client.dispatch(Action::StreamPreview {
            room: room.clone(),
            user,
        });
    }
}

/// Whose volume a streamer's stream is kept by.
fn stream_key(app: &App, user: &str) -> String {
    app.client
        .state()
        .voice
        .participants
        .iter()
        .find(|person| person.id == user)
        .map_or_else(|| user.to_owned(), |person| person.person.clone())
}

/// Mutes a stream's sound, by whose volume it is, or unmutes it, and keeps
/// that.
pub(crate) fn mute_stream(app: &mut App, key: &str, muted: bool) {
    backcord_core::volume::mute_stream(key, muted);
    let people = &mut app.layout.voice.stream_muted_people;
    if muted {
        people.insert(key.to_owned());
    } else {
        people.remove(key);
    }
    crate::layout::store(app.client.root(), "stream_muted_people", &*people);
}

/// A watched stream's menu, where its "..." was pressed: Stop Watching,
/// Mute, its volume.
fn open_stream_menu(app: &mut App, user: String) -> Task<Message> {
    use crate::controls::Check;
    use crate::menus::{Menu, MenuAct, MenuRow};
    use backcord_iced_kit::i18n::tr;
    let key = stream_key(app, &user);
    let name = app
        .client
        .state()
        .voice
        .participants
        .iter()
        .find(|person| person.id == user)
        .map(|person| person.user.clone())
        .unwrap_or_default();
    let stream = backcord_core::volume::to_slider(backcord_core::volume::get(
        backcord_core::volume::Kind::Stream,
        &key,
    ))
    .round() as u32;
    let rows = vec![
        MenuRow {
            key: user,
            danger: true,
            ..MenuRow::new(
                MenuAct::StopWatchingStream,
                tr("Stop Watching"),
                crate::pictures::Icon::ScreenShareOff,
            )
        },
        MenuRow {
            key: key.clone(),
            check: Check::Box(backcord_core::volume::stream_muted(&key)),
            separated: true,
            ..MenuRow::plain(MenuAct::MuteStream, tr("Mute"))
        },
    ];
    app.win.menu = Some(Menu {
        volumes: Some(Volumes {
            person: key,
            name,
            live: true,
            voice: 0,
            stream,
            only_stream: true,
        }),
        ..Menu::new(app.pressed.get(), rows).width(crate::menus::FIELD_WIDTH)
    });
    iced::widget::operation::focus(crate::app::NOBODY)
}
