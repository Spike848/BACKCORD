//! What the camera button opens, as the share button's menu does for a
//! stream: the camera to film with, the resolution and frame rate it films
//! at (which change at once while it's on), and turning it on or off.

use backcord_core::voice::{self, CameraQuality};
use iced::{
    Element, Padding,
    widget::{column, container},
};

use crate::call::Event;
use crate::{
    app::Message,
    controls::{self, Check, heading},
    pictures::Icon,
};
use backcord_iced_kit::i18n::tr;

/// The camera menu's width.
pub(super) const CAMERA_WIDTH: f32 = 270.0;

pub(super) fn camera_menu<'a>(ctx: crate::ctx::Ctx<'a>) -> Element<'a, Message> {
    let theme = ctx.look.theme;
    let voice = &ctx.state.voice;
    let quality = voice.camera_quality;
    let current = |on: bool| if on { Check::Current } else { Check::None };
    let mut rows = column![].spacing(2);
    if voice.cameras.len() > 1 {
        rows = rows.push(heading(theme, tr("CAMERA"), 4.0));
        let (names, index) =
            crate::call::popups::device_rows(&voice.cameras, voice.camera.as_deref());
        for (at, (name, device)) in names.into_iter().zip(&voice.cameras).enumerate() {
            rows = rows.push(crate::call::popups::popup_row(
                ctx,
                name,
                None,
                None,
                current(index == Some(at)),
                Some(Message::Call(Event::ChooseCamera(device.id.clone()))),
            ));
        }
    }
    rows = rows.push(super::quality::quality_picker(
        ctx,
        quality.resolution,
        quality.fps,
        |resolution| {
            Message::Call(Event::CameraQuality(CameraQuality {
                resolution,
                ..quality
            }))
        },
        |fps| Message::Call(Event::CameraQuality(CameraQuality { fps, ..quality })),
    ));
    let on = voice.camera_phase != voice::CameraPhase::Off;
    let (words, kind, icon, ink) = if on {
        (
            tr("Turn off camera"),
            controls::Kind::Danger,
            Icon::VideoOff,
            theme.on_danger,
        )
    } else {
        (
            tr("Turn on camera"),
            controls::Kind::Primary,
            Icon::Video,
            theme.on_accent,
        )
    };
    let toggle: Element<'a, Message> =
        controls::action(theme, words, kind, Some(Message::Call(Event::Camera)))
            .icon(ctx.look.icon(icon, 18, crate::theme::rgb(ink)))
            .wide(36.0)
            .into();
    column![
        rows,
        container(toggle).padding(Padding {
            top: 10.0,
            left: 4.0,
            right: 4.0,
            bottom: 4.0,
        }),
    ]
    .padding(6)
    .into()
}
