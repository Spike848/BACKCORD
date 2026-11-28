//! A Discord stage the call is on, as Discord's client shows it: over the
//! tiles, its topic (or that nobody started it) with this account's place
//! there: Request to Speak or Lower Hand in the audience, Move to Audience
//! as a speaker, Accept and Decline when invited; and for its moderators
//! Start Stage, Edit Topic and End Stage, the topic written in place. The
//! tiles are its speakers alone; under them its audience, the raised hands
//! first, each with Invite to Speak for a moderator, and the speakers'
//! Move to Audience in the same row. The core keeps the stage
//! (`Application::call_stage`, `AppState::stage`); this is its view.

use backcord_core::application::{Action, CallStage, STAGE_TOPIC_LIMIT, StageInput, StageRole};
use backcord_core::voice::Participant;
use iced::{
    Alignment, Element, Length, Padding, Task,
    widget::{Space, column, container, operation, row, scrollable, text},
};

use crate::{
    app::{App, Message},
    controls::{Kind, action, field},
    fonts::BOLD,
    pictures::Icon,
};

/// The bar over the tiles, and the audience's row under them.
pub const BAR: f32 = 56.0;
pub const AUDIENCE: f32 = 124.0;
/// The topic's field, for focus and the tests.
pub const TOPIC_FIELD: &str = "stage-topic";

/// What's done on the stage.
#[derive(Clone, Debug)]
pub enum StageEvent {
    EditTopic(bool),
    WriteTopic(String),
    SaveTopic,
    End,
    Hand(bool),
    Speak,
    ToAudience,
    Invite(String),
    MoveToAudience(String),
    DismissError,
}

fn stage(event: StageEvent) -> Message {
    Message::Call(crate::call::Event::Stage(event))
}

pub fn update(app: &mut App, event: StageEvent) -> Task<Message> {
    let focus = matches!(event, StageEvent::EditTopic(true));
    let input = match event {
        StageEvent::EditTopic(open) => StageInput::EditTopic(open),
        StageEvent::WriteTopic(topic) => StageInput::WriteTopic(topic),
        StageEvent::SaveTopic => StageInput::SaveTopic,
        StageEvent::End => StageInput::End,
        StageEvent::Hand(raise) => StageInput::Hand(raise),
        StageEvent::Speak => StageInput::Speak,
        StageEvent::ToAudience => StageInput::ToAudience,
        StageEvent::Invite(user) => StageInput::Invite(user),
        StageEvent::MoveToAudience(user) => StageInput::MoveToAudience(user),
        StageEvent::DismissError => StageInput::DismissError,
    };
    app.client.dispatch(Action::StageChannel(input));
    if focus {
        operation::focus(TOPIC_FIELD)
    } else {
        Task::none()
    }
}

/// The stage the call shown is on, while this account is in it.
fn shown(app: &App) -> Option<CallStage> {
    crate::call::stage::voice_here(app)
        .then(|| app.client.app.call_stage())
        .flatten()
}

/// The audience, raised hands first, as Discord lists them.
fn audience(app: &App) -> Vec<&Participant> {
    let mut audience: Vec<&Participant> = app
        .client
        .state()
        .voice
        .participants
        .iter()
        .filter(|person| person.audience)
        .collect();
    audience.sort_by_key(|person| person.requested_to_speak.is_none());
    audience
}

/// How much of the call's height the stage's bar and audience take.
pub fn height(app: &App) -> f32 {
    if shown(app).is_none() {
        return 0.0;
    }
    BAR + if audience(app).is_empty() {
        0.0
    } else {
        AUDIENCE
    }
}

/// The bar over the tiles: the topic and what this account may do.
pub fn bar(app: &App) -> Option<Element<'_, Message>> {
    let shown = shown(app)?;
    let theme = &app.theme;
    let tr = backcord_iced_kit::i18n::tr;
    let state = &app.client.state().stage;
    let ink = theme.text;
    let mut line = row![app.ctx().look.glyph(Icon::Stage, 20, theme.live)]
        .spacing(12)
        .align_y(Alignment::Center);
    if let Some(editor) = &state.editor {
        let mut input = field(
            theme,
            TOPIC_FIELD,
            tr("What's your stage about?"),
            &editor.topic,
            false,
            !editor.busy,
        )
        .size(14)
        .padding([6, 10])
        .width(Length::Fill);
        if !editor.busy {
            input = input.on_input(|topic| stage(StageEvent::WriteTopic(topic)));
            if editor.ready() {
                input = input.on_submit(stage(StageEvent::SaveTopic));
            }
        }
        line = line.push(input);
        line = line.push(
            text(format!(
                "{}/{STAGE_TOPIC_LIMIT}",
                editor.topic.chars().count()
            ))
            .size(12)
            .color(theme.muted),
        );
        line = line.push(action(
            theme,
            tr("Cancel"),
            Kind::Ghost,
            (!editor.busy).then(|| stage(StageEvent::EditTopic(false))),
        ));
        line = line.push(action(
            theme,
            if editor.starting {
                tr("Start Stage")
            } else {
                tr("Save")
            },
            Kind::Primary,
            editor.ready().then(|| stage(StageEvent::SaveTopic)),
        ));
    } else {
        let words: Element<'_, Message> = match &shown.topic {
            Some(topic) => column![
                text(topic.clone())
                    .size(15)
                    .font(BOLD)
                    .color(ink)
                    .wrapping(text::Wrapping::None),
                text(tr("Live stage")).size(12).color(theme.live),
            ]
            .spacing(2)
            .into(),
            None => text(if shown.moderator {
                tr("Nobody started this stage. Start it with a topic for its audience.")
            } else {
                tr("Nobody started this stage yet.")
            })
            .size(14)
            .color(theme.muted)
            .into(),
        };
        line = line.push(container(words).width(Length::Fill).clip(true));
        if shown.moderator {
            match shown.topic {
                Some(_) => {
                    line = line.push(action(
                        theme,
                        tr("Edit Topic"),
                        Kind::Plain,
                        Some(stage(StageEvent::EditTopic(true))),
                    ));
                    line = line.push(action(
                        theme,
                        tr("End Stage"),
                        Kind::Danger,
                        Some(stage(StageEvent::End)),
                    ));
                }
                None => {
                    line = line.push(action(
                        theme,
                        tr("Start Stage"),
                        Kind::Primary,
                        Some(stage(StageEvent::EditTopic(true))),
                    ));
                }
            }
        }
        match shown.role {
            StageRole::Invited => {
                line = line.push(
                    text(tr("You're invited to speak."))
                        .size(14)
                        .font(BOLD)
                        .color(ink),
                );
                line = line.push(action(
                    theme,
                    tr("Decline"),
                    Kind::Plain,
                    Some(stage(StageEvent::ToAudience)),
                ));
                line = line.push(action(
                    theme,
                    tr("Accept"),
                    Kind::Success,
                    Some(stage(StageEvent::Speak)),
                ));
            }
            StageRole::Audience { .. } if shown.moderator => {
                line = line.push(action(
                    theme,
                    tr("Become a Speaker"),
                    Kind::Primary,
                    Some(stage(StageEvent::Speak)),
                ));
            }
            StageRole::Audience { hand: true } => {
                line = line.push(action(
                    theme,
                    tr("Lower Hand"),
                    Kind::Plain,
                    Some(stage(StageEvent::Hand(false))),
                ));
            }
            StageRole::Audience { hand: false } => {
                line = line.push(action(
                    theme,
                    tr("Request to Speak"),
                    Kind::Primary,
                    shown.may_request.then(|| stage(StageEvent::Hand(true))),
                ));
            }
            StageRole::Speaker => {
                line = line.push(action(
                    theme,
                    tr("Move to Audience"),
                    Kind::Plain,
                    Some(stage(StageEvent::ToAudience)),
                ));
            }
        }
    }
    let error = state
        .editor
        .as_ref()
        .and_then(|editor| editor.error.as_ref())
        .or(state.error.as_ref());
    let mut bar = column![line].spacing(4);
    if let Some(error) = error {
        let mut said = row![
            backcord_iced_kit::form::error(theme, backcord_iced_kit::i18n::t(error.clone()))
                .width(Length::Fill)
        ];
        if state.editor.is_none() {
            said = said.push(app.ctx().look.icon_button(
                Icon::X,
                tr("Dismiss"),
                stage(StageEvent::DismissError),
            ));
        }
        bar = bar.push(said);
    }
    Some(
        container(bar)
            .padding(Padding {
                top: 8.0,
                right: 16.0,
                bottom: 8.0,
                left: 16.0,
            })
            .height(BAR)
            .width(Length::Fill)
            .clip(true)
            .into(),
    )
}

/// The audience under the tiles, raised hands first, and for a moderator
/// Invite to Speak on each, then the speakers' Move to Audience.
pub fn audience_row(app: &App) -> Option<Element<'_, Message>> {
    let shown = shown(app)?;
    let people = audience(app);
    if people.is_empty() {
        return None;
    }
    let theme = &app.theme;
    let tr = backcord_iced_kit::i18n::tr;
    let ctx = app.ctx();
    let mut list = row![].spacing(12).align_y(Alignment::Center);
    for person in people {
        let mut face = column![ctx.avatar(person.avatar.as_deref(), &person.user, 32.0, 16.0, 1.0)]
            .spacing(4)
            .align_x(Alignment::Center);
        let mut name = row![text(person.user.as_str()).size(12).color(theme.text)]
            .spacing(4)
            .align_y(Alignment::Center);
        if person.requested_to_speak.is_some() {
            name = name.push(ctx.look.glyph(Icon::Hand, 14, theme.live));
        }
        face = face.push(name);
        if shown.moderator && !person.local {
            face = face.push(action(
                theme,
                tr("Invite to Speak"),
                Kind::Plain,
                Some(stage(StageEvent::Invite(person.id.clone()))),
            ));
        }
        list = list.push(face);
    }
    let speakers: Vec<&Participant> = app
        .client
        .state()
        .voice
        .participants
        .iter()
        .filter(|person| !person.audience && !person.local && shown.moderator)
        .collect();
    if !speakers.is_empty() {
        list = list.push(Space::new().width(24));
        for person in speakers {
            list = list.push(
                column![
                    text(person.user.as_str()).size(12).color(theme.muted),
                    action(
                        theme,
                        tr("Move to Audience"),
                        Kind::Ghost,
                        Some(stage(StageEvent::MoveToAudience(person.id.clone()))),
                    ),
                ]
                .spacing(4)
                .align_x(Alignment::Center),
            );
        }
    }
    let count = app
        .client
        .state()
        .voice
        .participants
        .iter()
        .filter(|person| person.audience)
        .count();
    Some(
        container(
            column![
                text(backcord_iced_kit::i18n::t(format!("AUDIENCE — {count}")))
                    .size(12)
                    .font(BOLD)
                    .color(theme.muted),
                scrollable(list).direction(scrollable::Direction::Horizontal(
                    scrollable::Scrollbar::new().width(4).scroller_width(4),
                )),
            ]
            .spacing(6),
        )
        .padding([8, 16])
        .height(AUDIENCE)
        .width(Length::Fill)
        .clip(true)
        .into(),
    )
}
