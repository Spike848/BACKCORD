//! The card under a message for each Discord message it links to, as
//! Vencord's MessageLinkEmbeds shows one: the linked message's author by
//! their avatar and name, when it was sent, its text (a few lines at most,
//! its Markdown as a message's), its first picture and the channel it's in.
//! A click on it goes to the message, as the link itself does. The core
//! takes each from the timeline or asks Discord once
//! (`AppState::message_cards`); until it answers the card is a
//! placeholder, and one this account can't see is a grey "Message not
//! available".

use std::{collections::HashMap, sync::Arc};

use backcord_core::{
    application::{AppState, MessageCard},
    chat::{ChatMessage, Placement},
    discord::MessageLink,
};
use backcord_iced_kit::i18n::tr;
use iced::{
    Alignment, Element, Length, Padding, Point,
    widget::{Space, button, column, container, row, text},
};

use crate::{
    app::{App, Message},
    controls,
    elide::elide,
    fonts::{ITALIC, SEMIBOLD},
    found::Found,
    markdown::{FIRST_PART, Look},
    media::{Gallery, Shot},
    pictures::Face,
    style::{hex, rounded},
};

use super::{EMBED_WIDTH, PICTURE_RADIUS};

/// A card's height before it's measured, for a row's first estimate.
pub const LINKED_CARD: f32 = 120.0;
/// The author's avatar, the text's lines at most and their height, and the
/// picture at its widest.
const AVATAR: f32 = 20.0;
const LINES: f32 = 4.0;
const LINE: f32 = 20.0;
const PICTURE: f32 = 300.0;
/// No timeline's row: the card's text selects nothing.
const NO_ROW: usize = usize::MAX;

/// The linked messages the core found, laid out once for their cards, by
/// their ids: those it found anew are laid out, those it let go of go.
pub fn linked_sync(app: &mut App) {
    let cards = &app.client.state().message_cards;
    app.timeline.linked.retain(|id, laid| {
        matches!(cards.get(id), Some(MessageCard::Found(message)) if Arc::ptr_eq(message, &laid.message))
    });
    let new: Vec<(String, Arc<ChatMessage>)> = cards
        .iter()
        .filter_map(|(id, card)| match card {
            MessageCard::Found(message) if !app.timeline.linked.contains_key(id) => {
                Some((id.clone(), message.clone()))
            }
            _ => None,
        })
        .collect();
    for (id, message) in new {
        let laid = crate::found::found(app, message);
        app.timeline.linked.insert(id, laid);
    }
}

/// The linked messages laid out, by id, kept with the timeline.
pub type Linked = HashMap<String, Found>;

/// The card for the message `link` names; none while the core can't say
/// what it is.
pub(super) fn linked_card<'a>(
    ctx: crate::ctx::Ctx<'a>,
    link: &'a MessageLink,
) -> Option<Element<'a, Message>> {
    let card = match ctx.state.message_cards.get(&link.message) {
        Some(MessageCard::Found(message)) => {
            match ctx
                .lanes
                .timeline
                .linked
                .get(&link.message)
                .filter(|laid| Arc::ptr_eq(&laid.message, message))
            {
                Some(laid) => found_card(ctx, link, laid),
                None => loading(ctx),
            }
        }
        Some(MessageCard::Unavailable) => unavailable(ctx, link),
        Some(MessageCard::Loading) => loading(ctx),
        // About to be asked for.
        None if ctx.state.discord_online || ctx.state.preview => loading(ctx),
        None => return None,
    };
    Some(
        container(card)
            .max_width(EMBED_WIDTH)
            .padding(Padding::ZERO.top(4).bottom(3))
            .into(),
    )
}

/// Where the message is, as its card's foot says it: "#channel", or a DM's
/// name.
pub fn place_name(state: &AppState, link: &MessageLink) -> Option<String> {
    let room = link.room();
    let room = state.rooms.iter().find(|known| known.id == room)?;
    Some(if room.direct {
        room.name.clone()
    } else {
        format!("#{}", room.name)
    })
}

/// A message found: its author and when, its text, its first picture and
/// where it is; a click goes there.
fn found_card<'a>(
    ctx: crate::ctx::Ctx<'a>,
    link: &'a MessageLink,
    laid: &'a Found,
) -> Element<'a, Message> {
    let theme = ctx.look.theme;
    let message = &laid.message;
    let mut words = column![
        row![
            ctx.face(Face::new(
                message.avatar.as_deref(),
                &message.author,
                AVATAR,
            )),
            elide(message.author.clone())
                .size(14)
                .font(SEMIBOLD)
                .color(message.color.map_or(theme.text, hex)),
            elide(laid.when.as_str()).size(12).color(theme.muted),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
    ]
    .spacing(4)
    .width(Length::Fill);
    if !message.body.trim().is_empty() {
        let body = ctx.body_view(
            &laid.body.body,
            NO_ROW,
            FIRST_PART,
            Look {
                size: 14.0,
                line_height: LINE,
                ink: theme.text,
            },
        );
        words = words.push(container(body).max_height(LINE * LINES).clip(true));
    }
    // Its first picture, or video's poster.
    if let Some(at) = message.attachments.iter().position(|attachment| {
        attachment.placement == Placement::Message && (attachment.image || attachment.video)
    }) {
        let gallery = Gallery::of(&message.attachments[at..=at], Placement::Message, PICTURE);
        if !gallery.is_empty() {
            let attachment = &message.attachments[at];
            words = words.push(
                container(
                    ctx.shot(
                        Shot::new(attachment.preview.as_ref(), gallery.size)
                            .radius(PICTURE_RADIUS)
                            .covered(attachment.spoiler),
                    ),
                )
                .padding(Padding::ZERO.top(4)),
            );
        }
    }
    if let Some(place) = place_name(ctx.state, link) {
        words = words.push(elide(place).size(12).color(theme.muted));
    }
    let (rest, lit, edge) = (theme.sidebar, theme.hover, theme.line);
    button(container(words).padding(12))
        .padding(0)
        .width(Length::Fill)
        .on_press(Message::Link(link.url(), Point::ORIGIN))
        .style(controls::outlined_lit_style(rest, lit, edge, edge, 8.0))
        .into()
}

/// A message gone, or that this account can't see.
fn unavailable<'a>(ctx: crate::ctx::Ctx<'a>, link: &MessageLink) -> Element<'a, Message> {
    let theme = ctx.look.theme;
    let mut words = column![
        text(tr("Message not available"))
            .size(14)
            .font(ITALIC)
            .color(theme.muted)
    ]
    .spacing(4)
    .width(Length::Fill);
    if let Some(place) = place_name(ctx.state, link) {
        words = words.push(elide(place).size(12).color(theme.faint));
    }
    let ground = theme.sidebar;
    container(words)
        .padding(12)
        .width(Length::Fill)
        .style(move |_| rounded(ground, 8.0))
        .into()
}

/// Still asked for: grey bars where its author and text will be.
fn loading<'a>(ctx: crate::ctx::Ctx<'a>) -> Element<'a, Message> {
    let theme = ctx.look.theme;
    let blank = |width: f32, height: f32, radius: f32| {
        let ground = theme.hover;
        container(Space::new().width(width).height(height)).style(move |_| rounded(ground, radius))
    };
    let ground = theme.sidebar;
    container(
        column![
            row![blank(AVATAR, AVATAR, AVATAR / 2.0), blank(120.0, 14.0, 7.0)]
                .spacing(6)
                .align_y(Alignment::Center),
            blank(240.0, 14.0, 7.0),
            blank(180.0, 14.0, 7.0),
        ]
        .spacing(8),
    )
    .padding(12)
    .width(Length::Fill)
    .style(move |_| rounded(ground, 8.0))
    .into()
}
