//! Cancella tutto: deletes every message this account sent in a Discord
//! DM or server, one at a time, telling the app how many went after each;
//! see "Cancella tutto" in docs/discord/messages.md. One runs at a time in
//! each place, and its menu stops it.

use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};

use backcord_discord::service::Purged;

use super::{commands::NOT_DISCORD, *};

/// The places Cancella tutto runs in, each with its stop.
fn running() -> std::sync::MutexGuard<'static, HashMap<String, Arc<AtomicBool>>> {
    static RUNNING: OnceLock<Mutex<HashMap<String, Arc<AtomicBool>>>> = OnceLock::new();
    RUNNING
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Starts or stops Cancella tutto, or gives back any other command.
pub(super) fn command(
    command: Command,
    discord: Result<&Discord, &str>,
    events: &UnboundedSender<Event>,
) -> Option<Command> {
    match command {
        Command::Discord(DiscordCommand::DeleteAll { place }) => {
            start(place, discord, events);
            None
        }
        Command::Discord(DiscordCommand::StopDeleteAll { place }) => {
            if let Some(stop) = running().get(&place) {
                stop.store(true, Ordering::Relaxed);
            }
            None
        }
        command => Some(command),
    }
}

/// Where a place's messages are searched: a server, or a DM's channel.
fn target(place: &str) -> Option<(Option<String>, String)> {
    if let Some(guild) = discord::parse_server(place) {
        return Some((Some(guild.to_owned()), String::new()));
    }
    discord::parse_room(place)
        .filter(|(guild, _)| *guild == discord::DM_GUILD)
        .map(|(_, channel)| (None, channel.to_owned()))
}

fn start(place: String, discord: Result<&Discord, &str>, events: &UnboundedSender<Event>) {
    let ended = |error: &str| Event::DeletedAll {
        place: place.clone(),
        deleted: 0,
        ended: true,
        error: Some(error.to_owned()),
    };
    let (discord, (guild, channel)) = match (discord, target(&place)) {
        (Ok(discord), Some(target)) => (discord.clone(), target),
        (Err(why), _) => {
            let _ = events.send(ended(why));
            return;
        }
        (Ok(_), None) => {
            let _ = events.send(ended(NOT_DISCORD));
            return;
        }
    };
    let stop = Arc::new(AtomicBool::new(false));
    {
        let mut running = running();
        if running.contains_key(&place) {
            return;
        }
        running.insert(place.clone(), stop.clone());
    }
    let events = events.clone();
    tokio::spawn(async move {
        let progress = {
            let (events, place) = (events.clone(), place.clone());
            move |done: Purged| {
                let _ = events.send(Event::DeletedAll {
                    place: place.clone(),
                    deleted: done.deleted,
                    ended: false,
                    error: None,
                });
            }
        };
        let result = discord
            .delete_own_messages(guild.as_deref(), &channel, &stop, &progress)
            .await;
        running().remove(&place);
        let (deleted, error) = match result {
            Ok(done) => {
                tracing::info!(
                    deleted = done.deleted,
                    skipped = done.skipped,
                    "Cancella tutto ended"
                );
                (done.deleted, None)
            }
            Err(error) => {
                tracing::warn!(error = format!("{error:#}"), "Cancella tutto stopped");
                (0, Some(error.to_string()))
            }
        };
        let _ = events.send(Event::DeletedAll {
            place,
            deleted,
            ended: true,
            error,
        });
    });
}
