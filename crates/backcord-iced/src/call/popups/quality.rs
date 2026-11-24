//! The compact quality picker the camera's menu and the stream's share:
//! the resolutions, 144p to the source, and the frame rates, as rows of
//! small buttons, the chosen one lit.

use backcord_core::voice::Resolution;
use iced::{
    Element, Length,
    widget::{button, column, container, row, text},
};

use crate::app::Message;
use crate::controls;
use backcord_iced_kit::i18n::tr;

/// The frame rates offered.
pub const RATES: [u32; 3] = [15, 30, 60];

pub fn resolution_name(resolution: Resolution) -> &'static str {
    match resolution {
        Resolution::P144 => "144p",
        Resolution::P240 => "240p",
        Resolution::P360 => "360p",
        Resolution::P480 => "480p",
        Resolution::P720 => "720p",
        Resolution::P1080 => "1080p",
        Resolution::Source => tr("Source"),
    }
}

/// The resolutions in two rows, four and three, then the frame rates.
pub(super) fn quality_picker<'a>(
    ctx: crate::ctx::Ctx<'a>,
    resolution: Resolution,
    fps: u32,
    pick_resolution: impl Fn(Resolution) -> Message,
    pick_fps: impl Fn(u32) -> Message,
) -> Element<'a, Message> {
    let theme = ctx.look.theme;
    let chip = |label: String, chosen: bool, press: Message| -> Element<'a, Message> {
        let (accent, on_accent, raised, hover, ink) = (
            theme.accent,
            theme.on_accent,
            theme.raised,
            theme.hover,
            theme.text,
        );
        button(
            container(
                text(label)
                    .size(13)
                    .color(if chosen { on_accent } else { ink }),
            )
            .center_x(Length::Fill),
        )
        .padding([5, 0])
        .width(Length::Fill)
        .on_press(press)
        .style(move |_, status| button::Style {
            background: Some(
                if chosen {
                    accent
                } else if controls::lights(status) {
                    hover
                } else {
                    raised
                }
                .into(),
            ),
            border: crate::style::round(6),
            ..Default::default()
        })
        .into()
    };
    let label = |words: &'static str| text(words).size(12).color(theme.muted);
    let line = |each: &[Resolution]| {
        row(each.iter().map(|&each| {
            chip(
                resolution_name(each).to_owned(),
                each == resolution,
                pick_resolution(each),
            )
        }))
        .spacing(4)
    };
    let rates = row(RATES
        .iter()
        .map(|&rate| chip(format!("{rate} fps"), rate == fps, pick_fps(rate))))
    .spacing(4);
    column![
        label(tr("Resolution")),
        line(&Resolution::ALL[..4]),
        line(&Resolution::ALL[4..]),
        label(tr("Frame rate")),
        rates,
    ]
    .spacing(4)
    .padding([6, 6])
    .into()
}
