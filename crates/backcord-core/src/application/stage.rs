//! A Discord stage the call is on, as Discord's client shows it: its topic
//! over the call, its speakers as the call's tiles and its audience under
//! them, with raised hands; this account's place there, with Request to
//! Speak, Move to Audience and an invitation's yes or no; and, for its
//! moderators (Manage Channels, Mute Members and Move Members on it),
//! starting it with a topic, editing it, ending it, and inviting someone to
//! speak or moving them to the audience. See "Stages" in
//! docs/discord/voice.md.

use super::*;
use crate::chat::StageChange;

/// The longest topic Discord takes.
pub const STAGE_TOPIC_LIMIT: usize = 120;

/// Where this account is on a stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageRole {
    Speaker,
    /// In the audience, its hand raised or not.
    Audience {
        hand: bool,
    },
    /// Invited to speak, not yet said yes or no.
    Invited,
}

/// The stage the call is on, as its view shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallStage {
    /// The stage channel's room id.
    pub room: String,
    /// Its topic while it's live; none while nobody started it.
    pub topic: Option<String>,
    /// This account moderates it.
    pub moderator: bool,
    /// This account may ask to speak.
    pub may_request: bool,
    pub role: StageRole,
}

/// Starting a stage, or changing its topic, while its dialog is open.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TopicEditor {
    pub topic: String,
    /// Starting it, rather than changing a live one's topic.
    pub starting: bool,
    pub busy: bool,
    pub error: Option<String>,
}

impl TopicEditor {
    pub fn ready(&self) -> bool {
        let length = self.topic.trim().chars().count();
        !self.busy && (1..=STAGE_TOPIC_LIMIT).contains(&length)
    }
}

/// What the stage's controls keep.
#[derive(Clone, Debug, Default)]
pub struct StageState {
    pub editor: Option<TopicEditor>,
    /// Why Discord didn't make the last change.
    pub error: Option<String>,
}

/// Something done on a stage.
pub enum StageInput {
    /// Opens the topic's dialog: to start the stage, or change its topic;
    /// `false` closes it.
    EditTopic(bool),
    WriteTopic(String),
    SaveTopic,
    End,
    Hand(bool),
    Speak,
    ToAudience,
    /// Invite someone in the audience to speak.
    Invite(String),
    /// Move someone speaking to the audience.
    MoveToAudience(String),
    DismissError,
}

impl Application {
    /// The stage the call is on, if it's on one.
    pub fn call_stage(&self) -> Option<CallStage> {
        #[cfg(feature = "discord")]
        {
            let voice = &self.state.voice;
            if !voice.active() {
                return None;
            }
            let room = voice.room.as_deref()?;
            let (server, Some(channel), _) = self.discord_place(room)? else {
                return None;
            };
            server
                .voice_channels
                .iter()
                .find(|found| found.id == channel && found.stage)?;
            let may = |permission| self.may(room, permission);
            let local = voice.participants.iter().find(|person| person.local);
            let role = match local {
                Some(person) if person.audience => StageRole::Audience {
                    hand: person.requested_to_speak.is_some(),
                },
                Some(person) if person.invited() => StageRole::Invited,
                _ => StageRole::Speaker,
            };
            Some(CallStage {
                room: room.to_owned(),
                topic: server.stage(channel).map(|stage| stage.topic.clone()),
                moderator: may(crate::chat::Permission::ManageChannels)
                    && may(crate::chat::Permission::MuteMembers)
                    && may(crate::chat::Permission::MoveMembers),
                may_request: may(crate::chat::Permission::RequestToSpeak),
                role,
            })
        }
        #[cfg(not(feature = "discord"))]
        None
    }

    pub(super) fn stage_input(&mut self, input: StageInput) -> Vec<Effect> {
        let Some(stage) = self.call_stage() else {
            self.state.stage.editor = None;
            return vec![];
        };
        let change = match input {
            StageInput::EditTopic(open) => {
                self.state.stage.editor = (open && stage.moderator).then(|| TopicEditor {
                    starting: stage.topic.is_none(),
                    topic: stage.topic.clone().unwrap_or_default(),
                    ..TopicEditor::default()
                });
                return vec![];
            }
            StageInput::WriteTopic(topic) => {
                if let Some(editor) = self.state.stage.editor.as_mut().filter(|e| !e.busy) {
                    editor.topic = topic.chars().take(STAGE_TOPIC_LIMIT).collect();
                    editor.error = None;
                }
                return vec![];
            }
            StageInput::SaveTopic => {
                let Some(editor) = self.state.stage.editor.as_mut().filter(|e| e.ready()) else {
                    return vec![];
                };
                editor.busy = true;
                let topic = editor.topic.trim().to_owned();
                if editor.starting {
                    StageChange::Start(topic)
                } else {
                    StageChange::Topic(topic)
                }
            }
            StageInput::End if stage.moderator && stage.topic.is_some() => StageChange::End,
            StageInput::Hand(raise)
                if matches!(stage.role, StageRole::Audience { .. })
                    && (stage.may_request || !raise) =>
            {
                StageChange::Hand(raise)
            }
            // Saying yes, or a moderator stepping up.
            StageInput::Speak
                if stage.role == StageRole::Invited
                    || (stage.moderator && stage.role != StageRole::Speaker) =>
            {
                StageChange::Speak
            }
            StageInput::ToAudience if stage.role != (StageRole::Audience { hand: false }) => {
                StageChange::Audience
            }
            StageInput::Invite(user) if stage.moderator => StageChange::Invite(user),
            StageInput::MoveToAudience(user) if stage.moderator => StageChange::ToAudience(user),
            StageInput::DismissError => {
                self.state.stage.error = None;
                return vec![];
            }
            _ => return vec![],
        };
        self.state.stage.error = None;
        vec![Effect::Backend(Command::Discord(DiscordCommand::Stage {
            room: stage.room,
            change,
        }))]
    }

    pub(super) fn stage_changed(&mut self, change: StageChange, result: Result<(), String>) {
        let topic = matches!(change, StageChange::Start(_) | StageChange::Topic(_));
        match result {
            Ok(()) if topic => self.state.stage.editor = None,
            Ok(()) => {}
            Err(error) => match self.state.stage.editor.as_mut().filter(|_| topic) {
                Some(editor) => {
                    editor.busy = false;
                    editor.error = Some(error);
                }
                None => self.state.stage.error = Some(error),
            },
        }
    }
}
