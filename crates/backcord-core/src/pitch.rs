//! A person's voice raised or lowered in a call, as their menu's Pitch
//! chooses, in semitones: 0 is off and costs nothing. Two read heads run
//! through a short delay line at the new speed, half a window apart, each
//! faded in and out so one is always whole while the other jumps back.
//! Where a head jumps back to is chosen so its wave lines up with the other
//! head's (the place in a short span that matches it best), so the two
//! never cancel as they cross: that keeps the voice natural rather than
//! robotic. The delay this adds is at most [`WINDOW`] and [`SEARCH`], 1024
//! samples or about 21 ms, and on average about half that. Formants move
//! with the pitch, as a voice changer's do.

/// The heads' window, in samples at 48 kHz: 16 ms.
pub const WINDOW: usize = 768;
/// How far past its window a head may start to line up with the other:
/// about 5 ms, a pitch period of a voice at 190 Hz.
pub const SEARCH: usize = 256;
/// The samples compared to line the heads up.
const MATCH: usize = 192;
/// The delay line's length, a power of two past the window, the search
/// and the stretch compared.
const LINE: usize = 2048;
/// The most a voice goes either way, in semitones.
pub const MOST: f32 = 12.0;

/// One voice's shifter, both channels of its interleaved stereo.
pub struct Shifter {
    lines: [Vec<f32>; 2],
    written: usize,
    /// Where the first head is through its window, 0 to 1; the second is
    /// half a window on.
    phase: f32,
    /// Each head's extra delay, chosen as it jumps back, in samples.
    offsets: [usize; 2],
}

impl Default for Shifter {
    fn default() -> Self {
        Self {
            lines: [vec![0.0; LINE], vec![0.0; LINE]],
            written: 0,
            phase: 0.0,
            offsets: [0; 2],
        }
    }
}

impl Shifter {
    /// Shifts `samples`, interleaved stereo, by `semitones` in place.
    pub fn process(&mut self, samples: &mut [i16], semitones: f32) {
        let semitones = semitones.clamp(-MOST, MOST);
        let ratio = 2f32.powf(semitones / 12.0);
        // How far the heads' delay changes each sample.
        let step = (1.0 - ratio) / WINDOW as f32;
        for frame in samples.chunks_exact_mut(2) {
            for (channel, sample) in frame.iter_mut().enumerate() {
                self.lines[channel][self.written] = f32::from(*sample);
            }
            let phases = [self.phase, (self.phase + 0.5).fract()];
            // Each head's loudness: sin² of its place, which together make 1.
            let fade = |phase: f32| (std::f32::consts::PI * phase).sin().powi(2);
            for (channel, sample) in frame.iter_mut().enumerate() {
                let line = &self.lines[channel];
                let mixed: f32 = (0..2)
                    .map(|head| {
                        fade(phases[head]) * self.read(line, self.delay(phases[head], head))
                    })
                    .sum();
                *sample = mixed
                    .round()
                    .clamp(f32::from(i16::MIN), f32::from(i16::MAX))
                    as i16;
            }
            self.written = (self.written + 1) % LINE;
            let next = (self.phase + step).rem_euclid(1.0);
            let nexts = [next, (next + 0.5).fract()];
            // A head that jumped back, silent there, lines up with the
            // other, whole there.
            for head in 0..2 {
                if (nexts[head] - phases[head]).abs() > 0.5 {
                    let other = 1 - head;
                    let lead = self.delay(nexts[other], other);
                    let from = 1.0 + nexts[head] * WINDOW as f32;
                    self.offsets[head] = self.best_offset(lead, from);
                }
            }
            self.phase = next;
        }
    }

    /// A head's delay at `phase`, in samples behind the last written.
    fn delay(&self, phase: f32, head: usize) -> f32 {
        1.0 + phase * WINDOW as f32 + self.offsets[head] as f32
    }

    /// The extra delay, up to [`SEARCH`], at which the line from `from`
    /// samples back looks most like it does `lead` samples back: the
    /// highest correlation over [`MATCH`] samples, both channels together.
    fn best_offset(&self, lead: f32, from: f32) -> usize {
        let at = |delay: usize, back: usize| {
            let index = (self.written + 2 * LINE - delay - back) % LINE;
            self.lines[0][index] + self.lines[1][index]
        };
        let lead = (lead.round() as usize).min(LINE - MATCH - 1);
        let from = from.round() as usize;
        let reference: Vec<f32> = (0..MATCH).map(|back| at(lead, back)).collect();
        let quiet: f32 = reference.iter().map(|value| value * value).sum();
        if quiet < 1.0 {
            return 0;
        }
        let mut best = (f32::MIN, 0);
        for offset in 0..=SEARCH {
            let delay = from + offset;
            if delay + MATCH >= LINE {
                break;
            }
            let (mut product, mut energy) = (0.0f32, 1e-3f32);
            for (back, wanted) in reference.iter().enumerate() {
                let value = at(delay, back);
                product += wanted * value;
                energy += value * value;
            }
            let score = product / energy.sqrt();
            if score > best.0 {
                best = (score, offset);
            }
        }
        best.1
    }

    /// The line `delay` samples behind the last sample written, between
    /// samples by a straight line.
    fn read(&self, line: &[f32], delay: f32) -> f32 {
        let at = self.written as f32 - delay + LINE as f32;
        let whole = at.floor();
        let part = at - whole;
        let index = whole as usize % LINE;
        let next = (index + 1) % LINE;
        line[index] * (1.0 - part) + line[next] * part
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The left channel's power at `hertz` (Goertzel).
    fn power(samples: &[i16], hertz: f32) -> f32 {
        let coefficient = 2.0 * (std::f32::consts::TAU * hertz / 48_000.0).cos();
        let (mut before, mut last) = (0.0f32, 0.0f32);
        for frame in samples.chunks_exact(2) {
            let next = f32::from(frame[0]) / 32768.0 + coefficient * last - before;
            before = last;
            last = next;
        }
        last * last + before * before - coefficient * last * before
    }

    fn tone(hertz: f32, seconds: f32) -> Vec<i16> {
        let count = (48_000.0 * seconds) as usize;
        (0..count)
            .flat_map(|i| {
                let value = (std::f32::consts::TAU * hertz * i as f32 / 48_000.0).sin() * 10_000.0;
                [value as i16, value as i16]
            })
            .collect()
    }

    #[test]
    fn an_octave_up_doubles_a_tone_and_down_halves_it() {
        let original = tone(220.0, 1.0);
        for (semitones, hertz) in [(12.0, 440.0), (-12.0, 110.0), (7.0, 329.6)] {
            let mut shifted = original.clone();
            let mut shifter = Shifter::default();
            for frame in shifted.chunks_mut(1920) {
                shifter.process(frame, semitones);
            }
            let shifted = &shifted[9600..];
            // Most of the sound is at the new pitch, little at the old.
            let new = power(shifted, hertz);
            let old = power(shifted, 220.0);
            assert!(new > 4.0 * old, "{semitones}: {new} against {old}");
        }
    }

    #[test]
    fn it_keeps_the_loudness() {
        let mut shifted = tone(300.0, 0.5);
        let mut shifter = Shifter::default();
        shifter.process(&mut shifted, 5.0);
        let peak = shifted[4800..]
            .iter()
            .map(|sample| sample.unsigned_abs())
            .max()
            .unwrap();
        assert!((8_000..=11_000).contains(&peak), "{peak}");
    }

    #[test]
    fn the_heads_crossing_never_cancel_a_tone() {
        // Unaligned, two heads crossing out of phase dip a steady tone's
        // loudness every window: the robotic flutter.
        for semitones in [3.0, -4.0, 7.0] {
            let mut shifted = tone(180.0, 1.0);
            let mut shifter = Shifter::default();
            for frame in shifted.chunks_mut(1920) {
                shifter.process(frame, semitones);
            }
            // The loudness of each 5 ms, past the start.
            let levels: Vec<f32> = shifted[9600..]
                .chunks(480)
                .map(|chunk| {
                    let sum: f32 = chunk.iter().map(|&s| f32::from(s).powi(2)).sum();
                    (sum / chunk.len() as f32).sqrt()
                })
                .collect();
            let quietest = levels.iter().copied().fold(f32::MAX, f32::min);
            let loudest = levels.iter().copied().fold(0.0, f32::max);
            assert!(
                quietest > 0.75 * loudest,
                "{semitones}: {quietest} against {loudest}"
            );
        }
    }
}
