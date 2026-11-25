//! A Discord server's rules and questions for its members (`state.membership`):
//! its form, with its rules to agree to and its questions to answer, then
//! its onboarding questions one at a time, each choice a row to tick.

use backcord_core::{
    application::MembershipStep,
    chat::{MemberAnswer, MemberField, MemberFieldKind, MemberOption},
};
use backcord_iced_kit::{i18n::tr, widgets::scroll_view};
use iced::{
    Border, Element, Length,
    widget::{Space, button, column, container, row, text},
};

use super::{Event, NARROW, View, message};
use crate::{
    app::Message,
    controls::{Kind, action},
    dialog::buttons,
    form,
    style::rounded,
    widgets::Pic,
};

/// The fields' ids, one for each of a form's at most five.
pub(super) const FIELDS: [&str; 5] = [
    "member-field-0",
    "member-field-1",
    "member-field-2",
    "member-field-3",
    "member-field-4",
];
/// How tall a question's choices get before they scroll.
const CHOICES_HEIGHT: f32 = 360.0;
/// A choice's emoji.
const EMOJI: u32 = 24;

impl<'a> View<'a> {
    pub(super) fn membership_view(self) -> Option<Element<'a, Message>> {
        let dialog = self
            .cx
            .state
            .membership
            .as_ref()
            .filter(|dialog| dialog.shown())?;
        let theme = self.cx.look.theme;
        let close = message(Event::CloseMembership);
        let name = if dialog.name.is_empty() {
            tr("this server").to_owned()
        } else {
            dialog.name.clone()
        };
        let mut body = vec![self.cx.look.dialog_head(
            backcord_iced_kit::i18n::t(format!("Channels & Roles of {name}")),
            close.clone(),
        )];
        let gate = dialog.gate.as_ref();
        let mut row_: Vec<Element<'a, Message>> = vec![];
        if dialog.loading {
            body.push(form::note(theme, tr("Loading...")));
        } else if let Some(gate) = gate {
            match dialog.step {
                MembershipStep::Form => {
                    body.push(form::note(
                        theme,
                        tr("Complete these steps before you can start talking."),
                    ));
                    if let Some(description) = &gate.description {
                        body.push(form::note_in(description.as_str(), theme.secondary));
                    }
                    let mut fields = column![].spacing(18);
                    for (at, field) in gate.form.iter().enumerate() {
                        fields = fields.push(self.member_field(at, field));
                    }
                    body.push(
                        container(scroll_view(fields.padding(iced::Padding {
                            right: 12.0,
                            ..iced::Padding::ZERO
                        })))
                        .max_height(CHOICES_HEIGHT + 60.0)
                        .into(),
                    );
                    row_.push(action(theme, tr("Cancel"), Kind::Ghost, Some(close.clone())).into());
                    row_.push(
                        action(
                            theme,
                            tr("Submit"),
                            Kind::Primary,
                            (dialog.form_ready() && !dialog.sending)
                                .then(|| message(Event::SendMemberForm)),
                        )
                        .into(),
                    );
                }
                MembershipStep::Waiting => {
                    body.push(form::note(
                        theme,
                        tr("Your application was sent. The server's moderators will review it, and Discord will let you know."),
                    ));
                    row_.push(
                        action(theme, tr("Close"), Kind::Primary, Some(close.clone())).into(),
                    );
                }
                MembershipStep::Question(at) => {
                    let count = gate.questions.len();
                    let question = gate.questions.get(at)?;
                    body.push(form::heading(
                        theme,
                        backcord_iced_kit::i18n::t(format!("QUESTION {} OF {count}", at + 1)),
                    ));
                    body.push(
                        text(question.title.as_str())
                            .size(18)
                            .font(backcord_iced_kit::fonts::BOLD)
                            .color(theme.text)
                            .into(),
                    );
                    body.push(form::note(
                        theme,
                        match (question.single, question.required) {
                            (true, true) => tr("Pick one."),
                            (true, false) => tr("Pick one, if you like."),
                            (false, true) => tr("Pick at least one."),
                            (false, false) => tr("Pick as many as you like."),
                        },
                    ));
                    let mut choices = column![].spacing(8);
                    for option in &question.options {
                        let chosen = dialog.chosen.contains(&option.id);
                        choices =
                            choices.push(self.member_choice(at, option, chosen, question.single));
                    }
                    body.push(
                        container(scroll_view(choices.padding(iced::Padding {
                            right: 12.0,
                            ..iced::Padding::ZERO
                        })))
                        .max_height(CHOICES_HEIGHT)
                        .into(),
                    );
                    let ready = dialog.answered(at) && !dialog.sending;
                    row_.push(if at > 0 {
                        action(
                            theme,
                            tr("Back"),
                            Kind::Ghost,
                            (!dialog.sending).then(|| message(Event::MemberQuestion(at - 1))),
                        )
                        .into()
                    } else {
                        action(theme, tr("Cancel"), Kind::Ghost, Some(close.clone())).into()
                    });
                    row_.push(if at + 1 < count {
                        action(
                            theme,
                            tr("Next"),
                            Kind::Primary,
                            ready.then(|| message(Event::MemberQuestion(at + 1))),
                        )
                        .into()
                    } else {
                        action(
                            theme,
                            tr("Finish"),
                            Kind::Primary,
                            ready.then(|| message(Event::FinishOnboarding)),
                        )
                        .into()
                    });
                }
                MembershipStep::Nothing => {
                    body.push(form::note(
                        theme,
                        tr("This server doesn't ask its members anything."),
                    ));
                    row_.push(
                        action(theme, tr("Close"), Kind::Primary, Some(close.clone())).into(),
                    );
                }
            }
        }
        if dialog.sending {
            body.push(form::note(theme, tr("Sending...")));
        }
        if let Some(error) = &dialog.error {
            body.push(form::note_in(
                backcord_iced_kit::i18n::t(error),
                theme.danger,
            ));
            if gate.is_none() {
                row_.push(action(theme, tr("Close"), Kind::Ghost, Some(close.clone())).into());
                row_.push(
                    action(
                        theme,
                        tr("Open in browser"),
                        Kind::Primary,
                        Some(message(Event::MembershipBrowser)),
                    )
                    .into(),
                );
            }
        }
        if !row_.is_empty() {
            body.push(buttons(row_));
        }
        Some(self.cx.look.framed(body, NARROW + 40.0, close))
    }

    /// One field of the form: the rules with their box to tick, a field to
    /// type in, or the choices.
    fn member_field(self, at: usize, field: &'a MemberField) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let dialog = self.cx.state.membership.as_ref();
        let answer = dialog
            .and_then(|dialog| dialog.answers.get(at))
            .unwrap_or(&MemberAnswer::Nothing);
        let label = if field.required {
            format!("{} *", field.label)
        } else {
            field.label.clone()
        };
        let mut parts = column![
            text(label)
                .size(15)
                .font(backcord_iced_kit::fonts::BOLD)
                .color(theme.text)
        ]
        .spacing(8);
        if let Some(description) = &field.description {
            parts = parts.push(form::note(theme, description.as_str()));
        }
        match field.kind {
            MemberFieldKind::Rules => {
                let mut rules = column![].spacing(8);
                for (number, rule) in field.rules.iter().enumerate() {
                    rules = rules.push(
                        row![
                            text(format!("{}.", number + 1))
                                .size(14)
                                .font(backcord_iced_kit::fonts::BOLD)
                                .color(theme.muted),
                            text(rule.as_str())
                                .size(14)
                                .color(theme.text)
                                .width(Length::Fill),
                        ]
                        .spacing(8),
                    );
                }
                parts = parts.push(
                    container(rules)
                        .padding(12)
                        .width(Length::Fill)
                        .style(move |_| rounded(theme.sunken, 8.0)),
                );
                let agreed = *answer == MemberAnswer::Agreed;
                parts = parts.push(self.member_tick(
                    agreed,
                    tr("I have read and agree to the rules"),
                    message(Event::MemberField(
                        at,
                        if agreed {
                            MemberAnswer::Nothing
                        } else {
                            MemberAnswer::Agreed
                        },
                    )),
                ));
            }
            MemberFieldKind::Short | MemberFieldKind::Paragraph => {
                let typed = match answer {
                    MemberAnswer::Text(typed) => typed.as_str(),
                    _ => "",
                };
                let limit = if field.kind == MemberFieldKind::Short {
                    150
                } else {
                    1000
                };
                let id = FIELDS.get(at).copied().unwrap_or(FIELDS[0]);
                parts = parts.push(
                    form::field(theme, id, field.placeholder.as_deref().unwrap_or(""), typed)
                        .padding([8.0, 10.0])
                        .on_input(move |typed| {
                            let typed: String = typed.chars().take(limit).collect();
                            message(Event::MemberField(at, MemberAnswer::Text(typed)))
                        }),
                );
            }
            MemberFieldKind::Choice => {
                for (index, choice) in field.choices.iter().enumerate() {
                    let picked = *answer == MemberAnswer::Choice(index);
                    parts = parts.push(self.member_tick(
                        picked,
                        choice.as_str(),
                        message(Event::MemberField(at, MemberAnswer::Choice(index))),
                    ));
                }
            }
            MemberFieldKind::Unknown => {
                parts = parts.push(form::note(
                    theme,
                    tr("BACKCORD can't show this question yet."),
                ));
            }
        }
        parts.into()
    }

    /// A box to tick with its words, all of it clickable.
    fn member_tick(
        self,
        ticked: bool,
        words: impl text::IntoFragment<'a>,
        on: Message,
    ) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        button(
            row![
                container(self.cx.look.checkbox(ticked, false)).padding(iced::Padding {
                    top: 1.0,
                    ..iced::Padding::ZERO
                }),
                text(words)
                    .size(14)
                    .color(theme.secondary)
                    .width(Length::Fill),
            ]
            .spacing(10),
        )
        .padding(0)
        .width(Length::Fill)
        .on_press(on)
        .style(|_, _| button::Style::default())
        .into()
    }

    /// A question's choice: its box, emoji, title and what it says,
    /// ringed in the accent while picked.
    fn member_choice(
        self,
        question: usize,
        option: &'a MemberOption,
        chosen: bool,
        single: bool,
    ) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let ctx = self.cx;
        let handle = match &option.emoji_picture {
            Some(url) => ctx.picture(url, EMOJI, EMOJI, false),
            None => option
                .emoji
                .as_deref()
                .and_then(emojis::get)
                .and_then(|found| ctx.emoji_picture(found.as_str(), EMOJI)),
        };
        let mut line = row![container(self.cx.look.checkbox(chosen, single))]
            .spacing(12)
            .align_y(iced::Alignment::Center);
        if option.emoji.is_some() || option.emoji_picture.is_some() {
            line = line.push(match handle {
                Some(handle) => Element::from(Pic::square(handle, EMOJI as f32)),
                None => Space::new().width(EMOJI as f32).height(EMOJI as f32).into(),
            });
        }
        let mut words = column![
            text(option.title.as_str())
                .size(15)
                .font(backcord_iced_kit::fonts::BOLD)
                .color(theme.text)
        ]
        .spacing(2)
        .width(Length::Fill);
        if let Some(description) = &option.description {
            words = words.push(text(description.as_str()).size(13).color(theme.muted));
        }
        line = line.push(words);
        button(line)
            .padding([10.0, 12.0])
            .width(Length::Fill)
            .on_press(message(Event::MemberChoice(question, option.id.clone())))
            .style(move |_, status| {
                let ground = match status {
                    button::Status::Hovered | button::Status::Pressed => theme.selection,
                    _ => theme.sunken,
                };
                button::Style {
                    background: Some(ground.into()),
                    border: Border {
                        color: if chosen { theme.accent } else { ground },
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..button::Style::default()
                }
            })
            .into()
    }
}
