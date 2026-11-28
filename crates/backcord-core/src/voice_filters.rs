//! Filters for people's voices in a Discord call, before the compressor and
//! the voice limiter, as Voice & video chooses them: a high-pass at
//! [`HIGH_PASS`] that takes out rumble, hum and breath under the voice, and
//! a gate that silences a voice under a level, so the noise in someone's
//! pauses doesn't play. Bots go through neither. Neither adds latency: the
//! high-pass is a second-order IIR filter, the gate follows the voice as it
//! comes.

/// The high-pass's cutoff, in Hz.
pub const HIGH_PASS: f32 = 100.0;
const SAMPLE_RATE: f32 = 48_000.0;

/// A 12 dB per octave Butterworth high-pass, one per voice, on both
/// channels.
#[derive(Clone, Debug)]
pub struct HighPass {
    b: [f64; 3],
    a: [f64; 2],
    /// Each channel's last two inputs and outputs.
    state: [[f64; 4]; 2],
    cutoff: f32,
}

/// The cutoffs Voice & video offers, in Hz: rumble and hum; plosives and
/// a close microphone's boom too; and all a voice's body under its words.
pub const HIGH_PASS_CUTOFFS: [u16; 3] = [100, 150, 200];

impl Default for HighPass {
    fn default() -> Self {
        Self::new(HIGH_PASS)
    }
}

impl HighPass {
    /// A high-pass at `cutoff` Hz.
    pub fn new(cutoff: f32) -> Self {
        let w0 = 2.0 * std::f64::consts::PI * f64::from(cutoff) / f64::from(SAMPLE_RATE);
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / (2.0 * std::f64::consts::FRAC_1_SQRT_2);
        let a0 = 1.0 + alpha;
        Self {
            b: [
                (1.0 + cos) / 2.0 / a0,
                -(1.0 + cos) / a0,
                (1.0 + cos) / 2.0 / a0,
            ],
            a: [-2.0 * cos / a0, (1.0 - alpha) / a0],
            state: [[0.0; 4]; 2],
            cutoff,
        }
    }

    pub fn cutoff(&self) -> f32 {
        self.cutoff
    }
    /// Filters `samples`, 48 kHz interleaved stereo, in place.
    pub fn process(&mut self, samples: &mut [i16]) {
        for pair in samples.chunks_exact_mut(2) {
            for (sample, state) in pair.iter_mut().zip(&mut self.state) {
                let [x1, x2, y1, y2] = *state;
                let x = f64::from(*sample);
                let y = self.b[0] * x + self.b[1] * x1 + self.b[2] * x2
                    - self.a[0] * y1
                    - self.a[1] * y2;
                *state = [x, x1, y, y1];
                *sample = y.round().clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16;
            }
        }
    }
}

/// How quickly the gate opens and closes, in seconds; how long it stays
/// open once the voice is under its level; and how far under, in dB, the
/// voice must go to close it, so a voice at the level doesn't flutter.
const GATE_OPEN: f32 = 0.001;
const GATE_CLOSE: f32 = 0.08;
const GATE_HOLD: f32 = 0.15;
const GATE_HYSTERESIS: f32 = 4.0;
/// How quickly the gate's level falls after a peak, in seconds.
const GATE_LEVEL_FALL: f32 = 0.02;

/// A noise gate, one per voice: it silences the voice while it stays under
/// the level, both channels together, opening in 1 ms and closing over
/// 80 ms after 150 ms of hold, so words' ends aren't cut.
#[derive(Clone, Debug, Default)]
pub struct Gate {
    /// The voice's level, a peak that falls slowly, in full scale.
    level: f32,
    open: bool,
    /// Samples it stays open for, after the voice went under the level.
    hold: u32,
    /// The gain applied now, 0 shut to 1 open.
    gain: f32,
}

fn coefficient(seconds: f32) -> f32 {
    1.0 - (-1.0 / (seconds * SAMPLE_RATE)).exp()
}

impl Gate {
    /// Gates `samples`, 48 kHz interleaved stereo, in place, at
    /// `threshold` dBFS.
    pub fn process(&mut self, samples: &mut [i16], threshold: f32) {
        let open_at = 10f32.powf(threshold / 20.0);
        let close_at = 10f32.powf((threshold - GATE_HYSTERESIS) / 20.0);
        let fall = coefficient(GATE_LEVEL_FALL);
        let (opening, closing) = (coefficient(GATE_OPEN), coefficient(GATE_CLOSE));
        let hold = (GATE_HOLD * SAMPLE_RATE) as u32;
        for pair in samples.chunks_exact_mut(2) {
            let peak = f32::from(pair[0].unsigned_abs().max(pair[1].unsigned_abs())) / 32768.0;
            if peak > self.level {
                self.level = peak;
            } else {
                self.level -= self.level * fall;
            }
            if self.level >= open_at {
                self.open = true;
                self.hold = hold;
            } else if self.level < close_at {
                if self.hold > 0 {
                    self.hold -= 1;
                } else {
                    self.open = false;
                }
            }
            let (target, step) = if self.open {
                (1.0, opening)
            } else {
                (0.0, closing)
            };
            self.gain += (target - self.gain) * step;
            for sample in pair {
                *sample = (f32::from(*sample) * self.gain).round() as i16;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(amplitude: f32, hz: f32, frames: usize) -> Vec<i16> {
        (0..frames)
            .flat_map(|at| {
                let phase = 2.0 * std::f32::consts::PI * hz * at as f32 / SAMPLE_RATE;
                let value = (amplitude * phase.sin()) as i16;
                [value, value]
            })
            .collect()
    }

    fn rms(samples: &[i16]) -> f32 {
        let sum: f64 = samples.iter().map(|s| f64::from(*s).powi(2)).sum();
        (sum / samples.len() as f64).sqrt() as f32
    }

    fn high_passed(hz: f32) -> f32 {
        let mut filter = HighPass::default();
        let input = tone(10_000.0, hz, 48_000);
        let mut output = input.clone();
        filter.process(&mut output);
        let tail = output.len() / 2;
        20.0 * (rms(&output[tail..]) / rms(&input[tail..])).log10()
    }

    #[test]
    fn a_higher_cutoff_takes_out_more_of_the_voices_body() {
        let at = |cutoff: f32, hz: f32| {
            let mut filter = HighPass::new(cutoff);
            let mut samples: Vec<i16> = (0..9_600)
                .flat_map(|at| {
                    let sample = ((at as f32 * std::f32::consts::TAU * hz / SAMPLE_RATE).sin()
                        * 10_000.0) as i16;
                    [sample, sample]
                })
                .collect();
            filter.process(&mut samples);
            let rms = |samples: &[i16]| {
                (samples
                    .iter()
                    .skip(4_800)
                    .map(|&s| f64::from(s).powi(2))
                    .sum::<f64>()
                    / (samples.len() - 4_800) as f64)
                    .sqrt()
            };
            20.0 * (rms(&samples) / (10_000.0 / 2f64.sqrt())).log10()
        };
        for cutoff in [100.0, 150.0, 200.0] {
            assert!(
                (at(cutoff, cutoff) + 3.0).abs() < 0.5,
                "{cutoff} Hz: {}",
                at(cutoff, cutoff)
            );
        }
        assert!(at(200.0, 120.0) < at(100.0, 120.0) - 6.0);
    }

    #[test]
    fn the_high_pass_takes_out_rumble_and_leaves_the_voice() {
        assert!(
            (high_passed(HIGH_PASS) + 3.0).abs() < 0.5,
            "-3 dB at the cutoff"
        );
        assert!(high_passed(25.0) < -20.0, "{}", high_passed(25.0));
        assert!(high_passed(1_000.0).abs() < 0.2);
    }

    #[test]
    fn the_gate_silences_what_is_under_its_level_and_lets_speech_through() {
        let mut gate = Gate::default();
        // Hiss at -50 dBFS under a -40 dB gate: shut.
        let mut hiss = tone(32768.0 * 10f32.powf(-50.0 / 20.0), 3_000.0, 9_600);
        gate.process(&mut hiss, -40.0);
        assert!(rms(&hiss[hiss.len() / 2..]) < 1.0);
        // Speech at -20 dBFS: open within a few ms, then as it comes.
        let speech = tone(32768.0 * 0.1, 300.0, 9_600);
        let mut out = speech.clone();
        gate.process(&mut out, -40.0);
        for (out, sample) in out[960..].iter().zip(&speech[960..]) {
            assert!((out - sample).abs() <= 2);
        }
        // Back to hiss: it stays open for the hold, then closes smoothly.
        let input = tone(32768.0 * 10f32.powf(-50.0 / 20.0), 3_000.0, 24_000);
        let mut hiss = input.clone();
        gate.process(&mut hiss, -40.0);
        let held = rms(&hiss[..2 * 4_800]);
        assert!(held > 50.0, "{held}");
        // Half a second on, closing smoothly: 25 dB down and falling.
        let left = rms(&hiss[hiss.len() - 4_800..]) / rms(&input);
        assert!(20.0 * left.log10() < -25.0, "{left}");
    }
}
