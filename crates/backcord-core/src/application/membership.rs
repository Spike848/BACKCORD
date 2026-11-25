//! A Discord server's rules and questions for its members: the form a new
//! member fills in before talking, such as agreeing to its rules, and its
//! onboarding questions, whose answers give roles and channels. Joining a
//! server opens the dialog when the server asks anything, and Channels &
//! Roles in a Community server's menu opens it to change the answers.

use super::*;
use crate::chat::{MemberAnswer, MemberFieldKind, Membership};

/// Where the dialog is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MembershipStep {
    /// The server's form.
    Form,
    /// The form went to the server's moderators, who decide.
    Waiting,
    /// One of the questions, by index.
    Question(usize),
    /// The server asks nothing.
    Nothing,
}

/// The dialog of a server's rules and questions, while it's open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MembershipDialog {
    /// The server, by its id, and its name.
    pub server: String,
    pub name: String,
    /// From the server's menu rather than from joining: it shows while
    /// loading, and says so when the server asks nothing.
    pub asked: bool,
    /// Joining said the server has a form.
    pub form: bool,
    pub loading: bool,
    pub gate: Option<Membership>,
    pub step: MembershipStep,
    /// The answers to the form's fields, in order.
    pub answers: Vec<MemberAnswer>,
    /// The choices picked among the questions'.
    pub chosen: Vec<String>,
    pub sending: bool,
    pub error: Option<String>,
}

impl MembershipDialog {
    /// Whether it shows: from the menu at once, after joining once the
    /// server turned out to ask something.
    pub fn shown(&self) -> bool {
        self.asked || self.form || (!self.loading && self.gate.is_some())
    }

    /// Whether the form has every answer it needs.
    pub fn form_ready(&self) -> bool {
        let Some(gate) = &self.gate else {
            return false;
        };
        gate.form.iter().enumerate().all(|(at, field)| {
            let answer = self.answers.get(at).unwrap_or(&MemberAnswer::Nothing);
            match field.kind {
                MemberFieldKind::Rules => *answer == MemberAnswer::Agreed,
                _ if !field.required => true,
                MemberFieldKind::Short | MemberFieldKind::Paragraph => {
                    matches!(answer, MemberAnswer::Text(typed) if !typed.trim().is_empty())
                }
                MemberFieldKind::Choice => matches!(answer, MemberAnswer::Choice(_)),
                MemberFieldKind::Unknown => true,
            }
        })
    }

    /// Whether a question has what it needs to go on: a choice, when it
    /// requires one.
    pub fn answered(&self, question: usize) -> bool {
        let Some(question) = self
            .gate
            .as_ref()
            .and_then(|gate| gate.questions.get(question))
        else {
            return true;
        };
        !question.required
            || question
                .options
                .iter()
                .any(|option| self.chosen.contains(&option.id))
    }
}

impl Application {
    /// Opens Channels & Roles of a Discord server, from its menu.
    pub(super) fn open_membership(&mut self, server: String) -> Vec<Effect> {
        self.ask_membership(server, true, false)
    }

    /// Asks what a server just joined asks of its new members.
    pub(super) fn ask_membership(
        &mut self,
        server: String,
        asked: bool,
        form: bool,
    ) -> Vec<Effect> {
        if self.state.preview || !self.state.discord_online {
            return vec![];
        }
        let name = self
            .state
            .servers
            .iter()
            .find(|listed| listed.id == server)
            .map(|listed| listed.name.clone())
            .unwrap_or_default();
        self.state.membership = Some(MembershipDialog {
            server: server.clone(),
            name,
            asked,
            form,
            loading: true,
            gate: None,
            step: MembershipStep::Nothing,
            answers: vec![],
            chosen: vec![],
            sending: false,
            error: None,
        });
        vec![Effect::Backend(Command::Discord(
            DiscordCommand::Membership { server, form },
        ))]
    }

    pub(super) fn membership_loaded(&mut self, server: String, result: Result<Membership, String>) {
        let Some(dialog) = self
            .state
            .membership
            .as_mut()
            .filter(|dialog| dialog.loading && dialog.server == server)
        else {
            return;
        };
        dialog.loading = false;
        let gate = match result {
            Ok(gate) => gate,
            Err(error) => {
                if dialog.shown() {
                    dialog.error = Some(error);
                } else {
                    self.state.membership = None;
                }
                return;
            }
        };
        if dialog.name.is_empty()
            && let Some(listed) = self.state.servers.iter().find(|listed| listed.id == server)
        {
            dialog.name.clone_from(&listed.name);
        }
        dialog.step = if !gate.form.is_empty() {
            MembershipStep::Form
        } else if !gate.questions.is_empty() {
            MembershipStep::Question(0)
        } else {
            MembershipStep::Nothing
        };
        if dialog.step == MembershipStep::Nothing && !dialog.asked {
            self.state.membership = None;
            return;
        }
        dialog.answers = vec![MemberAnswer::Nothing; gate.form.len()];
        dialog.chosen.clone_from(&gate.chosen);
        dialog.gate = Some(gate);
    }

    /// Answers a field of the form, by its index.
    pub(super) fn answer_member_field(&mut self, index: usize, answer: MemberAnswer) {
        if let Some(dialog) = self
            .state
            .membership
            .as_mut()
            .filter(|dialog| !dialog.sending)
            && let Some(slot) = dialog.answers.get_mut(index)
        {
            *slot = answer;
        }
    }

    /// Sends the form, once it has every answer it needs.
    pub(super) fn send_member_form(&mut self) -> Vec<Effect> {
        let Some(dialog) = self.state.membership.as_mut().filter(|dialog| {
            !dialog.sending && dialog.step == MembershipStep::Form && dialog.form_ready()
        }) else {
            return vec![];
        };
        dialog.sending = true;
        dialog.error = None;
        vec![Effect::Backend(Command::Discord(
            DiscordCommand::SendMemberForm {
                server: dialog.server.clone(),
                answers: dialog.answers.clone(),
            },
        ))]
    }

    pub(super) fn member_form_sent(&mut self, server: String, result: Result<bool, String>) {
        let Some(dialog) = self
            .state
            .membership
            .as_mut()
            .filter(|dialog| dialog.sending && dialog.server == server)
        else {
            return;
        };
        dialog.sending = false;
        match result {
            Err(error) => dialog.error = Some(error),
            Ok(false) => dialog.step = MembershipStep::Waiting,
            Ok(true) => {
                if dialog
                    .gate
                    .as_ref()
                    .is_some_and(|gate| !gate.questions.is_empty())
                {
                    dialog.step = MembershipStep::Question(0);
                } else {
                    self.state.membership = None;
                }
            }
        }
    }

    /// Picks a question's choice, or takes it back; picking one of a
    /// single-choice question takes back the others.
    pub(super) fn toggle_member_choice(&mut self, question: usize, option: String) {
        let Some(dialog) = self
            .state
            .membership
            .as_mut()
            .filter(|dialog| !dialog.sending)
        else {
            return;
        };
        let Some(asked) = dialog
            .gate
            .as_ref()
            .and_then(|gate| gate.questions.get(question))
        else {
            return;
        };
        if let Some(at) = dialog.chosen.iter().position(|id| *id == option) {
            dialog.chosen.remove(at);
            return;
        }
        if !asked.options.iter().any(|known| known.id == option) {
            return;
        }
        if asked.single {
            dialog
                .chosen
                .retain(|id| !asked.options.iter().any(|known| known.id == *id));
        }
        dialog.chosen.push(option);
    }

    /// Goes to a question, by its index: back always, on only once the
    /// questions before it are answered.
    pub(super) fn go_to_member_question(&mut self, question: usize) {
        let Some(dialog) = self.state.membership.as_mut() else {
            return;
        };
        let count = dialog.gate.as_ref().map_or(0, |gate| gate.questions.len());
        if question < count && (0..question).all(|before| dialog.answered(before)) {
            dialog.step = MembershipStep::Question(question);
        }
    }

    /// Sends the choices, once the required questions are answered.
    pub(super) fn finish_onboarding(&mut self) -> Vec<Effect> {
        let Some(dialog) =
            self.state.membership.as_mut().filter(|dialog| {
                !dialog.sending && matches!(dialog.step, MembershipStep::Question(_))
            })
        else {
            return vec![];
        };
        let count = dialog.gate.as_ref().map_or(0, |gate| gate.questions.len());
        if !(0..count).all(|question| dialog.answered(question)) {
            return vec![];
        }
        dialog.sending = true;
        dialog.error = None;
        vec![Effect::Backend(Command::Discord(
            DiscordCommand::AnswerOnboarding {
                server: dialog.server.clone(),
                chosen: dialog.chosen.clone(),
            },
        ))]
    }

    pub(super) fn onboarding_answered(&mut self, server: String, result: Result<(), String>) {
        let Some(dialog) = self
            .state
            .membership
            .as_mut()
            .filter(|dialog| dialog.sending && dialog.server == server)
        else {
            return;
        };
        dialog.sending = false;
        match result {
            Ok(()) => self.state.membership = None,
            Err(error) => dialog.error = Some(error),
        }
    }

    /// Opens the server in Discord's web app, where its form waits, when
    /// BACKCORD couldn't load it.
    pub(super) fn open_membership_in_browser(&mut self) -> Vec<Effect> {
        let Some(dialog) = self.state.membership.take() else {
            return vec![];
        };
        let guild = discord::parse_server(&dialog.server).unwrap_or_default();
        vec![Effect::OpenPage(format!(
            "https://discord.com/channels/{guild}"
        ))]
    }
}
