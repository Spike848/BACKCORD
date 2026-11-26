//! The voice limiter: each person's voice in a Discord call is held under
//! the ceiling Voice & video's Voice limiter sets, so someone shouting or
//! playing loud into their microphone plays no louder than it, while
//! everyone else plays as they come. Only voices: streams' sound, the
//! soundboard and BACKCORD's own sounds don't go through it.
//!
//! A hard limiter, one per voice, with no attack or release: each stereo
//! frame over the ceiling is turned down to it at once, both channels by
//! the same share, and the next under it plays as it comes.

/// One voice's limiter.
#[derive(Clone, Copy, Debug, Default)]
pub struct Limiter;

impl Limiter {
    /// Adds `samples`, 48 kHz interleaved stereo, at `gain` into `sum`,
    /// held under `ceiling`, in samples' units.
    pub fn add(&mut self, samples: &[i16], gain: f32, ceiling: f32, sum: &mut [f32]) {
        for (pair, total) in samples.chunks_exact(2).zip(sum.chunks_exact_mut(2)) {
            let (left, right) = (f32::from(pair[0]) * gain, f32::from(pair[1]) * gain);
            let peak = left.abs().max(right.abs());
            let down = if peak > ceiling { ceiling / peak } else { 1.0 };
            total[0] += left * down;
            total[1] += right * down;
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
        let mut limiter = Limiter;
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
        let mut limiter = Limiter;
        let mut sum = vec![0.0; quiet.len()];
        limiter.add(&quiet, 1.0, ceiling, &mut sum);
        for (out, sample) in sum.iter().zip(&quiet) {
            assert!((out - f32::from(*sample)).abs() < 0.01);
        }
    }

    #[test]
    fn the_voice_comes_back_up_at_once_after_a_shout() {
        let ceiling = ceiling(10f32.powf(-20.0 / 20.0));
        let mut limiter = Limiter;
        let mut sum = vec![0.0; 1920];
        limiter.add(&tone(32_000.0, 960), 1.0, ceiling, &mut sum);
        // The very next frame, quiet, plays in full.
        let quiet = tone(1_500.0, 960);
        let mut next = vec![0.0; quiet.len()];
        limiter.add(&quiet, 1.0, ceiling, &mut next);
        for (out, sample) in next.iter().zip(&quiet) {
            assert!((out - f32::from(*sample)).abs() < 0.01);
        }
    }
}
