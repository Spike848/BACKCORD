//! A stage the call is on, as Discord's client changes it: starting it with
//! a topic, changing the topic, ending it, raising a hand, saying yes or no
//! to an invitation, and a moderator inviting someone or moving them to the
//! audience. See "Stages" in docs/discord/voice.md.

use super::{commands::NOT_DISCORD, *};
use crate::chat::StageChange;

/// Changes a stage, or gives back any other command.
pub(super) fn command(
    command: Command,
    discord: Result<&Discord, &str>,
    events: &UnboundedSender<Event>,
) -> Option<Command> {
    let Command::Discord(DiscordCommand::Stage { room, change }) = command else {
        return Some(command);
    };
    let (Ok(discord), Some((guild, channel))) = (discord, discord::parse_room(&room)) else {
        let why = discord.err().unwrap_or(NOT_DISCORD).to_owned();
        let _ = events.send(Event::StageChanged {
            change,
            result: Err(why),
        });
        return None;
    };
    let (discord, guild, channel, events) = (
        discord.clone(),
        guild.to_owned(),
        channel.to_owned(),
        events.clone(),
    );
    tokio::spawn(async move {
        let (guild, channel) = (guild.as_str(), channel.as_str());
        let result = match &change {
            StageChange::Start(topic) => discord.start_stage(guild, channel, topic.trim()).await,
            StageChange::Topic(topic) => discord.edit_stage_topic(channel, topic.trim()).await,
            StageChange::End => discord.end_stage(channel).await,
            StageChange::Hand(raise) => discord.request_to_speak(guild, channel, *raise).await,
            StageChange::Speak => discord.become_speaker(guild, channel).await,
            StageChange::Audience => discord.move_to_audience(guild, channel).await,
            StageChange::Invite(user) => discord.set_speaker(guild, channel, user, true).await,
            StageChange::ToAudience(user) => discord.set_speaker(guild, channel, user, false).await,
        }
        .map_err(|error| error.to_string());
        if let Err(error) = &result {
            tracing::info!(error, ?change, "Discord kept the stage as it was");
        }
        let _ = events.send(Event::StageChanged { change, result });
    });
    None
}
