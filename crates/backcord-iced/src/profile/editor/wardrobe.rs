//! The choice of a collectible for the main profile: the avatar decoration,
//! profile effect, nameplate or profile frame this account owns, and the
//! shop's. One clicked is chosen, and the preview beside the list, and the
//! editor's card, wear it; Apply asks before Discord is told. One of the
//! shop's opens its page in Discord's shop to buy.

use backcord_core::chat::{Collectible, CollectibleKind, Wardrobe};
use iced::{
    Alignment, Color, Element, Length, Point, Size,
    widget::{Space, button, column, container, mouse_area, opaque, row, stack, text},
};

use crate::{
    app::Message,
    controls::{self, Kind, action, placed},
    fonts::{BOLD, SEMIBOLD},
    pictures::Icon,
    profile::{Mode, Shown},
    style::{rounded, scrollbar},
    widgets::{Pic, PlateRow, scroll_view},
};

use super::{Event, SHOP, message};

const WIDTH: f32 = 900.0;
const HEIGHT: f32 = 620.0;
const ROOM: f32 = 32.0;
/// The preview's column, beside the list.
const PREVIEW: f32 = 280.0;
const TILE: f32 = 104.0;
/// Two lines of a tile's name.
const LABEL: f32 = 27.0;
/// The shop's items shown at first, and how many more each Show More adds:
/// a frame's pictures are large, so they come a page at a time.
pub(super) const SHOP_PAGE: usize = 48;

/// The window's title for a kind.
pub(super) fn title(kind: CollectibleKind) -> &'static str {
    backcord_iced_kit::i18n::tr(match kind {
        CollectibleKind::Decoration => "Avatar Decoration",
        CollectibleKind::Effect => "Profile Effect",
        CollectibleKind::Nameplate => "Nameplate",
        CollectibleKind::Frame => "Profile Frame",
    })
}

/// The address of an item's page in Discord's shop.
fn shop_page(item: &Collectible) -> String {
    format!("{SHOP}#itemSkuId={}", item.product)
}

/// The items of a kind, owned and the first `shown` of the shop's not
/// owned, and how many the shop has of them.
fn items(
    wardrobe: &Wardrobe,
    kind: CollectibleKind,
    shown: usize,
) -> (Vec<&Collectible>, Vec<&Collectible>, usize) {
    let Wardrobe::Loaded { owned, shop } = wardrobe else {
        return (Vec::new(), Vec::new(), 0);
    };
    let owned: Vec<&Collectible> = owned.iter().filter(|item| item.kind == kind).collect();
    let sold: Vec<&Collectible> = shop
        .iter()
        .filter(|item| item.kind == kind && !owned.iter().any(|mine| mine.sku == item.sku))
        .collect();
    let total = sold.len();
    (owned, sold.into_iter().take(shown).collect(), total)
}

/// The pictures the open choice shows.
pub(super) fn pictures(
    ctx: crate::ctx::Ctx<'_>,
    kind: CollectibleKind,
    shown: usize,
) -> Vec<String> {
    let Some(core) = ctx.state.profile_editor.as_ref() else {
        return Vec::new();
    };
    let (owned, shop, _) = items(&core.wardrobe, kind, shown);
    owned
        .into_iter()
        .chain(shop)
        .flat_map(|item| {
            // A frame is drawn whole, from its layers.
            if item.frame.is_empty() {
                item.picture.clone().into_iter().collect::<Vec<_>>()
            } else {
                item.frame.iter().map(|layer| layer.src.clone()).collect()
            }
        })
        .collect()
}

/// A tile: ringed in the accent when chosen, in the text's colour when worn.
fn tile<'a>(
    ctx: crate::ctx::Ctx<'a>,
    face: Element<'a, Message>,
    label: &'a str,
    (chosen, worn): (bool, bool),
    press: Option<Message>,
) -> Element<'a, Message> {
    let theme = ctx.look.theme;
    let (ground, hover, accent, line) = (theme.raised, theme.hover, theme.accent, theme.muted);
    button(
        column![
            container(face)
                .width(Length::Fill)
                .height(TILE - 30.0)
                .center(Length::Fill)
                .clip(true),
            // A long name wraps to a second line and is cut there, inside
            // its tile.
            container(
                text(label)
                    .size(11)
                    .line_height(1.2)
                    .color(theme.text)
                    .wrapping(text::Wrapping::WordOrGlyph)
                    .width(Length::Fill)
                    .align_x(Alignment::Center),
            )
            .width(Length::Fill)
            .height(LABEL)
            .clip(true),
        ]
        .spacing(4)
        .padding(6),
    )
    .width(TILE)
    .height(TILE - 30.0 + 4.0 + LABEL + 12.0)
    .clip(true)
    .padding(0)
    .on_press_maybe(press)
    .style(move |_, status| {
        let fill = match status {
            button::Status::Hovered | button::Status::Pressed => hover,
            _ => ground,
        };
        let mut style = button::Style::default().with_background(fill);
        style.border = iced::Border {
            color: if chosen {
                accent
            } else if worn {
                line
            } else {
                Color::TRANSPARENT
            },
            width: 2.0,
            radius: 8.0.into(),
        };
        style
    })
    .into()
}

fn picture<'a>(ctx: crate::ctx::Ctx<'a>, item: &'a Collectible) -> Element<'a, Message> {
    let blank = || Space::new().width(TILE - 36.0).height(TILE - 36.0).into();
    if !item.frame.is_empty() {
        return crate::profile::cosmetic::frame_preview(
            ctx,
            &item.frame,
            Size::new(TILE - 12.0, TILE - 30.0),
        )
        .unwrap_or_else(blank);
    }
    let size = (TILE - 36.0) as u32;
    match item
        .picture
        .as_deref()
        .and_then(|url| ctx.picture(url, size, size, false))
    {
        Some(still) => Pic::square(still, TILE - 36.0).into(),
        None => blank(),
    }
}

/// The item chosen of a kind, from what's loaded.
fn chosen_item<'a>(wardrobe: &'a Wardrobe, sku: &str) -> Option<&'a Collectible> {
    let Wardrobe::Loaded { owned, shop } = wardrobe else {
        return None;
    };
    owned.iter().chain(shop).find(|item| item.sku == sku)
}

/// The preview's picture: the editor's card wearing what's chosen, as the
/// styles' tiles draw it, larger.
fn preview_face<'a>(
    ctx: crate::ctx::Ctx<'a>,
    shown: &'a Shown,
    kind: CollectibleKind,
    plate: Option<&'a str>,
) -> Element<'a, Message> {
    let theme = ctx.look.theme;
    let profile = shown.look.profile();
    let none = || {
        text(backcord_iced_kit::i18n::tr("None"))
            .size(13)
            .color(theme.muted)
            .into()
    };
    match kind {
        CollectibleKind::Decoration => crate::profile::body::profile_avatar(
            ctx,
            shown,
            120.0,
            false,
            Color::TRANSPARENT,
            Mode::Card,
        ),
        CollectibleKind::Effect => {
            crate::profile::cosmetic::effect_tile(ctx, shown, Size::new(PREVIEW - 40.0, 300.0))
                .unwrap_or_else(|| {
                    if profile.effect.is_some() {
                        text(backcord_iced_kit::i18n::tr("Loading..."))
                            .size(13)
                            .color(theme.muted)
                            .into()
                    } else {
                        none()
                    }
                })
        }
        CollectibleKind::Frame => {
            crate::profile::cosmetic::frame_tile(ctx, shown, Size::new(PREVIEW - 40.0, 300.0))
                .unwrap_or_else(none)
        }
        CollectibleKind::Nameplate => {
            let face = row![
                ctx.avatar(profile.avatar.as_deref(), &profile.name, 35.0, 17.5, 1.0),
                text(profile.name.as_str())
                    .size(15)
                    .font(BOLD)
                    .color(theme.text)
                    .wrapping(text::Wrapping::None),
                Space::new().width(Length::Fill),
            ]
            .spacing(8)
            .padding([0, 8])
            .height(48)
            .align_y(Alignment::Center);
            let strip = plate
                .and_then(|url| crate::members::plate_picture(ctx, url, 48.0, 8.0, 0.6, false));
            container(PlateRow::new(face, strip, None, theme.raised, 0.0, 1.0).account())
                .width(PREVIEW - 40.0)
                .clip(true)
                .into()
        }
    }
}

/// The choice's window over the editor.
pub(super) fn view<'a>(
    ctx: crate::ctx::Ctx<'a>,
    shown: &'a Shown,
    kind: CollectibleKind,
    worn: Option<String>,
    shop_shown: usize,
    confirming: bool,
) -> Element<'a, Message> {
    let theme = ctx.look.theme;
    let width = WIDTH.min(ctx.look.window.width - ROOM);
    let height = HEIGHT.min(ctx.look.window.height - ROOM);
    let label = |words: &'static str| {
        text(backcord_iced_kit::i18n::tr(words))
            .size(13)
            .font(SEMIBOLD)
            .color(theme.secondary)
    };
    let note = |words: &'static str| {
        text(backcord_iced_kit::i18n::tr(words))
            .size(13)
            .color(theme.muted)
    };
    let core = ctx.state.profile_editor.as_ref();
    let wardrobe = core.map_or(&Wardrobe::Unasked, |core| &core.wardrobe);
    let busy = core.is_some_and(|core| core.wearing.is_some());
    // What's chosen of this kind: none when nothing is, else the SKU or
    // none to take it off.
    let chosen: Option<Option<&str>> = core
        .and_then(|core| core.chosen.as_ref())
        .filter(|(chosen_kind, _)| *chosen_kind == kind)
        .map(|(_, sku)| sku.as_deref());
    let worn = worn.as_deref();
    // What the tiles ring: the one chosen, else the one worn.
    let picked = chosen.unwrap_or(worn);
    let mut body = column![].spacing(12);
    match wardrobe {
        Wardrobe::Loaded { .. } => {
            let (owned, shop, total) = items(wardrobe, kind, shop_shown);
            body = body.push(label("Yours"));
            let mut mine = row![tile(
                ctx,
                ctx.look.glyph(Icon::X, 26, theme.muted).into(),
                backcord_iced_kit::i18n::tr("None"),
                (picked.is_none(), worn.is_none()),
                Some(message(Event::Choose(kind, None))),
            )]
            .spacing(8);
            for item in &owned {
                let sku = Some(item.sku.as_str());
                mine = mine.push(tile(
                    ctx,
                    picture(ctx, item),
                    &item.name,
                    (picked == sku, worn == sku),
                    Some(message(Event::Choose(kind, Some(item.sku.clone())))),
                ));
            }
            body = body.push(mine.wrap().vertical_spacing(8));
            if owned.is_empty() {
                body = body.push(note("You don't own any yet."));
            }
            body = body.push(label("In the shop"));
            let mut sold = row![].spacing(8);
            for item in shop {
                let sku = Some(item.sku.as_str());
                sold = sold.push(tile(
                    ctx,
                    picture(ctx, item),
                    &item.name,
                    (picked == sku, false),
                    Some(message(Event::Choose(kind, Some(item.sku.clone())))),
                ));
            }
            body = body.push(sold.wrap().vertical_spacing(8));
            if total > shop_shown {
                body = body.push(
                    container(action(
                        theme,
                        format!(
                            "{} ({})",
                            backcord_iced_kit::i18n::tr("Show More"),
                            total - shop_shown
                        ),
                        Kind::Plain,
                        Some(message(Event::ShopMore)),
                    ))
                    .center_x(Length::Fill),
                );
            }
        }
        Wardrobe::Failed(error) => {
            body = body.push(text(error.as_str()).size(13).color(theme.danger));
        }
        _ => body = body.push(note("Loading...")),
    }
    // The preview, and what can be done with what's chosen.
    let item = chosen.flatten().and_then(|sku| chosen_item(wardrobe, sku));
    let owned = |sku: &str| {
        matches!(wardrobe, Wardrobe::Loaded { owned, .. }
            if owned.iter().any(|mine| mine.kind == kind && mine.sku == sku))
    };
    let plate = match chosen {
        Some(Some(_)) => item.and_then(|item| item.picture.as_deref()),
        Some(None) => None,
        None => ctx
            .state
            .account_nameplate
            .as_ref()
            .map(|plate| &*plate.picture),
    };
    let name: String = match chosen {
        Some(Some(_)) => item.map_or_else(String::new, |item| item.name.clone()),
        Some(None) => backcord_iced_kit::i18n::tr("None").to_owned(),
        None => String::new(),
    };
    let mut side = column![
        label("Preview"),
        container(preview_face(ctx, shown, kind, plate))
            .width(Length::Fill)
            .height(320)
            .center(Length::Fill)
            .clip(true)
            .style({
                let sunken = theme.sunken;
                move |_| rounded(sunken, 8)
            }),
        text(name)
            .size(15)
            .font(SEMIBOLD)
            .color(theme.text)
            .wrapping(text::Wrapping::WordOrGlyph),
    ]
    .spacing(10);
    if kind == CollectibleKind::Effect && core.is_some_and(|core| !core.nitro()) {
        side = side.push(note("Discord lets only Nitro wear a profile effect."));
    }
    // Apply for one owned, or None, that isn't what's worn; Buy for the
    // shop's.
    let can_wear = !busy
        && match chosen {
            Some(Some(sku)) => owned(sku) && worn != Some(sku),
            Some(None) => worn.is_some(),
            None => false,
        };
    let buy = match (chosen, item) {
        (Some(Some(sku)), Some(item)) if !owned(sku) => Some(item),
        _ => None,
    };
    side = side.push(match buy {
        Some(item) => action(
            theme,
            backcord_iced_kit::i18n::tr("Buy on Discord"),
            Kind::Primary,
            Some(Message::Link(shop_page(item), Point::ORIGIN)),
        )
        .wide(38.0),
        None => action(
            theme,
            if busy {
                backcord_iced_kit::i18n::tr("Saving...")
            } else {
                backcord_iced_kit::i18n::tr("Apply")
            },
            Kind::Primary,
            can_wear.then(|| message(Event::Confirm(true))),
        )
        .wide(38.0),
    });
    if let Some(error) = core.and_then(|core| core.wear_error.as_deref()) {
        side = side.push(text(error).size(13).color(theme.danger));
    } else if core.is_some_and(|core| core.wear_saved) {
        side = side.push(
            text(backcord_iced_kit::i18n::tr("Saved on your profile."))
                .size(13)
                .color(theme.secondary),
        );
    }
    let head = row![
        text(title(kind))
            .size(18)
            .font(SEMIBOLD)
            .color(theme.text)
            .width(Length::Fill),
    ]
    .padding([16, 20]);
    let foot = row![
        Space::new().width(Length::Fill),
        action(
            theme,
            backcord_iced_kit::i18n::tr("Open the shop"),
            Kind::Plain,
            Some(Message::Link(SHOP.into(), Point::ORIGIN))
        ),
        action(
            theme,
            backcord_iced_kit::i18n::tr("Close"),
            Kind::Plain,
            Some(message(Event::Wardrobe(None)))
        ),
    ]
    .spacing(8)
    .padding(16);
    let list = container(
        scroll_view(container(body).padding([0, 20]))
            .height(Length::Fill)
            .style(scrollbar(theme)),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .clip(true);
    let ground = theme.sidebar;
    let card = container(column![
        head,
        row![list, container(side).width(PREVIEW).padding([0, 20]),].height(Length::Fill),
        controls::bar(theme.line, Length::Fill, 1.0),
        foot,
    ])
    .width(width)
    .height(height)
    .clip(true)
    .style(move |_| rounded(ground, 16).shadow(crate::style::window_shadow()));
    let close = ctx
        .look
        .icon_button(
            Icon::X,
            backcord_iced_kit::i18n::tr("Close"),
            message(Event::Wardrobe(None)),
        )
        .size(32.0, 20)
        .below();
    let mut card = stack![card, placed(close, width - 32.0 - 12.0, 12.0)];
    if confirming && let Some(chosen) = chosen {
        card = card.push(confirmation(ctx, kind, chosen, item, width, height));
    }
    let dim = Color {
        a: 0.5,
        ..Color::BLACK
    };
    let backdrop = mouse_area(
        container(Space::new())
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| crate::style::fill(dim)),
    )
    .on_press(message(Event::Wardrobe(None)));
    opaque(stack![
        opaque(backdrop),
        container(opaque(card)).center(Length::Fill)
    ])
}

/// Asks, over the window, whether to wear what's chosen, or take its kind
/// off.
fn confirmation<'a>(
    ctx: crate::ctx::Ctx<'a>,
    kind: CollectibleKind,
    chosen: Option<&'a str>,
    item: Option<&'a Collectible>,
    width: f32,
    height: f32,
) -> Element<'a, Message> {
    let theme = ctx.look.theme;
    let what = title(kind);
    let question = match item {
        Some(item) => format!(
            "{} «{}» {}",
            backcord_iced_kit::i18n::tr("Wear"),
            item.name,
            backcord_iced_kit::i18n::tr("on your profile?")
        ),
        None => format!(
            "{} {}",
            backcord_iced_kit::i18n::tr("Take off the profile's"),
            what.to_lowercase()
        ),
    };
    let box_ = container(
        column![
            text(backcord_iced_kit::i18n::tr("Change your profile"))
                .size(18)
                .font(SEMIBOLD)
                .color(theme.text),
            text(question).size(14).color(theme.text),
            text(backcord_iced_kit::i18n::tr(
                "Discord saves it at once, and everyone sees it."
            ))
            .size(13)
            .color(theme.muted),
            row![
                Space::new().width(Length::Fill),
                action(
                    theme,
                    backcord_iced_kit::i18n::tr("Cancel"),
                    Kind::Plain,
                    Some(message(Event::Confirm(false)))
                ),
                action(
                    theme,
                    backcord_iced_kit::i18n::tr("Confirm"),
                    Kind::Primary,
                    Some(message(Event::Wear(kind, chosen.map(str::to_owned))))
                ),
            ]
            .spacing(8),
        ]
        .spacing(12)
        .padding(20)
        .width(400),
    )
    .style({
        let ground = theme.panel;
        move |_| rounded(ground, 12).shadow(crate::style::window_shadow())
    });
    let dim = Color {
        a: 0.5,
        ..Color::BLACK
    };
    opaque(stack![
        mouse_area(
            container(Space::new())
                .width(width)
                .height(height)
                .style(move |_| rounded(dim, 16))
        )
        .on_press(message(Event::Confirm(false))),
        container(opaque(box_))
            .width(width)
            .height(height)
            .center(Length::Fill),
    ])
}
