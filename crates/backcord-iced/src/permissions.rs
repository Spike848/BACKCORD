//! Permissions Viewer's window: whose permissions and where at its top,
//! what gives them all when something does (owning the server,
//! Administrator), then Discord's groups, each permission with a green
//! tick or a red cross and what gives or takes it: a role, a channel's
//! override, a timeout. What it shows is the core's
//! (`AppState::permissions`); it closes on Escape, its X or a click
//! outside.

use crate::widgets::scroll_view;
use backcord_core::application::{Action, PermissionSource, PermissionsView};
use backcord_iced_kit::i18n::{t, tr};
use iced::{
    Alignment, Element, Length, Padding, Task,
    widget::{button, column, container, row, text},
};

use crate::{
    app::Message,
    controls::{bar, lit_style},
    ctx::{Ctx, Cx, Focus, Request},
    dialog::dialog,
    fonts::{BOLD, MEDIUM},
    form,
    pictures::Icon,
    style::scrollbar,
};

/// The window's width and its most height, less the room around it.
const WIDTH: f32 = 520.0;
const HEIGHT: f32 = 720.0;
/// A permission's tick or cross.
const MARK: u32 = 18;

#[derive(Clone, Debug)]
pub enum Event {
    Close,
}

pub fn update(event: Event, cx: &mut Cx<'_>) -> Task<Message> {
    match event {
        Event::Close => {
            cx.dispatch(Action::ClosePermissions);
            cx.request(Request::Focus(Focus::Default));
        }
    }
    Task::none()
}

/// What gives a permission or takes it, in words.
pub fn source_words(source: &PermissionSource, held: bool) -> String {
    match (source, held) {
        (PermissionSource::Owner, _) => tr("Server owner").to_owned(),
        (PermissionSource::Administrator(role), _) => t(format!("Administrator, from {role}")),
        (PermissionSource::Role(role), _) => t(format!("Given by {role}")),
        (PermissionSource::RoleOverwrite(role), true) => {
            t(format!("Allowed by the channel override for {role}"))
        }
        (PermissionSource::RoleOverwrite(role), false) => {
            t(format!("Denied by the channel override for {role}"))
        }
        (PermissionSource::MemberOverwrite, true) => {
            tr("Allowed by the channel override for this member").to_owned()
        }
        (PermissionSource::MemberOverwrite, false) => {
            tr("Denied by the channel override for this member").to_owned()
        }
        (PermissionSource::Timeout, _) => tr("Taken away by their timeout").to_owned(),
        (PermissionSource::Nothing, _) => tr("No role gives it").to_owned(),
    }
}

/// What gives them every permission, when something does, in words.
fn everything_words(source: &PermissionSource) -> Option<String> {
    match source {
        PermissionSource::Owner => {
            Some(tr("Owns the server: every permission, in every channel.").to_owned())
        }
        PermissionSource::Administrator(role) => Some(t(format!(
            "Administrator from {role}: every permission, in every channel."
        ))),
        _ => None,
    }
}

/// The window over the dimmed window, while it's open.
pub fn view(cx: Ctx<'_>) -> Option<Element<'_, Message>> {
    let shown = cx.state.permissions.as_ref()?;
    let theme = cx.look.theme;
    let close = button(cx.look.glyph(Icon::X, 18, theme.secondary).boxed(28))
        .padding(0)
        .on_press(Message::Permissions(Event::Close))
        .style(lit_style(None, theme.hover, 6.0));
    let whose = row![
        text(shown.name.as_str())
            .size(14)
            .font(MEDIUM)
            .color(shown.color.map_or(theme.text, crate::style::hex))
            .wrapping(text::Wrapping::None),
        text("·").size(14).color(theme.muted),
        text(shown.place.as_str())
            .size(14)
            .color(theme.secondary)
            .wrapping(text::Wrapping::None),
    ]
    .spacing(6)
    .align_y(Alignment::Center);
    let head = row![
        column![
            text(tr("Permissions"))
                .size(20)
                .font(BOLD)
                .color(theme.text),
            whose,
        ]
        .spacing(4)
        .width(Length::Fill),
        close
    ]
    .align_y(Alignment::Center)
    .padding(Padding {
        top: 0.0,
        left: 24.0,
        right: 16.0,
        bottom: 12.0,
    });
    let body = column![head, bar(theme.line, Length::Fill, 1), list(shown, cx)].padding(Padding {
        top: 16.0,
        ..Padding::ZERO
    });
    let width = WIDTH.min(cx.look.window.width - 48.0);
    let height = HEIGHT.min(cx.look.window.height - 96.0);
    Some(dialog(
        theme,
        container(body).height(height).into(),
        width,
        Message::Permissions(Event::Close),
    ))
}

/// What gives them all, then each group of permissions under its heading.
fn list<'a>(shown: &'a PermissionsView, cx: Ctx<'a>) -> Element<'a, Message> {
    let theme = cx.look.theme;
    let mut list = column![].spacing(2).padding(Padding {
        top: 8.0,
        right: 16.0,
        bottom: 16.0,
        left: 24.0,
    });
    if let Some(words) = shown.everything.as_ref().and_then(everything_words) {
        list = list.push(
            container(text(words).size(14).font(MEDIUM).color(theme.text)).padding(Padding {
                top: 4.0,
                bottom: 4.0,
                ..Padding::ZERO
            }),
        );
    }
    for group in &shown.groups {
        if group.rows.is_empty() {
            continue;
        }
        list = list.push(
            container(form::heading(theme, tr(group.heading).to_uppercase())).padding(Padding {
                top: 12.0,
                bottom: 4.0,
                ..Padding::ZERO
            }),
        );
        for permission in &group.rows {
            let mark = if permission.held {
                cx.look.glyph(Icon::Check, MARK, theme.live)
            } else {
                cx.look.glyph(Icon::X, MARK, theme.danger)
            };
            list = list.push(
                row![
                    mark,
                    column![
                        text(tr(permission.title)).size(15).color(theme.text),
                        text(source_words(&permission.source, permission.held))
                            .size(12)
                            .color(theme.muted),
                    ]
                    .spacing(2)
                    .width(Length::Fill),
                ]
                .spacing(10)
                .padding([4, 0])
                .align_y(Alignment::Center),
            );
        }
    }
    scroll_view(list)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(scrollbar(theme))
        .into()
}
