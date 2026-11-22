//! About: the build and BACKCORD's GitHub.

use iced::{
    Element,
    widget::{column, row, text},
};

use super::{AccountEvent, account_message};
use crate::settings::{Event, Page, build_tint};
use crate::{
    app::Message,
    controls::{Kind, action},
    fonts::BOLD,
    form,
    pictures::Icon,
};

/// Where BACKCORD's code and releases are.
pub(super) const GITHUB: &str = "https://github.com/Spike848/BACKCORD";

impl<'a> Page<'a> {
    pub(in crate::settings) fn about_page(self) -> Element<'a, Message> {
        let settings = self.s;
        let theme = self.cx.look.theme;
        let build = self.cx.build;
        let mut channel = row![
            text(build.channel.as_str())
                .size(13)
                .font(BOLD)
                .color(build_tint(theme, build.tone))
        ];
        if !build.details.is_empty() {
            channel = channel.push(
                text(format!(" · {}", build.details))
                    .size(13)
                    .color(theme.muted)
                    .wrapping(text::Wrapping::None),
            );
        }
        let card = form::card(
            theme,
            form::beside(
                column![
                    text(build.headline.as_str())
                        .size(20)
                        .font(BOLD)
                        .color(theme.text)
                        .wrapping(text::Wrapping::None),
                    channel
                ]
                .spacing(4),
                action(
                    theme,
                    if settings.copied {
                        backcord_iced_kit::i18n::tr("Copied")
                    } else {
                        backcord_iced_kit::i18n::tr("Copy version")
                    },
                    Kind::Plain,
                    Some(Message::Settings(Event::CopyVersion)),
                ),
            ),
        );
        let ink = crate::theme::rgb(theme.text);
        let github: Element<'a, Message> = action(
            theme,
            backcord_iced_kit::i18n::tr("GitHub"),
            Kind::Plain,
            Some(account_message(AccountEvent::Open(GITHUB))),
        )
        .icon(self.cx.look.icon(Icon::GitHub, 18, ink))
        .into();
        form::page(vec![
            form::title(theme, backcord_iced_kit::i18n::tr("About")),
            card,
            form::buttons([github]),
        ])
    }
}
