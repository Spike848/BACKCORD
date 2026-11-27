//! Room mode's popup, from its button in the call: a square with this
//! account at its middle and everyone else's face where they're heard
//! from, dragged anywhere on it (left and right, ahead at the top, behind
//! at the bottom), and the room's reverb under it. The core's
//! `backcord_core::room` places their voices.

use iced::{
    Alignment, Color, Element, Length, Point,
    widget::{Space, column, container, mouse_area, stack, text},
};

use crate::{
    app::{App, Message},
    call::Event,
    controls::{self, heading, slider},
};
use backcord_iced_kit::i18n::tr;

/// The popup's width, and the square's side inside it.
pub(super) const ROOM_WIDTH: f32 = 280.0;
const SIDE: f32 = 248.0;
/// A face's size on the square.
const FACE: f32 = 34.0;

/// Where `at`, on the square, puts someone: left -1 to right 1, ahead -1
/// to behind 1.
fn place_of(at: Point) -> (f32, f32) {
    (
        (at.x / SIDE * 2.0 - 1.0).clamp(-1.0, 1.0),
        (at.y / SIDE * 2.0 - 1.0).clamp(-1.0, 1.0),
    )
}

/// Where on the square someone at `place` shows, their face's top left.
fn spot_of((x, y): (f32, f32)) -> Point {
    Point::new(
        (x + 1.0) / 2.0 * (SIDE - FACE),
        (y + 1.0) / 2.0 * (SIDE - FACE),
    )
}

/// Those in the call this account hears, by their Discord user id.
fn others(
    app_people: &[backcord_core::voice::Participant],
) -> impl Iterator<Item = &backcord_core::voice::Participant> {
    app_people.iter().filter(|person| !person.local)
}

/// As the popup opens: those not placed yet go in a row ahead, spread
/// from left to right, and are kept there.
pub(super) fn place_newcomers(app: &mut App) {
    let people: Vec<String> = others(&app.client.state().voice.participants)
        .map(|person| person.person.clone())
        .collect();
    let kept = &mut app.layout.voice.room_places;
    let new: Vec<&String> = people
        .iter()
        .filter(|person| !kept.contains_key(*person))
        .collect();
    if new.is_empty() {
        return;
    }
    let count = new.len();
    for (index, person) in new.into_iter().enumerate() {
        let x = if count == 1 {
            0.0
        } else {
            -0.8 + 1.6 * index as f32 / (count - 1) as f32
        };
        let place = (x, -0.6);
        backcord_core::room::set_place(person, place.0, place.1);
        kept.insert(person.clone(), place);
    }
    crate::layout::store(app.client.root(), "room_places", &*kept);
}

/// Someone taken, moved to `at` on the square, or let go (kept).
pub(in crate::call) fn room_event(app: &mut App, event: RoomEvent) {
    match event {
        RoomEvent::Take(person) => app.call.room_drag = Some(person),
        RoomEvent::Move(at) => {
            let Some(person) = app.call.room_drag.clone() else {
                return;
            };
            let place = place_of(at);
            backcord_core::room::set_place(&person, place.0, place.1);
            app.layout.voice.room_places.insert(person, place);
        }
        RoomEvent::Drop => {
            if app.call.room_drag.take().is_some() {
                crate::layout::store(
                    app.client.root(),
                    "room_places",
                    &app.layout.voice.room_places,
                );
            }
        }
        RoomEvent::Reverb(value, done) => {
            let value = value.min(100) as u8;
            app.layout.voice.room_reverb = Some(value);
            backcord_core::room::set_reverb(f32::from(value) / 100.0);
            if done {
                crate::layout::store(app.client.root(), "room_reverb", value);
            }
        }
        RoomEvent::Reset => {
            let people: Vec<String> = app.layout.voice.room_places.keys().cloned().collect();
            for person in people {
                backcord_core::room::set_place(&person, 0.0, 0.0);
            }
            app.layout.voice.room_places.clear();
            place_newcomers(app);
        }
    }
}

/// What's done on the room's square.
#[derive(Clone, Debug)]
pub enum RoomEvent {
    /// Someone's face pressed, by their Discord user id.
    Take(String),
    /// The pointer on the square, in its own terms.
    Move(Point),
    Drop,
    /// The reverb's slider, 0 to 100, and whether it was let go.
    Reverb(u32, bool),
    /// Everyone back in a row ahead.
    Reset,
}

fn room_message(event: RoomEvent) -> Message {
    Message::Call(Event::Room(event))
}

pub(super) fn room_menu<'a>(ctx: crate::ctx::Ctx<'a>) -> Element<'a, Message> {
    let theme = ctx.look.theme;
    let (line, accent, sunken) = (theme.line, theme.accent, theme.sunken);
    // The square: its ground, a cross through its middle, this account at
    // the middle.
    let ground = container(Space::new())
        .width(SIDE)
        .height(SIDE)
        .style(move |_| container::Style {
            background: Some(sunken.into()),
            border: iced::Border {
                color: line,
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        });
    let bar = |width: f32, height: f32| {
        container(Space::new())
            .width(width)
            .height(height)
            .style(move |_| container::Style {
                background: Some(line.into()),
                ..Default::default()
            })
    };
    let label = |words: &'static str| text(words).size(11).color(theme.muted);
    let mut square = stack![
        ground,
        controls::placed(bar(1.0, SIDE), SIDE / 2.0, 0.0),
        controls::placed(bar(SIDE, 1.0), 0.0, SIDE / 2.0),
        controls::placed(label(tr("Ahead")), SIDE / 2.0 + 6.0, 4.0),
        controls::placed(label(tr("Behind")), SIDE / 2.0 + 6.0, SIDE - 18.0),
        controls::placed(label(tr("L")), 6.0, SIDE / 2.0 + 4.0),
        controls::placed(label(tr("R")), SIDE - 14.0, SIDE / 2.0 + 4.0),
    ]
    .width(SIDE)
    .height(SIDE);
    // You, at the middle.
    let me = container(Space::new())
        .width(14)
        .height(14)
        .style(move |_| container::Style {
            background: Some(accent.into()),
            border: iced::Border::default().rounded(7.0),
            ..Default::default()
        });
    square = square.push(controls::placed(me, SIDE / 2.0 - 7.0, SIDE / 2.0 - 7.0));
    let voice = &ctx.state.voice;
    let dragged = ctx.call.room_drag.as_deref();
    let mut count = 0;
    for person in others(&voice.participants) {
        count += 1;
        let place = ctx
            .layout
            .voice
            .room_places
            .get(&person.person)
            .copied()
            .unwrap_or_default();
        let ring = if dragged == Some(person.person.as_str()) {
            accent
        } else if person.speaking {
            theme.live
        } else {
            Color::TRANSPARENT
        };
        let face = container(ctx.avatar(
            person.avatar.as_deref(),
            &person.user,
            FACE - 4.0,
            (FACE - 4.0) / 2.0,
            1.0,
        ))
        .center(FACE)
        .style(move |_| container::Style {
            border: iced::Border {
                color: ring,
                width: 2.0,
                radius: (FACE / 2.0).into(),
            },
            ..Default::default()
        });
        let face = iced::widget::tooltip(
            mouse_area(face)
                .on_press(room_message(RoomEvent::Take(person.person.clone())))
                .interaction(iced::mouse::Interaction::Grab),
            controls::tip(theme, person.user.clone()),
            iced::widget::tooltip::Position::Top,
        )
        .gap(4);
        let spot = spot_of(place);
        square = square.push(controls::placed(face, spot.x, spot.y));
    }
    // The pointer's moves while a face is held, and letting go anywhere on
    // it or leaving it.
    let square = mouse_area(square)
        .on_move(|at| room_message(RoomEvent::Move(at)))
        .on_release(room_message(RoomEvent::Drop))
        .on_exit(room_message(RoomEvent::Drop))
        .interaction(if dragged.is_some() {
            iced::mouse::Interaction::Grabbing
        } else {
            iced::mouse::Interaction::Idle
        });
    let reverb = u32::from(ctx.layout.voice.room_reverb.unwrap_or(0).min(100));
    let mut rows = column![heading(theme, tr("ROOM MODE"), 4.0)].spacing(8);
    rows = rows.push(container(square).center_x(Length::Fill));
    if count == 0 {
        rows = rows.push(
            text(tr("Nobody else is in the call."))
                .size(12)
                .color(theme.muted),
        );
    }
    rows = rows.push(
        slider(tr("Reverb"), reverb, 100, |value| {
            room_message(RoomEvent::Reverb(value, false))
        })
        .step(1)
        .mark(false)
        .shown(if reverb == 0 {
            tr("Off").to_owned()
        } else {
            format!("{reverb}%")
        })
        .on_release(room_message(RoomEvent::Reverb(reverb, true)))
        .view(theme),
    );
    rows = rows.push(crate::call::popups::popup_row(
        ctx,
        tr("Put everyone back ahead"),
        None,
        None,
        controls::Check::None,
        Some(room_message(RoomEvent::Reset)),
    ));
    container(rows.padding(8).align_x(Alignment::Start))
        .width(ROOM_WIDTH)
        .into()
}
