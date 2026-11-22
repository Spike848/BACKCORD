//! The call off screen and over it: the stream watched, this account's
//! own or someone's camera kept over the window while the call isn't on
//! screen (a click returns to the call, a drag moves it anywhere in the
//! window, clear of a voice channel's Join, and its edges resize it, its
//! shape kept); and the call's large tile over the whole window, full
//! screen.

use backcord_core::voice::SharePhase;
use iced::{
    Alignment, Background, Color, Element, Length, Padding, Point, Rectangle, Size, Vector,
    gradient,
    widget::{Space, column, container, mouse_area, pin, stack, text},
};

use super::{
    Event,
    stage::{CORNER, OWN_SCREEN, Pip, share_status},
};
use crate::{
    app::{App, Message},
    controls,
    fonts::BOLD,
    header,
    pictures::Icon,
    tile::{self, GLASS, NOTE},
    viewer::Viewer,
    widgets::Hover,
};

/// From the window's edges, and the corner's own corners.
const MARGIN: f32 = 16.0;
const RADIUS: f32 = 8.0;
/// How far apart it keeps from what it mustn't cover.
const GAP: f32 = 12.0;
/// Its narrowest; its widest is what the window holds.
const NARROWEST: f32 = 192.0;
/// How thick its edges and corners are to grab for a resize.
const EDGE: f32 = 6.0;
const GRIP: f32 = 14.0;

/// Which of the picture's edges a resize holds: a corner holds two.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Edges {
    pub left: bool,
    pub right: bool,
    pub top: bool,
    pub bottom: bool,
}

/// Where the picture may go: the window right of the channel list and
/// under the header, inside its margin.
fn area(app: &App) -> Rectangle {
    let left = crate::conversation::chat_left(app) + MARGIN;
    let top = header::HEIGHT + MARGIN;
    Rectangle::new(
        Point::new(left, top),
        Size::new(
            (app.window.width - MARGIN - left).max(0.0),
            (app.window.height - MARGIN - top).max(0.0),
        ),
    )
}

/// The picture's width and height to width, kept as it's resized.
fn aspect() -> f32 {
    CORNER.width / CORNER.height
}

/// `rect` as large as `area` lets it be, keeping its shape, and inside it.
fn fit_in(rect: Rectangle, area: Rectangle) -> Rectangle {
    let widest = area.width.min(area.height * aspect());
    let width = rect.width.min(widest).max(NARROWEST.min(widest));
    let height = width / aspect();
    let x = rect.x.min(area.x + area.width - width).max(area.x);
    let y = rect.y.min(area.y + area.height - height).max(area.y);
    Rectangle::new(Point::new(x, y), Size::new(width, height))
}

/// `rect` with `edges` pulled `by`, its shape kept: the edge pulled further
/// sets the size, and the edges across from those held stay.
pub(super) fn resized(rect: Rectangle, edges: Edges, by: Vector) -> Rectangle {
    let across = if edges.right {
        by.x
    } else if edges.left {
        -by.x
    } else {
        0.0
    };
    let down = if edges.bottom {
        by.y
    } else if edges.top {
        -by.y
    } else {
        0.0
    } * aspect();
    let grown = if across.abs() >= down.abs() {
        across
    } else {
        down
    };
    let width = (rect.width + grown).max(NARROWEST);
    let height = width / aspect();
    let x = if edges.left {
        rect.x + rect.width - width
    } else {
        rect.x
    };
    let y = if edges.top {
        rect.y + rect.height - height
    } else {
        rect.y
    };
    Rectangle::new(Point::new(x, y), Size::new(width, height))
}

/// `rect` moved the least way clear of `obstacle` that stays in `area`;
/// as it was when it's clear already, or nowhere is.
pub(super) fn clear_of(rect: Rectangle, obstacle: Rectangle, area: Rectangle) -> Rectangle {
    let overlaps = obstacle.width > 0.0
        && obstacle.height > 0.0
        && rect.x < obstacle.x + obstacle.width + GAP
        && obstacle.x < rect.x + rect.width + GAP
        && rect.y < obstacle.y + obstacle.height + GAP
        && obstacle.y < rect.y + rect.height + GAP;
    if !overlaps {
        return rect;
    }
    let at = |x: f32, y: f32| Rectangle::new(Point::new(x, y), rect.size());
    let inside = |moved: &Rectangle| {
        moved.x >= area.x - 0.5
            && moved.y >= area.y - 0.5
            && moved.x + moved.width <= area.x + area.width + 0.5
            && moved.y + moved.height <= area.y + area.height + 0.5
    };
    [
        at(obstacle.x - GAP - rect.width, rect.y),
        at(obstacle.x + obstacle.width + GAP, rect.y),
        at(rect.x, obstacle.y - GAP - rect.height),
        at(rect.x, obstacle.y + obstacle.height + GAP),
    ]
    .into_iter()
    .filter(inside)
    .min_by(|a, b| {
        let moved = |to: &Rectangle| (to.x - rect.x).abs() + (to.y - rect.y).abs();
        moved(a).total_cmp(&moved(b))
    })
    .unwrap_or(rect)
}

/// Where the picture is and how large: where it was left (at first the
/// top right, at its first size), moved or resized as far as it's
/// dragged, inside the window.
pub fn placed(app: &App) -> Rectangle {
    let area = area(app);
    let left = app.call.pip.unwrap_or_else(|| {
        Rectangle::new(
            Point::new(area.x + area.width - CORNER.width, area.y),
            CORNER,
        )
    });
    let by = app.call.drag.unwrap_or_default();
    let moved = match app.call.resize {
        Some(edges) => resized(left, edges, by),
        None => left + by,
    };
    fit_in(moved, area)
}

/// The picture let go after a drag or a resize: it stays there, moved
/// clear of what the view under it keeps clear (a voice channel's Join and
/// who is in it).
pub(super) fn drop_corner(app: &mut App) {
    let area = area(app);
    let placed = clear_of(placed(app), app.call.keep_clear.get(), area);
    app.call.pip = Some(placed);
    app.call.drag = None;
    app.call.resize = None;
}

/// What the picture keeps clear of, as it's drawn: the bounds of `child`.
pub fn keep_clear<'a>(
    app: &'a App,
    child: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    crate::widgets::Wrap::new(child, KeptClear(&app.call.keep_clear)).into()
}

struct KeptClear<'a>(&'a std::cell::Cell<Rectangle>);

impl crate::widgets::Hooks<Message> for KeptClear<'_> {
    type State = crate::widgets::Stateless;

    fn draw(
        &self,
        _: &crate::widgets::Stateless,
        renderer: &mut iced::Renderer,
        at: crate::widgets::At,
        child: &mut dyn FnMut(&mut iced::Renderer),
    ) {
        self.0.set(at.bounds);
        child(renderer);
    }
}

/// The picture in its corner, while the call is off screen.
pub fn corner_view(app: &App) -> Option<Element<'_, Message>> {
    let pip = crate::call::stage::corner(app);
    let voice = &app.client.state().voice;
    let (title, status, key, live, stop, viewers) = match pip {
        Pip::None => return None,
        Pip::Watched => {
            let user = crate::call::stage::corner_watched(app)?;
            let watching = voice
                .watching
                .iter()
                .find(|watching| watching.user == user)?;
            (
                backcord_iced_kit::i18n::t(format!("{}'s screen", watching.name)),
                crate::call::stage::watch_status(app, &watching.user),
                format!("stream:{}", watching.user),
                false,
                Some((
                    Icon::X,
                    backcord_iced_kit::i18n::tr("Stop watching"),
                    Event::StopWatching(Some(watching.user.clone())),
                )),
                crate::call::stage::stream_viewers(app, &watching.viewers, Some(&watching.user)),
            )
        }
        Pip::Shared => {
            let sharing = voice.sharing.as_ref()?;
            (
                backcord_iced_kit::i18n::tr("Your screen").to_owned(),
                share_status(voice).to_owned(),
                OWN_SCREEN.to_owned(),
                sharing.phase == SharePhase::Live,
                Some((
                    Icon::ScreenShareOff,
                    backcord_iced_kit::i18n::tr("Stop sharing"),
                    Event::StopSharing,
                )),
                crate::call::stage::stream_viewers(app, &sharing.viewers, None),
            )
        }
        Pip::Camera(index) => {
            let person = voice.participants.get(index)?;
            (
                person.user.clone(),
                String::new(),
                person.id.clone(),
                false,
                None,
                None,
            )
        }
    };
    let theme = &app.theme;
    let content = crate::call::stage::frame(app, &key, None).unwrap_or_else(|| {
        container(text(status).size(14).color(NOTE).align_x(Alignment::Center))
            .padding([0, 16])
            .center(Length::Fill)
            .into()
    });
    let at = placed(app);
    let (width, height) = (at.width, at.height);
    let mut base = stack![content].width(width).height(height);
    if live {
        base = base.push(pin(controls::live_pill(theme, None)).x(8).y(8));
    }
    if let Some(viewers) = viewers {
        base = base.push(
            container(viewers)
                .padding(8)
                .width(Length::Fill)
                .align_x(Alignment::End),
        );
    }
    // Its title over a shade along its foot, and its stop, while the
    // pointer is on it and it isn't dragged.
    let shade = Background::Gradient(
        gradient::Linear::new(std::f32::consts::PI)
            .add_stop(0.55, Color::TRANSPARENT)
            .add_stop(1.0, tile::SHADE)
            .into(),
    );
    let mut over = stack![
        container(Space::new())
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| container::Style {
                background: Some(shade),
                ..Default::default()
            }),
        pin(container(
            text(title)
                .size(14)
                .font(BOLD)
                .color(Color::WHITE)
                .wrapping(text::Wrapping::None)
        )
        .width(width - 72.0)
        .height(24)
        .clip(true)
        .align_y(Alignment::Center))
        .x(12)
        .y(height - 34.0),
    ]
    .width(width)
    .height(height);
    if let Some((icon, label, event)) = stop {
        over = over.push(
            container(tile::glass_button(
                app.look(),
                icon,
                label,
                Message::Call(event),
            ))
            .padding(8)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Alignment::End)
            .align_y(Alignment::End),
        );
    }
    let dragging = app.call.drag.is_some();
    let face: Element<'_, Message> = if dragging {
        base.into()
    } else {
        Hover::new(base, over).into()
    };
    let edge = theme.edge;
    let framed = container(face)
        .width(width)
        .height(height)
        .clip(true)
        .style(move |_| container::Style {
            background: Some(Color::BLACK.into()),
            border: crate::style::edge(edge, RADIUS),
            ..Default::default()
        });
    let held = mouse_area(framed)
        .on_press(Message::Call(Event::Grab))
        .interaction(if dragging {
            iced::mouse::Interaction::Grabbing
        } else {
            iced::mouse::Interaction::Pointer
        });
    // Its edges and corners resize it, over the rest of it.
    let grip = |edges: Edges, w: f32, h: f32, interaction| {
        mouse_area(Space::new().width(w).height(h))
            .on_press(Message::Call(Event::Resize(edges)))
            .interaction(interaction)
    };
    use iced::mouse::Interaction::{
        ResizingDiagonallyDown as Down, ResizingDiagonallyUp as Up, ResizingHorizontally as Across,
        ResizingVertically as Upright,
    };
    let edge = |left, right, top, bottom| Edges {
        left,
        right,
        top,
        bottom,
    };
    let resizing = stack![
        held,
        pin(grip(edge(true, false, false, false), EDGE, height, Across)),
        pin(grip(edge(false, true, false, false), EDGE, height, Across)).x(width - EDGE),
        pin(grip(edge(false, false, true, false), width, EDGE, Upright)),
        pin(grip(edge(false, false, false, true), width, EDGE, Upright)).y(height - EDGE),
        pin(grip(edge(true, false, true, false), GRIP, GRIP, Down)),
        pin(grip(edge(false, true, false, true), GRIP, GRIP, Down))
            .x(width - GRIP)
            .y(height - GRIP),
        pin(grip(edge(false, true, true, false), GRIP, GRIP, Up)).x(width - GRIP),
        pin(grip(edge(true, false, false, true), GRIP, GRIP, Up)).y(height - GRIP),
    ]
    .width(width)
    .height(height);
    Some(
        stack![pin(resizing).x(at.x).y(at.y)]
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
    )
}

/// The call's large tile over the whole window, in full screen, until
/// Escape, its button or a double click.
pub fn stage_full_view(app: &App) -> Option<Element<'_, Message>> {
    if !crate::call::stage::full_shown(app) {
        return None;
    }
    Some(
        Viewer::new(
            Color::BLACK,
            crate::call::stage::stage_view(app),
            app.window,
            Message::Nothing,
        )
        .view(app.look()),
    )
}

/// A message's video full screen: its name, Save and Exit full screen
/// over it, the video fitted between (a click plays or pauses it, a right
/// click opens its menu), its controls under it.
pub fn clip_viewer(app: &App) -> Option<Element<'_, Message>> {
    if !app.playback.clip_full {
        return None;
    }
    let video = app.client.state().video.as_ref()?;
    let size = crate::viewer::fitted(app.window, video.size);
    let key = format!("clip:{}/{}", video.item, video.index);
    let mut screen = stack![
        container(Space::new())
            .width(size.width)
            .height(size.height)
            .style(|_| crate::style::fill(Color::BLACK))
    ];
    if let Some(frame) = crate::call::stage::frame(app, &key, None) {
        screen = screen.push(frame);
    }
    if let Some(error) = &video.error {
        screen = screen.push(
            container(
                container(
                    text(backcord_iced_kit::i18n::t(error))
                        .size(13)
                        .color(Color::WHITE),
                )
                .padding([0, 10])
                .height(28)
                .align_y(Alignment::Center)
                .style(|_| crate::style::rounded(crate::playback::CLIP_SHADE, 6)),
            )
            .center(Length::Fill),
        );
    } else if video.loading {
        screen = screen.push(
            container(
                container(
                    text(backcord_iced_kit::i18n::tr("Loading..."))
                        .size(13)
                        .color(Color::WHITE),
                )
                .padding([0, 10])
                .height(28)
                .align_y(Alignment::Center)
                .style(|_| crate::style::rounded(GLASS, 14)),
            )
            .center(Length::Fill),
        );
    }
    let screen = mouse_area(screen.width(size.width).height(size.height))
        .on_press(Message::Clip(crate::playback::Clip::Toggle))
        .interaction(iced::mouse::Interaction::Pointer);
    // A right click offers what the message's menu does for a video.
    let (item, index) = (video.item.clone(), video.index);
    let screen = crate::widgets::RightPress::new(screen, move |_| {
        Message::Image(crate::image_viewer::Event::Menu(item.clone(), index, true))
    });
    let name = app
        .client
        .state()
        .message(&video.item)
        .and_then(|message| message.attachments.get(video.index))
        .map(|attachment| attachment.name.clone())
        .unwrap_or_default();
    let mut buttons: Vec<Element<'_, Message>> = Vec::new();
    if !app.client.state().preview {
        buttons.push(
            app.look()
                .face_button(
                    app.look().glyph(Icon::Download, 20, Color::WHITE),
                    32.0,
                    backcord_iced_kit::i18n::t(format!("Save {name}")),
                    Message::Image(crate::image_viewer::Event::Save(
                        video.item.clone(),
                        video.index,
                    )),
                )
                .below()
                .into(),
        );
    }
    buttons.push(
        app.look()
            .face_button(
                app.look().glyph(Icon::X, 20, Color::WHITE),
                32.0,
                backcord_iced_kit::i18n::tr("Exit full screen"),
                Message::Clip(crate::playback::Clip::Full(false)),
            )
            .below()
            .into(),
    );
    let width = size.width.max(360.0);
    Some(
        Viewer::new(
            crate::viewer::GROUND,
            screen,
            Size::new(size.width, size.height),
            Message::Clip(crate::playback::Clip::Full(false)),
        )
        .title(name, buttons)
        .foot(
            column![crate::playback::clip_controls(
                app.ctx(),
                video,
                width,
                true
            )]
            .padding(Padding::ZERO),
        )
        .view(app.look()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32, width: f32) -> Rectangle {
        Rectangle::new(Point::new(x, y), Size::new(width, width / aspect()))
    }

    #[test]
    fn a_resize_keeps_its_shape_and_the_edges_across_stay() {
        let start = rect(100.0, 100.0, 320.0);
        let right = Edges {
            right: true,
            ..Edges::default()
        };
        assert_eq!(
            resized(start, right, Vector::new(160.0, 0.0)),
            rect(100.0, 100.0, 480.0)
        );
        // The top left corner pulled up and out: the bottom right stays.
        let top_left = Edges {
            left: true,
            top: true,
            ..Edges::default()
        };
        let grown = resized(start, top_left, Vector::new(-160.0, -10.0));
        assert_eq!(grown, rect(-60.0, 10.0, 480.0));
        // The bottom edge alone grows it by its own height's share.
        let bottom = Edges {
            bottom: true,
            ..Edges::default()
        };
        assert_eq!(
            resized(start, bottom, Vector::new(0.0, 90.0)),
            rect(100.0, 100.0, 480.0)
        );
        // Never narrower than its narrowest.
        assert_eq!(
            resized(start, right, Vector::new(-1000.0, 0.0)).width,
            NARROWEST
        );
    }

    #[test]
    fn it_stays_inside_the_window_at_most_as_large_as_it() {
        let area = Rectangle::new(Point::new(0.0, 0.0), Size::new(800.0, 600.0));
        assert_eq!(
            fit_in(rect(700.0, 550.0, 320.0), area),
            rect(480.0, 420.0, 320.0)
        );
        assert_eq!(fit_in(rect(0.0, 0.0, 2000.0), area).width, 800.0);
    }

    #[test]
    fn let_go_over_what_it_keeps_clear_of_it_moves_the_least_way_off() {
        let area = Rectangle::new(Point::new(0.0, 0.0), Size::new(1200.0, 800.0));
        let join = Rectangle::new(Point::new(300.0, 300.0), Size::new(400.0, 200.0));
        // Over its right side: out to the right.
        let over = rect(600.0, 320.0, 320.0);
        let moved = clear_of(over, join, area);
        assert_eq!(moved.x, 700.0 + GAP);
        assert_eq!(moved.y, 320.0);
        // Where that leaves the window, the next least way.
        let narrow = Rectangle::new(Point::new(0.0, 0.0), Size::new(1000.0, 800.0));
        assert_eq!(clear_of(over, join, narrow).y, 500.0 + GAP);
        // Clear already: where it was.
        let clear = rect(10.0, 10.0, 200.0);
        assert_eq!(clear_of(clear, join, area), clear);
        // Nothing to keep clear of.
        assert_eq!(clear_of(over, Rectangle::default(), area), over);
    }
}
