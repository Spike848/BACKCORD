//! The voice limiter: each person's voice in a Discord call is held under
//! the ceiling Voice & video's Voice limiter sets, so someone shouting or
//! playing loud into their microphone plays no louder than it, while
//! everyone else plays as they come. Only voices: streams' sound, the
//! soundboard and BACKCORD's own sounds don't go through it.
//!
//! A peak limiter, one per voice: its level follows the voice's peaks,
//! rising over 1 ms and falling back over 200 ms, and the voice is turned
//! down by as much as that level is over the ceiling. A sample still over
//! it in that first millisecond is cut at it.

/// 48 kHz stereo frames of rising, about 1 ms.
const ATTACK: f32 = 0.020_6;
/// The share of the level kept each stereo frame as it falls, about 200 ms.
const RELEASE: f32 = 0.999_896;

/// One voice's limiter.
#[derive(Clone, Copy, Debug, Default)]
pub struct Limiter {
    /// The voice's peak level as it follows it, in samples' units.
    level: f32,
}

impl Limiter {
    /// Adds `samples`, 48 kHz interleaved stereo, at `gain` into `sum`,
    /// held under `ceiling`, in samples' units.
    pub fn add(&mut self, samples: &[i16], gain: f32, ceiling: f32, sum: &mut [f32]) {
        for (pair, total) in samples.chunks_exact(2).zip(sum.chunks_exact_mut(2)) {
            let (left, right) = (f32::from(pair[0]) * gain, f32::from(pair[1]) * gain);
            let peak = left.abs().max(right.abs());
            if peak > self.level {
                self.level += (peak - self.level) * ATTACK;
            } else {
                self.level *= RELEASE;
            }
            let down = if self.level > ceiling {
                ceiling / self.level
            } else {
                1.0
            };
            total[0] += (left * down).clamp(-ceiling, ceiling);
            total[1] += (right * down).clamp(-ceiling, ceiling);
        }
    }
}

/// The ceiling in samples' units for one given as a gain of full scale.
pub fn ceiling(gain: f32) -> f32 {
    gain * f32::from(i16::MAX)
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

    #[test]
    fn a_loud_voice_never_goes_over_the_ceiling() {
        let ceiling = ceiling(10f32.powf(-12.0 / 20.0));
        let loud = tone(32_000.0, 960);
        let mut limiter = Limiter::default();
        for _ in 0..10 {
            let mut sum = vec![0.0; loud.len()];
            limiter.add(&loud, 1.0, ceiling, &mut sum);
            assert!(sum.iter().all(|sample| sample.abs() <= ceiling + 0.5));
        }
    }

    #[test]
    fn a_quiet_voice_plays_as_it_comes() {
        let ceiling = ceiling(10f32.powf(-12.0 / 20.0));
        let quiet = tone(2_000.0, 960);
        let mut limiter = Limiter::default();
        let mut sum = vec![0.0; quiet.len()];
        limiter.add(&quiet, 1.0, ceiling, &mut sum);
        for (out, sample) in sum.iter().zip(&quiet) {
            assert!((out - f32::from(*sample)).abs() < 0.01);
        }
    }

    #[test]
    fn after_a_shout_the_voice_comes_back_up() {
        let ceiling = ceiling(10f32.powf(-20.0 / 20.0));
        let mut limiter = Limiter::default();
        let mut sum = vec![0.0; 1920];
        limiter.add(&tone(32_000.0, 960), 1.0, ceiling, &mut sum);
        // A second of quiet speech after it.
        let quiet = tone(1_500.0, 960);
        let mut last = vec![0.0; quiet.len()];
        for _ in 0..50 {
            last.iter_mut().for_each(|sample| *sample = 0.0);
            limiter.add(&quiet, 1.0, ceiling, &mut last);
        }
        let peak = last
            .iter()
            .fold(0f32, |most, sample| most.max(sample.abs()));
        assert!(peak > 1_400.0, "{peak}");
    }
}
