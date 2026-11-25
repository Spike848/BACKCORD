//! A Discord server's gate for new members: its form and its onboarding
//! questions, fetched and answered. See "Joining a server's rules and
//! questions" in docs/discord/servers.md.

use backcord_discord::membership as gate;

use super::{commands::NOT_DISCORD, *};
use crate::chat::{
    MemberAnswer, MemberField, MemberFieldKind, MemberOption, MemberQuestion, Membership,
};

/// Carries out a command about a server's gate, or gives back any other.
pub(super) fn command(
    command: Command,
    discord: Result<&Discord, &str>,
    events: &UnboundedSender<Event>,
) -> Option<Command> {
    let Command::Discord(asked) = command else {
        return Some(command);
    };
    let events = events.clone();
    match asked {
        DiscordCommand::Membership { server, form } => {
            let (Ok(discord), Some(guild)) = (discord, discord::parse_server(&server)) else {
                let _ = events.send(Event::Membership {
                    server,
                    result: Err(NOT_DISCORD.into()),
                });
                return None;
            };
            let (discord, guild) = (discord.clone(), guild.to_owned());
            tokio::spawn(async move {
                let result = discord
                    .membership(&guild, form)
                    .await
                    .map(|(form, onboarding)| membership(form, onboarding))
                    .map_err(|error| {
                        tracing::warn!(
                            error = format!("{error:#}"),
                            "could not fetch a server's gate"
                        );
                        format!("Discord didn't send the server's questions: {error}")
                    });
                let _ = events.send(Event::Membership { server, result });
            });
        }
        DiscordCommand::SendMemberForm { server, answers } => {
            let (Ok(discord), Some(guild)) = (discord, discord::parse_server(&server)) else {
                let _ = events.send(Event::MemberFormSent {
                    server,
                    result: Err(NOT_DISCORD.into()),
                });
                return None;
            };
            let (discord, guild) = (discord.clone(), guild.to_owned());
            tokio::spawn(async move {
                let answers: Vec<gate::Answer> = answers.into_iter().map(answer).collect();
                let result = discord
                    .send_member_form(&guild, &answers)
                    .await
                    .map_err(|error| {
                        tracing::warn!(
                            error = format!("{error:#}"),
                            "could not send a server's form"
                        );
                        format!("Discord didn't take your answers: {error}")
                    });
                let _ = events.send(Event::MemberFormSent { server, result });
            });
        }
        DiscordCommand::AnswerOnboarding { server, chosen } => {
            let (Ok(discord), Some(guild)) = (discord, discord::parse_server(&server)) else {
                let _ = events.send(Event::OnboardingAnswered {
                    server,
                    result: Err(NOT_DISCORD.into()),
                });
                return None;
            };
            let (discord, guild) = (discord.clone(), guild.to_owned());
            tokio::spawn(async move {
                let result = discord
                    .answer_onboarding(&guild, &chosen)
                    .await
                    .map_err(|error| {
                        tracing::warn!(error = format!("{error:#}"), "could not answer onboarding");
                        format!("Discord didn't take your answers: {error}")
                    });
                let _ = events.send(Event::OnboardingAnswered { server, result });
            });
        }
        other => return Some(Command::Discord(other)),
    }
    None
}

fn answer(answer: MemberAnswer) -> gate::Answer {
    match answer {
        MemberAnswer::Nothing => gate::Answer::Nothing,
        MemberAnswer::Agreed => gate::Answer::Agreed,
        MemberAnswer::Text(typed) => gate::Answer::Text(typed),
        MemberAnswer::Choice(index) => gate::Answer::Choice(index),
    }
}

/// The gate as the dialog shows it: the form, and the questions asked when
/// joining first, then those only Channels & Roles asks. A server whose
/// onboarding is off asks none.
fn membership(form: Option<gate::Form>, onboarding: gate::Onboarding) -> Membership {
    let (description, fields) = form.map_or((None, vec![]), |form| (form.description, form.fields));
    let mut questions = if onboarding.enabled {
        onboarding.questions
    } else {
        vec![]
    };
    questions.sort_by_key(|question| !question.in_onboarding);
    Membership {
        description,
        form: fields
            .into_iter()
            .map(|field| MemberField {
                kind: match field.kind {
                    gate::FieldKind::Terms => MemberFieldKind::Rules,
                    gate::FieldKind::Text => MemberFieldKind::Short,
                    gate::FieldKind::Paragraph => MemberFieldKind::Paragraph,
                    gate::FieldKind::Choice => MemberFieldKind::Choice,
                    gate::FieldKind::Other => MemberFieldKind::Unknown,
                },
                label: field.label,
                description: field.description,
                placeholder: field.placeholder,
                rules: field.rules,
                choices: field.choices,
                required: field.required,
            })
            .collect(),
        questions: questions
            .into_iter()
            .map(|question| MemberQuestion {
                id: question.id,
                title: question.title,
                single: question.single,
                required: question.required && question.in_onboarding,
                options: question
                    .options
                    .into_iter()
                    .map(|option| MemberOption {
                        id: option.id,
                        title: option.title,
                        description: option.description,
                        emoji: option.emoji,
                        emoji_picture: option.emoji_picture,
                    })
                    .collect(),
            })
            .collect(),
        chosen: onboarding.chosen,
    }
}
