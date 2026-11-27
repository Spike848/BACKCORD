//! How loud each person sounds in calls, and their stream's sound, as the
//! gain they play at: 1.0 as they come, 0.0 silent, up to [`MAX`]. People are
//! named as their network does, by Discord user id or Matrix user id. The UI
//! remembers the choices and sets them here; the calls' audio reads them as
//! it plays. Its sliders show them on Discord's scale; see [`from_slider`].
//! The microphone's volume and the speakers' apply to every call on top:
//! everyone plays at their own volume times the speakers'. Muting someone's
//! soundboard here silences the sounds they play, not their voice.

use std::{
    collections::{HashMap, HashSet},
    sync::{
        LazyLock, RwLock,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
};

use tokio::sync::watch;

/// Four times as loud: past Discord's 200%, a person's volume and the
/// microphone's go to 400%.
pub const MAX: f32 = 4.0;

/// The most a person's or the microphone's slider goes to.
pub const MOST: u32 = 400;

/// The gain at 200, about twice as loud.
fn at_200() -> f32 {
    10f32.powf(6.0 / 20.0)
}

/// The gain of a slider at `value`, from 0 to [`MOST`], as Discord's volume
/// sliders map it up to 200: below 100 on a power curve, `(value / 100)^2.8`,
/// so 50 is about -17 dB; from 100 to 200, 6 dB more for each 100, so 200 is
/// about twice as loud. A linear slider made everything loud: 40 on
/// Discord's is 0.077, not 0.4. Past 200, as far as Discord's sliders don't
/// go, the gain grows with the value, so 400 is about four times as loud.
pub fn from_slider(value: f32) -> f32 {
    let share = value.max(0.0) / 100.0;
    if share < 1.0 {
        share.powf(2.8)
    } else if share <= 2.0 {
        10f32.powf((share - 1.0) * 6.0 / 20.0)
    } else {
        at_200() * share / 2.0
    }
}

/// The slider value that plays at `gain`; see [`from_slider`].
pub fn to_slider(gain: f32) -> f32 {
    if gain <= 0.0 {
        0.0
    } else if gain < 1.0 {
        gain.powf(1.0 / 2.8) * 100.0
    } else if gain <= at_200() {
        (20.0 * gain.log10() / 6.0 + 1.0) * 100.0
    } else {
        gain / at_200() * 200.0
    }
}

/// A person's voice, or their stream's sound.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Voice,
    Stream,
}

impl Kind {
    /// The gain unless one is chosen: voices as they come, and a stream's
    /// sound at Discord's default of 0.18, about 54 on the slider, since
    /// it's played as loud as the streamer's computer makes it.
    pub fn default_gain(self) -> f32 {
        match self {
            Kind::Voice => 1.0,
            Kind::Stream => 0.18,
        }
    }
}

/// The volumes chosen, voices' then streams', by person; looked up by
/// borrowed id, as the calls' audio does for every frame.
static VOLUMES: LazyLock<[RwLock<HashMap<String, f32>>; 2]> = LazyLock::new(Default::default);
/// Those muted for this account alone, as Discord's local mute does.
static MUTED: LazyLock<RwLock<HashSet<String>>> = LazyLock::new(RwLock::default);
/// Those whose streams' sound this account doesn't hear.
static STREAM_MUTED: LazyLock<RwLock<HashSet<String>>> = LazyLock::new(RwLock::default);
/// Those whose soundboard sounds this account doesn't hear.
static SOUNDBOARD_MUTED: LazyLock<RwLock<HashSet<String>>> = LazyLock::new(RwLock::default);
/// Each person's voice's pitch, in semitones, where it isn't 0.
static PITCHES: LazyLock<RwLock<HashMap<String, f32>>> = LazyLock::new(RwLock::default);
static CHANGED: LazyLock<watch::Sender<u64>> = LazyLock::new(|| watch::channel(0).0);

/// The voice limiter's ceiling, in dBFS, as f32 bits; 0 dB is off.
static VOICE_LIMIT: AtomicU32 = AtomicU32::new(0f32.to_bits());

/// The lowest ceiling the voice limiter takes, in dBFS.
pub const VOICE_LIMIT_LOWEST: f32 = -40.0;

/// The microphone's and the speakers' gains in every call, as f32 bits.
static MICROPHONE: AtomicU32 = AtomicU32::new(1f32.to_bits());
static SPEAKERS: AtomicU32 = AtomicU32::new(1f32.to_bits());

fn volumes(kind: Kind) -> &'static RwLock<HashMap<String, f32>> {
    &VOLUMES[kind as usize]
}

/// Sets how loud `person` sounds; the default gain forgets the choice.
pub fn set(kind: Kind, person: &str, volume: f32) {
    let volume = volume.clamp(0.0, MAX);
    {
        let mut volumes = volumes(kind).write().unwrap();
        if (volume - kind.default_gain()).abs() < f32::EPSILON {
            volumes.remove(person);
        } else {
            volumes.insert(person.to_owned(), volume);
        }
    }
    CHANGED.send_modify(|changes| *changes += 1);
}

/// Raises or lowers `person`'s voice by `semitones` ([`crate::pitch`]);
/// 0 turns it off.
pub fn set_pitch(person: &str, semitones: f32) {
    let semitones = semitones.clamp(-crate::pitch::MOST, crate::pitch::MOST);
    let mut pitches = PITCHES.write().unwrap();
    if semitones.abs() < 0.01 {
        pitches.remove(person);
    } else {
        pitches.insert(person.to_owned(), semitones);
    }
}

/// `person`'s voice's pitch, in semitones; 0 for as it comes.
pub fn pitch(person: &str) -> f32 {
    PITCHES.read().unwrap().get(person).copied().unwrap_or(0.0)
}

/// Whether anyone's pitch is changed, which the call's audio looks at
/// before it looks anyone up.
pub fn any_pitch() -> bool {
    !PITCHES.read().unwrap().is_empty()
}

/// How loud `person` sounds, as chosen.
pub fn get(kind: Kind, person: &str) -> f32 {
    volumes(kind)
        .read()
        .unwrap()
        .get(person)
        .copied()
        .unwrap_or(kind.default_gain())
}

/// Mutes `person`'s voice for this account alone, keeping their volume
/// for when they're unmuted, or unmutes them.
pub fn mute(person: &str, muted: bool) {
    let changed = {
        let mut set = MUTED.write().unwrap();
        if muted {
            set.insert(person.to_owned())
        } else {
            set.remove(person)
        }
    };
    if changed {
        CHANGED.send_modify(|changes| *changes += 1);
    }
}

/// Whether `person` is muted for this account alone.
pub fn muted(person: &str) -> bool {
    MUTED.read().unwrap().contains(person)
}

/// Mutes the soundboard sounds `person` plays, for this account alone, or
/// unmutes them; the sounds they play from then on.
pub fn mute_soundboard(person: &str, muted: bool) {
    let mut set = SOUNDBOARD_MUTED.write().unwrap();
    if muted {
        set.insert(person.to_owned());
    } else {
        set.remove(person);
    }
}

/// Whether `person`'s soundboard sounds are muted for this account.
pub fn soundboard_muted(person: &str) -> bool {
    SOUNDBOARD_MUTED.read().unwrap().contains(person)
}

/// Mutes `person`'s stream's sound for this account, keeping its volume,
/// or unmutes it.
pub fn mute_stream(person: &str, muted: bool) {
    let changed = {
        let mut set = STREAM_MUTED.write().unwrap();
        if muted {
            set.insert(person.to_owned())
        } else {
            set.remove(person)
        }
    };
    if changed {
        CHANGED.send_modify(|changes| *changes += 1);
    }
}

/// Whether `person`'s stream is muted for this account.
pub fn stream_muted(person: &str) -> bool {
    STREAM_MUTED.read().unwrap().contains(person)
}

/// How loud `person` plays: as chosen, times the speakers' volume, or
/// silent while muted here.
pub fn heard(kind: Kind, person: &str) -> f32 {
    if (kind == Kind::Voice && muted(person)) || (kind == Kind::Stream && stream_muted(person)) {
        0.0
    } else {
        get(kind, person) * speakers()
    }
}

/// Sets how loud the microphone sends in every call, from 0.0 to [`MAX`].
pub fn set_microphone(gain: f32) {
    MICROPHONE.store(gain.clamp(0.0, MAX).to_bits(), Ordering::Relaxed);
}

/// How loud the microphone sends; 1.0, as it comes, unless set.
pub fn microphone() -> f32 {
    f32::from_bits(MICROPHONE.load(Ordering::Relaxed))
}

/// Sets how loud every call plays, from 0.0 to [`MAX`].
pub fn set_speakers(gain: f32) {
    let gain = gain.clamp(0.0, MAX).to_bits();
    if SPEAKERS.swap(gain, Ordering::Relaxed) != gain {
        CHANGED.send_modify(|changes| *changes += 1);
    }
}

/// How loud every call plays; 1.0 unless set.
pub fn speakers() -> f32 {
    f32::from_bits(SPEAKERS.load(Ordering::Relaxed))
}

/// Holds people's voices in calls under `db` dBFS, from
/// [`VOICE_LIMIT_LOWEST`] up; 0 or above turns the limiter off. See
/// [`crate::voice_limiter`].
pub fn set_voice_limit(db: f32) {
    let db = if db >= 0.0 {
        0.0
    } else {
        db.max(VOICE_LIMIT_LOWEST)
    };
    VOICE_LIMIT.store(db.to_bits(), Ordering::Relaxed);
}

/// The ceiling people's voices are held under, as a gain of full scale, or
/// none while the limiter is off.
pub fn voice_limit() -> Option<f32> {
    let db = f32::from_bits(VOICE_LIMIT.load(Ordering::Relaxed));
    (db < 0.0).then(|| 10f32.powf(db / 20.0))
}

/// Whether people's voices in calls go through the voice compressor; off
/// unless turned on. See [`crate::voice_compressor`].
static VOICE_COMPRESSOR: AtomicBool = AtomicBool::new(false);
/// Whether people's voices go through the high-pass; off unless turned on.
/// See [`crate::voice_filters`].
static VOICE_HIGH_PASS: AtomicBool = AtomicBool::new(false);
/// The level under which the gate silences people's voices, in dBFS, as
/// f32 bits; 0 is off.
static VOICE_GATE: AtomicU32 = AtomicU32::new(0f32.to_bits());
/// The gate's lowest and highest levels, in dBFS.
pub const VOICE_GATE_LOWEST: f32 = -70.0;
pub const VOICE_GATE_HIGHEST: f32 = -20.0;
/// Those whose voices skip the high-pass, the gate and the compressor,
/// not the limiter.
static EFFECTS_BYPASSED: LazyLock<RwLock<HashSet<String>>> = LazyLock::new(RwLock::default);

pub fn set_voice_high_pass(on: bool) {
    VOICE_HIGH_PASS.store(on, Ordering::Relaxed);
}

pub fn voice_high_pass() -> bool {
    VOICE_HIGH_PASS.load(Ordering::Relaxed)
}

/// Gates people's voices under `db` dBFS, from [`VOICE_GATE_LOWEST`] to
/// [`VOICE_GATE_HIGHEST`]; 0 or above turns the gate off.
pub fn set_voice_gate(db: f32) {
    let db = if db >= 0.0 {
        0.0
    } else {
        db.clamp(VOICE_GATE_LOWEST, VOICE_GATE_HIGHEST)
    };
    VOICE_GATE.store(db.to_bits(), Ordering::Relaxed);
}

/// The gate's level in dBFS, or none while it's off.
pub fn voice_gate() -> Option<f32> {
    let db = f32::from_bits(VOICE_GATE.load(Ordering::Relaxed));
    (db < 0.0).then_some(db)
}

/// Lets `person`'s voice skip the high-pass, the gate and the compressor,
/// or go through them again; the limiter holds it either way.
pub fn bypass_effects(person: &str, bypassed: bool) {
    let mut set = EFFECTS_BYPASSED.write().unwrap();
    if bypassed {
        set.insert(person.to_owned());
    } else {
        set.remove(person);
    }
}

pub fn effects_bypassed(person: &str) -> bool {
    EFFECTS_BYPASSED.read().unwrap().contains(person)
}

/// Widens the microphone in Discord calls, so it fills the listeners'
/// ears; off unless set.
pub fn set_microphone_wide(on: bool) {
    #[cfg(feature = "discord")]
    backcord_discord::spatial::set_wide(on);
    let _ = on;
}

/// How wide the widener makes the microphone, 0 to 1.
pub fn set_microphone_width(width: f32) {
    #[cfg(feature = "discord")]
    backcord_discord::spatial::set_width(width);
    let _ = width;
}

/// The bit depth the microphone goes out at in Discord calls, 1 to 31, or
/// 32 for floats, as it is.
pub fn set_microphone_bits(bits: u32) {
    #[cfg(feature = "discord")]
    backcord_discord::spatial::set_bits(bits);
    let _ = bits;
}

/// The rate the microphone's sampled down to in Discord calls, in Hz;
/// 48 000 leaves it as it is.
pub fn set_microphone_sample_rate(rate: u32) {
    #[cfg(feature = "discord")]
    backcord_discord::spatial::set_sample_rate(rate);
    let _ = rate;
}

static MICROPHONE_PITCH: AtomicU32 = AtomicU32::new(0);

/// Raises or lowers the microphone in Discord calls, in semitones, as
/// someone's Pitch does to their voice; 0 leaves it as it is.
pub fn set_microphone_pitch(semitones: f32) {
    let semitones = if semitones.is_finite() {
        semitones.clamp(-crate::pitch::MOST, crate::pitch::MOST)
    } else {
        0.0
    };
    MICROPHONE_PITCH.store(semitones.to_bits(), Ordering::Relaxed);
}

pub fn microphone_pitch() -> f32 {
    f32::from_bits(MICROPHONE_PITCH.load(Ordering::Relaxed))
}

static STREAM_SOUND: AtomicBool = AtomicBool::new(true);

/// Shares the computer's sound with this account's stream, or the picture
/// alone; a stream going on starts or stops its sound at its next frame.
pub fn set_share_stream_sound(on: bool) {
    STREAM_SOUND.store(on, Ordering::Relaxed);
}

pub fn share_stream_sound() -> bool {
    STREAM_SOUND.load(Ordering::Relaxed)
}

/// Places the microphone in Discord calls between the listeners' ears,
/// from -1 (the left alone) to 1 (the right); 0, the middle, unless set.
pub fn set_microphone_pan(pan: f32) {
    #[cfg(feature = "discord")]
    backcord_discord::spatial::set_pan(pan);
    let _ = pan;
}

pub fn set_voice_compressor(on: bool) {
    VOICE_COMPRESSOR.store(on, Ordering::Relaxed);
}

pub fn voice_compressor() -> bool {
    VOICE_COMPRESSOR.load(Ordering::Relaxed)
}

/// Changes whenever a volume or a mute does.
pub fn changes() -> watch::Receiver<u64> {
    CHANGED.subscribe()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Held by the tests that set the speakers' volume or depend on it.
    static SPEAKERS_SET: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn remembers_each_persons_voice_and_stream() {
        let mut changes = changes();
        set(Kind::Voice, "volume-test-1", 0.5);
        set(Kind::Stream, "volume-test-1", 5.0);
        assert!(changes.has_changed().unwrap());
        changes.mark_unchanged();
        assert_eq!(get(Kind::Voice, "volume-test-1"), 0.5);
        assert_eq!(
            get(Kind::Stream, "volume-test-1"),
            MAX,
            "at most four times as loud"
        );
        assert_eq!(
            get(Kind::Voice, "volume-test-2"),
            1.0,
            "unchanged by default"
        );
        set(Kind::Voice, "volume-test-1", 1.0);
        assert_eq!(get(Kind::Voice, "volume-test-1"), 1.0);
        assert!(changes.has_changed().unwrap());
    }

    #[test]
    fn a_local_mute_silences_the_voice_and_keeps_its_volume() {
        let _speakers = SPEAKERS_SET.lock().unwrap();
        set(Kind::Voice, "volume-test-3", 0.5);
        let changes = changes();
        mute("volume-test-3", true);
        assert!(changes.has_changed().unwrap());
        assert!(muted("volume-test-3"));
        assert_eq!(heard(Kind::Voice, "volume-test-3"), 0.0);
        assert_eq!(
            heard(Kind::Stream, "volume-test-3"),
            Kind::Stream.default_gain(),
            "their stream plays"
        );
        assert_eq!(
            get(Kind::Voice, "volume-test-3"),
            0.5,
            "the slider keeps it"
        );
        mute("volume-test-3", false);
        assert_eq!(heard(Kind::Voice, "volume-test-3"), 0.5);
    }

    #[test]
    fn muting_the_soundboard_leaves_the_voice() {
        mute_soundboard("volume-test-6", true);
        assert!(soundboard_muted("volume-test-6"));
        assert!(!muted("volume-test-6"), "their voice plays");
        mute_soundboard("volume-test-6", false);
        assert!(!soundboard_muted("volume-test-6"));
    }

    #[test]
    fn the_speakers_volume_scales_everyone() {
        let _speakers = SPEAKERS_SET.lock().unwrap();
        set(Kind::Voice, "volume-test-5", 0.5);
        let changes = changes();
        set_speakers(1.5);
        assert!(changes.has_changed().unwrap(), "calls play it at once");
        assert_eq!(heard(Kind::Voice, "volume-test-5"), 0.75);
        assert_eq!(
            heard(Kind::Stream, "volume-test-5"),
            Kind::Stream.default_gain() * 1.5,
            "streams too"
        );
        assert_eq!(get(Kind::Voice, "volume-test-5"), 0.5, "their own stays");
        set_speakers(1.0);
    }

    #[test]
    fn sliders_follow_discords_curve() {
        let close = |a: f32, b: f32| (a - b).abs() < 1e-3;
        assert_eq!(from_slider(0.0), 0.0, "silent");
        assert!(close(from_slider(100.0), 1.0), "as they come");
        assert!(close(from_slider(50.0), 0.143), "about -17 dB");
        assert!(close(from_slider(40.0), 0.077));
        assert!(close(from_slider(200.0), 1.995), "6 dB louder");
        assert!(close(from_slider(400.0), 3.991), "four times as loud");
        assert!(from_slider(400.0) <= MAX);
        for value in [1.0, 18.0, 40.0, 99.0, 100.0, 150.0, 200.0, 250.0, 400.0] {
            assert!(close(to_slider(from_slider(value)), value), "{value}");
        }
        // A stream's default shows at about 54, as on Discord.
        assert_eq!(to_slider(Kind::Stream.default_gain()).round(), 54.0);
        assert_eq!(get(Kind::Stream, "volume-test-4"), 0.18);
    }
}
