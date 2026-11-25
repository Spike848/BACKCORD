//! Explore servers: DISBOARD's pages of Italian servers, and a listed
//! server's public invite. See "Exploring Italian servers" in
//! docs/discord/servers.md.

use backcord_discord::explore;

use super::*;
use crate::chat::ExploreServer;

/// Carries out a command of Explore servers, or gives back any other. It
/// needs no Discord login: both are asked as anyone may.
pub(super) fn command(command: Command, events: &UnboundedSender<Event>) -> Option<Command> {
    let Command::Discord(asked) = command else {
        return Some(command);
    };
    let events = events.clone();
    match asked {
        DiscordCommand::ExplorePage { source } => {
            tokio::spawn(async move {
                let mut saved = false;
                let result = explore::page(source)
                    .await
                    .map(|(listed, kept)| {
                        saved = kept;
                        listed
                            .into_iter()
                            .map(|server| ExploreServer {
                                guild: server.id,
                                name: server.name,
                                icon: server.icon,
                                description: server.description,
                                tags: server.tags,
                                online: server.online,
                            })
                            .collect()
                    })
                    .map_err(|error| {
                        tracing::warn!(error = format!("{error:#}"), "could not read DISBOARD");
                        if error.to_string() == explore::SLOW_DOWN {
                            "DISBOARD asks to slow down for a few minutes. Try again later."
                                .to_owned()
                        } else {
                            format!("Couldn't load the servers: {error}")
                        }
                    });
                let _ = events.send(Event::ExplorePage {
                    source,
                    result,
                    saved,
                });
            });
        }
        DiscordCommand::ExploreInvite { guild } => {
            tokio::spawn(async move {
                let result = explore::widget_invite(&guild).await.map_err(|error| {
                    tracing::warn!(
                        error = format!("{error:#}"),
                        "could not read a server's widget"
                    );
                    format!("Couldn't find the server's invite: {error}")
                });
                let _ = events.send(Event::ExploreInvite { guild, result });
            });
        }
        other => return Some(Command::Discord(other)),
    }
    None
}
