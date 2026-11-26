//! The soundboard (`Soundboard`): the sounds of each server this
//! account may play, which also lists the entrance sounds to choose from.

use crate::widgets::scroll_view;
use backcord_core::voice::{self};
use iced::{
    Alignment, Color, Element, Length, Padding,
    widget::{Space, button, column, container, mouse_area, row, text},
};

use crate::call::Event;
use crate::{
    app::Message,
    controls::{self},
    fonts::BOLD,
    pictures::Icon,
    widgets::{Hover, Pic},
};

pub(crate) const SOUNDBOARD_WIDTH: f32 = 372.0;
/// The call's soundboard's rail of servers, left of its sounds.
pub(crate) const BOARD_RAIL: f32 = 52.0;
/// A rail's icon.
const RAIL_ICON: f32 = 34.0;

pub(super) const SOUNDBOARD_TALLEST: f32 = 440.0;

/// A soundboard row, and a sound's button in it.
pub(super) const SOUND_ROW: f32 = 42.0;

/// A board of sounds: `title`, a note (while there is one, no sound can be
/// chosen), the groups' sounds three to a row under their names, `chosen`'s
/// message for a click on one, and with `extras` hearing it first and its
/// star under the pointer. The call's soundboard, and the entrance sound's
/// choice in Settings.
pub(crate) fn sound_board<'a>(
    ctx: crate::ctx::Ctx<'a>,
    title: &'a str,
    groups: &'a [voice::SoundGroup],
    note: Option<&'a str>,
    extras: bool,
    chosen: impl Fn(&voice::SoundChoice) -> Message,
    corner: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let theme = ctx.look.theme;
    let mut heading = row![
        text(title)
            .size(16)
            .font(BOLD)
            .color(theme.text)
            .width(Length::Fill)
    ]
    .align_y(Alignment::Center);
    if let Some(corner) = corner {
        heading = heading.push(corner);
    }
    let mut top = column![heading].spacing(4).padding(Padding {
        top: 14.0,
        left: 14.0,
        right: 14.0,
        bottom: 4.0,
    });
    if let Some(note) = note {
        top = top.push(
            text(backcord_iced_kit::i18n::t(note))
                .size(13)
                .color(theme.muted),
        );
    }
    if groups.is_empty() {
        top = top.push(
            text(backcord_iced_kit::i18n::tr("Loading sounds..."))
                .size(13)
                .color(theme.muted),
        );
    }
    // Two to a row, each name whole, over as many lines as it takes.
    let cell = (SOUNDBOARD_WIDTH - 20.0 - 6.0) / 2.0;
    let per_line = ((cell - 20.0 - 26.0) / 7.4).floor().max(1.0) as usize;
    let lines_of = |name: &str| name.chars().count().div_ceil(per_line).max(1);
    let mut rows = column![].padding([0, 10]);
    let mut count = 0;
    let mut listed_rows = 0.0;
    for group in groups {
        rows = rows.push(
            container(
                text(match group.name.as_str() {
                    "Favorites" | "Default" => {
                        backcord_iced_kit::i18n::t(group.name.as_str()).to_uppercase()
                    }
                    name => name.to_uppercase(),
                })
                .size(12)
                .font(BOLD)
                .color(theme.muted)
                .wrapping(text::Wrapping::None),
            )
            .height(SOUND_ROW)
            .padding(Padding {
                bottom: 8.0,
                ..Padding::ZERO
            })
            .align_y(Alignment::End),
        );
        count += 1;
        for chunk in group.sounds.chunks(2) {
            let mut line = row![].spacing(6).align_y(Alignment::Center);
            let lines = chunk
                .iter()
                .map(|sound| lines_of(&sound.name))
                .max()
                .unwrap_or(1);
            for sound in chunk {
                let press = note.is_none().then(|| chosen(sound));
                line = line.push(sound_button(ctx, sound, cell, press, extras));
            }
            rows = rows.push(container(line).padding([3, 0]));
            listed_rows += lines as f32 * 18.0 + 18.0 + 6.0;
        }
    }
    let listed = (count.max(1) as f32) * SOUND_ROW + listed_rows + 10.0;
    let room = (SOUNDBOARD_TALLEST - 80.0).max(SOUND_ROW);
    column![
        top,
        // It scrolls with no bar showing.
        scroll_view(rows).barless().height(if listed > room {
            Length::Fixed(room)
        } else {
            Length::Shrink
        })
    ]
    .width(SOUNDBOARD_WIDTH)
    .padding(Padding {
        bottom: 10.0,
        ..Padding::ZERO
    })
    .into()
}

/// A sound to play: its emoji and name; with the pointer on it, a button to
/// hear it first and its star.
fn sound_button<'a>(
    ctx: crate::ctx::Ctx<'a>,
    sound: &'a voice::SoundChoice,
    width: f32,
    press: Option<Message>,
    extras: bool,
) -> Element<'a, Message> {
    let theme = ctx.look.theme;
    let enabled = press.is_some();
    let picture = match &sound.picture {
        Some(url) => ctx.picture(url, 18, 18, false),
        None => emojis::get(&sound.emoji).and_then(|emoji| ctx.emoji_picture(emoji.as_str(), 18)),
    };
    let mut face = row![].spacing(8).align_y(Alignment::Center);
    if let Some(picture) = picture {
        face = face.push(Pic::square(picture, 18.0));
    } else if sound.picture.is_some() {
        face = face.push(Space::new().width(18).height(18));
    }
    face = face.push(
        text(sound.name.as_str())
            .size(14)
            .color(theme.text)
            .wrapping(text::Wrapping::WordOrGlyph)
            .width(Length::Fill),
    );
    let (rest, lit) = (theme.button_default, theme.button_default_hover);
    let opacity = if enabled { 1.0 } else { 0.5 };
    let base = button(
        container(face)
            .align_y(Alignment::Center)
            .clip(true)
            .style(move |_| container::Style {
                text_color: None,
                ..Default::default()
            }),
    )
    .padding([9, 10])
    .width(width)
    .on_press_maybe(press)
    .style(move |_, status| button::Style {
        background: Some(
            crate::style::faded(
                if enabled && controls::lights(status) {
                    lit
                } else {
                    rest
                },
                opacity,
            )
            .into(),
        ),
        border: crate::style::round(8),
        ..Default::default()
    });
    let small = |icon, label: String, tint, ground: Color, message| {
        let face = button(ctx.look.glyph(icon, 15, tint).boxed(26))
            .padding(0)
            .width(26)
            .height(26)
            .on_press(message)
            .style(move |_, _| button::Style {
                background: Some(ground.into()),
                border: crate::style::round(6),
                ..Default::default()
            });
        iced::widget::tooltip(
            face,
            controls::tip(theme, label),
            iced::widget::tooltip::Position::Top,
        )
        .gap(6)
    };
    let star = |ground| {
        let (icon, tint, label) = if sound.favorite {
            (
                Icon::StarFilled,
                crate::style::hex(crate::style::CROWN),
                backcord_iced_kit::i18n::tr("Remove from Favorites"),
            )
        } else {
            (
                Icon::Star,
                theme.secondary,
                backcord_iced_kit::i18n::tr("Add to Favorites"),
            )
        };
        small(
            icon,
            label.to_owned(),
            tint,
            ground,
            Message::Call(Event::Favorite(sound.id.clone(), !sound.favorite)),
        )
    };
    let corner = |content: Element<'a, Message>| {
        container(content)
            .padding([0, 4])
            .width(width)
            .height(Length::Fill)
            .align_x(Alignment::End)
            .align_y(Alignment::Center)
    };
    if !extras {
        return base.into();
    }
    // A favourite's star stays.
    let base: Element<'a, Message> = if sound.favorite {
        iced::widget::stack![base, corner(star(rest).into())].into()
    } else {
        base.into()
    };
    let over = corner(
        row![
            small(
                Icon::Volume,
                backcord_iced_kit::i18n::t(format!("Preview {}", sound.name)),
                theme.text,
                lit,
                Message::Call(Event::Preview(sound.id.clone())),
            ),
            star(lit),
        ]
        .spacing(4)
        .into(),
    );
    mouse_area(Hover::new(base, over)).into()
}

/// Discord's soundboard in the call: the favourites, the call's
/// server's sounds, others', Discord's own, three to a row. A click
/// plays one for everyone; the pointer on one offers hearing it first
/// and starring it. While deafened or server muted, why none plays.
pub(super) fn soundboard<'a>(ctx: crate::ctx::Ctx<'a>) -> Element<'a, Message> {
    let voice = &ctx.state.voice;
    let server_muted = voice
        .participants
        .iter()
        .any(|person| person.local && person.server_muted);
    let note = if voice.deafened {
        Some(backcord_iced_kit::i18n::tr(
            "Soundboard disabled when deafened",
        ))
    } else if server_muted {
        Some(backcord_iced_kit::i18n::tr(
            "Soundboard disabled when server muted",
        ))
    } else {
        None
    };
    let all = voice.soundboard.as_deref().unwrap_or(&[]);
    // The rail's group alone, while it still has sounds.
    let only = ctx
        .call
        .board_group
        .as_deref()
        .and_then(|name| all.iter().position(|group| group.name == name));
    let groups = match only {
        Some(at) => &all[at..=at],
        None => all,
    };
    let board = sound_board(
        ctx,
        backcord_iced_kit::i18n::tr("Soundboard"),
        groups,
        note,
        true,
        |sound| Message::Call(Event::Play(sound.id.clone())),
        Some(volume_button(ctx)),
    );
    row![rail(ctx, all, only.map(|at| all[at].name.as_str())), board].into()
}

/// The soundboard's rail: All, then each group by its picture (the
/// favourites' star, a server's icon, Discord's own), the one shown lit;
/// a click shows that group alone.
fn rail<'a>(
    ctx: crate::ctx::Ctx<'a>,
    groups: &'a [voice::SoundGroup],
    shown: Option<&'a str>,
) -> Element<'a, Message> {
    let theme = ctx.look.theme;
    let lit = theme.button_default_hover;
    let rest = Color::TRANSPARENT;
    let entry = |face: Element<'a, Message>, label: String, on: bool, group: Option<String>| {
        let face = button(container(face).center(RAIL_ICON))
            .padding(0)
            .on_press(Message::Call(Event::BoardGroup(group)))
            .style(move |_, status| button::Style {
                background: Some(
                    if on || controls::lights(status) {
                        lit
                    } else {
                        rest
                    }
                    .into(),
                ),
                border: crate::style::round(10),
                ..Default::default()
            });
        iced::widget::tooltip(
            face,
            controls::tip(theme, label),
            iced::widget::tooltip::Position::Right,
        )
        .gap(6)
    };
    let mut column = column![entry(
        ctx.look.glyph(Icon::Music, 18, theme.text).into(),
        backcord_iced_kit::i18n::tr("All Sounds").to_owned(),
        shown.is_none(),
        None,
    )]
    .spacing(6)
    .align_x(Alignment::Center);
    for group in groups {
        let face: Element<'a, Message> = match &group.server {
            Some(guild) => {
                let place = backcord_core::discord::server_id(guild);
                let server = ctx.state.servers.iter().find(|server| server.id == place);
                match server
                    .and_then(|server| server.icon.as_deref())
                    .and_then(|icon| ctx.picture(icon, 64, 64, true))
                {
                    Some(picture) => Pic::square(picture, 26.0).radius(8.0).into(),
                    None => text(
                        group
                            .name
                            .chars()
                            .next()
                            .map(String::from)
                            .unwrap_or_default(),
                    )
                    .size(14)
                    .font(BOLD)
                    .color(theme.text)
                    .into(),
                }
            }
            None if group.name == "Favorites" => ctx
                .look
                .glyph(Icon::StarFilled, 18, crate::style::hex(crate::style::CROWN))
                .into(),
            None => ctx.look.glyph(Icon::Volume, 18, theme.secondary).into(),
        };
        let label = match group.name.as_str() {
            "Favorites" | "Default" => backcord_iced_kit::i18n::t(group.name.as_str()).clone(),
            name => name.to_owned(),
        };
        column = column.push(entry(
            face,
            label,
            shown == Some(group.name.as_str()),
            Some(group.name.clone()),
        ));
    }
    container(
        scroll_view(column.padding([12, 0]))
            .barless()
            .height(Length::Fixed(SOUNDBOARD_TALLEST - 20.0)),
    )
    .width(BOARD_RAIL)
    .height(Length::Shrink)
    .align_x(Alignment::Center)
    .into()
}

/// The soundboard's volume, top right: a speaker that opens its slider.
fn volume_button<'a>(ctx: crate::ctx::Ctx<'a>) -> Element<'a, Message> {
    let theme = ctx.look.theme;
    let open = ctx.call.board_volume_open;
    let value = ctx.call.board_volume.unwrap_or_else(|| {
        backcord_core::volume::to_slider(ctx.state.soundboard_volume / 100.0).round() as u32
    });
    let icon = if value == 0 {
        Icon::VolumeOff
    } else {
        Icon::Volume
    };
    let lit = theme.button_default_hover;
    let face = button(ctx.look.glyph(icon, 18, theme.text).boxed(30))
        .padding(0)
        .width(30)
        .height(30)
        .on_press(Message::Call(Event::BoardVolumeOpen(!open)))
        .style(move |_, status| button::Style {
            background: (open || controls::lights(status)).then(|| lit.into()),
            border: crate::style::round(8),
            ..Default::default()
        });
    let panel = open.then(|| {
        iced::widget::opaque(
            container(
                controls::slider(
                    backcord_iced_kit::i18n::tr("Soundboard volume"),
                    value,
                    100,
                    |value| Message::Call(Event::BoardVolume(value, false)),
                )
                .mark(false)
                .on_release(Message::Call(Event::BoardVolume(value, true)))
                .typed(|value| Message::Call(Event::BoardVolume(value, true)))
                .view(theme),
            )
            .padding(12)
            .width(Length::Fill)
            .style(controls::menu_panel(theme)),
        )
    });
    crate::popover::Popover::new(face, panel, crate::popover::Side::Below)
        .width(crate::popover::Width::Own(300.0))
        .gap(4.0)
        .outside(Message::Call(Event::BoardVolumeOpen(false)))
        .into()
}
