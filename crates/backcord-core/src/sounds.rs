//! BACKCORD's own short sounds for calls and messages: joining and leaving,
//! mute and deafen, a new message or mention, someone starting or stopping a
//! stream, and someone starting or stopping watching one. Each is Discord's
//! own, downloaded from Discord's web client the first time it plays and
//! kept in the data folder, or a file chosen in its place.

use std::path::{Path, PathBuf};
#[cfg(feature = "discord")]
use std::sync::Arc;

use serde::{Deserialize, Serialize};

#[cfg(feature = "discord")]
use crate::audio_io::{Output, RATE};

/// What a sound announces.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sound {
    /// Someone, this account included, joined the call this account is in.
    Join,
    /// Someone left that call, or this account did.
    Leave,
    Mute,
    Unmute,
    Deafen,
    Undeafen,
    /// A message that notifies.
    Message,
    /// A message that mentions this account.
    Mention,
    /// Someone else started watching this account's stream, or the one it
    /// watches.
    ViewerJoin,
    /// Someone else stopped watching it.
    ViewerLeave,
    /// Someone else in the call started streaming.
    StreamStart,
    /// Someone else stopped streaming, staying in the call.
    StreamStop,
}

impl Sound {
    pub const ALL: [Self; 12] = [
        Self::Join,
        Self::Leave,
        Self::Mute,
        Self::Unmute,
        Self::Deafen,
        Self::Undeafen,
        Self::Message,
        Self::Mention,
        Self::ViewerJoin,
        Self::ViewerLeave,
        Self::StreamStart,
        Self::StreamStop,
    ];

    /// Discord's own sound for this, as its web client names the file under
    /// `discord.com/assets/`: user_join, user_leave, mute, unmute, deafen,
    /// undeafen, message1, mention1, stream_user_joined, stream_user_left,
    /// stream_started and stream_ended. A file's name is its contents' hash,
    /// so a name stays the same file.
    pub fn discord_file(self) -> &'static str {
        match self {
            Self::Join => "b135ff6c8e091b43.mp3",
            Self::Leave => "7b9a183742515fc2.mp3",
            Self::Mute => "2d3b4ba32c34c862.mp3",
            Self::Unmute => "e74c4a06134a20e4.mp3",
            Self::Deafen => "529ff198eac567af.mp3",
            Self::Undeafen => "b150f03c89944403.mp3",
            Self::Message => "3ed22d14f3c30bc4.mp3",
            Self::Mention => "3baf62fc7aebb0a3.mp3",
            Self::ViewerJoin => "3d2af2764581bb2e.mp3",
            Self::ViewerLeave => "24ae9dc0184bd570.mp3",
            Self::StreamStart => "abe52a3c92953edb.mp3",
            Self::StreamStop => "cbff07b5a83e0321.mp3",
        }
    }
}

/// The data folder's folder for sounds: Discord's, as downloaded, and the
/// files chosen in their place, copied in.
pub const FOLDER: &str = "sounds";

/// The most a sound's file may weigh, Discord's or chosen.
pub const LARGEST_FILE: usize = 4 * 1024 * 1024;

/// Where Discord's sound for `sound` is kept in the data folder `root`.
pub fn discord_path(root: &Path, sound: Sound) -> PathBuf {
    root.join(FOLDER)
        .join(format!("discord-{}", sound.discord_file()))
}

/// A slider's percentage, up to 200%, as a gain. Loudness is heard on a log
/// scale, so the square spreads quiet settings along more of the slider. At
/// 100% a sound plays as its file has it.
pub fn gain(percent: u8) -> f32 {
    (f32::from(percent.min(200)) / 100.0).powi(2)
}

/// Whether `data` is a sound BACKCORD can play, MP3 or Ogg, and how long it
/// lasts in seconds (at most the soundboard's longest).
#[cfg(feature = "discord")]
pub fn check(data: &[u8]) -> anyhow::Result<f32> {
    let samples = crate::soundboard::decode(data)?;
    Ok(samples.len() as f32 / 2.0 / RATE as f32)
}

/// Plays `sound` once on `speaker`, a device id as in [`crate::voice::State`],
/// or the default output, at `volume` from [`gain`]: `file` when given and
/// playable, otherwise Discord's, downloaded into `root`'s [`FOLDER`] the
/// first time. It's read and played on a thread of its own, and overlaps any
/// sound still playing.
pub fn play(root: &Path, file: Option<PathBuf>, sound: Sound, volume: f32, speaker: Option<&str>) {
    if volume <= 0.0 {
        return;
    }
    #[cfg(feature = "discord")]
    {
        let root = root.to_owned();
        let speaker: Option<Arc<str>> = speaker.map(Into::into);
        let started = std::thread::Builder::new()
            .name("backcord-sound".into())
            .spawn(move || {
                let played = samples(&root, file.as_deref(), sound)
                    .and_then(|samples| play_samples(&samples, volume, speaker.as_deref()));
                if let Err(error) = played {
                    tracing::warn!(
                        error = format!("{error:#}"),
                        ?sound,
                        "could not play a sound"
                    );
                }
            });
        if let Err(error) = started {
            tracing::warn!(%error, "could not start a sound");
        }
    }
    #[cfg(not(feature = "discord"))]
    let _ = (root, file, sound, speaker);
}

/// `sound`'s samples, as 48 kHz interleaved stereo: `file`'s, or when it
/// won't play, Discord's.
#[cfg(feature = "discord")]
fn samples(root: &Path, file: Option<&Path>, sound: Sound) -> anyhow::Result<Vec<i16>> {
    use anyhow::Context;
    if let Some(file) = file {
        match std::fs::read(file)
            .context("could not read it")
            .and_then(|data| crate::soundboard::decode(&data))
        {
            Ok(samples) => return Ok(samples),
            Err(error) => tracing::warn!(
                error = format!("{error:#}"),
                file = %file.display(),
                "a chosen sound won't play; playing Discord's"
            ),
        }
    }
    let path = discord_path(root, sound);
    let data = match std::fs::read(&path) {
        Ok(data) => data,
        Err(_) => download(&path, sound)?,
    };
    crate::soundboard::decode(&data)
}

/// Downloads Discord's sound for `sound` and keeps it at `path`, written
/// beside it first so a download cut short leaves nothing there.
#[cfg(feature = "discord")]
fn download(path: &Path, sound: Sound) -> anyhow::Result<Vec<u8>> {
    use anyhow::Context;
    if (cfg!(test) || cfg!(feature = "test-fixtures"))
        && backcord_discord::client::test_server().is_none()
    {
        anyhow::bail!("tests download nothing");
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("could not start the download")?;
    let data = runtime.block_on(async {
        backcord_discord::api::Api::anonymous()?
            .client_asset(sound.discord_file(), LARGEST_FILE)
            .await
    })?;
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder).context("could not make the sounds' folder")?;
    }
    let partial = path.with_extension("part");
    std::fs::write(&partial, &data)
        .and_then(|()| std::fs::rename(&partial, path))
        .context("could not keep the sound")?;
    Ok(data)
}

/// Plays interleaved stereo samples to the end, `volume` times as loud.
#[cfg(feature = "discord")]
fn play_samples(samples: &[i16], volume: f32, speaker: Option<&str>) -> anyhow::Result<()> {
    use anyhow::Context;
    // Speakers that are gone play on the default ones.
    let mut output = Output::<i16>::open_or_default(speaker, "Sound", RATE, 2, None)
        .context("could not open the speakers")?;
    let mut chunk = [0i16; RATE as usize / 50 * 2];
    for part in samples.chunks(chunk.len()) {
        for (sample, value) in chunk.iter_mut().zip(part) {
            *sample =
                (f32::from(*value) * volume).clamp(f32::from(i16::MIN), f32::from(i16::MAX)) as i16;
        }
        output
            .write(&chunk[..part.len()])
            .context("could not play")?;
    }
    output.drain().context("could not finish playing")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_sound_has_its_own_discord_file() {
        let files: std::collections::BTreeSet<_> = Sound::ALL
            .iter()
            .map(|sound| sound.discord_file())
            .collect();
        assert_eq!(files.len(), Sound::ALL.len());
        for file in files {
            let (hash, extension) = file.split_once('.').unwrap();
            assert_eq!(extension, "mp3");
            assert!(hash.len() == 16 && hash.bytes().all(|b| b.is_ascii_hexdigit()));
        }
        let root = Path::new("data");
        assert_eq!(
            discord_path(root, Sound::Join),
            root.join("sounds").join("discord-b135ff6c8e091b43.mp3")
        );
    }

    #[test]
    fn volume_follows_the_square_of_the_slider() {
        assert_eq!(gain(0), 0.0);
        assert_eq!(gain(100), 1.0);
        assert!((gain(50) - 0.25).abs() < 1e-6);
        assert_eq!(gain(200), 4.0);
        assert_eq!(gain(255), 4.0);
    }

    #[cfg(feature = "discord")]
    #[test]
    fn a_chosen_file_plays_in_place_of_discords() {
        let folder = tempfile::tempdir().unwrap();
        let mp3 = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../testing/fixtures/soundboard-tone.mp3");
        let chosen = samples(folder.path(), Some(&mp3), Sound::Message).unwrap();
        assert!(!chosen.is_empty());
        assert!(check(&std::fs::read(&mp3).unwrap()).unwrap() > 0.1);
        assert!(check(b"not a sound").is_err());
        // Without the file, and nothing downloaded, Discord's can't be had
        // in a test.
        let gone = folder.path().join("gone.mp3");
        assert!(samples(folder.path(), Some(&gone), Sound::Message).is_err());
        // Discord's, once kept, plays without a download.
        let kept = discord_path(folder.path(), Sound::Message);
        std::fs::create_dir_all(kept.parent().unwrap()).unwrap();
        std::fs::copy(&mp3, &kept).unwrap();
        assert!(samples(folder.path(), None, Sound::Message).is_ok());
    }
}
