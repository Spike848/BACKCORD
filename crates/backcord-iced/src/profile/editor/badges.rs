//! The badges' window: this account's badges, each picked to see what it
//! is, hidden or shown again (not Nitro's), and with Nitro moved earlier or
//! later; a preview of the row others see; Save asks before Discord is
//! told. Those it could still earn follow, as Discord's window has them.

use backcord_core::chat::{OwnBadge, OwnBadges};
use iced::{
    Alignment, Color, Element, Length,
    widget::{Space, column, container, mouse_area, opaque, row, stack, text},
};

use crate::{
    app::Message,
    controls::{self, Kind, action, placed},
    fonts::SEMIBOLD,
    pictures::Icon,
    style::{rounded, scrollbar},
    widgets::{Pic, scroll_view},
};

use super::{Event, message};

const WIDTH: f32 = 820.0;
const HEIGHT: f32 = 600.0;
const ROOM: f32 = 32.0;
const SIDE: f32 = 280.0;
const TILE: f32 = 64.0;
const ICON: f32 = 36.0;

/// The badges as arranged in the window, until they're saved.
#[derive(Clone, Debug, Default)]
pub struct Draft {
    /// The badges owned, in the order they'd show; empty until one is moved
    /// or hidden, following Discord's meanwhile.
    pub order: Vec<u32>,
    pub hidden: Vec<u32>,
    pub touched: bool,
    pub selected: Option<u32>,
    pub confirming: bool,
    /// The badge being dragged to a new place, with Nitro.
    pub dragging: Option<u32>,
}

/// What Discord has: the badges owned in order, and those hidden.
fn kept(badges: &[OwnBadge]) -> (Vec<u32>, Vec<u32>) {
    let owned = badges.iter().filter(|badge| badge.owned);
    (
        owned.clone().map(|badge| badge.id).collect(),
        owned
            .filter(|badge| badge.hidden)
            .map(|badge| badge.id)
            .collect(),
    )
}

/// The badges loaded, if they are.
pub(super) fn loaded(ctx: crate::ctx::Ctx<'_>) -> Option<&[OwnBadge]> {
    match &ctx.state.profile_editor.as_ref()?.badges {
        OwnBadges::Loaded(badges) => Some(badges),
        _ => None,
    }
}

impl Draft {
    /// The order and the hidden ones as they stand: the draft's once touched,
    /// else Discord's.
    pub fn current(&self, badges: &[OwnBadge]) -> (Vec<u32>, Vec<u32>) {
        if self.touched {
            (self.order.clone(), self.hidden.clone())
        } else {
            kept(badges)
        }
    }

    /// Takes in Discord's before a first change.
    fn touch(&mut self, badges: &[OwnBadge]) {
        if !self.touched {
            (self.order, self.hidden) = kept(badges);
            self.touched = true;
        }
    }

    pub fn hide(&mut self, badges: &[OwnBadge], id: u32) {
        self.touch(badges);
        if let Some(at) = self.hidden.iter().position(|hidden| *hidden == id) {
            self.hidden.remove(at);
        } else {
            self.hidden.push(id);
        }
    }

    /// The badge dragged comes over `target`: it takes that one's place,
    /// those between moving up one.
    pub fn drag_over(&mut self, badges: &[OwnBadge], target: u32) {
        let Some(dragged) = self.dragging.filter(|dragged| *dragged != target) else {
            return;
        };
        self.touch(badges);
        let (Some(from), Some(to)) = (
            self.order.iter().position(|id| *id == dragged),
            self.order.iter().position(|id| *id == target),
        ) else {
            return;
        };
        let moved = self.order.remove(from);
        self.order.insert(to, moved);
    }

    /// It differs from what Discord has.
    pub fn changed(&self, badges: &[OwnBadge]) -> bool {
        let (order, mut hidden) = self.current(badges);
        let (kept_order, mut kept_hidden) = kept(badges);
        hidden.sort_unstable();
        kept_hidden.sort_unstable();
        order != kept_order || hidden != kept_hidden
    }
}

/// The pictures the window shows.
pub(super) fn pictures(ctx: crate::ctx::Ctx<'_>) -> Vec<String> {
    loaded(ctx)
        .into_iter()
        .flatten()
        .filter_map(|badge| badge.icon.clone())
        .collect()
}

fn icon<'a>(
    ctx: crate::ctx::Ctx<'a>,
    badge: &OwnBadge,
    size: f32,
    faded: bool,
) -> Element<'a, Message> {
    let px = size as u32;
    match badge
        .icon
        .as_deref()
        .and_then(|url| ctx.picture(url, px, px, false))
    {
        Some(picture) => Pic::square(picture, size)
            .opacity(if faded { 0.35 } else { 1.0 })
            .into(),
        None => Space::new().width(size).height(size).into(),
    }
}

/// A badge's tile: pressed, it's picked, and with `drag` starts being
/// dragged; a badge dragged over it takes its place.
fn tile<'a>(
    ctx: crate::ctx::Ctx<'a>,
    badge: &OwnBadge,
    hidden: bool,
    (selected, dragged): (bool, bool),
    drag: bool,
) -> Element<'a, Message> {
    let theme = ctx.look.theme;
    let (ground, accent) = (
        if dragged { theme.hover } else { theme.raised },
        theme.accent,
    );
    let mut face = stack![container(icon(ctx, badge, ICON, hidden)).center(Length::Fill)];
    if hidden {
        face = face.push(
            container(ctx.look.glyph(Icon::EyeOff, 14, theme.text))
                .width(Length::Fill)
                .align_x(Alignment::End)
                .padding(4),
        );
    }
    let face = container(face).width(TILE).height(TILE).style(move |_| {
        let mut style = rounded(ground, 8);
        style.border = iced::Border {
            color: if selected || dragged {
                accent
            } else {
                Color::TRANSPARENT
            },
            width: 2.0,
            radius: 8.0.into(),
        };
        style
    });
    let id = badge.id;
    let mut area = mouse_area(face).on_press(message(Event::BadgeGrab(id, drag)));
    if drag {
        area = area
            .on_enter(message(Event::BadgeOver(id)))
            .interaction(iced::mouse::Interaction::Grab);
    } else {
        area = area.interaction(iced::mouse::Interaction::Pointer);
    }
    area.into()
}

/// The window over the editor.
pub(super) fn view<'a>(ctx: crate::ctx::Ctx<'a>, draft: &'a Draft) -> Element<'a, Message> {
    let theme = ctx.look.theme;
    let tr = backcord_iced_kit::i18n::tr;
    let width = WIDTH.min(ctx.look.window.width - ROOM);
    let height = HEIGHT.min(ctx.look.window.height - ROOM);
    let label = |words: &'static str| {
        text(tr(words))
            .size(13)
            .font(SEMIBOLD)
            .color(theme.secondary)
    };
    let note = |words: &'static str| text(tr(words)).size(13).color(theme.muted);
    let core = ctx.state.profile_editor.as_ref();
    let nitro = core.is_some_and(|core| core.premium > 0);
    let busy = core.is_some_and(|core| core.arranging);
    let mut body = column![].spacing(12);
    let mut side = column![].spacing(10);
    let badges: &[OwnBadge] = match core.map(|core| &core.badges) {
        Some(OwnBadges::Loaded(badges)) => badges,
        Some(OwnBadges::Failed(error)) => {
            body = body.push(text(error.as_str()).size(13).color(theme.danger));
            &[]
        }
        _ => {
            body = body.push(note("Loading..."));
            &[]
        }
    };
    let (order, hidden) = draft.current(badges);
    let by_id = |id: u32| badges.iter().find(|badge| badge.id == id);
    if !badges.is_empty() {
        body = body.push(label("Your badges"));
        body = body.push(note(if nitro {
            "Drag the badges to reorder them; pick one to hide it."
        } else {
            "Pick a badge to hide it. With Nitro you can also reorder them."
        }));
        let mut mine = row![].spacing(8);
        for badge in order.iter().filter_map(|id| by_id(*id)) {
            mine = mine.push(tile(
                ctx,
                badge,
                hidden.contains(&badge.id),
                (
                    draft.selected == Some(badge.id),
                    draft.dragging == Some(badge.id),
                ),
                nitro && !busy,
            ));
        }
        body = body.push(mine.wrap().vertical_spacing(8));
        let earnable: Vec<&OwnBadge> = badges
            .iter()
            .filter(|badge| !badge.owned && badge.earnable)
            .collect();
        if !earnable.is_empty() {
            body = body.push(label("Available to unlock"));
            let mut more = row![].spacing(8);
            for badge in earnable {
                more = more.push(tile(
                    ctx,
                    badge,
                    true,
                    (draft.selected == Some(badge.id), false),
                    false,
                ));
            }
            body = body.push(more.wrap().vertical_spacing(8));
        }
        // The row others see: those shown, in order.
        body = body.push(label("What others see"));
        let mut seen = row![].spacing(6).align_y(Alignment::Center);
        for badge in order
            .iter()
            .filter(|id| !hidden.contains(id))
            .filter_map(|id| by_id(*id))
        {
            seen = seen.push(icon(ctx, badge, 22.0, false));
        }
        body = body.push(
            container(seen.wrap())
                .padding(10)
                .width(Length::Fill)
                .style({
                    let sunken = theme.sunken;
                    move |_| rounded(sunken, 8)
                }),
        );
    }
    // The badge picked, and what can be done with it.
    match draft.selected.and_then(by_id) {
        Some(badge) => {
            let is_hidden = hidden.contains(&badge.id);
            side = side.push(
                container(icon(ctx, badge, 96.0, !badge.owned))
                    .width(Length::Fill)
                    .center_x(Length::Fill),
            );
            side = side.push(
                text(badge.name.as_str())
                    .size(18)
                    .font(SEMIBOLD)
                    .color(theme.text)
                    .width(Length::Fill)
                    .align_x(Alignment::Center),
            );
            if !badge.info.is_empty() {
                side = side.push(
                    text(badge.info.as_str())
                        .size(13)
                        .color(theme.secondary)
                        .width(Length::Fill)
                        .align_x(Alignment::Center),
                );
            }
            if !badge.description.is_empty() {
                side = side.push(text(badge.description.as_str()).size(13).color(theme.text));
            }
            if badge.owned {
                side = side.push(
                    action(
                        theme,
                        tr(if is_hidden { "Show" } else { "Hide" }),
                        Kind::Plain,
                        (!busy).then(|| message(Event::BadgeHide(badge.id))),
                    )
                    .wide(36.0),
                );
            }
        }
        None if !badges.is_empty() => side = side.push(note("Pick a badge to see it.")),
        None => {}
    }
    side = side.push(Space::new().height(Length::Fill));
    match core.and_then(|core| core.arranged.as_ref()) {
        Some(Err(error)) => side = side.push(text(error.as_str()).size(13).color(theme.danger)),
        Some(Ok(())) if !draft.changed(badges) => {
            side = side.push(note("Saved on your profile."));
        }
        _ => {}
    }
    let changed = !badges.is_empty() && draft.changed(badges);
    side = side.push(
        action(
            theme,
            tr(if busy { "Saving..." } else { "Save" }),
            Kind::Primary,
            (changed && !busy).then(|| message(Event::BadgeConfirm(true))),
        )
        .wide(38.0),
    );
    let head = row![
        text(tr("Badges"))
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
            tr("Close"),
            Kind::Plain,
            Some(message(Event::Badges(false)))
        ),
    ]
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
        row![
            list,
            container(side)
                .width(SIDE)
                .height(Length::Fill)
                .padding([0, 20])
        ]
        .height(Length::Fill),
        controls::bar(theme.line, Length::Fill, 1.0),
        foot,
    ])
    .width(width)
    .height(height)
    .clip(true)
    .style(move |_| rounded(ground, 16).shadow(crate::style::window_shadow()));
    // The frosted window under its see-through ground, as the dialogs'.
    let card = backcord_iced_kit::widgets::frost::Frost::new([16.0; 4], card);
    let close = ctx
        .look
        .icon_button(Icon::X, tr("Close"), message(Event::Badges(false)))
        .size(32.0, 20)
        .below();
    let mut card = stack![card, placed(close, width - 32.0 - 12.0, 12.0)];
    if draft.confirming {
        card = card.push(confirmation(ctx, width, height));
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
    .on_press(message(Event::Badges(false)));
    // Letting go anywhere ends a drag.
    let window = mouse_area(container(opaque(card)).center(Length::Fill))
        .on_release(message(Event::BadgeDrop));
    opaque(stack![opaque(backdrop), window])
}

/// Asks, over the window, whether to save the badges as arranged.
fn confirmation<'a>(ctx: crate::ctx::Ctx<'a>, width: f32, height: f32) -> Element<'a, Message> {
    let theme = ctx.look.theme;
    let tr = backcord_iced_kit::i18n::tr;
    let box_ = container(
        column![
            text(tr("Change your profile"))
                .size(18)
                .font(SEMIBOLD)
                .color(theme.text),
            text(tr("Save how your badges show?"))
                .size(14)
                .color(theme.text),
            text(tr("Discord saves it at once, and everyone sees it."))
                .size(13)
                .color(theme.muted),
            row![
                Space::new().width(Length::Fill),
                action(
                    theme,
                    tr("Cancel"),
                    Kind::Plain,
                    Some(message(Event::BadgeConfirm(false)))
                ),
                action(
                    theme,
                    tr("Confirm"),
                    Kind::Primary,
                    Some(message(Event::BadgeSave))
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
        .on_press(message(Event::BadgeConfirm(false))),
        container(opaque(box_))
            .width(width)
            .height(height)
            .center(Length::Fill),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn badge(id: u32, hidden: bool) -> OwnBadge {
        OwnBadge {
            id,
            name: String::new(),
            description: String::new(),
            info: String::new(),
            icon: None,
            owned: true,
            hidden,
            earnable: true,
        }
    }

    #[test]
    fn a_draft_starts_from_discords_and_says_when_it_differs() {
        let badges = [badge(1, false), badge(18, false), badge(12, true)];
        let mut draft = Draft::default();
        assert_eq!(draft.current(&badges), (vec![1, 18, 12], vec![12]));
        assert!(!draft.changed(&badges));
        draft.hide(&badges, 1);
        assert_eq!(draft.current(&badges).1, [12, 1], "Nitro's hides too");
        draft.hide(&badges, 1);
        assert!(!draft.changed(&badges));
        // Dragging 12 over 1 puts it first; over itself does nothing.
        draft.dragging = Some(12);
        draft.drag_over(&badges, 12);
        assert!(!draft.changed(&badges));
        draft.drag_over(&badges, 1);
        assert_eq!(draft.current(&badges).0, [12, 1, 18]);
        // Then on over 18 it goes last again.
        draft.drag_over(&badges, 18);
        assert_eq!(draft.current(&badges).0, [1, 18, 12]);
        assert!(!draft.changed(&badges), "back as Discord has it");
    }
}
