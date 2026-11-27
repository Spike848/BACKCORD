//! Room mode: each voice in a Discord call heard from where it was put on
//! a square around this account, which sits at its middle. Left and right
//! pan it, with the far ear hearing it a little later as a head does; the
//! back muffles it and takes it down a little, as does distance. A room's
//! reverb, as loud as chosen, goes over the whole call. Off unless turned
//! on; nobody placed is heard as they come.

use std::{
    collections::HashMap,
    sync::{
        LazyLock, RwLock,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
};

static ON: AtomicBool = AtomicBool::new(false);
/// Where each person is, by Discord user id: left -1 to right 1, front -1
/// to back 1.
static PLACES: LazyLock<RwLock<HashMap<String, (f32, f32)>>> = LazyLock::new(RwLock::default);
/// The reverb's share, 0 to 1, as f32 bits.
static REVERB: AtomicU32 = AtomicU32::new(0);

/// Turns room mode on or off.
pub fn set_on(on: bool) {
    ON.store(on, Ordering::Relaxed);
}

/// Whether room mode is on.
pub fn on() -> bool {
    ON.load(Ordering::Relaxed)
}

/// Puts `person` at `x` (left -1 to right 1) and `y` (front -1 to back
/// 1); the middle is as they come.
pub fn set_place(person: &str, x: f32, y: f32) {
    let (x, y) = (x.clamp(-1.0, 1.0), y.clamp(-1.0, 1.0));
    let mut places = PLACES.write().unwrap();
    if x.abs() < 0.01 && y.abs() < 0.01 {
        places.remove(person);
    } else {
        places.insert(person.to_owned(), (x, y));
    }
}

/// Where `person` is; the middle unless placed.
pub fn place(person: &str) -> (f32, f32) {
    PLACES
        .read()
        .unwrap()
        .get(person)
        .copied()
        .unwrap_or_default()
}

/// Sets the reverb's share, 0 to 1.
pub fn set_reverb(amount: f32) {
    REVERB.store(amount.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
}

/// The reverb's share, 0 to 1.
pub fn reverb() -> f32 {
    f32::from_bits(REVERB.load(Ordering::Relaxed))
}

/// The most the far ear hears later, in samples at 48 kHz: 0.65 ms, a
/// head's width.
const MOST_DELAY: usize = 31;
/// A voice not heard for this many 20 ms frames is forgotten.
const FORGET_AFTER: u32 = 50;

/// The room's state across frames: each voice's last samples, for the far
/// ear's delay, and its muffling; the reverb's.
#[derive(Default)]
pub struct Room {
    voices: HashMap<u64, Voice>,
    /// Made as the reverb turns up, dropped as it goes to nothing.
    reverb: Option<Box<Reverb>>,
}

#[derive(Default)]
struct Voice {
    /// The last mono samples, for the far ear.
    history: Vec<f32>,
    /// The back's low-pass, per ear.
    muffled: (f32, f32),
    idle: u32,
}

/// Constant-power gains for `x`, the middle at full in both ears.
fn pan(x: f32) -> (f32, f32) {
    let angle = (x + 1.0) * std::f32::consts::FRAC_PI_4;
    let root2 = std::f32::consts::SQRT_2;
    (
        (angle.cos() * root2).min(1.0),
        (angle.sin() * root2).min(1.0),
    )
}

impl Room {
    /// `user`'s 20 ms of interleaved stereo, from where they were put; none
    /// when they're in the middle, as they come.
    pub fn place(&mut self, user: u64, samples: &[i16]) -> Option<Vec<i16>> {
        let (x, y) = place(&user.to_string());
        if x == 0.0 && y == 0.0 {
            self.voices.remove(&user);
            return None;
        }
        let voice = self.voices.entry(user).or_default();
        voice.idle = 0;
        let (left, right) = pan(x);
        // Distance takes up to a third off; the back a quarter more.
        let distance = (x * x + y * y).sqrt() / std::f32::consts::SQRT_2;
        let behind = y.max(0.0);
        let gain = (1.0 - distance / 3.0) * (1.0 - behind / 4.0);
        // The far ear's delay, in samples.
        let delay = (x.abs() * MOST_DELAY as f32).round() as usize;
        // The back's low-pass: 0 lets all through, near 1 muffles.
        let muffle = behind * 0.7;
        let mono: Vec<f32> = samples
            .chunks_exact(2)
            .map(|pair| (f32::from(pair[0]) + f32::from(pair[1])) / 2.0)
            .collect();
        let past = voice.history.len();
        let mut out = Vec::with_capacity(samples.len());
        for (index, &now) in mono.iter().enumerate() {
            // What the far ear hears now: `delay` samples ago.
            let late = if index >= delay {
                mono[index - delay]
            } else {
                let back = delay - index;
                if back <= past {
                    voice.history[past - back]
                } else {
                    0.0
                }
            };
            let (heard_left, heard_right) = if x < 0.0 { (now, late) } else { (late, now) };
            let (low_left, low_right) = &mut voice.muffled;
            *low_left += (heard_left - *low_left) * (1.0 - muffle);
            *low_right += (heard_right - *low_right) * (1.0 - muffle);
            let clip = |sample: f32| {
                sample
                    .round()
                    .clamp(f32::from(i16::MIN), f32::from(i16::MAX)) as i16
            };
            out.push(clip(*low_left * left * gain));
            out.push(clip(*low_right * right * gain));
        }
        let keep = mono.len().min(MOST_DELAY);
        voice.history.clear();
        voice.history.extend_from_slice(&mono[mono.len() - keep..]);
        Some(out)
    }

    /// Ends a 20 ms frame: voices not heard for a second are forgotten.
    pub fn end_frame(&mut self) {
        self.voices.retain(|_, voice| {
            voice.idle += 1;
            voice.idle <= FORGET_AFTER
        });
    }

    /// Adds the room's reverb, at `amount` (0 to 1), to 20 ms of
    /// interleaved stereo.
    pub fn reverberate(&mut self, frame: &mut [i16], amount: f32) {
        if amount <= 0.0 {
            self.reverb = None;
            return;
        }
        self.reverb
            .get_or_insert_with(Box::default)
            .process(frame, amount);
    }

    /// Forgets everything, as room mode turns off.
    pub fn clear(&mut self) {
        self.voices.clear();
        self.reverb = None;
    }
}

/// A comb filter with damping, as Freeverb's.
struct Comb {
    buffer: Vec<f32>,
    at: usize,
    store: f32,
}

impl Comb {
    fn new(length: usize) -> Self {
        Self {
            buffer: vec![0.0; length],
            at: 0,
            store: 0.0,
        }
    }

    fn process(&mut self, input: f32, feedback: f32, damp: f32) -> f32 {
        let output = self.buffer[self.at];
        self.store = output * (1.0 - damp) + self.store * damp;
        self.buffer[self.at] = input + self.store * feedback;
        self.at = (self.at + 1) % self.buffer.len();
        output
    }
}

/// An all-pass, as Freeverb's.
struct AllPass {
    buffer: Vec<f32>,
    at: usize,
}

impl AllPass {
    fn new(length: usize) -> Self {
        Self {
            buffer: vec![0.0; length],
            at: 0,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let buffered = self.buffer[self.at];
        let output = buffered - input;
        self.buffer[self.at] = input + buffered * 0.5;
        self.at = (self.at + 1) % self.buffer.len();
        output
    }
}

/// Freeverb's combs and all-passes for each ear, the right's a little
/// longer so the room sounds wide.
struct Reverb {
    combs: [Vec<Comb>; 2],
    passes: [Vec<AllPass>; 2],
}

const COMBS: [usize; 6] = [1557, 1617, 1491, 1422, 1277, 1356];
const PASSES: [usize; 3] = [556, 441, 341];
const SPREAD: usize = 23;

impl Default for Reverb {
    fn default() -> Self {
        let ear = |spread: usize| {
            (
                COMBS
                    .iter()
                    .map(|length| Comb::new(length + spread))
                    .collect(),
                PASSES
                    .iter()
                    .map(|length| AllPass::new(length + spread))
                    .collect(),
            )
        };
        let (left_combs, left_passes) = ear(0);
        let (right_combs, right_passes) = ear(SPREAD);
        Self {
            combs: [left_combs, right_combs],
            passes: [left_passes, right_passes],
        }
    }
}

impl Reverb {
    fn process(&mut self, frame: &mut [i16], amount: f32) {
        // A larger room as it goes up: longer tails, less damping.
        let feedback = 0.7 + amount * 0.18;
        let damp = 0.4 - amount * 0.2;
        let wet = amount * 0.9;
        let dry = 1.0 - amount * 0.35;
        for pair in frame.chunks_exact_mut(2) {
            let input = (f32::from(pair[0]) + f32::from(pair[1])) * 0.015;
            for (ear, sample) in pair.iter_mut().enumerate() {
                let mut out: f32 = self.combs[ear]
                    .iter_mut()
                    .map(|comb| comb.process(input, feedback, damp))
                    .sum();
                for pass in &mut self.passes[ear] {
                    out = pass.process(out);
                }
                let mixed = f32::from(*sample) * dry + out * wet;
                *sample = mixed
                    .round()
                    .clamp(f32::from(i16::MIN), f32::from(i16::MAX))
                    as i16;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level(samples: impl Iterator<Item = i16>) -> f32 {
        samples.map(|sample| f32::from(sample).abs()).sum()
    }

    #[test]
    fn a_voice_on_the_left_is_heard_in_the_left_ear() {
        let tone: Vec<i16> = (0..960)
            .flat_map(|at| {
                let sample = ((at as f32 * 0.05).sin() * 8000.0) as i16;
                [sample, sample]
            })
            .collect();
        set_place("41", -1.0, 0.0);
        let mut room = Room::default();
        let placed = room.place(41, &tone).expect("placed");
        let left = level(placed.iter().step_by(2).copied());
        let right = level(placed.iter().skip(1).step_by(2).copied());
        assert!(left > right * 10.0, "left {left}, right {right}");
        // In the middle, as it comes.
        set_place("41", 0.0, 0.0);
        assert!(room.place(41, &tone).is_none());
    }

    #[test]
    fn the_back_is_quieter_than_the_front() {
        let tone: Vec<i16> = (0..960)
            .flat_map(|at| {
                let sample = ((at as f32 * 0.9).sin() * 8000.0) as i16;
                [sample, sample]
            })
            .collect();
        set_place("42", 0.0, -1.0);
        set_place("43", 0.0, 1.0);
        let mut room = Room::default();
        let front = level(room.place(42, &tone).unwrap().into_iter());
        let back = level(room.place(43, &tone).unwrap().into_iter());
        assert!(back < front * 0.7, "front {front}, back {back}");
    }

    #[test]
    fn reverb_rings_on_after_the_voice_stops() {
        let mut room = Room::default();
        let mut loud: Vec<i16> = (0..1920)
            .map(|at| ((at as f32 * 0.3).sin() * 12000.0) as i16)
            .collect();
        room.reverberate(&mut loud, 0.8);
        let mut silence = vec![0i16; 1920];
        room.reverberate(&mut silence, 0.8);
        assert!(level(silence.iter().copied()) > 0.0, "a tail");
        // Off: nothing added, and the tail forgotten.
        let mut quiet = vec![0i16; 1920];
        room.reverberate(&mut quiet, 0.0);
        assert!(quiet.iter().all(|sample| *sample == 0));
    }
}
