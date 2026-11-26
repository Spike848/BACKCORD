//! Recording from the microphone, for a voice message or dictation: on a
//! thread of its own, the microphone chosen for calls, 48 kHz mono, until
//! stopped. A voice message's recording is encoded to Opus as it comes and
//! kept as an Ogg Opus file, as Discord's apps send one, with the loudness
//! of each 20 ms for its waveform; dictation keeps the sound itself, at the
//! 16 kHz Whisper reads, instead.

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
    },
    thread::JoinHandle,
    time::Duration,
};

use anyhow::{Context, Result};

use crate::audio_io::Input;

const RATE: u32 = 48_000;
/// 20 ms at 48 kHz, an Opus frame.
const FRAME: usize = 960;
/// What Opus's encoder holds back at 48 kHz, which a player skips.
const PRE_SKIP: u16 = 312;
/// How fast a voice message's Opus goes, in bits a second.
const BITRATE: i32 = 48_000;
/// A recording stops by itself after this long.
pub const MAX_MS: u64 = 20 * 60 * 1000;
/// The most points a voice message's waveform has, as Discord's apps send.
const WAVEFORM_POINTS: usize = 256;

/// What a recording keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keep {
    /// An Ogg Opus voice message, with its waveform.
    VoiceMessage,
    /// The sound at 16 kHz, for dictation.
    Speech,
}

/// A recording going on, until [`Recording::stop`] or a drop, which throws
/// it away.
pub struct Recording {
    shared: Arc<Shared>,
    thread: Option<JoinHandle<Result<Recorded>>>,
}

#[derive(Default)]
struct Shared {
    stop: AtomicBool,
    /// How long it has recorded, in ms.
    elapsed: AtomicU64,
    /// The last 20 ms's peak, from 0 to 1, as f32 bits.
    level: AtomicU32,
    failed: AtomicBool,
}

/// A recording stopped.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Recorded {
    pub duration_ms: u64,
    /// The Ogg Opus file, for a voice message.
    pub ogg: Vec<u8>,
    /// Its loudness along its length, up to 256 points from 0 to 255.
    pub waveform: Vec<u8>,
    /// The sound, mono at 16 kHz from -1 to 1, for dictation.
    pub speech: Vec<f32>,
}

impl Recording {
    /// Starts recording from `microphone`, none for the default, keeping
    /// what `keep` says.
    pub fn start(microphone: Option<&str>, keep: Keep) -> Self {
        let shared = Arc::new(Shared::default());
        let microphone = microphone.map(str::to_owned);
        let worker = shared.clone();
        let thread = std::thread::Builder::new()
            .name("backcord-recording".into())
            .spawn(move || {
                let recorded = record(microphone.as_deref(), keep, &worker);
                if let Err(error) = &recorded {
                    tracing::warn!(error = format!("{error:#}"), "could not record");
                    worker.failed.store(true, Ordering::Relaxed);
                }
                recorded
            });
        let thread = match thread {
            Ok(thread) => Some(thread),
            Err(_) => {
                shared.failed.store(true, Ordering::Relaxed);
                None
            }
        };
        Self { shared, thread }
    }

    /// How long it has recorded, in ms.
    pub fn elapsed_ms(&self) -> u64 {
        self.shared.elapsed.load(Ordering::Relaxed)
    }

    /// How loud the last 20 ms were, their peak from 0 to 1.
    pub fn level(&self) -> f32 {
        f32::from_bits(self.shared.level.load(Ordering::Relaxed))
    }

    /// The microphone wouldn't open or stopped.
    pub fn failed(&self) -> bool {
        self.shared.failed.load(Ordering::Relaxed)
    }

    /// It stopped by itself, at [`MAX_MS`].
    pub fn full(&self) -> bool {
        self.elapsed_ms() >= MAX_MS
    }

    /// Stops it and gives what it recorded, waiting for its last 20 ms.
    pub fn stop(mut self) -> Result<Recorded> {
        self.shared.stop.store(true, Ordering::Relaxed);
        let thread = self.thread.take().context("the recording didn't start")?;
        thread
            .join()
            .map_err(|_| anyhow::anyhow!("the recording stopped"))?
    }
}

impl Drop for Recording {
    fn drop(&mut self) {
        // Thrown away: its thread ends after its last read.
        self.shared.stop.store(true, Ordering::Relaxed);
    }
}

fn record(microphone: Option<&str>, keep: Keep, shared: &Shared) -> Result<Recorded> {
    let mut input = Input::<f32>::open(microphone, "Recording", RATE, 1, Duration::from_millis(20))
        .or_else(|_| Input::<f32>::open(None, "Recording", RATE, 1, Duration::from_millis(20)))
        .context("could not open the microphone")?;
    let mut encoder = match keep {
        Keep::VoiceMessage => Some(backcord_discord::opus::Encoder::new(
            1,
            backcord_discord::opus::Application::Voice,
            BITRATE,
        )?),
        Keep::Speech => None,
    };
    let mut ogg = OggOpus::new(rand::random());
    let mut levels = Vec::new();
    let mut speech = Vec::new();
    let mut frame = vec![0f32; FRAME];
    let mut packet = vec![0u8; 4000];
    let mut frames = 0u64;
    while !shared.stop.load(Ordering::Relaxed) && frames * 20 < MAX_MS {
        input.read(&mut frame)?;
        frames += 1;
        let peak = frame
            .iter()
            .fold(0f32, |most, sample| most.max(sample.abs()))
            .min(1.0);
        shared.level.store(peak.to_bits(), Ordering::Relaxed);
        shared.elapsed.store(frames * 20, Ordering::Relaxed);
        match &mut encoder {
            Some(encoder) => {
                let length = encoder.encode_float(&frame, &mut packet)?;
                ogg.push(&packet[..length]);
                levels.push(peak);
            }
            // 48 kHz to 16 kHz: each three samples' mean.
            None => speech.extend(
                frame
                    .chunks_exact(3)
                    .map(|three| three.iter().sum::<f32>() / 3.0),
            ),
        }
    }
    let duration_ms = frames * 20;
    Ok(Recorded {
        duration_ms,
        ogg: if encoder.is_some() {
            ogg.finish()
        } else {
            Vec::new()
        },
        waveform: waveform(&levels),
        speech,
    })
}

/// A waveform of up to [`WAVEFORM_POINTS`] from each 20 ms's peak: each
/// point the loudest of its stretch, the loudest of all at 255.
pub fn waveform(levels: &[f32]) -> Vec<u8> {
    if levels.is_empty() {
        return Vec::new();
    }
    let points = levels.len().min(WAVEFORM_POINTS);
    let stretch = levels.len() as f64 / points as f64;
    let peaks: Vec<f32> = (0..points)
        .map(|point| {
            let from = (point as f64 * stretch) as usize;
            let to = (((point + 1) as f64 * stretch) as usize).clamp(from + 1, levels.len());
            levels[from..to]
                .iter()
                .fold(0f32, |most, level| most.max(*level))
        })
        .collect();
    let loudest = peaks.iter().fold(0f32, |most, peak| most.max(*peak));
    if loudest <= 0.0 {
        return vec![0; points];
    }
    peaks
        .iter()
        .map(|peak| (peak / loudest * 255.0).round() as u8)
        .collect()
}

/// An Ogg Opus file being written: its headers, then pages of up to a
/// second of packets, as RFC 7845 lays them out.
struct OggOpus {
    serial: u32,
    pages: Vec<u8>,
    sequence: u32,
    /// The packets waiting for the next page, and each one's length.
    waiting: Vec<u8>,
    lengths: Vec<usize>,
    /// Samples written, past the pre-skip.
    samples: u64,
}

impl OggOpus {
    /// Packets to a page: a second of 20 ms frames.
    const PACKETS: usize = 50;

    fn new(serial: u32) -> Self {
        let mut ogg = Self {
            serial,
            pages: Vec::new(),
            sequence: 0,
            waiting: Vec::new(),
            lengths: Vec::new(),
            samples: 0,
        };
        let mut head = b"OpusHead".to_vec();
        head.push(1);
        head.push(1);
        head.extend_from_slice(&PRE_SKIP.to_le_bytes());
        head.extend_from_slice(&RATE.to_le_bytes());
        head.extend_from_slice(&0i16.to_le_bytes());
        head.push(0);
        ogg.page(&head, &[head.len()], 0, 0x02);
        let vendor = b"BACKCORD";
        let mut tags = b"OpusTags".to_vec();
        tags.extend_from_slice(&(vendor.len() as u32).to_le_bytes());
        tags.extend_from_slice(vendor);
        tags.extend_from_slice(&0u32.to_le_bytes());
        ogg.page(&tags, &[tags.len()], 0, 0);
        ogg
    }

    fn push(&mut self, packet: &[u8]) {
        self.waiting.extend_from_slice(packet);
        self.lengths.push(packet.len());
        self.samples += FRAME as u64;
        if self.lengths.len() == Self::PACKETS {
            self.flush(0);
        }
    }

    fn finish(mut self) -> Vec<u8> {
        self.flush(0x04);
        self.pages
    }

    fn flush(&mut self, flags: u8) {
        if self.lengths.is_empty() && flags & 0x04 == 0 {
            return;
        }
        let (data, lengths) = (
            std::mem::take(&mut self.waiting),
            std::mem::take(&mut self.lengths),
        );
        let granule = self.samples + u64::from(PRE_SKIP);
        self.page(&data, &lengths, granule, flags);
    }

    /// Writes one page of `data`, its packets `lengths` long.
    fn page(&mut self, data: &[u8], lengths: &[usize], granule: u64, flags: u8) {
        let mut lacing = Vec::new();
        for length in lengths {
            lacing.extend(std::iter::repeat_n(255u8, length / 255));
            lacing.push((length % 255) as u8);
        }
        let start = self.pages.len();
        self.pages.extend_from_slice(b"OggS");
        self.pages.push(0);
        self.pages.push(flags);
        self.pages.extend_from_slice(&granule.to_le_bytes());
        self.pages.extend_from_slice(&self.serial.to_le_bytes());
        self.pages.extend_from_slice(&self.sequence.to_le_bytes());
        self.pages.extend_from_slice(&[0; 4]);
        self.pages.push(lacing.len() as u8);
        self.pages.extend_from_slice(&lacing);
        self.pages.extend_from_slice(data);
        let crc = crc(&self.pages[start..]);
        self.pages[start + 22..start + 26].copy_from_slice(&crc.to_le_bytes());
        self.sequence += 1;
    }
}

/// Ogg's CRC-32: polynomial 0x04C11DB7, not reflected, from 0.
fn crc(data: &[u8]) -> u32 {
    let mut crc = 0u32;
    for byte in data {
        crc ^= u32::from(*byte) << 24;
        for _ in 0..8 {
            crc = if crc & 0x8000_0000 != 0 {
                (crc << 1) ^ 0x04C1_1DB7
            } else {
                crc << 1
            };
        }
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_ogg_opus_file_plays_for_as_long_as_was_encoded() {
        let mut encoder = backcord_discord::opus::Encoder::new(
            1,
            backcord_discord::opus::Application::Voice,
            BITRATE,
        )
        .unwrap();
        let mut ogg = OggOpus::new(7);
        let mut packet = vec![0u8; 4000];
        // Three seconds of a tone, more than a page.
        for frame in 0..150 {
            let pcm: Vec<f32> = (0..FRAME)
                .map(|at| (((frame * FRAME + at) as f32) * 0.05).sin() * 0.5)
                .collect();
            let length = encoder.encode_float(&pcm, &mut packet).unwrap();
            ogg.push(&packet[..length]);
        }
        let file = ogg.finish();
        let video = crate::video::Video::open_sound(std::io::Cursor::new(file)).expect("opens");
        assert_eq!(video.sound.map(|(rate, _)| rate), Some(48_000));
        let duration = video.duration_ms.expect("a duration");
        assert!((2_950..=3_050).contains(&duration), "{duration}");
    }

    #[test]
    fn the_crc_is_oggs() {
        // libogg's check value: CRC-32 with Ogg's polynomial, from 0, with
        // no reflection and no final xor.
        assert_eq!(crc(b"123456789"), 0x89A1_897F);
        assert_eq!(crc(b""), 0);
    }

    #[test]
    fn a_waveform_has_up_to_256_points_and_its_loudest_at_255() {
        assert!(waveform(&[]).is_empty());
        assert_eq!(waveform(&[0.0, 0.5, 0.25]), [0, 255, 128]);
        let long: Vec<f32> = (0..3000).map(|at| (at % 100) as f32 / 100.0).collect();
        let points = waveform(&long);
        assert_eq!(points.len(), 256);
        assert_eq!(points.iter().max(), Some(&255));
        assert_eq!(waveform(&[0.0; 10]), [0; 10], "silence");
    }
}
