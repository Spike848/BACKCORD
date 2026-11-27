//! What opens from the call's controls, above the control: the device menus
//! (with the microphone's level and noise suppression), the share menu, the
//! soundboard; and someone's voice menu, their volumes over their rows.

use std::borrow::Cow;

use backcord_core::{meter::InputMeter, voice::Command};
use iced::{
    Element, Point, Task,
    widget::{container, opaque, operation},
};

use super::Popup;
use crate::{
    app::{App, Message, NOBODY},
    controls::{Check, Item, Light, menu_panel},
    pictures::Icon,
    widgets::{Anchor, Place},
};

pub mod camera;
pub mod device;
pub mod limit;
pub mod person;
pub mod quality;
pub mod room;
pub mod share;
pub mod soundboard;
use device::DEVICE_WIDTH;
pub use device::device_rows;
pub use person::Volumes;
#[cfg(test)]
pub use share::Listing;
use share::{LIVE_WIDTH, list_sources};
pub use share::{Share, ShareEvent};
pub(crate) use soundboard::SOUNDBOARD_WIDTH;

pub(super) fn open_popup(app: &mut App, popup: Popup) -> Task<Message> {
    // A second click closes it, or cancels the share menu's listing.
    let listing = popup == Popup::Share && app.call.share_listing.is_some();
    if app.call.popup == Some(popup) || listing {
        close_popup(app);
        return Task::none();
    }
    close_popup(app);
    match popup {
        Popup::Input | Popup::Output => {
            app.client
                .dispatch(backcord_core::application::Action::Voice(Command::Devices));
            if popup == Popup::Input {
                let microphone = app.client.state().voice.microphone.clone();
                app.call.meter = Some(InputMeter::start(microphone.as_deref()));
            }
            app.call.popup = Some(popup);
        }
        Popup::Soundboard => {
            let pictures: Vec<String> = app
                .client
                .state()
                .voice
                .soundboard
                .iter()
                .flat_map(|groups| groups.iter())
                .flat_map(|group| &group.sounds)
                .filter_map(|sound| sound.picture.clone())
                .collect();
            if !pictures.is_empty() {
                app.client
                    .dispatch(backcord_core::application::Action::LoadPictures(pictures));
            }
            app.call.popup = Some(popup);
        }
        // Shown once its sources are listed, so it opens at its height;
        // meanwhile it's as good as open (Escape or a click elsewhere
        // cancels it).
        Popup::Share => {
            app.call.listings += 1;
            app.call.share_listing = Some(app.call.listings);
            return Task::batch([list_sources(app.call.listings), operation::focus(NOBODY)]);
        }
        Popup::Link => {
            app.call.popup = Some(popup);
            crate::call::details::read_link(app);
            return Task::none();
        }
        Popup::Live | Popup::Limit => app.call.popup = Some(popup),
        Popup::Room => {
            room::place_newcomers(app);
            app.call.popup = Some(popup);
        }
        Popup::Camera => {
            app.client
                .dispatch(backcord_core::application::Action::Voice(Command::Devices));
            app.call.popup = Some(popup);
        }
    }
    Task::batch([app.sync(), operation::focus(NOBODY)])
}

pub fn close_popup(app: &mut App) {
    let link = app.call.popup == Some(Popup::Link);
    app.call.popup = None;
    if link {
        app.call.details.tab = super::LinkTab::Connection;
        crate::call::details::read_link(app);
    }
    app.call.share = None;
    app.call.share_listing = None;
    app.call.listing = false;
    app.call.meter = None;
}

/// The popup open from a control, above it.
pub fn call_popup_view<'a>(ctx: crate::ctx::Ctx<'a>) -> Option<Element<'a, Message>> {
    let popup = ctx.call.popup?;
    let (panel, width) = match popup {
        Popup::Input => (
            crate::call::popups::device::device_menu(ctx, true),
            DEVICE_WIDTH,
        ),
        Popup::Output => (
            crate::call::popups::device::device_menu(ctx, false),
            DEVICE_WIDTH,
        ),
        // A window over the call, dimmed under it, as Discord's.
        Popup::Share => {
            let share = ctx.call.share.as_ref()?;
            return Some(opaque(iced::widget::stack![
                crate::controls::dimmed(Message::Call(super::Event::Close), crate::sheet::DIM),
                container(opaque(crate::call::popups::share::share_menu(ctx, share)))
                    .center(iced::Length::Fill),
            ]));
        }
        Popup::Live => {
            ctx.state.voice.sharing.as_ref()?;
            (crate::call::popups::share::live_menu(ctx), LIVE_WIDTH)
        }
        Popup::Camera => (
            crate::call::popups::camera::camera_menu(ctx),
            camera::CAMERA_WIDTH,
        ),
        Popup::Limit => (
            crate::call::popups::limit::limit_menu(ctx),
            limit::LIMIT_WIDTH,
        ),
        Popup::Room => (room::room_menu(ctx), room::ROOM_WIDTH),
        Popup::Soundboard => (
            crate::call::popups::soundboard::soundboard(ctx),
            SOUNDBOARD_WIDTH + crate::call::popups::soundboard::BOARD_RAIL,
        ),
        // Under its signal, or above it without room.
        Popup::Link => {
            let spot = ctx.call.spot.get();
            return Some(
                Place::new(
                    opaque(crate::call::details::link_card(ctx)),
                    spot.position(),
                    8.0,
                )
                .anchor(Anchor::Span(spot.height))
                .into(),
            );
        }
    };
    let spot = ctx.call.spot.get();
    // The frosted background under the panel's see-through ground.
    let panel = backcord_iced_kit::widgets::frost::Frost::new(
        [8.0; 4],
        container(panel)
            .width(width)
            .style(menu_panel(ctx.look.theme)),
    );
    Some(
        Place::new(opaque(panel), Point::new(spot.x - 6.0, spot.y - 6.0), 8.0)
            .anchor(Anchor::Bottom)
            .into(),
    )
}

/// A row of a popup, lit under the pointer.
fn popup_row<'a>(
    ctx: crate::ctx::Ctx<'a>,
    label: impl Into<Cow<'a, str>>,
    subtitle: Option<Cow<'a, str>>,
    icon: Option<Icon>,
    check: Check,
    press: Option<Message>,
) -> Element<'a, Message> {
    let item = Item {
        picture: None,
        label: label.into(),
        icon,
        danger: false,
        slot: false,
        disabled: press.is_none(),
        subtitle,
        check,
    };
    ctx.look
        .menu_item(item, Light::Pointer, press.unwrap_or(Message::Nothing))
}
