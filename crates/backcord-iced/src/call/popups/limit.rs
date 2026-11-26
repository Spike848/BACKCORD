//! What the dock's limit button opens: the largest picture and the most
//! frames a second this account receives from the call's cameras and the
//! streams it watches, which change at once and are kept. Cameras are
//! asked for smaller, so less comes; streams come as their streamer sends
//! them and are shown within the limit.

use iced::{Element, widget::column};

use crate::call::Event;
use crate::{
    app::Message,
    controls::{Check, heading},
};
use backcord_iced_kit::i18n::tr;

/// The limit menu's width.
pub(super) const LIMIT_WIDTH: f32 = 250.0;

/// The heights offered, 0 for none: every resolution the quality pickers
/// send at, the source's 4K too, largest first.
pub fn heights() -> impl Iterator<Item = u32> {
    std::iter::once(0).chain(
        backcord_core::voice::Resolution::ALL
            .iter()
            .rev()
            .map(|resolution| resolution.lines()),
    )
}

/// The frame rates offered, 0 for none: every one the quality pickers
/// send at, most first.
pub fn rates() -> impl Iterator<Item = u32> {
    std::iter::once(0).chain(super::quality::RATES.iter().rev().copied())
}

pub(super) fn limit_menu<'a>(ctx: crate::ctx::Ctx<'a>) -> Element<'a, Message> {
    let theme = ctx.look.theme;
    let limit = ctx.layout.voice.receive_limit();
    let current = |on: bool| if on { Check::Current } else { Check::None };
    let mut rows = column![heading(theme, tr("RECEIVED QUALITY"), 4.0)].spacing(2);
    for height in heights() {
        let label = match height {
            0 => tr("No limit").to_owned(),
            2160 => "2160p (4K)".to_owned(),
            _ => format!("{height}p"),
        };
        rows = rows.push(crate::call::popups::popup_row(
            ctx,
            label,
            None,
            None,
            current(limit.height == height),
            Some(Message::Call(Event::ReceiveLimit(
                backcord_core::voice::ReceiveLimit { height, ..limit },
            ))),
        ));
    }
    rows = rows.push(heading(theme, tr("RECEIVED FRAME RATE"), 4.0));
    for fps in rates() {
        let label = if fps == 0 {
            tr("No limit").to_owned()
        } else {
            format!("{fps} FPS")
        };
        rows = rows.push(crate::call::popups::popup_row(
            ctx,
            label,
            None,
            None,
            current(limit.fps == fps),
            Some(Message::Call(Event::ReceiveLimit(
                backcord_core::voice::ReceiveLimit { fps, ..limit },
            ))),
        ));
    }
    rows.padding(6).into()
}
