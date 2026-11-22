//! The microphone's volume ([`crate::volume::microphone`]) as one gain that
//! never lifts a peak into clipping, ramped across each block so a change
//! doesn't click; in Discord calls and in LiveKit's, which WebRTC captures
//! itself.

/// Samples are in 16-bit range, as WebRTC processes them.
const FULL_SCALE: f32 = 32768.0;
/// Raising never pushes a peak past this.
const CEILING: f32 = 0.9 * FULL_SCALE;

/// Sets LiveKit's calls' microphone to its volume too.
pub fn install() {
    static INSTALL: std::sync::Once = std::sync::Once::new();
    INSTALL.call_once(|| {
        webrtc_sys::capture::set_capture_processor(|| Box::new(Capture(MicGain::default())));
    });
}

struct Capture(MicGain);

impl webrtc_sys::capture::CaptureProcessor for Capture {
    fn ramp(&mut self, samples: &[f32], _sample_rate: u32) -> (f32, f32) {
        self.0.ramp(samples, crate::volume::microphone())
    }
}

/// One microphone's gain.
#[derive(Debug)]
pub struct MicGain {
    /// The linear gain the last block ended at.
    applied: f32,
}

impl Default for MicGain {
    fn default() -> Self {
        Self { applied: 1.0 }
    }
}

impl MicGain {
    /// Sets a block of about 10 ms to `volume`, a linear gain, in place.
    pub fn process(&mut self, samples: &mut [f32], volume: f32) {
        let (from, to) = self.ramp(samples, volume);
        let count = samples.len() as f32;
        for (index, sample) in samples.iter_mut().enumerate() {
            *sample *= from + (to - from) * (index + 1) as f32 / count;
        }
    }

    /// The linear gains the block should start and end at, which it ramps
    /// between.
    pub fn ramp(&mut self, samples: &[f32], volume: f32) -> (f32, f32) {
        if samples.is_empty() {
            return (self.applied, self.applied);
        }
        let peak = samples
            .iter()
            .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
        let room = if peak > 0.0 {
            (CEILING / peak).max(1.0)
        } else {
            f32::MAX
        };
        let from = self.applied.min(room);
        let to = volume.min(room);
        self.applied = to;
        (from, to)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_volume_applies_without_lifting_a_peak_into_clipping() {
        let mut gain = MicGain::default();
        let mut quiet = vec![1000.0; 480];
        gain.process(&mut quiet, 2.0);
        assert!((quiet[479] - 2000.0).abs() < 1.0, "doubled by its end");
        let mut loud = vec![20000.0; 480];
        gain.process(&mut loud, 2.0);
        assert!(loud.iter().all(|sample| *sample <= CEILING + 1.0));
        let mut halved = vec![1000.0; 480];
        let mut gain = MicGain::default();
        gain.process(&mut halved, 0.5);
        assert!((halved[479] - 500.0).abs() < 1.0);
    }
}
