//! Export Chat: fetches a Discord channel's, DM's or group's history a
//! page at a time, telling the app how many came after each, then writes
//! the file [`crate::chat_export::render`] makes of it; see "Export Chat"
//! in docs/discord/messages.md. One runs at a time, and the dialog stops
//! it.

use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
};

use crate::chat_export::{ChatExport, ExportEnd, ExportRange};

use super::{commands::NOT_DISCORD, *};

/// An export going on: its room, with its stop.
type Running = Option<(String, Arc<AtomicBool>)>;

/// The export going on.
fn running() -> std::sync::MutexGuard<'static, Running> {
    static RUNNING: OnceLock<Mutex<Running>> = OnceLock::new();
    RUNNING
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Starts or stops an export, or gives back any other command.
pub(super) fn command(
    command: Command,
    discord: Result<&Discord, &str>,
    events: &UnboundedSender<Event>,
) -> Option<Command> {
    match command {
        Command::Discord(DiscordCommand::ExportChat(export)) => {
            start(*export, discord, events);
            None
        }
        Command::Discord(DiscordCommand::StopExport { room }) => {
            if let Some((going, stop)) = running().as_ref()
                && *going == room
            {
                stop.store(true, Ordering::Relaxed);
            }
            None
        }
        command => Some(command),
    }
}

fn start(export: ChatExport, discord: Result<&Discord, &str>, events: &UnboundedSender<Event>) {
    let ended = |end: ExportEnd| Event::ChatExported {
        room: export.room.clone(),
        messages: 0,
        end: Some(end),
    };
    let channel = discord::parse_room(&export.room).map(|(_, channel)| channel.to_owned());
    let (discord, channel) = match (discord, channel) {
        (Ok(discord), Some(channel)) => (discord.clone(), channel),
        (Err(why), _) => {
            let _ = events.send(ended(ExportEnd::Failed(why.to_owned())));
            return;
        }
        (Ok(_), None) => {
            let _ = events.send(ended(ExportEnd::Failed(NOT_DISCORD.to_owned())));
            return;
        }
    };
    let stop = Arc::new(AtomicBool::new(false));
    {
        let mut running = running();
        if running.is_some() {
            let _ = events.send(ended(ExportEnd::Failed(
                "Another chat is being exported.".to_owned(),
            )));
            return;
        }
        *running = Some((export.room.clone(), stop.clone()));
    }
    let events = events.clone();
    tokio::spawn(async move {
        let room = export.room.clone();
        let progress = {
            let (events, room) = (events.clone(), room.clone());
            move |count: usize| {
                let _ = events.send(Event::ChatExported {
                    room: room.clone(),
                    messages: u32::try_from(count).unwrap_or(u32::MAX),
                    end: None,
                });
            }
        };
        let (most, since) = match export.range {
            ExportRange::All => (None, None),
            ExportRange::Last(count) => (Some(count as usize), None),
            ExportRange::Since(ms) => (None, Some(backcord_discord::message::snowflake_at(ms))),
        };
        let fetched = discord
            .export_history(&channel, most, since, &stop, &progress)
            .await;
        *running() = None;
        let (count, end) = match fetched {
            Ok(None) => (0, ExportEnd::Stopped),
            Ok(Some(messages)) => {
                let count = messages.len();
                // A long history takes a while to write out, off the runtime.
                let (title, format, attachments) =
                    (export.title.clone(), export.format, export.attachments);
                let contents = tokio::task::spawn_blocking(move || {
                    crate::chat_export::render(&messages, &title, format, attachments)
                })
                .await
                .unwrap_or_default();
                match tokio::fs::write(&export.path, contents).await {
                    Ok(()) => {
                        tracing::info!(messages = count, "exported a chat");
                        (count, ExportEnd::Saved(export.path.clone()))
                    }
                    Err(error) => {
                        tracing::warn!(%error, "could not write an exported chat");
                        (
                            count,
                            ExportEnd::Failed(format!("Couldn't save the file: {error}")),
                        )
                    }
                }
            }
            Err(error) => {
                tracing::warn!(error = format!("{error:#}"), "Export Chat stopped");
                (0, ExportEnd::Failed(error.to_string()))
            }
        };
        let _ = events.send(Event::ChatExported {
            room,
            messages: u32::try_from(count).unwrap_or(u32::MAX),
            end: Some(end),
        });
    });
}
