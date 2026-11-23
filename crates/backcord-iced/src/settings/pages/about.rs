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
        let mut parts = vec![
            form::title(theme, backcord_iced_kit::i18n::tr("Updates")),
            card,
        ];
        parts.extend(self.updates_card());
        parts.push(form::buttons([github]));
        form::page(parts)
    }

    /// Beside the version: whether a newer one is out, Update and restart or
    /// Check for updates, how far an update has got, and the automatic look.
    fn updates_card(self) -> Option<Element<'a, Message>> {
        use crate::{desktop::UpdateStatus, updates::Event as Updates};
        let shown = crate::updates::shown(self.cx.layout)?;
        let theme = self.cx.look.theme;
        let tr = backcord_iced_kit::i18n::tr;
        let t = backcord_iced_kit::i18n::t;
        let headline: String = match shown.status {
            UpdateStatus::Idle => tr("Look for a newer BACKCORD.").into(),
            UpdateStatus::Checking => tr("Checking for updates…").into(),
            UpdateStatus::Current => tr("BACKCORD is up to date.").into(),
            UpdateStatus::Available => t(format!("BACKCORD {} is available", shown.latest)),
            UpdateStatus::Skipped => t(format!("BACKCORD {} was skipped", shown.latest)),
            UpdateStatus::Failed => tr("Couldn't check for updates.").into(),
            UpdateStatus::Source => tr("A build from source doesn't update itself.").into(),
        };
        let mut words = column![text(headline).size(16).font(BOLD).color(theme.text)].spacing(4);
        let note = if shown.installing {
            shown.install_text.clone()
        } else if !shown.install_error.is_empty() {
            shown.install_error.clone()
        } else if shown.status == UpdateStatus::Failed {
            shown.error.clone()
        } else if shown.status == UpdateStatus::Available {
            shown.install_note.clone()
        } else if !shown.manager.is_empty() {
            t(format!("{} updates this copy.", shown.manager))
        } else {
            String::new()
        };
        if !note.is_empty() {
            let failed = !shown.installing && !shown.install_error.is_empty();
            words = words.push(text(note).size(13).color(if failed {
                theme.danger
            } else {
                theme.muted
            }));
        }
        let button: Element<'a, Message> = if shown.installing {
            action(
                theme,
                tr("Cancel"),
                Kind::Plain,
                shown
                    .can_cancel
                    .then_some(Message::Updates(Updates::Cancel)),
            )
            .into()
        } else if shown.status == UpdateStatus::Available && shown.can_install {
            action(
                theme,
                tr("Update and restart"),
                Kind::Primary,
                Some(Message::Updates(Updates::Install)),
            )
            .into()
        } else if shown.status == UpdateStatus::Available {
            action(
                theme,
                tr("Download"),
                Kind::Primary,
                Some(Message::Updates(Updates::View)),
            )
            .into()
        } else {
            action(
                theme,
                tr("Check for updates"),
                Kind::Plain,
                (shown.status != UpdateStatus::Checking)
                    .then_some(Message::Updates(Updates::CheckNow)),
            )
            .into()
        };
        let mut body = column![form::beside(words, button)].spacing(12);
        if shown.status == UpdateStatus::Available && !shown.notes.is_empty() {
            body = body.push(text(shown.notes).size(13).color(theme.secondary));
        }
        let on = self.cx.layout.prefs.checks_updates();
        body = body.push(
            form::switch(
                tr("Check for updates automatically"),
                on,
                Some(Message::Settings(Event::Pref(super::Pref::CheckUpdates(
                    !on,
                )))),
            )
            .description(tr("A notification tells you when a new version is out."))
            .view(self.cx.look),
        );
        Some(form::card(theme, body))
    }
}
