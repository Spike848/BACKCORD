//! The full profile: the card down its left, and on the right the tabs,
//! Board, Activity and what's shared, each with Discord's words
//! while it's empty and rows the shape of what's coming while Discord
//! hasn't said.

use crate::widgets::scroll_view;
use backcord_core::chat::Presence;
use iced::{
    Alignment, Color, Element, Length, Padding,
    widget::{Column, Space, button, column, container, mouse_area, row, stack, text},
};

use super::{
    Event, Mode, Shown,
    look::{Inks, Tab, time_ago},
};
use crate::{
    app::Message,
    controls::{Kind, action, lit_style, placed},
    fonts::{BOLD, MEDIUM, SEMIBOLD},
    pictures::{Icon, Mark, initials},
    style::{faded, rounded, scrollbar},
    theme::rgb,
    widgets::Pic,
};

/// The full profile's card, as wide as Discord's.
pub(super) const SIDE_WIDTH: f32 = 340.0;

/// A line of the card's words, wrapped.
fn note<'a>(ink: Color, words: &'a str) -> Element<'a, Message> {
    text(words).size(13).color(ink).into()
}

/// A mutual server or friend; a click goes there.
fn list_row<'a>(
    face: Element<'a, Message>,
    name: &'a str,
    detail: Option<&'a str>,
    inks: &Inks,
    message: Message,
) -> Element<'a, Message> {
    let mut words = column![
        text(name)
            .size(15)
            .font(MEDIUM)
            .color(inks.ink)
            .wrapping(text::Wrapping::None)
    ];
    if let Some(detail) = detail {
        words = words.push(text(detail).size(12).color(inks.ink3));
    }
    let hover = inks.hover;
    let height = if detail.is_some() { 46.0 } else { 40.0 };
    mouse_area(
        button(
            row![face, words.width(Length::Fill)]
                .spacing(12)
                .align_y(Alignment::Center),
        )
        .padding([0, 8])
        .height(height)
        .width(Length::Fill)
        .on_press(message)
        .style(crate::controls::lit_style(None, hover, 6.0)),
    )
    .interaction(iced::mouse::Interaction::Pointer)
    .into()
}

/// The full profile at its size.
pub(super) fn full_view<'a>(
    ctx: crate::ctx::Ctx<'a>,
    shown: &'a Shown,
    width: f32,
    height: f32,
) -> Element<'a, Message> {
    let look = &shown.look;
    let inks = crate::profile::inks(ctx, look, Mode::Side);
    // The card keeps the width of Discord's, whatever the window's: wider,
    // its banner, frame and type would grow with it.
    let side_width = SIDE_WIDTH;
    let side = container(
        scroll_view(crate::profile::body::profile_body(
            ctx,
            shown,
            Mode::Side,
            side_width,
            true,
        ))
        .height(Length::Fill)
        .style(scrollbar(ctx.look.theme)),
    )
    .width(side_width)
    .height(Length::Fill)
    .clip(true)
    .style(super::frame_style(inks, ctx.look.theme.edge, 12.0, true));
    let side = mouse_area(crate::profile::cosmetic::dressed(
        ctx,
        shown,
        Mode::Side,
        side_width,
        side.into(),
    ))
    .on_enter(Message::Profile(Event::Pointed(true)))
    .on_exit(Message::Profile(Event::Pointed(false)));
    let tab = crate::profile::profile_tab(ctx, look);
    let mut faces = Vec::new();
    for entry in &look.tabs {
        let label = match entry {
            Tab::Board => backcord_iced_kit::i18n::tr("Board"),
            Tab::Activity => backcord_iced_kit::i18n::tr("Activity"),
            Tab::Friends => look.friends_label.as_str(),
            Tab::Servers => look.servers_label.as_str(),
        };
        let face = crate::controls::tab(
            crate::controls::Tab {
                label,
                chosen: *entry == tab,
                ink: inks.ink,
                rest: inks.ink3,
                bar: inks.ink,
                height: 38.0,
                rest_font: MEDIUM,
            },
            Message::Profile(Event::Tab(*entry)),
        );
        faces.push(Element::from(face));
    }
    // The tabs wrap onto a second line in a narrow window, rather than run
    // past the profile.
    let tabs = container(row(faces).spacing(16).wrap()).width(Length::Fill);
    let close = button(ctx.look.glyph(Icon::X, 18, inks.ink2).boxed(32))
        .padding(0)
        .width(32)
        .height(32)
        .on_press(Message::Profile(Event::Close))
        .style(lit_style(None, inks.hover, 16.0));
    let head = row![
        tabs,
        container(close).padding(Padding {
            top: 4.0,
            ..Padding::ZERO
        })
    ];
    let rule_ink = if inks.themed {
        faded(inks.ink, 0.12)
    } else {
        ctx.look.theme.line
    };
    let rule = container(Space::new())
        .width(Length::Fill)
        .height(1)
        .style(move |_| rounded(rule_ink, 0));
    let page = tab_page(ctx, shown, tab, &inks);
    let right = column![head, rule, page]
        .spacing(12)
        .padding(Padding {
            top: 4.0,
            left: 12.0,
            right: 12.0,
            bottom: 0.0,
        })
        .width(Length::Fill)
        .height(Length::Fill);
    let edge = ctx.look.theme.edge;
    container(row![side, right].spacing(8).padding(8))
        .width(width)
        .height(height)
        .style(super::frame_style(inks, edge, 8.0, true))
        .into()
}

/// A tab's page: rows the shape of what's coming while Discord hasn't
/// said, its words while there's nothing, else what it shows.
pub(super) fn tab_page<'a>(
    ctx: crate::ctx::Ctx<'a>,
    shown: &'a Shown,
    tab: Tab,
    inks: &Inks,
) -> Element<'a, Message> {
    let look = &shown.look;
    let profile = look.profile();
    let blank = match tab {
        Tab::Activity => look.activities.is_empty() && profile.recent.is_empty(),
        Tab::Friends => profile.friends.is_empty(),
        Tab::Servers => profile.servers.is_empty(),
        Tab::Board => false,
    };
    let loading = blank
        && if tab == Tab::Activity {
            !look.recent_known
        } else {
            !look.mutuals_known
        };
    if loading {
        return skeleton_rows(ctx, tab == Tab::Servers);
    }
    if blank {
        return empty_tab(ctx, shown, tab, inks);
    }
    let content: Element<'a, Message> = match tab {
        Tab::Activity => activity_tab(ctx, shown, inks),
        Tab::Board => board_tab(ctx, shown, inks),
        Tab::Friends => {
            let mut list = column![];
            for (index, friend) in profile.friends.iter().enumerate() {
                let presence = look.presences.get(index).copied();
                let face = list_face(
                    ctx,
                    friend.avatar.as_deref(),
                    &friend.name,
                    true,
                    presence,
                    inks,
                );
                list = list.push(list_row(
                    face,
                    &friend.name,
                    None,
                    inks,
                    Message::Profile(Event::Friend(friend.id.clone())),
                ));
            }
            list.into()
        }
        Tab::Servers => {
            let mut list = column![];
            for server in &profile.servers {
                let face = list_face(ctx, server.icon.as_deref(), &server.name, false, None, inks);
                list = list.push(list_row(
                    face,
                    &server.name,
                    server.nick.as_deref(),
                    inks,
                    Message::Profile(Event::Server(server.id.clone())),
                ));
            }
            list.into()
        }
    };
    scroll_view(content)
        .height(Length::Fill)
        .style(scrollbar(ctx.look.theme))
        .into()
}

/// Rows the shape of what's coming, as Discord's client shows while it
/// waits: round faces for people, rounded squares for servers.
fn skeleton_rows<'a>(ctx: crate::ctx::Ctx<'a>, servers: bool) -> Element<'a, Message> {
    let shimmer = ctx.look.theme.shimmer;
    let bone = move |width: f32, height: f32, radius: f32| {
        container(Space::new())
            .width(width)
            .height(height)
            .style(move |_| rounded(shimmer, radius))
    };
    let mut rows = column![].spacing(12).padding(Padding {
        top: 12.0,
        ..Padding::ZERO
    });
    for line in [180.0, 140.0, 210.0, 120.0, 170.0, 150.0] {
        rows = rows.push(
            row![
                bone(40.0, 40.0, if servers { 12.0 } else { 20.0 }),
                column![bone(line, 12.0, 6.0), bone(line * 0.6, 10.0, 5.0)].spacing(8),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        );
    }
    rows.into()
}

/// Discord's words for an empty tab; a bot's profile words its servers
/// as the client's does.
fn empty_tab<'a>(
    ctx: crate::ctx::Ctx<'a>,
    shown: &'a Shown,
    tab: Tab,
    inks: &Inks,
) -> Element<'a, Message> {
    let look = &shown.look;
    let profile = look.profile();
    let (heading, body) = match tab {
        Tab::Activity if look.own => (
            backcord_iced_kit::i18n::tr("You don't have any activity here").to_owned(),
            backcord_iced_kit::i18n::tr(
                "Connect your accounts to share your in-game status and activity. You're in control of what you share.",
            ),
        ),
        Tab::Activity => (
            backcord_iced_kit::i18n::t(format!(
                "{} doesn't have any activity to share here",
                profile.name
            )),
            backcord_iced_kit::i18n::tr(
                "Looks like this profile is still a work in progress - send them a message to say hi!",
            ),
        ),
        Tab::Friends => (
            backcord_iced_kit::i18n::tr("You don't have any friends in common").to_owned(),
            backcord_iced_kit::i18n::tr(
                "No mutual friends yet - perfect time to make a new connection",
            ),
        ),
        _ if look.bot => (
            String::new(),
            backcord_iced_kit::i18n::tr("No servers in common"),
        ),
        _ => (
            backcord_iced_kit::i18n::tr("You don't have any servers in common").to_owned(),
            backcord_iced_kit::i18n::tr(
                "Time to impress them with your taste...drop an invite link to your favorite server",
            ),
        ),
    };
    let mut words = column![].spacing(8).align_x(Alignment::Center);
    if !heading.is_empty() {
        words = words.push(
            text(heading)
                .size(16)
                .font(BOLD)
                .color(inks.ink)
                .align_x(Alignment::Center),
        );
    }
    words = words.push(
        text(body)
            .size(14)
            .color(inks.ink2)
            .align_x(Alignment::Center),
    );
    if tab == Tab::Activity && !look.own && look.can_message {
        let icon = ctx
            .look
            .icon(Icon::MessageCircle, 16, rgb(ctx.look.theme.text));
        words = words.push(
            container(
                action(
                    ctx.look.theme,
                    backcord_iced_kit::i18n::tr("Message"),
                    Kind::Plain,
                    Some(Message::Profile(Event::Message(profile.id.clone()))),
                )
                .icon(icon),
            )
            .padding(Padding {
                top: 12.0,
                ..Padding::ZERO
            }),
        );
    }
    container(words)
        .padding([0, 32])
        .width(Length::Fill)
        .height(Length::Fill)
        .align_y(Alignment::Center)
        .align_x(Alignment::Center)
        .into()
}

/// What they're doing, and what they did lately from Discord's outbox.
fn activity_tab<'a>(
    ctx: crate::ctx::Ctx<'a>,
    shown: &'a Shown,
    inks: &Inks,
) -> Element<'a, Message> {
    let look = &shown.look;
    let profile = look.profile();
    let mut list = column![].spacing(8);
    for activity in &look.activities {
        list = list.push(crate::profile::activity::activity_card_view(
            ctx, activity, inks, None,
        ));
    }
    if !profile.recent.is_empty() {
        list = list.push(
            text(backcord_iced_kit::i18n::tr("Recent activity"))
                .size(12)
                .font(MEDIUM)
                .color(inks.ink2),
        );
    }
    let tile = inks.tile;
    for entry in &profile.recent {
        let mut lines = column![
            text(entry.title.as_str())
                .size(14)
                .font(SEMIBOLD)
                .color(inks.ink)
                .wrapping(text::Wrapping::None)
        ]
        .spacing(2)
        .width(Length::Fill);
        if !entry.subtitle.is_empty() {
            lines = lines.push(
                text(entry.subtitle.as_str())
                    .size(13)
                    .color(inks.ink2)
                    .wrapping(text::Wrapping::None),
            );
        }
        if let Some(at) = entry.at {
            let music = if entry.music { "\u{266b} " } else { "" };
            lines = lines.push(
                text(format!("{music}{}", time_ago(at)))
                    .size(12)
                    .color(inks.ink3),
            );
        }
        let picture = showcase(
            ctx,
            shown,
            entry.picture.as_deref(),
            &entry.title,
            40.0,
            16.0,
            inks,
        );
        list = list.push(
            container(row![picture, lines].spacing(12).align_y(Alignment::Center))
                .padding(12)
                .height(64)
                .width(Length::Fill)
                .clip(true)
                .style(move |_| rounded(tile, 10)),
        );
    }
    list.into()
}

/// Their board's widgets, each with its games.
fn board_tab<'a>(ctx: crate::ctx::Ctx<'a>, shown: &'a Shown, inks: &Inks) -> Element<'a, Message> {
    let profile = shown.look.profile();
    if profile.board_asked && !profile.board_named {
        return note(
            inks.ink2,
            backcord_iced_kit::i18n::tr("Loading the board..."),
        );
    }
    if profile.widgets.is_empty() {
        return note(
            inks.ink3,
            backcord_iced_kit::i18n::tr("Nothing on the board yet."),
        );
    }
    board_cards(ctx, shown, inks, column![].spacing(12)).into()
}

/// The board's widgets, each with its games, after what `list` holds.
fn board_cards<'a>(
    ctx: crate::ctx::Ctx<'a>,
    shown: &'a Shown,
    inks: &Inks,
    mut list: Column<'a, Message>,
) -> Column<'a, Message> {
    let profile = shown.look.profile();
    let tile = inks.tile;
    for widget in &profile.widgets {
        let mut card = column![
            text(backcord_iced_kit::i18n::t(&widget.title))
                .size(14)
                .font(SEMIBOLD)
                .color(inks.ink)
                .wrapping(text::Wrapping::None)
        ]
        .spacing(10);
        for game in &widget.games {
            let mut words = column![
                text(backcord_iced_kit::i18n::t(&game.name))
                    .size(13)
                    .font(MEDIUM)
                    .color(inks.ink)
                    .wrapping(text::Wrapping::None)
            ]
            .spacing(2);
            if let Some(comment) = &game.comment {
                words = words.push(text(comment.as_str()).size(12).color(inks.ink2));
            }
            card = card.push(
                row![
                    showcase(
                        ctx,
                        shown,
                        game.icon.as_deref(),
                        &game.name,
                        44.0,
                        16.0,
                        inks
                    ),
                    words
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            );
        }
        if !widget.lists_games {
            card = card.push(
                text(backcord_iced_kit::i18n::tr(
                    "This one only shows in Discord for now.",
                ))
                .size(12)
                .color(inks.ink3),
            );
        }
        list = list.push(
            container(card)
                .padding(14)
                .width(Length::Fill)
                .style(move |_| rounded(tile, 12)),
        );
    }
    list
}

/// What the profile editor's tabs show of the profile as it will be: the
/// widgets with what adding to them from here is, and
/// what's going on now. `board` is the board's note while the profile
/// loads or after it failed to.
pub(super) fn editor_tab_page<'a>(
    ctx: crate::ctx::Ctx<'a>,
    shown: &'a Shown,
    tab: Tab,
    inks: &Inks,
    board: Option<&'static str>,
) -> Element<'a, Message> {
    let look = &shown.look;
    let profile = look.profile();
    let heading = |words: String| text(words).size(13).font(SEMIBOLD).color(inks.ink2);
    let soon = || {
        text(backcord_iced_kit::i18n::tr(
            "Adding more from BACKCORD is coming soon.",
        ))
        .size(12)
        .color(inks.ink3)
    };
    let page = match tab {
        Tab::Activity => {
            let mut list = column![heading(
                backcord_iced_kit::i18n::tr("Current activity").to_owned()
            )]
            .spacing(10);
            if look.activities.is_empty() {
                list = list.push(note(
                    inks.ink3,
                    backcord_iced_kit::i18n::tr("Nothing going on right now."),
                ));
            }
            for activity in &look.activities {
                list = list.push(crate::profile::activity::activity_card_view(
                    ctx, activity, inks, None,
                ));
            }
            list
        }
        Tab::Board => {
            let mut list = column![].spacing(12);
            match board {
                Some(words) => list = list.push(note(inks.ink2, words)),
                None => {
                    if profile.widgets.is_empty() {
                        list = list.push(note(
                            inks.ink2,
                            backcord_iced_kit::i18n::tr("Nothing on your board yet."),
                        ));
                    } else {
                        list = list.push(heading(
                            backcord_iced_kit::i18n::tr("Your Widgets").to_owned(),
                        ));
                    }
                    list = list.push(soon());
                }
            }
            board_cards(ctx, shown, inks, list)
        }
        Tab::Friends | Tab::Servers => return tab_page(ctx, shown, tab, inks),
    };
    scroll_view(page)
        .height(Length::Fill)
        .style(scrollbar(ctx.look.theme))
        .into()
}

/// A picture on the Board or Activity tab; what has none
/// shows its name's first letter.
fn showcase<'a>(
    ctx: crate::ctx::Ctx<'a>,
    shown: &'a Shown,
    url: Option<&str>,
    name: &str,
    side: f32,
    initial: f32,
    inks: &Inks,
) -> Element<'a, Message> {
    let well = inks.tile;
    let picture = url.and_then(|url| {
        crate::profile::profile_picture(ctx, shown, url, side as u32, side as u32, false)
    });
    match picture {
        Some(picture) => Pic::square(picture, side).radius(8.0).into(),
        None => container(
            text(initials(name).chars().take(1).collect::<String>())
                .size(initial)
                .font(BOLD)
                .color(inks.ink3),
        )
        .center(side)
        .style(move |_| rounded(well, 8))
        .into(),
    }
}

/// A mutual friend's round face with their presence, or a server's
/// rounded square.
fn list_face<'a>(
    ctx: crate::ctx::Ctx<'a>,
    url: Option<&str>,
    name: &str,
    person: bool,
    presence: Option<Presence>,
    inks: &Inks,
) -> Element<'a, Message> {
    let face = ctx.avatar(url, name, 32.0, if person { 16.0 } else { 10.0 }, 1.0);
    let Some(dot) = presence
        .filter(|presence| *presence != Presence::Offline)
        .and_then(|presence| ctx.mark(Mark::Dot(presence, rgb(inks.ground))))
    else {
        return face;
    };
    stack![face, placed(Pic::square(dot, 16.0), 20.0, 20.0)]
        .width(36)
        .height(36)
        .into()
}
