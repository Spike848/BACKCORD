//! Messages: how messages show, the clock and the date.

use iced::{
    Alignment, Element, Length,
    widget::{column, container, row, text},
};

use super::{ANIMATES, Pref, message_size, pref};
use crate::settings::Page;
use crate::{
    app::Message,
    controls::choices,
    fonts::BOLD,
    form::{self, switch},
};

pub(super) const DATE_FORMATS: [&str; 3] = ["Day/Month/Year", "Month/Day/Year", "Year-Month-Day"];

impl<'a> Page<'a> {
    pub(in crate::settings) fn messages_page(self) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let prefs = &self.cx.layout.prefs;
        let size = message_size(prefs.text_size);
        let twelve = prefs.twelve_hour_clock;
        let clock = format!(
            "{}{}{}",
            if twelve { "9:41" } else { "21:41" },
            if prefs.clock_seconds { ":05" } else { "" },
            if twelve { " PM" } else { "" }
        );
        let edge = theme.line;
        // BACKCORD's logo as the sample's picture.
        let avatar: Element<'a, Message> = match crate::rail::home_logo(40) {
            Some(logo) => crate::widgets::Pic::square(logo, 40.0).radius(20.0).into(),
            None => self.cx.avatar(None, "Spike84", 40.0, 20.0, 1.0),
        };
        let preview = container(
            row![
                container(avatar).align_top(Length::Shrink),
                column![
                    row![
                        text("Spike84").size(size).font(BOLD).color(theme.text),
                        text(clock).size(size - 3.0).color(theme.muted)
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                    text("Pensavo di entrare su backdoor, discord.gg/backdoor666")
                        .size(size)
                        .color(theme.text)
                ]
                .spacing(3)
                .width(Length::Fill)
            ]
            .spacing(16),
        )
        .padding(16)
        .width(Length::Fill)
        .style(move |_| iced::widget::container::Style {
            border: crate::style::edge(edge, 8.0),
            ..Default::default()
        });
        let text_size = prefs.text_size.unwrap_or(1).clamp(0, 2) as usize;
        let date = prefs
            .date_format
            .and_then(|index| usize::try_from(index).ok())
            .unwrap_or(usize::from(twelve));
        let animate = ANIMATES
            .iter()
            .position(|kind| *kind == prefs.animate)
            .unwrap_or(0);
        form::page([
            form::title(theme, backcord_iced_kit::i18n::tr("Messages")),
            preview.into(),
            form::group([
                form::heading(theme, backcord_iced_kit::i18n::tr("TEXT SIZE")),
                choices(theme, &[backcord_iced_kit::i18n::tr("Small"), backcord_iced_kit::i18n::tr("Default"), backcord_iced_kit::i18n::tr("Large")], text_size, |index| {
                    pref(Pref::TextSize(index))
                }),
            ]),
            form::group([
                form::heading(theme, backcord_iced_kit::i18n::tr("CLOCK")),
                choices(theme, &[backcord_iced_kit::i18n::tr("24-hour"), backcord_iced_kit::i18n::tr("12-hour")], usize::from(twelve), |index| {
                    pref(Pref::TwelveHour(index == 1))
                }),
                switch(
                    backcord_iced_kit::i18n::tr("Show seconds"),
                    prefs.clock_seconds,
                    Some(pref(Pref::Seconds(!prefs.clock_seconds))),
                )
                .description(backcord_iced_kit::i18n::tr("Message times show them, as 21:41:05."))
                .view(self.cx.look),
                form::heading(theme, backcord_iced_kit::i18n::tr("DATE FORMAT")),
                choices(theme, &DATE_FORMATS.map(backcord_iced_kit::i18n::tr), date, |index| {
                    pref(Pref::DateFormat(index))
                }),
            ]),
            form::group([
                form::heading(theme, backcord_iced_kit::i18n::tr("PLAY GIFS, EMOJI AND STICKERS")),
                choices(
                    theme,
                    &[backcord_iced_kit::i18n::tr("While BACKCORD is focused"), backcord_iced_kit::i18n::tr("Always"), backcord_iced_kit::i18n::tr("Never")],
                    animate,
                    |index| pref(Pref::Animate(index)),
                ),
            ]),
            form::rule(theme),
            self.flip(
                backcord_iced_kit::i18n::tr("Show link previews"),
                backcord_iced_kit::i18n::tr("Cards for links, videos and bots under messages. Linked pictures still show."),
                prefs.link_previews.unwrap_or(true),
                Pref::LinkPreviews,
            ),
            self.flip(
                backcord_iced_kit::i18n::tr("Collapse long messages"),
                backcord_iced_kit::i18n::tr("Long messages and code show their first lines, with Show more to see the rest."),
                self.cx.layout.collapse_long,
                Pref::CollapseLong,
            ),
            self.flip(
                backcord_iced_kit::i18n::tr("Show boost gems"),
                backcord_iced_kit::i18n::tr("A gem by the name of each member boosting their Discord server, in the member list."),
                prefs.boost_gems.unwrap_or(true),
                Pref::BoostGems,
            ),
            self.flip(
                backcord_iced_kit::i18n::tr("Mention people you reply to"),
                backcord_iced_kit::i18n::tr("They get notified of your reply. Switch it for one reply with @ On on the reply bar."),
                prefs.reply_ping.unwrap_or(true),
                Pref::ReplyPing,
            ),
            self.flip(
                backcord_iced_kit::i18n::tr("Remove tracking from links you send"),
                "Takes out tags like utm_source and fbclid, and share ids from YouTube, Spotify and X. The links still go to the same page.",
                prefs.strip_tracking.unwrap_or(true),
                Pref::StripTracking,
            ),
        ])
    }
}
