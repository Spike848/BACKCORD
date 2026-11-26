//! Voice messages from the composer: its microphone button records one
//! (`backcord_core::recorder`) on the microphone chosen for calls, showing
//! in the field's place how long it has gone and how loud it is, with Stop
//! and Cancel. Stopped, the recording waits there to play once first, to
//! send, or to throw away. It's written to `voice-message.ogg` in a folder
//! of its own in the system's temporary folder, which sending uploads.

use std::{path::PathBuf, time::Duration};

use backcord_core::{
    application::Action,
    chat::VoiceClip,
    recorder::{Keep, Recording},
};
use iced::{
    Alignment, Element, Length, Subscription, Task,
    widget::{Space, container, row, text},
};

use crate::{
    app::{App, Message},
    composer::{Event as ComposerEvent, composer_button},
    pictures::Icon,
    style::rounded,
};

/// A recording that's too short to send, as a click on the button and
/// another at once makes.
const SHORTEST_MS: u64 = 500;
/// The name Discord's apps give a voice message's recording, which it's
/// sent as.
const FILE: &str = "voice-message.ogg";
/// How often the recording's time and level move.
const TICK: Duration = Duration::from_millis(100);
/// Bars of the recording's level as it goes, newest last.
const LEVELS: usize = 40;

/// The composer's voice message.
pub enum VoiceNote {
    Recording {
        room: String,
        recording: Recording,
        /// The level of each tick lately, from 0 to 1.
        levels: Vec<f32>,
    },
    Recorded {
        room: String,
        path: PathBuf,
        clip: VoiceClip,
    },
    /// The microphone wouldn't open, or the recording couldn't be kept.
    Failed { room: String },
}

impl VoiceNote {
    fn room(&self) -> &str {
        match self {
            Self::Recording { room, .. } | Self::Recorded { room, .. } | Self::Failed { room } => {
                room
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    /// The microphone button: start recording.
    Start,
    Stop,
    /// Throw the recording away, going or stopped.
    Cancel,
    /// Play the stopped recording, or pause it.
    Play,
    Send,
    Tick,
}

fn message(event: Event) -> Message {
    Message::Write(ComposerEvent::Voice(event))
}

/// The open room, when this account may send voice messages there.
fn voice_room(state: &backcord_core::application::AppState) -> Option<&str> {
    let selected = state.selected.as_deref()?;
    state
        .rooms
        .iter()
        .find(|room| room.id == selected && room.voice_messages && !room.read_only)
        .map(|room| room.id.as_str())
}

pub fn update(app: &mut App, event: Event) -> Task<Message> {
    match event {
        Event::Start => {
            let Some(room) = voice_room(app.client.state()).map(str::to_owned) else {
                return Task::none();
            };
            let microphone = app.client.state().voice.microphone.clone();
            app.composer.voice = Some(VoiceNote::Recording {
                room,
                recording: Recording::start(microphone.as_deref(), Keep::VoiceMessage),
                levels: Vec::new(),
            });
            Task::none()
        }
        Event::Tick => {
            let mut stop = false;
            if let Some(VoiceNote::Recording {
                room,
                recording,
                levels,
            }) = &mut app.composer.voice
            {
                if recording.failed() {
                    app.composer.voice = Some(VoiceNote::Failed { room: room.clone() });
                    return Task::none();
                }
                levels.push(recording.level());
                if levels.len() > LEVELS {
                    levels.remove(0);
                }
                stop = recording.full();
            }
            if stop {
                return update(app, Event::Stop);
            }
            Task::none()
        }
        Event::Stop => {
            let Some(VoiceNote::Recording {
                room, recording, ..
            }) = app.composer.voice.take()
            else {
                return Task::none();
            };
            app.composer.voice = match recording.stop().and_then(keep) {
                Ok(Some((path, clip))) => Some(VoiceNote::Recorded { room, path, clip }),
                Ok(None) => None,
                Err(error) => {
                    tracing::warn!(
                        error = format!("{error:#}"),
                        "could not keep a voice message"
                    );
                    Some(VoiceNote::Failed { room })
                }
            };
            Task::none()
        }
        Event::Cancel => {
            // A recording going stops with its drop, and keeps nothing.
            if let Some(VoiceNote::Recorded { path, .. }) = app.composer.voice.take() {
                app.client.dispatch(Action::StopRecording);
                throw_away(&path);
                return app.sync();
            }
            Task::none()
        }
        Event::Play => {
            if let Some(VoiceNote::Recorded { room, path, .. }) = &app.composer.voice {
                app.client.dispatch(Action::PlayRecording {
                    room: room.clone(),
                    path: path.clone(),
                });
                return app.sync();
            }
            Task::none()
        }
        Event::Send => {
            if let Some(VoiceNote::Recorded { room, path, clip }) = app.composer.voice.take() {
                app.client
                    .dispatch(Action::SendVoiceMessage { room, path, clip });
                return app.sync();
            }
            Task::none()
        }
    }
}

/// Writes a recording long enough to send to its file, with what its
/// message says of it.
fn keep(
    recorded: backcord_core::recorder::Recorded,
) -> anyhow::Result<Option<(PathBuf, VoiceClip)>> {
    if recorded.duration_ms < SHORTEST_MS {
        return Ok(None);
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    let folder = std::env::temp_dir()
        .join("backcord-voice-messages")
        .join(stamp.to_string());
    std::fs::create_dir_all(&folder)?;
    let path = folder.join(FILE);
    std::fs::write(&path, &recorded.ogg)?;
    Ok(Some((
        path,
        VoiceClip {
            duration_ms: recorded.duration_ms,
            waveform: recorded.waveform,
        },
    )))
}

/// Deletes a recording thrown away, and its folder.
fn throw_away(path: &std::path::Path) {
    let removed = std::fs::remove_file(path).and_then(|()| match path.parent() {
        Some(folder) => std::fs::remove_dir(folder),
        None => Ok(()),
    });
    if let Err(error) = removed {
        tracing::warn!(%error, "could not delete a voice message thrown away");
    }
}

/// The ticks while recording.
pub fn subscription(app: &App) -> Option<Subscription<Message>> {
    matches!(app.composer.voice, Some(VoiceNote::Recording { .. }))
        .then(|| iced::time::every(TICK).map(|_| message(Event::Tick)))
}

/// The microphone button, where this account may send voice messages and
/// nothing is recorded yet.
pub fn button<'a>(ctx: crate::ctx::Ctx<'a>) -> Option<Element<'a, Message>> {
    voice_room(ctx.state)?;
    if ctx.composer.voice.is_some() {
        return None;
    }
    Some(composer_button(
        ctx,
        Icon::Mic,
        backcord_iced_kit::i18n::tr("Record a voice message"),
        22,
        ctx.look.theme.secondary,
        Some(message(Event::Start)),
    ))
}

/// The voice message's bar, in the field's place, in its own room.
pub fn bar<'a>(ctx: crate::ctx::Ctx<'a>) -> Option<Element<'a, Message>> {
    let note = ctx.composer.voice.as_ref()?;
    if ctx.state.selected.as_deref() != Some(note.room()) {
        return None;
    }
    let theme = ctx.look.theme;
    let tr = backcord_iced_kit::i18n::tr;
    let small =
        |icon, label, ink, event| composer_button(ctx, icon, label, 20, ink, Some(message(event)));
    let line: Element<'a, Message> = match note {
        VoiceNote::Recording {
            recording, levels, ..
        } => {
            let dot = container(Space::new())
                .width(10)
                .height(10)
                .style(move |_| rounded(theme.danger, 5.0));
            let mut bars = row![].spacing(2).align_y(Alignment::Center);
            for level in levels {
                let tall = (level.sqrt() * 24.0).max(2.0);
                bars = bars.push(
                    container(Space::new())
                        .width(3)
                        .height(tall)
                        .style(move |_| rounded(theme.text, 1.5)),
                );
            }
            row![
                small(Icon::X, tr("Cancel"), theme.secondary, Event::Cancel),
                dot,
                text(crate::playback::voice_time(recording.elapsed_ms()))
                    .size(14)
                    .color(theme.text),
                container(bars)
                    .width(Length::Fill)
                    .height(32)
                    .align_y(Alignment::Center),
                small(
                    Icon::CircleStop,
                    tr("Stop recording"),
                    theme.danger,
                    Event::Stop
                ),
            ]
            .spacing(10)
            .align_y(Alignment::Center)
            .into()
        }
        VoiceNote::Recorded { path, clip, .. } => {
            let item = backcord_core::chat::recorded_item(path);
            let playback = ctx
                .state
                .voice_message
                .as_ref()
                .filter(|playback| playback.item == item);
            let playing = playback.is_some_and(|playback| playback.playing);
            let played = playback.map_or(0.0, |playback| {
                (playback.position_ms as f32 / clip.duration_ms.max(1) as f32).min(1.0)
            });
            let mut wave = row![].spacing(2).align_y(Alignment::Center);
            let bars: Vec<f32> = crate::playback::bars(&clip.waveform).collect();
            let count = bars.len().max(1);
            for (index, level) in bars.into_iter().enumerate() {
                let ink = if (index as f32 + 0.5) / count as f32 <= played {
                    theme.text
                } else {
                    theme.muted
                };
                wave = wave.push(
                    container(Space::new())
                        .width(3)
                        .height((level * 24.0).max(3.0))
                        .style(move |_| rounded(ink, 1.5)),
                );
            }
            let time = match playback {
                Some(playback) if playback.playing || playback.position_ms > 0 => {
                    clip.duration_ms.saturating_sub(playback.position_ms)
                }
                _ => clip.duration_ms,
            };
            row![
                small(Icon::Trash, tr("Delete"), theme.danger, Event::Cancel),
                small(
                    if playing { Icon::Pause } else { Icon::Play },
                    if playing { tr("Pause") } else { tr("Play") },
                    theme.text,
                    Event::Play
                ),
                container(wave)
                    .width(Length::Fill)
                    .height(32)
                    .align_y(Alignment::Center),
                text(crate::playback::voice_time(time))
                    .size(14)
                    .color(theme.muted),
                small(
                    Icon::Send,
                    tr("Send voice message"),
                    theme.accent,
                    Event::Send
                ),
            ]
            .spacing(10)
            .align_y(Alignment::Center)
            .into()
        }
        VoiceNote::Failed { .. } => row![
            text(tr("BACKCORD couldn't record from the microphone."))
                .size(14)
                .color(theme.danger)
                .width(Length::Fill),
            small(Icon::X, tr("Dismiss"), theme.secondary, Event::Cancel),
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .into(),
    };
    Some(
        container(line)
            .width(Length::Fill)
            .height(crate::composer::HEIGHT)
            .padding(iced::Padding::ZERO.left(10).right(8))
            .align_y(Alignment::Center)
            .into(),
    )
}
