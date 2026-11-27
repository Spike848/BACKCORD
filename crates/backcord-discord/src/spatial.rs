//! Where the microphone sounds in the others' ears, as Discord calls send
//! it in stereo: widened, so a voice from one microphone fills the space
//! between their ears, and panned, from the left ear through the middle to
//! the right. Both act on the voice as it goes out, whatever the microphone
//! gave: its two channels are taken as one first.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

static WIDE: AtomicBool = AtomicBool::new(false);
/// The pan, -1 (left) to 1 (right), as an f32's bits; 0 in the middle.
static PAN: AtomicU32 = AtomicU32::new(0);

/// Widens the microphone in every call, or leaves it as it is.
pub fn set_wide(on: bool) {
    WIDE.store(on, Ordering::Relaxed);
}

pub fn wide() -> bool {
    WIDE.load(Ordering::Relaxed)
}

/// Places the microphone from -1, the left ear alone, to 1, the right.
pub fn set_pan(pan: f32) {
    let pan = if pan.is_finite() {
        pan.clamp(-1.0, 1.0)
    } else {
        0.0
    };
    PAN.store(pan.to_bits(), Ordering::Relaxed);
}

pub fn pan() -> f32 {
    f32::from_bits(PAN.load(Ordering::Relaxed))
}

/// How wide, 0 to 1, as an f32's bits: three quarters unless set.
static WIDTH: AtomicU32 = AtomicU32::new(0x3F40_0000);

/// How wide the widener makes the microphone, from 0 (not at all) to 1.
pub fn set_width(width: f32) {
    let width = if width.is_finite() {
        width.clamp(0.0, 1.0)
    } else {
        DEFAULT_WIDTH
    };
    WIDTH.store(width.to_bits(), Ordering::Relaxed);
}

pub fn width() -> f32 {
    f32::from_bits(WIDTH.load(Ordering::Relaxed))
}

pub const DEFAULT_WIDTH: f32 = 0.75;

/// The bit depth the microphone goes out at: 32 is as it is, in floats.
static BITS: AtomicU32 = AtomicU32::new(FLOAT_BITS);
pub const FLOAT_BITS: u32 = 32;

/// Rounds the microphone to `bits`, 1 to 31, or leaves it whole at 32.
pub fn set_bits(bits: u32) {
    BITS.store(bits.clamp(1, FLOAT_BITS), Ordering::Relaxed);
}

pub fn bits() -> u32 {
    BITS.load(Ordering::Relaxed)
}

/// The rate the microphone is sampled down to, in Hz: 48 000 is as it is.
static SAMPLE_RATE: AtomicU32 = AtomicU32::new(FULL_RATE);
pub const FULL_RATE: u32 = 48_000;

/// Samples the microphone down to `rate` Hz, held between samples and
/// never smoothed, as old games' voices were: 48 000 leaves it as it is.
pub fn set_sample_rate(rate: u32) {
    SAMPLE_RATE.store(rate.clamp(500, FULL_RATE), Ordering::Relaxed);
}

pub fn sample_rate() -> u32 {
    SAMPLE_RATE.load(Ordering::Relaxed)
}

/// Rounds each sample to `bits`: 1 keeps only its sign, at half the
/// loudest, a hush staying silent.
fn crush(pcm: &mut [f32], bits: u32) {
    if bits >= FLOAT_BITS {
        return;
    }
    if bits == 1 {
        for sample in pcm {
            *sample = if sample.abs() < 1e-3 {
                0.0
            } else {
                0.5f32.copysign(*sample)
            };
        }
        return;
    }
    let steps = (1u64 << (bits - 1)) as f32;
    for sample in pcm {
        *sample = ((*sample * steps).round() / steps).clamp(-1.0, 1.0);
    }
}
const RATE: f32 = 48_000.0;
/// Where the voice's bands part, in Hz. Every band is widened, the bass
/// too; above, each band leans
/// the ears' phases the other way from the one before, so the voice
/// spreads to both sides at once and never leans to either.
const CROSSOVERS: [f32; SPLITS] = [80.0, 250.0, 800.0, 2_000.0, 5_000.0];
const SPLITS: usize = 5;
/// Each band's side, by the crossovers, turn about.
const SIDES: [f32; SPLITS + 1] = [1.0, -1.0, 1.0, -1.0, 1.0, -1.0];
/// The two chains of a Hilbert pair (Olli Niemitalo's): their outputs are a
/// quarter round apart at every frequency a voice has, as loud.
const HILBERT_REAL: [f32; 4] = [0.692_387_8, 0.936_065_4, 0.988_229_5, 0.998_748_8];
const HILBERT_QUADRATURE: [f32; 4] = [0.402_192_1, 0.856_171_1, 0.972_290_9, 0.995_288_5];

/// A biquad, transposed direct form II.
#[derive(Clone, Copy)]
struct Biquad {
    b: [f32; 3],
    a: [f32; 2],
    state: [f32; 2],
}

#[derive(Clone, Copy)]
enum Shape {
    Low,
    High,
    All,
}

impl Biquad {
    /// Butterworth's, Q one over root two, as Linkwitz-Riley's halves are.
    fn new(shape: Shape, corner: f32) -> Self {
        let omega = std::f32::consts::TAU * corner / RATE;
        let (sin, cos) = omega.sin_cos();
        let alpha = sin / std::f32::consts::SQRT_2;
        let a0 = 1.0 + alpha;
        let b = match shape {
            Shape::Low => [(1.0 - cos) / 2.0, 1.0 - cos, (1.0 - cos) / 2.0],
            Shape::High => [(1.0 + cos) / 2.0, -(1.0 + cos), (1.0 + cos) / 2.0],
            Shape::All => [1.0 - alpha, -2.0 * cos, 1.0 + alpha],
        };
        Self {
            b: b.map(|b| b / a0),
            a: [-2.0 * cos / a0, (1.0 - alpha) / a0],
            state: [0.0; 2],
        }
    }

    fn run(&mut self, input: f32) -> f32 {
        let output = self.b[0] * input + self.state[0];
        self.state[0] = self.b[1] * input - self.a[0] * output + self.state[1];
        self.state[1] = self.b[2] * input - self.a[1] * output;
        output
    }
}

/// A Linkwitz-Riley filter of the fourth order: two Butterworths in a row.
/// Its low and high halves stay in step at every frequency and add up to
/// an all-pass, so the bands can be signed and summed without a dip.
#[derive(Clone, Copy)]
struct Riley([Biquad; 2]);

impl Riley {
    fn new(shape: Shape, corner: f32) -> Self {
        Self([Biquad::new(shape, corner); 2])
    }

    fn run(&mut self, input: f32) -> f32 {
        let once = self.0[0].run(input);
        self.0[1].run(once)
    }
}

/// One chain of a Hilbert pair: all-passes on every other sample.
struct Hilbert {
    squares: [f32; 4],
    inputs: [[f32; 2]; 4],
    outputs: [[f32; 2]; 4],
}

impl Hilbert {
    fn new(coefficients: [f32; 4]) -> Self {
        Self {
            squares: coefficients.map(|a| a * a),
            inputs: [[0.0; 2]; 4],
            outputs: [[0.0; 2]; 4],
        }
    }

    fn run(&mut self, input: f32) -> f32 {
        let mut sound = input;
        for stage in 0..4 {
            let [last, before] = self.inputs[stage];
            let [out_last, out_before] = self.outputs[stage];
            let output = self.squares[stage] * (sound + out_before) - before;
            self.inputs[stage] = [sound, last];
            self.outputs[stage] = [output, out_last];
            sound = output;
        }
        sound
    }
}

/// One call's widening: the voice split into bands, each band's side, and
/// the Hilbert pair that turns the side a quarter round from the middle.
/// Each ear then hears every frequency exactly as loud as the other, only
/// its phase leaning one way or the other by band: wide, and centred.
pub struct Spatial {
    /// Per crossover, from the highest: its low and high halves.
    lows: [Riley; SPLITS],
    highs: [Riley; SPLITS],
    /// Each band's all-passes, so all of them stay in step: band k, split
    /// off at crossover k, still owes the crossovers under it.
    owed: Vec<Vec<Biquad>>,
    real: Hilbert,
    quadrature: Hilbert,
    /// The real chain's output a sample late, as the pair wants.
    late: f32,
    /// The sample held down to a lower rate, and how far to the next.
    held: [f32; 2],
    hold: f32,
}

impl Default for Spatial {
    fn default() -> Self {
        Self::new()
    }
}

impl Spatial {
    pub fn new() -> Self {
        // Split from the top: the highest band first, then what's below.
        let mut from_top = CROSSOVERS;
        from_top.reverse();
        Self {
            lows: from_top.map(|corner| Riley::new(Shape::Low, corner)),
            highs: from_top.map(|corner| Riley::new(Shape::High, corner)),
            owed: (0..SPLITS)
                .map(|split| {
                    from_top[split + 1..]
                        .iter()
                        .map(|&corner| Biquad::new(Shape::All, corner))
                        .collect()
                })
                .collect(),
            real: Hilbert::new(HILBERT_REAL),
            quadrature: Hilbert::new(HILBERT_QUADRATURE),
            late: 0.0,
            held: [0.0; 2],
            hold: 1.0,
        }
    }

    /// Widens and pans one frame of interleaved stereo, as set. Nothing is
    /// done while neither is.
    pub fn process(&mut self, pcm: &mut [f32]) {
        self.apply(pcm, wide(), width(), pan());
        self.sample_down(pcm, sample_rate());
        crush(pcm, bits());
    }

    /// Holds each sample for as long as `rate` leaves between them.
    fn sample_down(&mut self, pcm: &mut [f32], rate: u32) {
        if rate >= FULL_RATE {
            self.hold = 1.0;
            return;
        }
        let step = rate as f32 / FULL_RATE as f32;
        for frame in pcm.chunks_exact_mut(2) {
            if self.hold >= 1.0 {
                self.hold -= 1.0;
                self.held = [frame[0], frame[1]];
            }
            self.hold += step;
            frame.copy_from_slice(&self.held);
        }
    }

    /// The voice, and its side: its bands, each signed.
    fn bands(&mut self, input: f32) -> (f32, f32) {
        let (mut rest, mut whole, mut side) = (input, 0.0, 0.0);
        for split in 0..SPLITS {
            let mut band = self.highs[split].run(rest);
            rest = self.lows[split].run(rest);
            for pass in &mut self.owed[split] {
                band = pass.run(band);
            }
            whole += band;
            // From the top: the highest band is the last of SIDES.
            side += SIDES[SPLITS - split] * band;
        }
        whole += rest;
        side += SIDES[0] * rest;
        (whole, side)
    }

    fn apply(&mut self, pcm: &mut [f32], wide: bool, width: f32, pan: f32) {
        if !wide && pan == 0.0 {
            return;
        }
        // Balance: the far side fades out, the near one stays as it is.
        let (left_gain, right_gain) = ((1.0 - pan).min(1.0), (1.0 + pan).min(1.0));
        // Above the bass each ear's as loud as 1 + width² of the voice, the
        // bass as it was: taken back halfway between, so neither moves much.
        let loudness = (1.0 + 0.5 * width * width).sqrt().recip();
        for frame in pcm.chunks_exact_mut(2) {
            let (left, right) = if wide {
                let mid = (frame[0] + frame[1]) * 0.5;
                let (whole, side) = self.bands(mid);
                let middle = std::mem::replace(&mut self.late, self.real.run(whole));
                let side = self.quadrature.run(side) * width;
                ((middle + side) * loudness, (middle - side) * loudness)
            } else {
                (frame[0], frame[1])
            };
            frame[0] = (left * left_gain).clamp(-1.0, 1.0);
            frame[1] = (right * right_gain).clamp(-1.0, 1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A second of a 440 Hz tone, the same in both channels.
    fn tone() -> Vec<f32> {
        (0..48_000)
            .flat_map(|at| {
                let sample = (at as f32 * std::f32::consts::TAU * 440.0 / RATE).sin() * 0.3;
                [sample, sample]
            })
            .collect()
    }

    #[test]
    fn neither_set_leaves_the_voice_as_it_is() {
        let mut pcm = tone();
        let before = pcm.clone();
        Spatial::new().apply(&mut pcm, false, DEFAULT_WIDTH, 0.0);
        assert_eq!(pcm, before);
    }

    /// Three seconds of a voice's band: noise, warm, 120 to 3500 Hz.
    fn voice() -> Vec<f32> {
        let mut seed = 7u32;
        let (mut pink, mut low, mut high) = (0f32, 0f32, 0f32);
        let top = 1.0 - (-std::f32::consts::TAU * 3_500.0 / RATE).exp();
        let bottom = 1.0 - (-std::f32::consts::TAU * 120.0 / RATE).exp();
        (0..48_000 * 3)
            .flat_map(|_| {
                seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                let white = (seed >> 8) as f32 / (1u32 << 24) as f32 * 2.0 - 1.0;
                pink = 0.97 * pink + white * 0.1;
                low += top * (pink - low);
                high += bottom * (low - high);
                let sample = low - high;
                [sample, sample]
            })
            .collect()
    }

    fn energy(pcm: &[f32], channel: usize) -> f32 {
        pcm.chunks(2)
            .skip(48_000)
            .map(|frame| frame[channel] * frame[channel])
            .sum()
    }

    #[test]
    fn widening_parts_the_ears_evenly_and_keeps_the_voice_as_loud() {
        let before = voice();
        let mut pcm = before.clone();
        Spatial::new().apply(&mut pcm, true, 1.0, 0.0);
        let differ = pcm
            .chunks(2)
            .filter(|frame| (frame[0] - frame[1]).abs() > 0.001)
            .count();
        assert!(differ > 100_000, "wide: {differ} frames differ");
        let (left, right) = (energy(&pcm, 0), energy(&pcm, 1));
        let tilt = 10.0 * (left / right).log10();
        assert!(tilt.abs() < 0.2, "one ear louder by {tilt} dB");
        let louder = 10.0 * ((left + right) / 2.0 / energy(&before, 0)).log10();
        assert!(louder.abs() < 1.0, "the voice changed by {louder} dB");
    }

    /// Every pitch, not just the voice's sum, is as loud in either ear.
    #[test]
    fn every_pitch_is_as_loud_in_either_ear() {
        for pitch in [
            60.0, 100.0, 150.0, 330.0, 600.0, 1_100.0, 2_600.0, 4_000.0, 8_000.0,
        ] {
            let mut pcm: Vec<f32> = (0..96_000)
                .flat_map(|at| {
                    let sample = (at as f32 * std::f32::consts::TAU * pitch / RATE).sin() * 0.3;
                    [sample, sample]
                })
                .collect();
            Spatial::new().apply(&mut pcm, true, 1.0, 0.0);
            let tilt = 10.0 * (energy(&pcm, 0) / energy(&pcm, 1)).log10();
            assert!(tilt.abs() < 0.3, "{pitch} Hz: one ear louder by {tilt} dB");
        }
    }

    #[test]
    fn the_bit_depth_rounds_the_voice_or_leaves_it() {
        let mut whole = vec![0.3, -0.123_456];
        crush(&mut whole, FLOAT_BITS);
        assert_eq!(whole, [0.3, -0.123_456]);
        let mut two = vec![0.3, -0.8, 0.0];
        crush(&mut two, 2);
        assert_eq!(two, [0.5, -1.0, 0.0]);
        let mut one = vec![0.3, -0.01, 0.0];
        crush(&mut one, 1);
        assert_eq!(one, [0.5, -0.5, 0.0]);
    }

    #[test]
    fn sampling_down_holds_each_sample() {
        let mut pcm: Vec<f32> = (0..12).flat_map(|at| [at as f32, at as f32]).collect();
        Spatial::new().sample_down(&mut pcm, 12_000);
        let left: Vec<f32> = pcm.chunks(2).map(|frame| frame[0]).collect();
        assert_eq!(
            left,
            [0.0, 0.0, 0.0, 0.0, 4.0, 4.0, 4.0, 4.0, 8.0, 8.0, 8.0, 8.0]
        );
    }

    /// Through Opus as a call sends it and back as a listener decodes it:
    /// panned left, the right ear stays quiet; widened, the ears differ.
    #[test]
    fn the_listeners_hear_it_panned_and_widened_through_opus() {
        let through = |pcm: &[f32]| -> Vec<i16> {
            let mut encoder =
                crate::opus::Encoder::new(2, crate::opus::Application::Audio, 128_000).unwrap();
            encoder.force_stereo(true);
            let mut decoder = crate::opus::Decoder::new(2).unwrap();
            let mut heard = Vec::new();
            for frame in pcm.chunks_exact(960 * 2) {
                let mut packet = vec![0; 4000];
                let length = encoder.encode_float(frame, &mut packet).unwrap();
                assert!(packet[0] & 0x04 != 0, "a stereo packet");
                let mut out = vec![0i16; 960 * 2];
                let samples = decoder.decode(&packet[..length], &mut out, false).unwrap();
                heard.extend_from_slice(&out[..samples * 2]);
            }
            heard
        };
        let level = |pcm: &[i16], channel: usize| -> f64 {
            pcm.chunks(2)
                .skip(48_000)
                .map(|frame| f64::from(frame[channel]).powi(2))
                .sum()
        };
        let mut left = voice();
        Spatial::new().apply(&mut left, false, 0.0, -1.0);
        let heard = through(&left);
        let quiet = 10.0 * (level(&heard, 1) / level(&heard, 0)).log10();
        assert!(quiet < -30.0, "the right ear at {quiet} dB");
        let mut wide = voice();
        Spatial::new().apply(&mut wide, true, 1.0, 0.0);
        let heard = through(&wide);
        let apart = heard
            .chunks(2)
            .filter(|frame| (i32::from(frame[0]) - i32::from(frame[1])).abs() > 300)
            .count();
        assert!(apart > 50_000, "the ears apart in {apart} frames");
    }

    #[test]
    fn no_width_leaves_the_ears_alike() {
        let mut pcm = voice();
        Spatial::new().apply(&mut pcm, true, 0.0, 0.0);
        assert!(
            pcm.chunks(2)
                .all(|frame| (frame[0] - frame[1]).abs() < 1e-6)
        );
    }

    #[test]
    fn panning_moves_the_voice_between_the_ears() {
        let level = |pcm: &[f32], channel: usize| -> f32 {
            pcm.chunks(2).map(|frame| frame[channel].abs()).sum()
        };
        let mut left = tone();
        Spatial::new().apply(&mut left, false, DEFAULT_WIDTH, -1.0);
        assert!(
            level(&left, 1) == 0.0 && level(&left, 0) > 0.0,
            "left ear alone"
        );
        let mut halfway = tone();
        Spatial::new().apply(&mut halfway, false, DEFAULT_WIDTH, 0.5);
        let (l, r) = (level(&halfway, 0), level(&halfway, 1));
        assert!((l / r - 0.5).abs() < 1e-3, "half as loud on the left");
    }
}
