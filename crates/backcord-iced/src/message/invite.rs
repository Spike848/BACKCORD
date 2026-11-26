//! The card under a message for each Discord invite it links to, as
//! Discord's "You've been invited to join a server": the server's icon,
//! name and counts, and a button that joins it, or opens it for a member.
//! The core looks each invite up once (`AppState::invite_cards`); until
//! it answers the card is a placeholder, and an invite Discord doesn't
//! know is a grey "Invalid Invite".

use backcord_core::{application::InviteCard, chat::ChatMessage};
use backcord_iced_kit::i18n::{t, tr};
use iced::{
    Alignment, Element, Length, Padding,
    widget::{Space, column, container, row, text},
};

use crate::{
    app::Message,
    controls::{Kind, action},
    fonts::{BOLD, SEMIBOLD},
    style::rounded,
};

use super::EMBED_WIDTH;

/// The card's height, whatever it shows, so a row keeps its height as the
/// invite is looked up.
pub const INVITE_CARD: f32 = 16.0 + 16.0 + 12.0 + ICON + 16.0;
/// The server's icon, and its corners.
const ICON: f32 = 50.0;
const ICON_RADIUS: f32 = 16.0;

/// The card for the invite with `code` that `message` links to.
pub(super) fn invite_card<'a>(
    ctx: crate::ctx::Ctx<'a>,
    message: &'a ChatMessage,
    code: &'a str,
) -> Element<'a, Message> {
    let theme = ctx.look.theme;
    let heading = |words: &str| {
        text(words.to_uppercase())
            .size(12)
            .font(BOLD)
            .color(theme.muted)
            .wrapping(text::Wrapping::None)
    };
    let name = |words: String, ink| {
        text(words)
            .size(16)
            .font(SEMIBOLD)
            .color(ink)
            .wrapping(text::Wrapping::None)
    };
    let blank = |width: f32, height: f32, radius: f32| {
        let ground = theme.hover;
        container(Space::new().width(width).height(height)).style(move |_| rounded(ground, radius))
    };
    let (head, picture, words, button): (
        String,
        Element<'a, Message>,
        Element<'a, Message>,
        Option<Element<'a, Message>>,
    ) = match ctx.state.invite_cards.get(code) {
        Some(InviteCard::Found(preview)) => {
            let member = ctx
                .state
                .servers
                .iter()
                .any(|server| server.id == preview.server);
            let joining = ctx
                .state
                .join
                .as_ref()
                .is_some_and(|join| join.joining && join.code.as_deref() == Some(code));
            let dot =
                |ink| container(Space::new().width(8).height(8)).style(move |_| rounded(ink, 4.0));
            let count = |words: String| text(words).size(14).color(theme.muted);
            let counts: Element<'a, Message> = match (preview.online, preview.members) {
                (Some(online), Some(members)) => row![
                    dot(theme.live),
                    count(t(format!("{online} Online"))),
                    Space::new().width(4),
                    dot(theme.muted),
                    count(t(format!("{members} Members"))),
                ]
                .spacing(4)
                .align_y(Alignment::Center)
                .into(),
                _ if preview.voice => count(preview.channel.clone()).into(),
                _ => count(format!("#{}", preview.channel)).into(),
            };
            let (label, kind) = if member {
                (tr("Joined"), Kind::Plain)
            } else if joining {
                (tr("Joining..."), Kind::Success)
            } else {
                (tr("Join"), Kind::Success)
            };
            let press = (!joining)
                .then(|| Message::Timeline(crate::timeline::Event::JoinInvite(code.to_owned())));
            (
                if message.own {
                    tr("You sent an invite to join a server")
                } else {
                    tr("You've been invited to join a server")
                }
                .to_owned(),
                ctx.avatar(
                    preview.icon.as_deref(),
                    &preview.name,
                    ICON,
                    ICON_RADIUS,
                    1.0,
                ),
                column![name(preview.name.clone(), theme.text), counts]
                    .spacing(2)
                    .width(Length::Fill)
                    .clip(true)
                    .into(),
                Some(action(theme, label, kind, press).into()),
            )
        }
        Some(InviteCard::Invalid) => (
            if message.own {
                tr("You sent an invite, but...")
            } else {
                tr("You received an invite, but...")
            }
            .to_owned(),
            container(text("?").size(24).font(BOLD).color(theme.faint))
                .center(ICON)
                .style({
                    let ground = theme.hover;
                    move |_| rounded(ground, ICON_RADIUS)
                })
                .into(),
            column![
                name(tr("Invalid Invite").to_owned(), theme.muted),
                text(if message.own {
                    tr("Try sending a new invite!")
                } else {
                    tr("Ask for a new invite!")
                })
                .size(14)
                .color(theme.muted),
            ]
            .spacing(2)
            .width(Length::Fill)
            .into(),
            None,
        ),
        // Still looked up, or about to be.
        Some(InviteCard::Loading) | None => (
            tr("Resolving invite").to_owned(),
            blank(ICON, ICON, ICON_RADIUS).into(),
            column![blank(160.0, 16.0, 8.0), blank(110.0, 14.0, 7.0)]
                .spacing(8)
                .width(Length::Fill)
                .into(),
            None,
        ),
    };
    let mut line = row![picture, words].spacing(16).align_y(Alignment::Center);
    if let Some(button) = button {
        line = line.push(button);
    }
    let ground = theme.sidebar;
    container(
        container(column![heading(&head), line].spacing(12))
            .padding(16)
            .width(Length::Fill)
            .height(INVITE_CARD)
            .max_width(EMBED_WIDTH)
            .clip(true)
            .style(move |_| rounded(ground, 8.0)),
    )
    .padding(Padding::ZERO.top(4).bottom(3))
    .into()
}
