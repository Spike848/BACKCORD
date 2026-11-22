//! What Settings shows of the updates the root looks for
//! (`crate::updates`): the notice under Settings' sections.

use iced::{
    Element, Length,
    widget::{container, text},
};

use super::{Event as SettingsEvent, Page, Section};
use crate::{app::Message, desktop::UpdateStatus};

impl<'a> Page<'a> {
    /// Under Settings' sections while a newer release is out: a click goes
    /// to About.
    pub(super) fn update_notice(self) -> Option<Element<'a, Message>> {
        let shown = crate::updates::shown(self.cx.layout)?;
        if shown.status != UpdateStatus::Available {
            return None;
        }
        let theme = self.cx.look.theme;
        let accent = theme.accent;
        let (rest, lit) = (
            iced::Color { a: 0.14, ..accent },
            iced::Color { a: 0.22, ..accent },
        );
        let face = iced::widget::button(
            text(backcord_iced_kit::i18n::t(format!(
                "Update available · {}",
                shown.latest
            )))
            .size(13)
            .font(crate::fonts::BOLD)
            .color(accent)
            .wrapping(text::Wrapping::None),
        )
        .padding([0, 12])
        .width(Length::Fill)
        .height(28)
        .on_press(Message::Settings(SettingsEvent::Open(Section::About)))
        .style(crate::controls::lit_style(Some(rest), lit, 4.0));
        Some(
            container(iced::widget::center_y(face))
                .padding([4, 16])
                .height(36)
                .into(),
        )
    }
}
