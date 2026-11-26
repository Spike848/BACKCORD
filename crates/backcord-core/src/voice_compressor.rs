//! The voice compressor: Voice & video's Voice compressor evens out each
//! person's voice in a Discord call, turning its loud parts down by a 4:1
//! ratio above -30 dBFS, so loud and quiet people play closer together.
//! Bots, music bots among them, aren't compressed; nor streams' sound, the
//! soundboard or BACKCORD's own sounds.
//!
//! Like the voice limiter it turns the gain, never each sample on its own:
//! the level is a short running mean of the voice's power, the gain follows
//! it down in [`ATTACK`] and back up in [`RELEASE`], with a soft knee, so
//! the wave keeps its shape. [`MAKEUP`] then brings speech at -20 dBFS back
//! to where it was: quieter voices come up, louder ones go down.

/// The level from which the voice is turned down, in dBFS.
pub const THRESHOLD: f32 = -30.0;
/// Above the threshold, 4 dB in come out as 1.
pub const RATIO: f32 = 4.0;
/// The soft knee's width, in dB, around the threshold.
const KNEE: f32 = 6.0;
/// How quickly the gain goes down and comes back up, in seconds.
pub const ATTACK: f32 = 0.005;
pub const RELEASE: f32 = 0.06;
/// How long the level's running mean of the power takes, in seconds.
const DETECTION: f32 = 0.005;
/// The gain added after compressing, in dB: what speech at -20 dBFS loses.
pub const MAKEUP: f32 = 7.5;
const SAMPLE_RATE: f32 = 48_000.0;

/// One voice's compressor.
#[derive(Clone, Debug, Default)]
pub struct Compressor {
    /// The running mean of the power, in full scale.
    power: f32,
    /// The gain change applied now, in dB, 0 or below.
    reduction: f32,
}

/// How far a level is turned down, in dB, before the makeup.
fn reduction(level: f32) -> f32 {
    let over = level - THRESHOLD;
    let slope = 1.0 / RATIO - 1.0;
    if 2.0 * over < -KNEE {
        0.0
    } else if 2.0 * over.abs() <= KNEE {
        slope * (over + KNEE / 2.0).powi(2) / (2.0 * KNEE)
    } else {
        slope * over
    }
}

fn coefficient(seconds: f32) -> f32 {
    1.0 - (-1.0 / (seconds * SAMPLE_RATE)).exp()
}

impl Compressor {
    /// Compresses `samples`, 48 kHz interleaved stereo, in place, both
    /// channels by the same gain.
    pub fn process(&mut self, samples: &mut [i16]) {
        let detection = coefficient(DETECTION);
        let attack = coefficient(ATTACK);
        let release = coefficient(RELEASE);
        for pair in samples.chunks_exact_mut(2) {
            let peak = f32::from(pair[0].unsigned_abs().max(pair[1].unsigned_abs())) / 32768.0;
            self.power += (peak * peak - self.power) * detection;
            let level = 10.0 * (self.power + 1e-12).log10();
            let target = reduction(level);
            let step = if target < self.reduction {
                attack
            } else {
                release
            };
            self.reduction += (target - self.reduction) * step;
            let gain = 10f32.powf((self.reduction + MAKEUP) / 20.0);
            for sample in pair {
                *sample = (f32::from(*sample) * gain)
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

    fn tone(amplitude: f32, frames: usize) -> Vec<i16> {
        (0..frames)
            .flat_map(|at| {
                let value = (amplitude * (at as f32 * 0.06).sin()) as i16;
                [value, value]
            })
            .collect()
    }

    fn rms(samples: &[i16]) -> f32 {
        let sum: f64 = samples.iter().map(|s| f64::from(*s).powi(2)).sum();
        let rms = (sum / samples.len() as f64).sqrt() as f32 / 32768.0;
        20.0 * rms.log10()
    }

    /// The level a steady tone settles at.
    fn settled(amplitude: f32) -> (f32, f32) {
        let mut compressor = Compressor::default();
        let input = tone(amplitude, 48_000);
        let mut output = input.clone();
        compressor.process(&mut output);
        let tail = output.len() / 2;
        (rms(&input[tail..]), rms(&output[tail..]))
    }

    #[test]
    fn above_the_threshold_four_db_come_out_as_one() {
        let (loud_in, loud_out) = settled(16_000.0);
        let (less_in, less_out) = settled(8_000.0);
        let ratio = (loud_in - less_in) / (loud_out - less_out);
        assert!((ratio - RATIO).abs() < 0.4, "{ratio}");
    }

    #[test]
    fn speech_at_minus_twenty_keeps_its_level_and_shouting_goes_down() {
        // A tone's power level is its RMS: -20 dBFS RMS.
        let amplitude = 32768.0 * 10f32.powf(-20.0 / 20.0) * 2f32.sqrt();
        let (input, output) = settled(amplitude);
        assert!((output - input).abs() < 1.0, "{input} {output}");
        let (input, output) = settled(30_000.0);
        assert!(output < input - 8.0, "{input} {output}");
    }

    #[test]
    fn a_loud_voice_keeps_its_shape() {
        let mut compressor = Compressor::default();
        let input = tone(30_000.0, 48_000);
        let mut output = input.clone();
        compressor.process(&mut output);
        let tail = output.len() / 2;
        let share = rms(&output[tail..]) - rms(&input[tail..]);
        let share = 10f32.powf(share / 20.0);
        for (out, sample) in output[tail..].iter().zip(&input[tail..]) {
            assert!((f32::from(*out) - f32::from(*sample) * share).abs() < 300.0);
        }
    }

    #[test]
    fn it_comes_back_up_quickly_after_a_shout() {
        let mut compressor = Compressor::default();
        let mut shout = tone(30_000.0, 9_600);
        compressor.process(&mut shout);
        assert!(compressor.reduction < -15.0);
        let mut quiet = tone(300.0, 14_400);
        compressor.process(&mut quiet);
        assert!(compressor.reduction > -0.5, "{}", compressor.reduction);
    }
}
