//! Voice & video: the microphone, speakers and camera with their volumes
//! and a camera's preview; under the mic test how the microphone is sent
//! (voice activity with the gate's sensitivity, or push to talk with its
//! key and release delay), the soundboard's volume, studio mode, noise
//! suppression and echo reduction; then how calls start, and entrance
//! sounds.
//!
//! While it shows it may run three things, each gone when it's left: the
//! camera being tried, the Discord gate's meter (while the gate's slider
//! shows, or the mic test plays the microphone back through it) and the
//! push-to-talk key's recording.

use backcord_core::{
    application::{Action, AppState},
    keybinds::Shortcut,
    noise::{Strength, Suppressor},
    picture::Feed,
    voice::Command,
};
use iced::{
    Alignment, Color, Element, Length,
    widget::{column, container, row, text},
};

use super::{Drop, Event as SettingsEvent, Page, Settings};
use crate::{
    app::{App, Message},
    call::device_rows,
    controls::{Kind, action, choices, slider},
    ctx::{Cx, Pref as Shown, Request},
    fonts::SEMIBOLD,
    form::{self, switch},
    layout::InputMode,
    pictures::Icon,
    video::Video,
};

/// The pan slider's middle: each of its steps is 10%.
pub(crate) const PAN_MIDDLE: u32 = 10;

/// Widens the microphone or not, and keeps it: Voice's switch and the
/// microphone's menu in a call both set it.
pub(crate) fn microphone_wide(voice: &mut crate::layout::Voice, root: &std::path::Path, on: bool) {
    voice.microphone_wide = on;
    backcord_core::volume::set_microphone_wide(on);
    crate::layout::store(root, "microphone_wide", on);
}

/// Pans the microphone to the pan slider's `value`, kept as it's let go
/// of (`done`): Voice's slider and the microphone's menu both set it.
pub(crate) fn microphone_pan(
    voice: &mut crate::layout::Voice,
    root: &std::path::Path,
    value: u32,
    done: bool,
) {
    let percent = pan_percent(value);
    voice.microphone_pan = (percent != 0).then_some(percent);
    backcord_core::volume::set_microphone_pan(f32::from(percent) / 100.0);
    if done {
        crate::layout::store(root, "microphone_pan", voice.microphone_pan);
    }
}

/// The widener's intensity unless set, in percent.
pub(crate) const DEFAULT_WIDTH: u8 = 75;
/// The intensity slider's steps, each 5%.
pub(crate) const WIDTH_STEPS: u32 = 20;

/// Sets how wide the widener makes the microphone from its slider's
/// `value`, kept as it's let go of (`done`).
pub(crate) fn microphone_width(
    voice: &mut crate::layout::Voice,
    root: &std::path::Path,
    value: u32,
    done: bool,
) {
    let percent = (value.min(WIDTH_STEPS) * 5) as u8;
    voice.microphone_width = (percent != DEFAULT_WIDTH).then_some(percent);
    backcord_core::volume::set_microphone_width(f32::from(percent) / 100.0);
    if done {
        crate::layout::store(root, "microphone_width", voice.microphone_width);
    }
}

/// The widener's intensity, in percent, and its slider's value.
pub(crate) fn width_value(voice: &crate::layout::Voice) -> (u8, u32) {
    let percent = voice.microphone_width.unwrap_or(DEFAULT_WIDTH).min(100);
    (percent, u32::from(percent) / 5)
}

/// The bit depth's slider goes from 1 bit to 32 bit float, as it is.
pub(crate) const FLOAT_BITS: u8 = 32;

/// Sets the bit depth from its slider's `value`, 0 for 1 bit, kept as it's
/// let go of (`done`).
pub(crate) fn microphone_bits(
    voice: &mut crate::layout::Voice,
    root: &std::path::Path,
    value: u32,
    done: bool,
) {
    let bits = (value.min(u32::from(FLOAT_BITS) - 1) + 1) as u8;
    voice.microphone_bits = (bits != FLOAT_BITS).then_some(bits);
    backcord_core::volume::set_microphone_bits(u32::from(bits));
    if done {
        crate::layout::store(root, "microphone_bits", voice.microphone_bits);
    }
}

/// The bit depth's slider: its value, its top, and what it says.
pub(crate) fn bits_slider(voice: &crate::layout::Voice) -> (u32, u32, String) {
    let bits = voice
        .microphone_bits
        .unwrap_or(FLOAT_BITS)
        .clamp(1, FLOAT_BITS);
    let shown = if bits == FLOAT_BITS {
        backcord_iced_kit::i18n::tr("32 bit float").to_owned()
    } else {
        format!("{bits} bit")
    };
    (u32::from(bits) - 1, u32::from(FLOAT_BITS) - 1, shown)
}

/// The sample rate slider's stops, lowest first: the last is as it is.
pub(crate) const RATES: [u32; 10] = [
    2_000, 4_000, 6_000, 8_000, 11_025, 16_000, 22_050, 24_000, 32_000, 48_000,
];
pub(crate) const FULL_RATE: u32 = 48_000;

/// Sets the sample rate from its slider's `value`, an index into
/// [`RATES`], kept as it's let go of (`done`).
pub(crate) fn microphone_rate(
    voice: &mut crate::layout::Voice,
    root: &std::path::Path,
    value: u32,
    done: bool,
) {
    let rate = RATES[(value as usize).min(RATES.len() - 1)];
    voice.microphone_rate = (rate != FULL_RATE).then_some(rate);
    backcord_core::volume::set_microphone_sample_rate(rate);
    if done {
        crate::layout::store(root, "microphone_rate", voice.microphone_rate);
    }
}

/// The sample rate's slider: its value, its top, and what it says.
pub(crate) fn rate_slider(voice: &crate::layout::Voice) -> (u32, u32, String) {
    let rate = voice.microphone_rate.unwrap_or(FULL_RATE);
    let index = RATES
        .iter()
        .position(|&stop| stop >= rate)
        .unwrap_or(RATES.len() - 1);
    let shown = RATES[index];
    let shown = if shown.is_multiple_of(1_000) {
        format!("{} kHz", shown / 1_000)
    } else {
        format!("{:.1} kHz", shown as f32 / 1_000.0)
    };
    (index as u32, (RATES.len() - 1) as u32, shown)
}

/// Sets the microphone's pitch from its slider's `value`, a semitone a
/// step from [`PITCH_OFF`](crate::call::popups::person::PITCH_OFF), kept
/// as it's let go of (`done`): as someone's Pitch, for this account.
pub(crate) fn microphone_pitch(
    voice: &mut crate::layout::Voice,
    root: &std::path::Path,
    value: u32,
    done: bool,
) {
    use crate::call::popups::person::PITCH_OFF;
    let semitones = (value.min(2 * PITCH_OFF) as i32 - PITCH_OFF as i32) as i8;
    voice.microphone_pitch = (semitones != 0).then_some(semitones);
    backcord_core::volume::set_microphone_pitch(f32::from(semitones));
    if done {
        crate::layout::store(root, "microphone_pitch", voice.microphone_pitch);
    }
}

/// The microphone's pitch slider: its value, its top, and what it says.
pub(crate) fn pitch_slider(voice: &crate::layout::Voice) -> (u32, u32, String) {
    use crate::call::popups::person::PITCH_OFF;
    let semitones =
        i32::from(voice.microphone_pitch.unwrap_or(0)).clamp(-(PITCH_OFF as i32), PITCH_OFF as i32);
    let shown = if semitones == 0 {
        backcord_iced_kit::i18n::tr("Off").to_owned()
    } else {
        format!("{semitones:+}")
    };
    ((PITCH_OFF as i32 + semitones) as u32, 2 * PITCH_OFF, shown)
}

/// The pan's percent, -100 to 100, for its slider's value.
fn pan_percent(value: u32) -> i8 {
    ((i64::from(value.min(PAN_MIDDLE * 2)) - i64::from(PAN_MIDDLE)) * 10) as i8
}

/// The pan slider's value for its percent.
pub(crate) fn pan_value(percent: i8) -> u32 {
    (i64::from(PAN_MIDDLE) + i64::from(percent.clamp(-100, 100)) / 10) as u32
}

/// What the pan slider says: the middle, or how far to a side.
pub(crate) fn pan_shown(percent: i8) -> String {
    match percent {
        0 => backcord_iced_kit::i18n::tr("Centre").to_owned(),
        p if p < 0 => backcord_iced_kit::i18n::t(format!("Left {}%", -i16::from(p))),
        p => backcord_iced_kit::i18n::t(format!("Right {p}%")),
    }
}

/// The camera's preview: its key among the window's video sources, and the
/// size it's filmed at.
pub const CAMERA_TEST: &str = "camera:settings";
const PREVIEW: (u32, u32) = (400, 225);
/// How often the gate's meter is drawn anew while it shows.
const METER_EVERY: std::time::Duration = std::time::Duration::from_millis(50);

const SUPPRESSORS: [Suppressor; 3] = [
    Suppressor::DeepFilterNet,
    Suppressor::RNNoise,
    Suppressor::Off,
];
const STRENGTHS: [Strength; 4] = [
    Strength::Max,
    Strength::Full,
    Strength::Medium,
    Strength::Light,
];

/// What Voice runs while it shows.
#[derive(Default)]
pub struct Voice {
    /// The camera being tried, and why it couldn't start.
    pub camera: Option<(backcord_core::camera::Camera, Feed)>,
    pub camera_note: String,
    /// The Discord gate's meter while its slider shows or the mic test
    /// runs, read as it's drawn.
    pub gate: Option<backcord_core::discord_audio::GateMeter>,
    /// The mic test: the microphone, as a call's processing leaves it,
    /// played back through the speakers.
    pub testing: bool,
    /// The test deafened the call, to undeafen as the test ends.
    pub test_deafened: bool,
    /// The call as the test last saw it: going on, and deafened.
    pub test_saw: (bool, bool),
    /// How many times the camera being tried started or stopped.
    pub tries: u32,
    /// The soundboard's volume while its slider is dragged.
    pub soundboard: Option<u32>,
    /// Entrance sounds: the server chosen ("All servers" first), and the
    /// sounds to choose from while that list is open.
    pub entrance_server: usize,
}

#[derive(Clone, Debug)]
pub enum Event {
    Device(Drop, usize),
    Refresh,
    TestCamera,
    MicTest,
    Volume(bool, u32, bool),
    /// The echo reduction chosen: off, normal or strong.
    Echo(usize),
    /// The microphone widened, or not.
    MicWide(bool),
    /// The pan's slider: 0 the left ear, [`PAN_MIDDLE`] the middle, twice
    /// that the right; and whether it's let go of.
    MicPan(u32, bool),
    /// The widener's intensity slider, in 5% steps; and whether it's let go of.
    MicWidth(u32, bool),
    /// The bit depth's slider, 0 for 1 bit; and whether it's let go of.
    MicBits(u32, bool),
    /// The sample rate's slider, an index into [`RATES`]; and whether it's
    /// let go of.
    MicRate(u32, bool),
    /// The microphone's pitch slider, a semitone a step; and whether it's
    /// let go of.
    MicPitch(u32, bool),
    Studio(bool),
    Noise(usize),
    Strength(usize),
    InputMode(usize),
    Sensitivity(u32, bool),
    /// The voice limiter's slider: 0 for -40 dB up to 40 for off, and
    /// whether it's let go of.
    VoiceLimit(u32, bool),
    VoiceCompressor(bool),
    /// The compressor's choice: 0 off, then Light, Medium and Strong.
    VoiceCompressorPreset(usize),
    VoiceHighPass(bool),
    /// The high-pass's choice: 0 off, then each of
    /// [`HIGH_PASS_CUTOFFS`](backcord_core::voice_filters::HIGH_PASS_CUTOFFS).
    VoiceHighPassCut(usize),
    /// The gate's slider: 0 for off, then -70 dB up to -20, and whether
    /// it's let go of.
    VoiceGate(u32, bool),
    TalkDelay(u32, bool),
    SoundboardVolume(u32, bool),
    EntranceServer(usize),
    Entrance(Option<String>),
}

impl Settings {
    /// Voice shows: the devices are listed again, the entrance sounds
    /// asked for, and the gate's meter starts if its slider shows.
    pub(super) fn enter_voice(&mut self, cx: &mut Cx<'_>) {
        cx.dispatch(Action::Voice(Command::Devices));
        if super::discord_linked(cx.state()) {
            cx.dispatch(Action::LoadSoundCatalog);
        }
        self.meter_gate(cx);
    }

    /// The gate's meter runs while its slider shows (on Voice, with
    /// Discord linked, in voice activity) or the mic test runs.
    pub(super) fn meter_gate(&mut self, cx: &Cx<'_>) {
        let slider = super::discord_linked(cx.state())
            && cx.layout.voice.input_mode == InputMode::VoiceActivity;
        let wanted = (slider || self.voice.testing) && self.section() == super::Section::Voice;
        match (wanted, self.voice.gate.is_some()) {
            (true, false) => {
                let voice = &cx.state().voice;
                self.voice.gate = Some(backcord_core::discord_audio::GateMeter::start(
                    voice.microphone.as_deref(),
                    voice.speaker.as_deref(),
                    self.voice.testing,
                ));
            }
            (false, true) => self.voice.gate = None,
            _ => {}
        }
    }

    /// Starts the mic test, as Discord's does: a call going on is
    /// deafened (and so muted) while it runs, unless it already is.
    fn start_mic_test(&mut self, cx: &mut Cx<'_>) {
        let voice = &cx.state().voice;
        self.voice.test_saw = (voice.active(), voice.deafened);
        if voice.active() && !voice.deafened {
            self.voice.test_deafened = true;
            cx.dispatch(Action::Voice(Command::Deafen(true)));
        }
        self.voice.testing = true;
        // Started again, to play back.
        self.voice.gate = None;
        self.meter_gate(cx);
    }

    /// Ends the mic test, if it runs: the call it deafened hears again,
    /// muted as it was, as undeafening leaves it. The meter goes;
    /// [`Self::meter_gate`] starts it again where its slider still shows.
    pub(super) fn end_mic_test(&mut self, cx: &mut Cx<'_>) {
        if !std::mem::take(&mut self.voice.testing) {
            return;
        }
        self.voice.gate = None;
        // Whether or not the backend has said the call is deafened yet.
        if std::mem::take(&mut self.voice.test_deafened) && cx.state().voice.active() {
            cx.dispatch(Action::Voice(Command::Deafen(false)));
        }
    }

    /// The call changed: undeafened, or joined undeafened, while the mic
    /// test runs, it ends, as Discord's does, rather than play the
    /// microphone back into a call that hears. Only a change ends it: the
    /// deafening the test asked for comes back from the backend later.
    pub fn voice_sync(&mut self, cx: &mut Cx<'_>) {
        if !self.voice.testing {
            return;
        }
        let voice = &cx.state().voice;
        let now = (voice.active(), voice.deafened);
        let before = std::mem::replace(&mut self.voice.test_saw, now);
        let hears = now == (true, false);
        if hears && (before == (true, true) || !before.0) {
            // Undeafened by hand: nothing to put back.
            self.voice.test_deafened = false;
            self.end_mic_test(cx);
            self.meter_gate(cx);
            cx.touch();
        }
    }

    /// The camera being tried, for the window's video sources, and how
    /// many times it started or stopped, so the root places the videos
    /// again as it does.
    pub fn camera_test(&self) -> (Option<&Feed>, u32) {
        let feed = self.voice.camera.as_ref().map(|(_, feed)| feed);
        (feed, self.voice.tries)
    }

    pub(super) fn voice_event(&mut self, event: Event, cx: &mut Cx<'_>) {
        let root = cx.client.root().to_owned();
        let voice = &mut cx.layout.voice;
        match event {
            Event::Device(drop, index) => {
                self.open = None;
                let state = &cx.state().voice;
                let devices = match drop {
                    Drop::Microphone => &state.microphones,
                    Drop::Speaker => &state.speakers,
                    _ => &state.cameras,
                };
                let Some(id) = devices.get(index).map(|device| device.id.clone()) else {
                    return;
                };
                let command = match drop {
                    Drop::Microphone => Command::Microphone(id),
                    Drop::Speaker => Command::Speaker(id),
                    _ => Command::Camera(id),
                };
                cx.dispatch(Action::Voice(command));
                // The meter follows the microphone chosen, and the mic
                // test the speakers too; the camera being tried the
                // camera.
                let restart = match drop {
                    Drop::Microphone => self.voice.gate.is_some(),
                    Drop::Speaker => self.voice.testing,
                    _ => false,
                };
                if restart {
                    self.voice.gate = None;
                    self.meter_gate(cx);
                }
                if drop == Drop::Camera && self.voice.camera.is_some() {
                    self.voice.camera = None;
                    self.test_camera(cx);
                }
            }
            Event::Refresh => cx.dispatch(Action::Voice(Command::Devices)),
            Event::MicTest => {
                if self.voice.testing {
                    self.end_mic_test(cx);
                    self.meter_gate(cx);
                } else {
                    self.start_mic_test(cx);
                }
            }
            Event::TestCamera => {
                let testing = self.voice.camera.is_some();
                self.voice.camera = None;
                self.voice.camera_note.clear();
                self.voice.tries += 1;
                if !testing {
                    self.test_camera(cx);
                }
                return;
            }
            Event::Volume(input, value, done) => {
                // Both go to 400%, the speakers' as the microphone's.
                let value = value.min(backcord_core::volume::MOST) as u16;
                let (kept, key) = if input {
                    (&mut voice.microphone_volume, "microphone_volume")
                } else {
                    (&mut voice.speaker_volume, "speaker_volume")
                };
                *kept = Some(value);
                voice.play_volumes();
                if done {
                    crate::layout::store(&root, key, value);
                }
                return;
            }
            Event::VoiceLimit(value, done) => {
                let db = limit_db(value);
                voice.voice_limit = (db < 0).then_some(db);
                backcord_core::volume::set_voice_limit(f32::from(db));
                if done {
                    crate::layout::store(&root, "voice_limit", voice.voice_limit);
                }
                return;
            }
            Event::VoiceHighPassCut(index) => {
                use backcord_core::voice_filters::HIGH_PASS_CUTOFFS;
                let on = index > 0;
                voice.voice_high_pass = on;
                backcord_core::volume::set_voice_high_pass(on);
                crate::layout::store(&root, "voice_high_pass", on);
                if on {
                    let hz = HIGH_PASS_CUTOFFS[(index - 1).min(HIGH_PASS_CUTOFFS.len() - 1)];
                    voice.voice_high_pass_cutoff = (hz != 100).then_some(hz);
                    backcord_core::volume::set_voice_high_pass_cutoff(f32::from(hz));
                    crate::layout::store(
                        &root,
                        "voice_high_pass_cutoff",
                        voice.voice_high_pass_cutoff,
                    );
                }
            }
            Event::VoiceHighPass(on) => {
                voice.voice_high_pass = on;
                backcord_core::volume::set_voice_high_pass(on);
                crate::layout::store(&root, "voice_high_pass", on);
            }
            Event::VoiceGate(value, done) => {
                voice.voice_gate = gate_db(value);
                backcord_core::volume::set_voice_gate(f32::from(voice.voice_gate.unwrap_or(0)));
                if done {
                    crate::layout::store(&root, "voice_gate", voice.voice_gate);
                }
                return;
            }
            Event::VoiceCompressorPreset(index) => {
                let on = index > 0;
                voice.voice_compressor = on;
                backcord_core::volume::set_voice_compressor(on);
                crate::layout::store(&root, "voice_compressor", on);
                if on {
                    let preset = (index - 1).min(2);
                    voice.voice_compressor_preset = (preset != 1).then_some(preset as u8);
                    backcord_core::volume::set_voice_compressor_preset(
                        backcord_core::voice_compressor::Preset::ALL[preset],
                    );
                    crate::layout::store(
                        &root,
                        "voice_compressor_preset",
                        voice.voice_compressor_preset,
                    );
                }
            }
            Event::VoiceCompressor(on) => {
                voice.voice_compressor = on;
                backcord_core::volume::set_voice_compressor(on);
                crate::layout::store(&root, "voice_compressor", on);
            }
            Event::MicWide(on) => microphone_wide(voice, &root, on),
            Event::MicPan(value, done) => {
                microphone_pan(voice, &root, value, done);
                return;
            }
            Event::MicWidth(value, done) => {
                microphone_width(voice, &root, value, done);
                return;
            }
            Event::MicBits(value, done) => {
                microphone_bits(voice, &root, value, done);
                return;
            }
            Event::MicRate(value, done) => {
                microphone_rate(voice, &root, value, done);
                return;
            }
            Event::MicPitch(value, done) => {
                microphone_pitch(voice, &root, value, done);
                return;
            }
            Event::Echo(level) => {
                let level = level.min(2) as u8;
                voice.echo_reduction = Some(level);
                backcord_core::noise::set_echo(level);
                crate::layout::store(&root, "echo_reduction", level);
            }
            Event::Studio(on) => {
                voice.studio_voice = on;
                backcord_core::studio::set(on);
                crate::layout::store(&root, "studio_voice", on);
            }
            Event::Noise(index) => {
                let suppressor = SUPPRESSORS[index.min(2)];
                voice.noise_suppression = suppressor;
                backcord_core::noise::set(suppressor);
                crate::layout::store(&root, "noise_suppression", suppressor);
                cx.request(Request::Changed(Shown::Noise));
            }
            Event::Strength(index) => {
                voice.noise_strength = STRENGTHS[index.min(STRENGTHS.len() - 1)];
                backcord_core::noise::set_strength(voice.noise_strength);
                crate::layout::store(&root, "noise_strength", voice.noise_strength);
            }
            Event::InputMode(index) => {
                voice.input_mode = if index == 1 {
                    InputMode::PushToTalk
                } else {
                    InputMode::VoiceActivity
                };
                backcord_core::push_to_talk::set_on(voice.input_mode == InputMode::PushToTalk);
                crate::layout::store(&root, "input_mode", voice.input_mode);
                cx.request(Request::Changed(Shown::Keybinds));
            }
            Event::Sensitivity(value, done) => {
                let decibels = value as i32 - 80;
                voice.input_sensitivity = Some(decibels);
                backcord_core::noise::set_sensitivity(decibels as f32);
                if done {
                    crate::layout::store(&root, "input_sensitivity", decibels);
                }
                return;
            }
            Event::TalkDelay(value, done) => {
                let delay = value.min(backcord_core::push_to_talk::MOST_RELEASE_DELAY);
                voice.push_to_talk_release_delay = Some(delay);
                backcord_core::push_to_talk::set_release_delay(delay);
                if done {
                    crate::layout::store(&root, "push_to_talk_release_delay", delay);
                }
                return;
            }
            Event::SoundboardVolume(value, done) => {
                if !done {
                    self.voice.soundboard = Some(value.min(100));
                    return;
                }
                // Where the drag or the keys left it; the core puts it on
                // the slider's curve.
                let value = self.voice.soundboard.take().unwrap_or(value).min(100);
                cx.dispatch(Action::SoundboardVolume(value as f32));
            }
            Event::EntranceServer(index) => {
                self.voice.entrance_server = index;
                self.open = None;
                return;
            }
            Event::Entrance(sound) => {
                let server = entrance_servers(cx.state())
                    .get(self.voice.entrance_server)
                    .cloned()
                    .flatten();
                self.open = None;
                cx.dispatch(Action::SetJoinSound { server, sound });
            }
        }
        cx.touch();
    }

    /// Starts trying the camera chosen, or says why it can't.
    fn test_camera(&mut self, cx: &Cx<'_>) {
        let camera = cx.state().voice.camera.as_deref();
        // In the window's pixels.
        let scale = cx.scale;
        let feed = Feed::new();
        feed.show_at(Some((
            (PREVIEW.0 as f32 * scale).round() as u32,
            (PREVIEW.1 as f32 * scale).round() as u32,
        )));
        self.voice.tries += 1;
        match backcord_core::camera::Camera::open(camera, Box::new(feed.sink())) {
            Ok(camera) => self.voice.camera = Some((camera, feed)),
            Err(error) => self.voice.camera_note = error.to_string(),
        }
    }
}

impl<'a> Page<'a> {
    pub(super) fn voice_page(self) -> Element<'a, Message> {
        let settings = self.s;
        let theme = self.cx.look.theme;
        let state = &self.cx.state.voice;
        let voice = &self.cx.layout.voice;
        let open = settings.open;
        let linked = self.linked();
        let (microphones, microphone) =
            device_rows(&state.microphones, state.microphone.as_deref());
        let (speakers, speaker) = device_rows(&state.speakers, state.speaker.as_deref());
        let (cameras, camera) = device_rows(&state.cameras, state.camera.as_deref());
        let device = |drop: Drop, options: Vec<String>, current: Option<usize>| {
            form::dropdown(
                options,
                current,
                open == Some(drop),
                Message::Settings(SettingsEvent::Drop(drop)),
                move |index| voice_message(Event::Device(drop, index)),
            )
            .view(self.cx.look)
        };
        let mic_volume = u32::from(voice.microphone_volume.unwrap_or(100));
        let speaker_volume = u32::from(voice.speaker_volume.unwrap_or(100));
        let limit = voice
            .voice_limit
            .unwrap_or(0)
            .clamp(-(LIMIT_STEPS as i8), 0);
        let limit_value = (i32::from(limit) + LIMIT_STEPS as i32) as u32;
        let limit_shown = if limit < 0 {
            format!("{limit} dB")
        } else {
            backcord_iced_kit::i18n::tr("Off").to_owned()
        };
        let compressor = if voice.voice_compressor {
            1 + usize::from(voice.voice_compressor_preset.unwrap_or(1)).min(2)
        } else {
            0
        };
        let high_pass = if voice.voice_high_pass {
            let hz = voice.voice_high_pass_cutoff.unwrap_or(100);
            1 + backcord_core::voice_filters::HIGH_PASS_CUTOFFS
                .iter()
                .position(|&cut| cut == hz)
                .unwrap_or(0)
        } else {
            0
        };
        let gate_value = gate_value(voice.voice_gate);
        let gate_shown = match voice.voice_gate {
            Some(db) => format!("{db} dB"),
            None => backcord_iced_kit::i18n::tr("Off").to_owned(),
        };
        // Beside the speakers, the microphone: its device and volume, how
        // it sends, and how Discord calls process it.
        let push = voice.input_mode == InputMode::PushToTalk;
        let mut microphone_side = vec![
            form::group([
                form::heading(theme, backcord_iced_kit::i18n::tr("MICROPHONE")),
                device(Drop::Microphone, microphones, microphone),
                slider(
                    backcord_iced_kit::i18n::tr("Microphone volume"),
                    mic_volume,
                    backcord_core::volume::MOST,
                    |value| voice_message(Event::Volume(true, value, false)),
                )
                .on_release(voice_message(Event::Volume(true, mic_volume, true)))
                .typed(|value| voice_message(Event::Volume(true, value, true)))
                .view(theme),
            ]),
            form::group([
                form::heading(theme, backcord_iced_kit::i18n::tr("INPUT MODE")),
                choices(
                    theme,
                    &[
                        backcord_iced_kit::i18n::tr("Voice activity"),
                        backcord_iced_kit::i18n::tr("Push to talk"),
                    ],
                    usize::from(push),
                    |index| voice_message(Event::InputMode(index)),
                ),
                form::note(
                    theme,
                    if push {
                        backcord_iced_kit::i18n::tr(
                            "BACKCORD sends your microphone only while you hold a key.",
                        )
                    } else {
                        backcord_iced_kit::i18n::tr(
                            "BACKCORD sends your voice when it hears you speak.",
                        )
                    },
                ),
            ]),
        ];
        if linked {
            microphone_side.extend(self.discord_processing());
        }
        let mut parts = vec![
            form::title(theme, backcord_iced_kit::i18n::tr("Voice & video")),
            form::halves(
                form::page(microphone_side),
                form::group([
                    form::heading(theme, backcord_iced_kit::i18n::tr("SPEAKERS")),
                    device(Drop::Speaker, speakers, speaker),
                    slider(
                        backcord_iced_kit::i18n::tr("Speaker volume"),
                        speaker_volume,
                        backcord_core::volume::MOST,
                        |value| voice_message(Event::Volume(false, value, false)),
                    )
                    .on_release(voice_message(Event::Volume(false, speaker_volume, true)))
                    .typed(|value| voice_message(Event::Volume(false, value, true)))
                    .view(theme),
                    slider(
                        backcord_iced_kit::i18n::tr("Voice limiter"),
                        limit_value,
                        LIMIT_STEPS,
                        |value| voice_message(Event::VoiceLimit(value, false)),
                    )
                    .step(1)
                    .mark(false)
                    .shown(limit_shown)
                    .on_release(voice_message(Event::VoiceLimit(limit_value, true)))
                    .view(theme),
                    form::note(
                        theme,
                        backcord_iced_kit::i18n::tr(
                            "Holds each person's voice under this level, so nobody plays louder however loud they send. Streams, the soundboard and other sounds aren't limited.",
                        ),
                    ),
                    column![
                        form::heading(theme, backcord_iced_kit::i18n::tr("HIGH-PASS FILTER")),
                        choices(
                            theme,
                            &[
                                backcord_iced_kit::i18n::tr("Off"),
                                "100 Hz",
                                "150 Hz",
                                "200 Hz",
                            ],
                            high_pass,
                            |index| voice_message(Event::VoiceHighPassCut(index)),
                        ),
                        form::note(
                            theme,
                            backcord_iced_kit::i18n::tr(
                                "Takes what's under the frequency out of the others' voices: 100 Hz rumble and hum, 150 Hz also the pops and boom of a close microphone, 200 Hz a thinner voice, like a radio.",
                            ),
                        ),
                    ]
                    .spacing(8)
                    .into(),
                    slider(
                        backcord_iced_kit::i18n::tr("Voice gate"),
                        gate_value,
                        GATE_STEPS,
                        |value| voice_message(Event::VoiceGate(value, false)),
                    )
                    .step(1)
                    .mark(false)
                    .shown(gate_shown)
                    .on_release(voice_message(Event::VoiceGate(gate_value, true)))
                    .view(theme),
                    form::note(
                        theme,
                        backcord_iced_kit::i18n::tr(
                            "Silences each person's voice while it stays under this level, so the noise in their pauses doesn't play.",
                        ),
                    ),
                    column![
                        form::heading(theme, backcord_iced_kit::i18n::tr("VOICE COMPRESSOR")),
                        choices(
                            theme,
                            &[
                                backcord_iced_kit::i18n::tr("Off"),
                                backcord_iced_kit::i18n::tr("Light"),
                                backcord_iced_kit::i18n::tr("Medium"),
                                backcord_iced_kit::i18n::tr("Strong"),
                            ],
                            compressor,
                            |index| voice_message(Event::VoiceCompressorPreset(index)),
                        ),
                        form::note(
                            theme,
                            backcord_iced_kit::i18n::tr(
                                "Evens out each person's voice, so loud and quiet people sound closer. Light (2:1) only smooths the peaks; Medium (4:1) brings voices close; Strong (8:1) makes everyone sound about as loud.",
                            ),
                        ),
                    ]
                    .spacing(8)
                    .into(),
                    form::note(
                        theme,
                        backcord_iced_kit::i18n::tr(
                            "Bots aren't touched by any of these. Right-click someone in a call to bypass all but the limiter for them.",
                        ),
                    ),
                ]),
            ),
        ];
        if state.microphones.is_empty() && state.speakers.is_empty() {
            parts.push(form::note(
                theme,
                backcord_iced_kit::i18n::tr("No audio devices found. Connect one, then refresh."),
            ));
        }
        if backcord_core::discord_audio::AVAILABLE {
            parts.push(self.mic_test());
        }
        // Under the test: push to talk's keys or the sensitivity, then the
        // soundboard.
        if push {
            parts.extend(self.push_to_talk());
        } else if linked {
            parts.push(self.sensitivity());
        }
        if linked {
            parts.push(self.soundboard());
        }
        let mut camera_group = vec![
            form::heading(theme, backcord_iced_kit::i18n::tr("CAMERA")),
            device(Drop::Camera, cameras, camera),
        ];
        if state.cameras.is_empty() {
            camera_group.push(form::note(
                theme,
                backcord_iced_kit::i18n::tr("No camera found. Connect one, then refresh."),
            ));
        }
        parts.push(form::halves(
            form::group(camera_group),
            form::group([
                form::heading(theme, backcord_iced_kit::i18n::tr("PREVIEW")),
                self.camera_preview(),
                form::buttons([action(
                    theme,
                    if settings.voice.camera.is_some() {
                        backcord_iced_kit::i18n::tr("Stop video")
                    } else {
                        backcord_iced_kit::i18n::tr("Test video")
                    },
                    Kind::Plain,
                    (!state.cameras.is_empty()).then(|| voice_message(Event::TestCamera)),
                )
                .small()
                .into()]),
            ]),
        ));
        parts.push(form::buttons([action(
            theme,
            backcord_iced_kit::i18n::tr("Refresh devices"),
            Kind::Ghost,
            Some(voice_message(Event::Refresh)),
        )
        .small()
        .icon(
            self.cx
                .look
                .icon(Icon::Refresh, 16, crate::theme::rgb(theme.secondary)),
        )
        .into()]));
        // The microphone turns on as a call is joined, unless it was muted
        // beforehand, and Windows' own microphone effects run: neither is
        // a setting.
        if linked {
            parts.push(form::rule(theme));
            parts.push(self.entrance_sounds());
        }
        form::page(parts)
    }

    /// The camera's preview, 16:9 at the column's width: black in every
    /// theme, as a camera's picture.
    fn camera_preview(self) -> Element<'a, Message> {
        let settings = self.s;
        iced::widget::responsive(move |size| {
            let testing = settings.voice.camera.is_some();
            let frame = (testing && self.cx.video.has(CAMERA_TEST)).then(|| {
                Element::from(
                    Video::new(self.cx.video, CAMERA_TEST, Color::BLACK).scale(self.cx.scale),
                )
            });
            let grey = crate::tile::NOTE;
            let body = frame.unwrap_or_else(|| {
                let note = if !settings.voice.camera_note.is_empty() {
                    settings.voice.camera_note.as_str()
                } else if testing {
                    backcord_iced_kit::i18n::tr("Starting your camera...")
                } else {
                    backcord_iced_kit::i18n::tr("Your camera is off.")
                };
                column![
                    self.cx.look.glyph(Icon::Video, 30, grey),
                    text(backcord_iced_kit::i18n::t(note))
                        .size(14)
                        .color(grey)
                        .center()
                ]
                .spacing(10)
                .padding(12)
                .align_x(Alignment::Center)
                .into()
            });
            container(body)
                .center_x(size.width)
                .height(size.width * 9.0 / 16.0)
                .align_y(Alignment::Center)
                .clip(true)
                .style(|_| crate::style::rounded(Color::BLACK, 8))
                .into()
        })
        .height(Length::Shrink)
        .into()
    }

    /// Discord calls' microphone processing, in Settings' order: the
    /// soundboard's volume, studio mode, then noise suppression and echo
    /// reduction, which apply in studio mode too.
    /// The soundboard's volume, for Discord calls.
    fn soundboard(self) -> Element<'a, Message> {
        let settings = self.s;
        let theme = self.cx.look.theme;
        let state = self.cx.state;
        let board = settings.voice.soundboard.unwrap_or_else(|| {
            backcord_core::volume::to_slider(state.soundboard_volume / 100.0)
                .round()
                .clamp(0.0, 100.0) as u32
        });
        form::group([
            form::heading(theme, backcord_iced_kit::i18n::tr("SOUNDBOARD VOLUME")),
            slider(
                backcord_iced_kit::i18n::tr("Soundboard volume"),
                board,
                100,
                |value| voice_message(Event::SoundboardVolume(value, false)),
            )
            .mark(false)
            .on_release(voice_message(Event::SoundboardVolume(board, true)))
            .view(theme),
            form::note(
                theme,
                backcord_iced_kit::i18n::tr(
                    "How loud soundboard sounds play in Discord calls, yours and everyone else's. Discord keeps it for all your devices.",
                ),
            ),
        ])
    }

    /// How Discord calls process the microphone: studio mode, noise
    /// suppression and its strength, echo reduction.
    fn discord_processing(self) -> Vec<Element<'a, Message>> {
        let theme = self.cx.look.theme;
        let voice = &self.cx.layout.voice;
        let studio = voice.studio_voice;
        let mut parts = vec![];
        parts.push(
            switch(
                backcord_iced_kit::i18n::tr("Studio mode"),
                studio,
                Some(voice_message(Event::Studio(!studio))),
            )
            .description(backcord_iced_kit::i18n::tr("Sends your microphone in Discord calls at the best the codec offers: Opus at 320 kbit/s, fullband, tuned for music. Noise suppression and echo reduction below still apply as you choose them; with both off, your microphone goes out as it is."))
            .view(self.cx.look),
        );
        let noise = SUPPRESSORS
            .iter()
            .position(|kind| *kind == voice.noise_suppression)
            .unwrap_or(0);
        parts.push(form::group([
            form::heading(
                theme,
                backcord_iced_kit::i18n::tr("NOISE SUPPRESSION IN DISCORD CALLS"),
            ),
            choices(
                theme,
                &[
                    backcord_iced_kit::i18n::tr("DeepFilterNet"),
                    backcord_iced_kit::i18n::tr("RNNoise"),
                    backcord_iced_kit::i18n::tr("Off"),
                ],
                noise,
                |index| voice_message(Event::Noise(index)),
            ),
            form::note(
                theme,
                backcord_iced_kit::i18n::tr(
                    "DeepFilterNet also takes out typing and other sudden noise. RNNoise uses less processing. Off sends your microphone as it is, for a quiet room. A call going on switches straight away.",
                ),
            ),
        ]));
        if voice.noise_suppression != Suppressor::Off {
            let strength = STRENGTHS
                .iter()
                .position(|kind| *kind == voice.noise_strength)
                .unwrap_or(2);
            parts.push(form::group([
                form::heading(theme, backcord_iced_kit::i18n::tr("NOISE SUPPRESSION STRENGTH")),
                choices(theme, &[backcord_iced_kit::i18n::tr("Extreme"), backcord_iced_kit::i18n::tr("Full"), backcord_iced_kit::i18n::tr("Medium"), backcord_iced_kit::i18n::tr("Light")], strength, |index| {
                    voice_message(Event::Strength(index))
                }),
                form::note(
                    theme,
                    backcord_iced_kit::i18n::tr("Extreme goes further than Full for loud rooms: DeepFilterNet also turns down what it isn't sure is your voice, RNNoise turns down all it hears between your words. Your voice can sound a little thinner. Full takes out the most noise the network alone can, but can cut off giggles and breathy laughs. Medium and Light leave a little of the room's sound in, which can sound more natural, and let a little more noise through."),
                ),
            ]));
        }
        let echo = usize::from(voice.echo_reduction.unwrap_or(2).min(2));
        parts.push(form::group([
            form::heading(theme, backcord_iced_kit::i18n::tr("ECHO REDUCTION")),
            choices(
                theme,
                &[
                    backcord_iced_kit::i18n::tr("Off"),
                    backcord_iced_kit::i18n::tr("Normal"),
                    backcord_iced_kit::i18n::tr("Strong"),
                ],
                echo,
                |index| voice_message(Event::Echo(index)),
            ),
            form::note(
                theme,
                backcord_iced_kit::i18n::tr("Keeps what your speakers play from going back out through your microphone, so others don't hear themselves. Strong also turns your microphone down while only that echo reaches it; your own voice still goes through. With headphones you can turn it off. A call going on switches straight away."),
            ),
        ]));
        // Under echo reduction: where the others hear you.
        let wide = voice.microphone_wide;
        parts.push(
            switch(
                backcord_iced_kit::i18n::tr("Stereo widener"),
                wide,
                Some(voice_message(Event::MicWide(!wide))),
            )
            .description(backcord_iced_kit::i18n::tr("Makes your microphone sound wide in the others' headphones, filling the space between their ears, whatever your microphone is. A call going on switches straight away."))
            .view(self.cx.look),
        );
        if wide {
            let (percent, value) = width_value(voice);
            parts.push(
                slider(
                    backcord_iced_kit::i18n::tr("Intensity"),
                    value,
                    WIDTH_STEPS,
                    |value| voice_message(Event::MicWidth(value, false)),
                )
                .step(1)
                .mark(false)
                .shown(format!("{percent}%"))
                .on_release(voice_message(Event::MicWidth(value, true)))
                .view(theme),
            );
        }
        let pan = voice.microphone_pan.unwrap_or(0);
        let pan_value = pan_value(pan);
        parts.push(form::group([
            slider(
                backcord_iced_kit::i18n::tr("Panning"),
                pan_value,
                PAN_MIDDLE * 2,
                |value| voice_message(Event::MicPan(value, false)),
            )
            .step(1)
            .mark(false)
            .shown(pan_shown(pan))
            .on_release(voice_message(Event::MicPan(pan_value, true)))
            .view(theme),
            form::note(
                theme,
                backcord_iced_kit::i18n::tr("Moves your voice between the others' ears: all the way left, they hear you in the left ear alone; in the middle, in both, as usual."),
            ),
        ]));
        let (bits, top, shown) = bits_slider(voice);
        parts.push(form::group([
            slider(
                backcord_iced_kit::i18n::tr("Bit depth"),
                bits,
                top,
                |value| voice_message(Event::MicBits(value, false)),
            )
            .step(1)
            .mark(false)
            .shown(shown)
            .on_release(voice_message(Event::MicBits(bits, true)))
            .view(theme),
            {
                let (rate, top, shown) = rate_slider(voice);
                slider(
                    backcord_iced_kit::i18n::tr("Sample rate"),
                    rate,
                    top,
                    |value| voice_message(Event::MicRate(value, false)),
                )
                .step(1)
                .mark(false)
                .shown(shown)
                .on_release(voice_message(Event::MicRate(rate, true)))
                .view(theme)
            },
            {
                let (pitch, top, shown) = pitch_slider(voice);
                slider(
                    backcord_iced_kit::i18n::tr("Voice pitch (semitones)"),
                    pitch,
                    top,
                    |value| voice_message(Event::MicPitch(value, false)),
                )
                .step(1)
                .mark(false)
                .shown(shown)
                .on_release(voice_message(Event::MicPitch(pitch, true)))
                .view(theme)
            },
            form::note(
                theme,
                backcord_iced_kit::i18n::tr("Rounds your voice to fewer bits, and samples it down, before it's sent: together, low, they give old games' voices. At 32 bit float and 48 kHz it goes out as it is."),
            ),
        ]));
        parts
    }

    /// Entrance sounds: the server to choose one for, its sound, Choose
    /// (which needs Nitro) and Remove.
    fn entrance_sounds(self) -> Element<'a, Message> {
        let settings = self.s;
        let theme = self.cx.look.theme;
        let state = self.cx.state;
        let servers = entrance_servers(self.cx.state);
        let chosen = settings.voice.entrance_server.min(servers.len() - 1);
        let server = servers[chosen].clone();
        let names: Vec<String> =
            std::iter::once(backcord_iced_kit::i18n::tr("All servers").to_owned())
                .chain(state.servers.iter().filter_map(|server| {
                    backcord_core::discord::parse_server(&server.id).map(|_| server.name.clone())
                }))
                .collect();
        let catalog = state.sound_catalog.as_ref();
        let sound = state
            .join_sounds
            .iter()
            .find(|join| join.server == server)
            .map(|join| {
                let named = catalog
                    .iter()
                    .flat_map(|groups| groups.iter())
                    .flat_map(|group| &group.sounds)
                    .find(|sound| sound.id == join.sound && sound.guild == join.guild);
                match (named, catalog) {
                    (Some(sound), _) => sound.name.clone(),
                    (None, None) => backcord_iced_kit::i18n::tr("Loading...").to_owned(),
                    (None, Some(_)) => {
                        backcord_iced_kit::i18n::tr("A sound that's gone").to_owned()
                    }
                }
            });
        let nitro = self.cx.app.discord_nitro();
        let open = settings.open;
        let mut line = vec![
            container(
                form::dropdown(
                    names,
                    Some(chosen),
                    open == Some(Drop::Entrance),
                    Message::Settings(SettingsEvent::Drop(Drop::Entrance)),
                    |index| voice_message(Event::EntranceServer(index)),
                )
                .view(self.cx.look),
            )
            .width(200)
            .into(),
            form::readout(
                theme,
                sound.clone().unwrap_or_default(),
                backcord_iced_kit::i18n::tr("None"),
                false,
            )
            .width(Length::Fill)
            .into(),
        ];
        // Choose opens the soundboard under it, a click on a sound
        // choosing it.
        let board = (nitro && open == Some(Drop::Start)).then(|| {
            let groups = catalog.map_or(&[][..], |groups| &groups[..]);
            let board = crate::call::popups::soundboard::sound_board(
                self.cx,
                backcord_iced_kit::i18n::tr("Choose an entrance sound"),
                groups,
                None,
                false,
                |sound| voice_message(Event::Entrance(Some(sound.id.clone()))),
                None,
            );
            container(board)
                .style(crate::controls::menu_panel(theme))
                .into()
        });
        let choose: Element<'a, Message> = crate::popover::Popover::new(
            action(
                theme,
                backcord_iced_kit::i18n::tr("Choose"),
                Kind::Plain,
                nitro.then(|| Message::Settings(SettingsEvent::Drop(Drop::Start))),
            ),
            board,
            crate::popover::Side::Below,
        )
        .width(crate::popover::Width::Own(
            crate::call::popups::SOUNDBOARD_WIDTH,
        ))
        .right()
        .gap(6.0)
        .outside(Message::Settings(SettingsEvent::Drop(Drop::Start)))
        .into();
        line.push(choose);
        if sound.is_some() {
            line.push(
                action(
                    theme,
                    backcord_iced_kit::i18n::tr("Remove"),
                    Kind::Ghost,
                    Some(voice_message(Event::Entrance(None))),
                )
                .into(),
            );
        }
        form::group([
            form::heading(theme, backcord_iced_kit::i18n::tr("ENTRANCE SOUNDS")),
            row(line).spacing(8).align_y(Alignment::Center).into(),
            form::note(
                theme,
                if nitro {
                    backcord_iced_kit::i18n::tr(
                        "Plays to everyone in a voice channel or DM call as you join it, outside stage channels. A server's own comes before the one for all servers.",
                    )
                } else {
                    backcord_iced_kit::i18n::tr(
                        "Entrance sounds play to everyone as you join a voice channel. They need Nitro.",
                    )
                },
            ),
        ])
    }

    /// Push to talk's key, with its recording, and its release delay.
    fn push_to_talk(self) -> Vec<Element<'a, Message>> {
        let settings = self.s;
        let theme = self.cx.look.theme;
        let voice = &self.cx.layout.voice;
        let recorded = settings.recorded(super::keybinds::Target::Talk);
        let recording = recorded.is_some();
        let key = voice
            .push_to_talk_key
            .as_ref()
            .map(Shortcut::name)
            .unwrap_or_default();
        let note = if recording {
            backcord_iced_kit::i18n::tr("Press the keys to hold while you talk, together. Escape stops without changing it.")
                .to_owned()
        } else if key.is_empty() {
            backcord_iced_kit::i18n::tr(
                "Push to talk is on, but no keybind is set, so your microphone sends nothing.",
            )
            .to_owned()
        } else {
            self.cx.keybinds.talk_note()
        };
        let empty = key.is_empty();
        let button =
            super::keybinds::record_button(theme, super::keybinds::Target::Talk, recording, empty);
        let delay = voice
            .push_to_talk_release_delay
            .unwrap_or(backcord_core::push_to_talk::DEFAULT_RELEASE_DELAY)
            .min(backcord_core::push_to_talk::MOST_RELEASE_DELAY);
        let shown = if delay >= 1000 {
            format!("{:.2} s", delay as f32 / 1000.0)
        } else {
            format!("{delay} ms")
        };
        vec![
            form::group([
                form::heading(theme, backcord_iced_kit::i18n::tr("PUSH TO TALK KEYBIND")),
                form::buttons([form::capture(theme, key, recorded, 200.0), button]),
                form::note(theme, backcord_iced_kit::i18n::t(note)),
            ]),
            form::group([
                form::heading(
                    theme,
                    backcord_iced_kit::i18n::tr("PUSH TO TALK RELEASE DELAY"),
                ),
                slider(
                    backcord_iced_kit::i18n::tr("Push to talk release delay"),
                    delay,
                    backcord_core::push_to_talk::MOST_RELEASE_DELAY,
                    |value| voice_message(Event::TalkDelay(value, false)),
                )
                .step(10)
                .mark(false)
                .shown(shown)
                .on_release(voice_message(Event::TalkDelay(delay, true)))
                .view(theme),
                form::note(
                    theme,
                    backcord_iced_kit::i18n::tr(
                        "How long your microphone keeps sending after you let go of the key, so the end of a word isn't cut off.",
                    ),
                ),
            ]),
        ]
    }

    /// The Discord gate's sensitivity, over the microphone's level as the
    /// gate hears it.
    fn sensitivity(self) -> Element<'a, Message> {
        let settings = self.s;
        let theme = self.cx.look.theme;
        let voice = &self.cx.layout.voice;
        let decibels = voice
            .input_sensitivity
            .unwrap_or(backcord_core::noise::DEFAULT_SENSITIVITY as i32)
            .clamp(-80, 0);
        // The gate's level and whether it sends, read as they're drawn,
        // twenty times a second: no message, no view.
        let gate = settings.voice.gate.as_ref();
        let inks = GateInks {
            live: theme.live,
            loud: theme.loud,
            danger: theme.danger,
            field: theme.field,
        };
        let meter = crate::widgets::gauge(METER_EVERY, move |renderer, bounds| {
            let level = gate.map_or(-100.0, |gate| gate.level().max(-100.0));
            gate_level(renderer, bounds, level, inks);
        })
        .height(18);
        let sending = move || gate.is_some_and(backcord_core::discord_audio::GateMeter::sending);
        let (live, muted) = (theme.live, theme.muted);
        let sending_line = crate::widgets::live_text(METER_EVERY, move || {
            if sending() {
                backcord_iced_kit::i18n::tr("Sending")
            } else {
                backcord_iced_kit::i18n::tr("Not sending")
            }
            .to_owned()
        })
        .size(13)
        .font(SEMIBOLD)
        .ink(move || if sending() { live } else { muted });
        let knob = theme.text;
        let track = iced::widget::slider(0..=80u32, (decibels + 80) as u32, |value| {
            voice_message(Event::Sensitivity(value, false))
        })
        .on_release(voice_message(Event::Sensitivity(
            (decibels + 80) as u32,
            true,
        )))
        .height(18)
        .style(move |_, _| iced::widget::slider::Style {
            rail: iced::widget::slider::Rail {
                backgrounds: (Color::TRANSPARENT.into(), Color::TRANSPARENT.into()),
                width: 8.0,
                border: iced::Border::default(),
            },
            handle: iced::widget::slider::Handle {
                shape: iced::widget::slider::HandleShape::Circle { radius: 7.0 },
                background: knob.into(),
                border_width: 0.0,
                border_color: Color::TRANSPARENT,
            },
        });
        let note = if !backcord_core::discord_audio::AVAILABLE {
            backcord_iced_kit::i18n::tr(
                "Discord voice and its level aren't available on this system yet.",
            )
        } else if settings
            .voice
            .gate
            .as_ref()
            .is_some_and(|gate| gate.failed())
        {
            backcord_iced_kit::i18n::tr(
                "BACKCORD can't read this microphone. Check that it's connected, or choose another.",
            )
        } else {
            backcord_iced_kit::i18n::tr(
                "BACKCORD sends your voice when it's louder than the handle, and other sounds when they're 18 dB louder still. The bar is your microphone after noise suppression and at its volume, as BACKCORD hears it to decide.",
            )
        };
        form::group([
            form::heading(
                theme,
                backcord_iced_kit::i18n::tr("INPUT SENSITIVITY IN DISCORD CALLS"),
            ),
            column![
                row![
                    text(backcord_iced_kit::i18n::tr("Input sensitivity"))
                        .size(13)
                        .font(SEMIBOLD)
                        .color(theme.secondary)
                        .width(Length::Fill),
                    sending_line,
                    text(format!("{decibels} dB"))
                        .size(13)
                        .font(SEMIBOLD)
                        .color(theme.text),
                ]
                .spacing(10),
                iced::widget::stack![meter, track]
            ]
            .spacing(6)
            .into(),
            form::note(theme, note),
        ])
    }
}

impl<'a> Page<'a> {
    /// Discord's mic test: its button, a meter of notches lit to the
    /// microphone's level while it plays back, and what it hears, read as
    /// it's drawn.
    fn mic_test(self) -> Element<'a, Message> {
        let settings = self.s;
        let theme = self.cx.look.theme;
        let testing = settings.voice.testing;
        let gate = settings.voice.gate.as_ref().filter(|_| testing);
        let (lit, unlit) = (theme.live, theme.button_default);
        let meter = crate::widgets::gauge(METER_EVERY, move |renderer, bounds| {
            let level = gate.map_or(-100.0, |gate| gate.level().max(-100.0));
            notches(renderer, bounds, level, lit, unlit);
        })
        .height(24);
        // Discord's captions, the failures first.
        let said = move || -> (&'static str, &'static str) {
            match gate {
                None => ("", ""),
                Some(gate) if gate.failed() => (
                    backcord_iced_kit::i18n::tr("BACKCORD can't read this microphone."),
                    backcord_iced_kit::i18n::tr("Check that it's connected, or choose another."),
                ),
                Some(gate) if gate.playback_failed() => (
                    backcord_iced_kit::i18n::tr("BACKCORD can't play your voice back."),
                    backcord_iced_kit::i18n::tr("Check your speakers, or choose others."),
                ),
                Some(gate) if gate.no_input() => (
                    backcord_iced_kit::i18n::tr(
                        "BACKCORD isn't detecting any input from your mic.",
                    ),
                    backcord_iced_kit::i18n::tr(
                        "Please check that you've selected the right input device.",
                    ),
                ),
                Some(_) => (
                    backcord_iced_kit::i18n::tr("Playing back your beautiful voice"),
                    "",
                ),
            }
        };
        // Trouble reads in the text's colour, the playing back quieter.
        let trouble = move || !said().1.is_empty();
        let (text_ink, quiet) = (theme.text, theme.muted);
        let caption = crate::widgets::live_text(METER_EVERY, move || said().0.to_owned())
            .size(12)
            .font(SEMIBOLD)
            .ink(move || if trouble() { text_ink } else { quiet });
        let advice = crate::widgets::live_text(METER_EVERY, move || said().1.to_owned())
            .size(12)
            .color(quiet);
        let button = action(
            theme,
            if testing {
                backcord_iced_kit::i18n::tr("Stop testing")
            } else {
                backcord_iced_kit::i18n::tr("Let's check")
            },
            Kind::Primary,
            Some(voice_message(Event::MicTest)),
        )
        .small();
        let mut group = vec![
            form::heading(theme, backcord_iced_kit::i18n::tr("MIC TEST")),
            form::note(
                theme,
                backcord_iced_kit::i18n::tr(
                    "Having mic issues? Start a test and say something fun, and BACKCORD plays your voice back to you, with your noise suppression and volumes. Wear headphones, or it echoes.",
                ),
            ),
            row![
                button,
                column![meter, caption, advice]
                    .spacing(4)
                    .width(Length::Fill)
            ]
            .spacing(16)
            .width(Length::Fill)
            .align_y(Alignment::Start)
            .into(),
        ];
        if !testing && self.cx.state.voice.active() {
            group.push(form::note(
                theme,
                backcord_iced_kit::i18n::tr(
                    "You'll be muted and deafened in your call while you test.",
                ),
            ));
        }
        form::group(group)
    }
}

/// The mic test's meter: notches across `bounds`, lit up to the
/// microphone's `level` (dB, -100 to 0), as Discord's.
fn notches(
    renderer: &mut crate::widgets::Renderer,
    bounds: iced::Rectangle,
    level: f32,
    lit: Color,
    unlit: Color,
) {
    use iced::advanced::{Renderer as _, renderer::Quad};
    const WIDTH: f32 = 4.0;
    const GAP: f32 = 4.0;
    let count = ((bounds.width + GAP) / (WIDTH + GAP)).floor().max(1.0) as usize;
    let shown = ((level + 100.0) / 100.0).clamp(0.0, 1.0);
    for index in 0..count {
        let on = (index as f32 + 0.5) / count as f32 <= shown;
        renderer.fill_quad(
            Quad {
                bounds: iced::Rectangle {
                    x: bounds.x + index as f32 * (WIDTH + GAP),
                    width: WIDTH,
                    ..bounds
                },
                border: iced::border::rounded(2),
                ..Default::default()
            },
            if on { lit } else { unlit },
        );
    }
}

fn voice_message(event: Event) -> Message {
    Message::Settings(SettingsEvent::Voice(event))
}

/// The gate meter's colours: green up to -18 dB, orange to -6 dB, red
/// above, on the field's ground.
#[derive(Clone, Copy)]
struct GateInks {
    live: Color,
    loud: Color,
    danger: Color,
    field: Color,
}

/// The gate's `level` (dB, -80 to 0): an 8 px bar across the middle of
/// `bounds`.
fn gate_level(
    renderer: &mut crate::widgets::Renderer,
    bounds: iced::Rectangle,
    level: f32,
    inks: GateInks,
) {
    use iced::advanced::{Renderer as _, renderer::Quad};
    let at = |decibels: f32| ((decibels + 80.0) / 80.0).clamp(0.0, 1.0);
    let (orange, red, shown) = (at(-18.0), at(-6.0), at(level));
    let bar = iced::Rectangle {
        y: bounds.y + (bounds.height - 8.0) / 2.0,
        height: 8.0,
        ..bounds
    };
    let mut fill = |from: f32, to: f32, color: Color| {
        if to > from {
            renderer.fill_quad(
                Quad {
                    bounds: iced::Rectangle {
                        x: bar.x + from * bar.width,
                        width: (to - from) * bar.width,
                        ..bar
                    },
                    ..Default::default()
                },
                color,
            );
        }
    };
    fill(0.0, 1.0, inks.field);
    fill(0.0, shown.min(orange), inks.live);
    fill(orange, shown.min(red), inks.loud);
    fill(red, shown, inks.danger);
}

/// The servers an entrance sound can be chosen for: every server (none),
/// then each Discord server by its own id.
fn entrance_servers(state: &AppState) -> Vec<Option<String>> {
    std::iter::once(None)
        .chain(state.servers.iter().filter_map(|server| {
            backcord_core::discord::parse_server(&server.id).map(|guild| Some(guild.to_owned()))
        }))
        .collect()
}

/// The entrance sounds' pictures, while Choose's soundboard is open.
pub fn entrance_pictures(app: &App) -> Vec<String> {
    let open = app.win.settings.get().is_some_and(|settings| {
        settings.open == Some(Drop::Start) && settings.section() == crate::settings::Section::Voice
    });
    if !open {
        return Vec::new();
    }
    app.client
        .state()
        .sound_catalog
        .iter()
        .flat_map(|groups| groups.iter())
        .flat_map(|group| &group.sounds)
        .filter_map(|sound| sound.picture.clone())
        .collect()
}

/// The gate's slider's steps: off, then one per dB from -70 dB to -20.
const GATE_STEPS: u32 = 51;

/// The gate's level a slider's value stands for: none for off.
fn gate_db(value: u32) -> Option<i8> {
    let lowest = backcord_core::volume::VOICE_GATE_LOWEST as i32;
    (value > 0).then(|| (lowest + value.min(GATE_STEPS) as i32 - 1) as i8)
}

fn gate_value(db: Option<i8>) -> u32 {
    let lowest = backcord_core::volume::VOICE_GATE_LOWEST as i32;
    db.map_or(0, |db| {
        (i32::from(db) - lowest + 1).clamp(1, GATE_STEPS as i32) as u32
    })
}

/// The voice limiter's slider's steps: one per dB from -40 dB up to off.
const LIMIT_STEPS: u32 = 40;

/// The ceiling the voice limiter's slider stands for, in dBFS: 0 for off.
fn limit_db(value: u32) -> i8 {
    (value.min(LIMIT_STEPS) as i32 - LIMIT_STEPS as i32) as i8
}
