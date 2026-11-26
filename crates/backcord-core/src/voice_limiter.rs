//! The voice limiter: each person's voice in a Discord call is held under
//! the ceiling Voice & video's Voice limiter sets, so someone shouting or
//! playing loud into their microphone plays no louder than it, while
//! everyone else plays as they come. Only voices: streams' sound, the
//! soundboard and BACKCORD's own sounds don't go through it.
//!
//! A look-ahead limiter, one per voice, that turns the gain, not the wave:
//! each voice is delayed by [`LOOK_AHEAD`] (5 ms), and the gain starts down
//! as a loud peak enters that delay, in a smooth ramp, so it is down to the
//! ceiling by the time the peak plays. The gain then comes back up over
//! [`RELEASE`] once the voice is quieter. Turning each sample down on its
//! own, as a hard limiter does, flattens the wave's tops and distorts the
//! voice; here the wave keeps its shape and only plays quieter.
//!
//! The gain applied to a sample is at most the share that brings it under
//! the ceiling: it is the mean, over the look-ahead, of a running minimum
//! over the look-ahead of each sample's share, and every minimum in that
//! mean covers the sample now leaving the delay.

use std::collections::VecDeque;

/// How far ahead the limiter looks, in stereo frames at 48 kHz: 5 ms.
pub const LOOK_AHEAD: usize = 240;
/// How long the gain takes to come most of the way back up, in seconds.
pub const RELEASE: f32 = 0.15;

/// One voice's limiter.
#[derive(Clone, Debug)]
pub struct Limiter {
    /// The last `LOOK_AHEAD - 1` stereo frames, at the voice's volume.
    delay: VecDeque<(f32, f32)>,
    /// Shares still in the look-ahead that a later smaller one doesn't
    /// cover, by sample number: their front is the window's minimum.
    shares: VecDeque<(u64, f32)>,
    /// The last `LOOK_AHEAD` minimums, and their sum.
    minimums: VecDeque<f32>,
    sum: f64,
    /// The gain applied now.
    gain: f32,
    /// Samples taken so far.
    at: u64,
}

impl Default for Limiter {
    fn default() -> Self {
        Self {
            delay: std::iter::repeat_n((0.0, 0.0), LOOK_AHEAD - 1).collect(),
            shares: VecDeque::with_capacity(LOOK_AHEAD + 1),
            minimums: std::iter::repeat_n(1.0, LOOK_AHEAD).collect(),
            sum: LOOK_AHEAD as f64,
            gain: 1.0,
            at: 0,
        }
    }
}

impl Limiter {
    /// Adds `samples`, 48 kHz interleaved stereo, at `gain` into `sum`,
    /// held under `ceiling`, in samples' units, `LOOK_AHEAD - 1` frames late.
    pub fn add(&mut self, samples: &[i16], gain: f32, ceiling: f32, sum: &mut [f32]) {
        let release = 1.0 - (-1.0 / (RELEASE * 48_000.0)).exp();
        for (pair, total) in samples.chunks_exact(2).zip(sum.chunks_exact_mut(2)) {
            let (left, right) = (f32::from(pair[0]) * gain, f32::from(pair[1]) * gain);
            let peak = left.abs().max(right.abs());
            let share = if peak > ceiling { ceiling / peak } else { 1.0 };
            self.delay.push_back((left, right));
            let (left, right) = self.delay.pop_front().unwrap_or_default();
            while self.shares.back().is_some_and(|&(_, back)| back >= share) {
                self.shares.pop_back();
            }
            self.shares.push_back((self.at, share));
            while self
                .shares
                .front()
                .is_some_and(|&(at, _)| at + LOOK_AHEAD as u64 <= self.at)
            {
                self.shares.pop_front();
            }
            let minimum = self.shares.front().map_or(1.0, |&(_, share)| share);
            self.minimums.push_back(minimum);
            self.sum += f64::from(minimum);
            if let Some(gone) = self.minimums.pop_front() {
                self.sum -= f64::from(gone);
            }
            let target = ((self.sum / LOOK_AHEAD as f64) as f32).min(1.0);
            if target < self.gain {
                self.gain = target;
            } else {
                self.gain += (target - self.gain) * release;
            }
            self.at += 1;
            total[0] += left * self.gain;
            total[1] += right * self.gain;
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

    const LATE: usize = (LOOK_AHEAD - 1) * 2;

    fn tone(amplitude: f32, frames: usize) -> Vec<i16> {
        (0..frames)
            .flat_map(|at| {
                let value = (amplitude * (at as f32 * 0.06).sin()) as i16;
                [value, value]
            })
            .collect()
    }

    fn run(limiter: &mut Limiter, samples: &[i16], ceiling: f32) -> Vec<f32> {
        let mut sum = vec![0.0; samples.len()];
        limiter.add(samples, 1.0, ceiling, &mut sum);
        sum
    }

    #[test]
    fn a_loud_voice_never_goes_over_the_ceiling() {
        let ceiling = ceiling(10f32.powf(-12.0 / 20.0));
        let loud = tone(32_000.0, 960);
        let mut limiter = Limiter::default();
        for _ in 0..10 {
            let sum = run(&mut limiter, &loud, ceiling);
            assert!(sum.iter().all(|sample| sample.abs() <= ceiling + 0.5));
        }
        // Nor a sudden shout after silence.
        let mut limiter = Limiter::default();
        run(&mut limiter, &vec![0; 1920], ceiling);
        let mut shout = vec![0i16; 1920];
        shout[1000..].fill(32_000);
        for _ in 0..2 {
            let sum = run(&mut limiter, &shout, ceiling);
            assert!(sum.iter().all(|sample| sample.abs() <= ceiling + 0.5));
        }
    }

    #[test]
    fn a_quiet_voice_plays_as_it_comes_a_little_late() {
        let ceiling = ceiling(10f32.powf(-12.0 / 20.0));
        let quiet = tone(2_000.0, 960);
        let mut limiter = Limiter::default();
        let sum = run(&mut limiter, &quiet, ceiling);
        assert!(sum[..LATE].iter().all(|sample| *sample == 0.0));
        for (out, sample) in sum[LATE..].iter().zip(&quiet) {
            assert!((out - f32::from(*sample)).abs() < 0.01);
        }
    }

    #[test]
    fn a_loud_voice_keeps_its_shape() {
        // Held down, a loud tone plays as the same tone, only quieter: its
        // wave isn't flattened at the ceiling.
        let ceiling = ceiling(10f32.powf(-20.0 / 20.0));
        let loud = tone(30_000.0, 960);
        let mut limiter = Limiter::default();
        let mut outs = Vec::new();
        for _ in 0..20 {
            outs.extend(run(&mut limiter, &loud, ceiling));
        }
        let ins: Vec<i16> = std::iter::repeat_n(loud.iter().copied(), 20)
            .flatten()
            .collect();
        let settled = 960 * 2 * 10;
        let share = ceiling / 30_000.0;
        for (out, sample) in outs[settled..].iter().zip(&ins[settled - LATE..]) {
            assert!(
                (out - f32::from(*sample) * share).abs() < 30.0,
                "{out} {sample}"
            );
        }
    }

    #[test]
    fn the_voice_comes_back_up_smoothly_after_a_shout() {
        let ceiling = ceiling(10f32.powf(-20.0 / 20.0));
        let mut limiter = Limiter::default();
        run(&mut limiter, &tone(32_000.0, 960), ceiling);
        let quiet = tone(1_500.0, 960);
        let mut gains = Vec::new();
        for _ in 0..50 {
            run(&mut limiter, &quiet, ceiling);
            gains.push(limiter.gain);
        }
        // Up a little each frame, never jumping, and all the way within a
        // second.
        assert!(gains.windows(2).all(|pair| pair[1] >= pair[0]));
        assert!(gains.windows(2).all(|pair| pair[1] - pair[0] < 0.2));
        assert!(gains[49] > 0.99);
    }
}
