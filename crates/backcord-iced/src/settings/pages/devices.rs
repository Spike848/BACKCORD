//! Devices, as Discord's page has them: every device signed in to this
//! account, this one apart, each with its system, client, rough place and
//! when it was last used; each other one signed out after asking, or all
//! of them at once. Discord may confirm it with two-factor authentication
//! first. The core lists them (`AppState::security`) each time the page
//! shows.

use backcord_core::{application::LogOut, chat::Device};
use iced::{
    Alignment, Element, Length,
    widget::{column, row, text},
};

use super::account_security::{SecurityEvent, security};
use crate::settings::Page;
use crate::{
    app::Message,
    controls::{Kind, action},
    fonts::BOLD,
    form,
    pictures::Icon,
};

/// What a device is, as its row's first line says it: its system and its
/// client, or "Unknown device".
fn device_name(device: &Device) -> String {
    match (device.os.as_deref(), device.platform.as_deref()) {
        (Some(os), Some(platform)) => format!("{os} · {platform}"),
        (Some(one), None) | (None, Some(one)) => one.to_owned(),
        (None, None) => backcord_iced_kit::i18n::tr("Unknown device").to_owned(),
    }
}

/// A phone or tablet's systems, which show a phone.
fn mobile(device: &Device) -> bool {
    device.os.as_deref().is_some_and(|os| {
        let os = os.to_ascii_lowercase();
        os.contains("android") || os.contains("ios") || os.contains("ipad")
    })
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

impl<'a> Page<'a> {
    pub(in crate::settings) fn devices_page(self) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let devices = &self.cx.state.security.devices;
        let tr = backcord_iced_kit::i18n::tr;
        let mut parts = vec![
            form::title(theme, tr("Devices")),
            form::note(
                theme,
                tr(
                    "Here are all the devices that are currently logged in with your Discord account. You can log out of each one individually or all other known devices.",
                ),
            ),
        ];
        if let Some(error) = &devices.error {
            parts.push(form::beside(
                form::error(theme, backcord_iced_kit::i18n::t(error.clone())),
                action(
                    theme,
                    tr("Try again"),
                    Kind::Plain,
                    Some(security(SecurityEvent::ListDevices)),
                ),
            ));
        }
        let Some(list) = devices.list.as_deref() else {
            parts.push(form::note(theme, tr("Loading...")));
            return form::page(parts);
        };
        let now = now_ms();
        let card = |device: &'a Device| -> Element<'a, Message> {
            let icon = if mobile(device) {
                Icon::Smartphone
            } else {
                Icon::Monitor
            };
            let mut detail: Vec<String> = device.location.iter().cloned().collect();
            if !device.current
                && let Some(at) = device.last_used
            {
                detail.push(backcord_iced_kit::i18n::t(crate::header_popovers::ago(
                    at, now,
                )));
            }
            let words = column![
                text(device_name(device).to_uppercase())
                    .size(12)
                    .font(BOLD)
                    .color(theme.text),
                text(detail.join(" · ")).size(13).color(theme.muted),
            ]
            .spacing(2)
            .width(Length::Fill);
            let mut line = row![self.cx.look.glyph(icon, 24, theme.text), words]
                .spacing(12)
                .align_y(Alignment::Center);
            if !device.current {
                line = line.push(self.cx.look.icon_button(
                    Icon::X,
                    tr("Log out"),
                    security(SecurityEvent::AskLogOut(Some(LogOut::One(
                        device.id.clone(),
                    )))),
                ));
            }
            form::card(theme, line)
        };
        if let Some(current) = list.iter().find(|device| device.current) {
            parts.push(form::group([
                form::heading(theme, tr("CURRENT DEVICE")),
                card(current),
            ]));
        }
        let others: Vec<&Device> = list.iter().filter(|device| !device.current).collect();
        if !others.is_empty() {
            let mut group = vec![form::heading(theme, tr("OTHER DEVICES"))];
            group.extend(others.iter().map(|device| card(device)));
            parts.push(form::group(group));
            parts.push(form::group([
                form::heading(theme, tr("LOG OUT OF ALL KNOWN DEVICES")),
                form::note(
                    theme,
                    tr("You'll have to log back in on all logged out devices."),
                ),
                form::buttons(vec![
                    action(
                        theme,
                        tr("Log out all known devices"),
                        Kind::Danger,
                        Some(security(SecurityEvent::AskLogOut(Some(LogOut::Others)))),
                    )
                    .into(),
                ]),
            ]));
        }
        form::page(parts)
    }

    /// Asks before signing out, while it's asked.
    pub(in crate::settings) fn log_out_dialog(self) -> Option<Element<'a, Message>> {
        let devices = &self.cx.state.security.devices;
        let asking = devices.asking.as_ref()?;
        let look = self.cx.look;
        let theme = look.theme;
        let tr = backcord_iced_kit::i18n::tr;
        let (title, words) = match asking {
            LogOut::One(_) => (
                tr("Log out of device?"),
                tr("You'll have to log back in on this device."),
            ),
            LogOut::Others => (
                tr("Log out of all known devices?"),
                tr("You'll have to log back in on all logged out devices."),
            ),
        };
        let cancel = security(SecurityEvent::AskLogOut(None));
        let open = !devices.busy;
        let mut parts = vec![
            look.dialog_head(title, cancel.clone()),
            form::note(theme, words),
        ];
        if let Some(error) = &devices.error {
            parts.push(
                form::error(theme, backcord_iced_kit::i18n::t(error.clone()))
                    .width(Length::Fill)
                    .into(),
            );
        }
        parts.push(backcord_iced_kit::dialog::buttons(vec![
            action(
                theme,
                tr("Cancel"),
                Kind::Ghost,
                open.then(|| cancel.clone()),
            )
            .into(),
            action(
                theme,
                if open {
                    tr("Log Out")
                } else {
                    tr("Logging out...")
                },
                Kind::Danger,
                open.then(|| security(SecurityEvent::ConfirmLogOut)),
            )
            .into(),
        ]));
        let width = (look.window.width - 48.0).min(backcord_iced_kit::dialog::CONFIRM_WIDTH);
        Some(look.framed(parts, width, if open { cancel } else { Message::Nothing }))
    }
}
