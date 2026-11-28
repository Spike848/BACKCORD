//! A server's scheduled events, as its Events dialog lists, makes and
//! changes them, and making a server, from nothing or from a template; see
//! "Scheduled events" and "Making a server" in docs/discord/servers.md.

use backcord_discord::scheduled_event::EventForm;

use super::{
    commands::NOT_DISCORD,
    own_profile::{picture, read_shaped},
    *,
};
use crate::chat::{
    EventDraft, EventPerson, EventsAnswer, EventsCommand, NewServerAnswer, NewServerCommand,
    TemplatePreview,
};

/// A cover's largest size, Discord's 2.5 by 1, and its preview's.
const COVER: (u32, u32) = (800, 320);
const COVER_PREVIEW: (u32, u32) = (200, 80);
/// A new server's icon, and its preview's.
const ICON: (u32, u32) = (1024, 1024);
const ICON_PREVIEW: (u32, u32) = (128, 128);

/// Carries out an Events dialog's or a new server's command, or gives back
/// any other.
pub(super) fn command(
    command: Command,
    discord: Result<&Discord, &str>,
    events: &UnboundedSender<Event>,
) -> Option<Command> {
    match command {
        Command::Discord(DiscordCommand::Events(asked)) => {
            let Ok(discord) = discord else {
                let why = discord.err().unwrap_or(NOT_DISCORD).to_owned();
                let _ = events.send(Event::Events(refused(asked, why)));
                return None;
            };
            let (discord, events) = (discord.clone(), events.clone());
            tokio::spawn(async move {
                let answer = events_answer(&discord, asked).await;
                let _ = events.send(Event::Events(answer));
            });
            None
        }
        Command::Discord(DiscordCommand::NewServer(asked)) => {
            let Ok(discord) = discord else {
                let why = discord.err().unwrap_or(NOT_DISCORD).to_owned();
                let answer = match asked {
                    NewServerCommand::Template { code } => NewServerAnswer::Template {
                        code,
                        result: Err(why),
                    },
                    NewServerCommand::Create { .. } => NewServerAnswer::Created { result: Err(why) },
                };
                let _ = events.send(Event::NewServer(answer));
                return None;
            };
            let (discord, events) = (discord.clone(), events.clone());
            tokio::spawn(async move {
                let answer = new_server_answer(&discord, asked).await;
                let _ = events.send(Event::NewServer(answer));
            });
            None
        }
        other => Some(other),
    }
}

/// The answer to a command that couldn't go, for why.
fn refused(asked: EventsCommand, why: String) -> EventsAnswer {
    match asked {
        EventsCommand::Load { guild } => EventsAnswer::Loaded {
            guild,
            result: Err(why),
        },
        EventsCommand::Users { event, .. } => EventsAnswer::Users {
            event,
            result: Err(why),
        },
        EventsCommand::Save { guild, event, .. } => EventsAnswer::Changed {
            guild,
            event,
            result: Err(why),
        },
        EventsCommand::Status { guild, event, .. } | EventsCommand::Delete { guild, event } => {
            EventsAnswer::Changed {
                guild,
                event: Some(event),
                result: Err(why),
            }
        }
    }
}

async fn events_answer(discord: &Discord, asked: EventsCommand) -> EventsAnswer {
    let failed = |what: &str| {
        let what = what.to_owned();
        move |error: anyhow::Error| {
            tracing::warn!(error = format!("{error:#}"), "could not {what}");
            error.to_string()
        }
    };
    match asked {
        EventsCommand::Load { guild } => EventsAnswer::Loaded {
            result: discord
                .load_events(&guild)
                .await
                .map_err(failed("list a server's events")),
            guild,
        },
        EventsCommand::Users { guild, event } => EventsAnswer::Users {
            result: discord
                .event_users(&guild, &event)
                .await
                .map(|users| {
                    users
                        .into_iter()
                        .map(|user| EventPerson {
                            id: user.id,
                            name: user.name,
                            avatar: user.avatar,
                        })
                        .collect()
                })
                .map_err(failed("list who is interested in an event")),
            event,
        },
        EventsCommand::Save { guild, event, draft } => {
            let result = match event_form(&draft) {
                Ok(form) => discord
                    .save_event(&guild, event.as_deref(), &form.body())
                    .await
                    .map_err(failed("save an event")),
                Err(error) => Err(error),
            };
            EventsAnswer::Changed {
                guild,
                event,
                result,
            }
        }
        EventsCommand::Status {
            guild,
            event,
            status,
        } => EventsAnswer::Changed {
            result: discord
                .save_event(&guild, Some(&event), &serde_json::json!({ "status": status }))
                .await
                .map_err(failed("change an event's status")),
            guild,
            event: Some(event),
        },
        EventsCommand::Delete { guild, event } => EventsAnswer::Changed {
            result: discord
                .delete_event(&guild, &event)
                .await
                .map_err(failed("delete an event")),
            guild,
            event: Some(event),
        },
    }
}

/// What the form sends for `draft`, its cover read from its file.
fn event_form(draft: &EventDraft) -> Result<EventForm, String> {
    let cover = match &draft.cover {
        Some(path) => Some(picture(&Some(read_shaped(
            std::path::Path::new(path),
            COVER,
            COVER_PREVIEW,
            "a cover picture",
        )?))),
        None => None,
    };
    Ok(EventForm {
        kind: draft.kind,
        channel: draft.channel.clone(),
        location: draft.location.clone(),
        name: draft.name.clone(),
        description: draft.description.clone(),
        start: iso(draft.start),
        end: draft.end.map(iso),
        image: cover.and_then(|cover| cover.as_str().map(str::to_owned)),
    })
}

/// Unix seconds as Discord's ISO 8601 time.
fn iso(seconds: i64) -> String {
    chrono::DateTime::from_timestamp(seconds, 0)
        .unwrap_or_default()
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

async fn new_server_answer(discord: &Discord, asked: NewServerCommand) -> NewServerAnswer {
    match asked {
        NewServerCommand::Template { code } => NewServerAnswer::Template {
            result: discord
                .guild_template(&code)
                .await
                .map(|template| TemplatePreview {
                    code: template.code,
                    name: template.name,
                    description: template.description,
                    usage_count: template.usage_count,
                    channels: template.channels,
                    roles: template.roles,
                })
                .map_err(|error| {
                    tracing::info!(error = format!("{error:#}"), "no such server template");
                    error.to_string()
                }),
            code,
        },
        NewServerCommand::Create {
            name,
            icon,
            template,
        } => {
            let icon = match icon {
                Some(path) => match read_shaped(std::path::Path::new(&path), ICON, ICON_PREVIEW, "a server icon") {
                    Ok(picked) => picture(&Some(picked)).as_str().map(str::to_owned),
                    Err(error) => return NewServerAnswer::Created { result: Err(error) },
                },
                None => None,
            };
            NewServerAnswer::Created {
                result: discord
                    .create_server(&name, icon.as_deref(), template.as_deref())
                    .await
                    .map_err(|error| {
                        tracing::warn!(error = format!("{error:#}"), "could not make a server");
                        format!("Couldn't make the server: {error}")
                    }),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_draft_becomes_what_discord_takes() {
        let form = event_form(&EventDraft {
            kind: 3,
            location: Some("Park".into()),
            name: "Picnic".into(),
            start: 1_900_000_000,
            end: Some(1_900_003_600),
            ..EventDraft::default()
        })
        .unwrap();
        assert_eq!(form.start, "2030-03-17T17:46:40Z");
        assert_eq!(form.end.as_deref(), Some("2030-03-17T18:46:40Z"));
        assert!(form.image.is_none());
        let missing = event_form(&EventDraft {
            cover: Some("/nowhere/cover.png".into()),
            ..EventDraft::default()
        });
        assert!(missing.unwrap_err().contains("cover.png"));
    }
}
