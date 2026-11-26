//! The Dynamic Island: a pill at the top of the window, off unless
//! Appearance turns it on, and only for an account whose Discord has a
//! Spotify linked. It shows while Spotify plays something (paused too) or
//! this account is in a call, and nothing otherwise. Its tabs, which a
//! click on its dots or the mouse wheel over it moves between: Spotify's
//! player (the cover, the song and its artists, like, back, play or pause,
//! on), the call's buttons (mute, deafen, hang up) and where the call is.
//! A glow runs round its edge, brightest along its foot. See "Dynamic
//! Island" in docs/chat-ui.md.

use std::time::{Duration, Instant};

use backcord_core::{application::Action, chat::SpotifyControl, voice::Phase};
use iced::{
    Alignment, Border, Color, Element, Length, Padding, Rectangle, Shadow, Task, Vector,
    advanced::{Shell, renderer},
    mouse::ScrollDelta,
    widget::{Space, button, column, container, mouse_area, row, text},
    window,
};

use crate::{
    app::{App, Message},
    fonts::{BOLD, MEDIUM},
    pictures::Icon,
    widgets::{At, Hooks, Inner, Pic, Renderer, Wrap},
};

/// The pill's height, and its cover's side.
pub const HEIGHT: f32 = 44.0;
const COVER: f32 = 30.0;
/// How often the glow is drawn anew while it runs.
const FRAME: Duration = Duration::from_millis(33);
/// The dots of light in each of the glow's two tails.
const TAIL: usize = 28;

/// The glow's colours offered, besides the theme's accent (none).
pub const GLOWS: [u32; 7] = [
    0x1D_B9_54, 0x00_E5_FF, 0xFF_2E_C4, 0xFF_8A_00, 0xFF_3B_30, 0xA8_55_F7, 0xFF_FF_FF,
];
/// The text's and icons' colours offered, besides white (none).
pub const INKS: [u32; 5] = [0xC8_C8_C8, 0x1D_B9_54, 0x00_E5_FF, 0xFF_2E_C4, 0xFF_D6_0A];

/// Its defaults, until Appearance changes them.
pub const OPACITY: u8 = 75;
pub const GLOW: u8 = 60;
pub const SPEED: u8 = 40;
pub const RADIUS: u8 = 22;
pub const BORDER: u8 = 1;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Player,
    Controls,
    Status,
}

/// The tab it's on, and how far the wheel turned toward the next.
#[derive(Default)]
pub struct Island {
    pub tab: Tab,
    turned: f32,
}

#[derive(Clone, Debug)]
pub enum Event {
    Tab(Tab),
    /// The wheel over it, in lines.
    Scroll(f32),
    Spotify(SpotifyControl),
}

/// How it looks, as Appearance sets it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    pub oled: bool,
    /// The glass's opacity, the glow's intensity and speed, 0 to 1.
    pub opacity: f32,
    pub intensity: f32,
    pub speed: f32,
    pub glow: Color,
    pub ink: Color,
    pub radius: f32,
    pub border: f32,
}

fn rgb(rgb: u32) -> Color {
    Color::from_rgb8((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

impl Look {
    pub fn of(prefs: &crate::layout::Prefs, accent: Color) -> Self {
        let percent =
            |value: Option<u8>, default: u8| f32::from(value.unwrap_or(default).min(100)) / 100.0;
        Self {
            oled: prefs.island_style == Some(1),
            opacity: percent(prefs.island_opacity, OPACITY),
            intensity: percent(prefs.island_glow, GLOW),
            speed: percent(prefs.island_speed, SPEED),
            glow: prefs.island_color.map_or(accent, rgb),
            ink: prefs.island_ink.map_or(Color::WHITE, rgb),
            radius: f32::from(prefs.island_radius.unwrap_or(RADIUS)).min(HEIGHT / 2.0),
            border: f32::from(prefs.island_border.unwrap_or(BORDER).min(4)),
        }
    }

    /// Its ground: black for OLED, else tinted glass.
    fn ground(self) -> Color {
        if self.oled {
            Color::BLACK
        } else {
            Color::from_rgba(0.07, 0.07, 0.09, self.opacity.max(0.2))
        }
    }

    /// Its edge: sharp grey on OLED, a thin reflection on glass.
    fn edge(self) -> Color {
        if self.oled {
            Color::from_rgb(0.18, 0.18, 0.18)
        } else {
            Color::from_rgba(1.0, 1.0, 1.0, 0.18)
        }
    }
}

/// The tabs that have something to show: the player while Spotify plays
/// something, the call's two while in one.
pub fn tabs(app: &App) -> Vec<Tab> {
    let state = app.client.state();
    let mut tabs = Vec::new();
    if state.spotify.is_some() {
        tabs.push(Tab::Player);
    }
    if state.voice.active() && state.voice.room.is_some() {
        tabs.extend([Tab::Controls, Tab::Status]);
    }
    tabs
}

/// Whether it shows: turned on, a Spotify linked, and a tab to show.
pub fn shown(app: &App) -> bool {
    app.layout.prefs.island == Some(true)
        && app.client.state().spotify_linked
        && !tabs(app).is_empty()
}

/// The tab shown: the one chosen while it has something, else the first.
fn current(app: &App, tabs: &[Tab]) -> Tab {
    if tabs.contains(&app.island.tab) {
        app.island.tab
    } else {
        tabs.first().copied().unwrap_or_default()
    }
}

pub fn update(app: &mut App, event: Event) -> Task<Message> {
    match event {
        Event::Tab(tab) => {
            app.island.tab = tab;
            Task::none()
        }
        Event::Scroll(lines) => {
            app.island.turned += lines;
            if app.island.turned.abs() < 1.0 {
                return Task::none();
            }
            let step: i32 = if app.island.turned > 0.0 { -1 } else { 1 };
            app.island.turned = 0.0;
            let tabs = tabs(app);
            if tabs.len() > 1 {
                let at = tabs
                    .iter()
                    .position(|tab| *tab == current(app, &tabs))
                    .unwrap_or(0);
                let next = (at as i32 + step).rem_euclid(tabs.len() as i32) as usize;
                app.island.tab = tabs[next];
            }
            Task::none()
        }
        Event::Spotify(control) => {
            app.client.dispatch(Action::Spotify(control));
            app.sync()
        }
    }
}

/// The island, over the window at its top, while it shows.
pub fn view(app: &App) -> Option<Element<'_, Message>> {
    // Not over the call full screen, where it covered the stream.
    if !shown(app) || crate::call::stage::full_shown(app) || crate::call::stage::theatre_shown(app)
    {
        return None;
    }
    let tabs = tabs(app);
    let tab = current(app, &tabs);
    let ctx = app.ctx();
    let look = Look::of(&app.layout.prefs, ctx.look.theme.accent);
    let face = match tab {
        Tab::Player => player(ctx, look),
        Tab::Controls => controls(ctx, look),
        Tab::Status => status(ctx, look),
    }?;
    let (ground, edge) = (look.ground(), look.edge());
    let halo = Color {
        a: 0.45 * look.intensity,
        ..look.glow
    };
    let pill = container(face)
        .height(HEIGHT)
        .padding([0, 12])
        .align_y(Alignment::Center)
        .style(move |_| container::Style {
            background: Some(ground.into()),
            border: Border {
                color: edge,
                width: look.border,
                radius: look.radius.into(),
            },
            shadow: Shadow {
                color: halo,
                offset: Vector::new(0.0, 3.0),
                blur_radius: 6.0 + 14.0 * look.intensity,
            },
            ..Default::default()
        });
    let pill = mouse_area(Wrap::new(pill, Comet { look })).on_scroll(|delta| {
        Message::Island(Event::Scroll(match delta {
            ScrollDelta::Lines { y, .. } => y,
            ScrollDelta::Pixels { y, .. } => y / 40.0,
        }))
    });
    let mut island = column![pill].spacing(4).align_x(Alignment::Center);
    if tabs.len() > 1 {
        island = island.push(dots(&tabs, tab, look));
    }
    Some(
        container(island)
            .width(Length::Fill)
            .align_x(Alignment::Center)
            .padding(Padding {
                top: 6.0,
                ..Padding::ZERO
            })
            .into(),
    )
}

/// A round button of its own, `side` across: its ground, or a wash while
/// pointed at.
fn round<'a>(
    face: impl Into<Element<'a, Message>>,
    side: f32,
    ground: Option<Color>,
    message: Message,
) -> Element<'a, Message> {
    button(container(face).center(side))
        .padding(0)
        .on_press(message)
        .style(move |_, status| {
            let lit = matches!(status, button::Status::Hovered | button::Status::Pressed);
            let ground = match ground {
                Some(ground) if lit => Color {
                    a: ground.a * 0.85,
                    ..ground
                },
                Some(ground) => ground,
                None if lit => Color::from_rgba(1.0, 1.0, 1.0, 0.12),
                None => Color::TRANSPARENT,
            };
            button::Style {
                background: Some(ground.into()),
                border: Border::default().rounded(side / 2.0),
                ..Default::default()
            }
        })
        .into()
}

fn spotify(control: SpotifyControl) -> Message {
    Message::Island(Event::Spotify(control))
}

/// Spotify's player: the cover, the song and its artists, like, back,
/// play or pause, on.
fn player<'a>(ctx: crate::ctx::Ctx<'a>, look: Look) -> Option<Element<'a, Message>> {
    let now = ctx.state.spotify.as_ref()?;
    let ink = look.ink;
    let cover: Element<'a, Message> = match now
        .cover
        .as_deref()
        .and_then(|url| ctx.picture(url, 64, 64, true))
    {
        Some(handle) => Pic::square(handle, COVER).radius(6.0).into(),
        None => container(ctx.look.glyph(Icon::Music, 16, ink))
            .center(COVER)
            .style(|_| container::Style {
                background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.1).into()),
                border: Border::default().rounded(6.0),
                ..Default::default()
            })
            .into(),
    };
    let words = column![
        backcord_iced_kit::elide::elide(now.title.as_str())
            .size(13)
            .font(BOLD)
            .color(ink),
        backcord_iced_kit::elide::elide(now.artists.as_str())
            .size(12)
            .font(MEDIUM)
            .color(Color { a: 0.6, ..ink }),
    ]
    .width(150);
    let mut face = row![cover, words].spacing(10).align_y(Alignment::Center);
    if now.likeable {
        let liked = now.liked == Some(true);
        let (icon, tint) = if liked {
            (Icon::HeartFilled, look.glow)
        } else {
            (Icon::Heart, Color { a: 0.7, ..ink })
        };
        face = face.push(round(
            ctx.look.glyph(icon, 16, tint),
            28.0,
            None,
            spotify(SpotifyControl::Like(!liked)),
        ));
    }
    let (icon, control) = if now.playing {
        (Icon::Pause, SpotifyControl::Pause)
    } else {
        (Icon::Play, SpotifyControl::Play)
    };
    face = face.push(
        row![
            round(
                ctx.look.glyph(Icon::SkipBack, 16, ink),
                28.0,
                None,
                spotify(SpotifyControl::Previous),
            ),
            round(
                ctx.look.glyph(icon, 16, Color::BLACK),
                32.0,
                Some(ink),
                spotify(control),
            ),
            round(
                ctx.look.glyph(Icon::SkipForward, 16, ink),
                28.0,
                None,
                spotify(SpotifyControl::Next),
            ),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    );
    Some(face.into())
}

/// The call's buttons: its channel, then mute, deafen and hang up.
fn controls<'a>(ctx: crate::ctx::Ctx<'a>, look: Look) -> Option<Element<'a, Message>> {
    let voice = &ctx.state.voice;
    let theme = ctx.look.theme;
    let ink = look.ink;
    let wash = Some(Color::from_rgba(1.0, 1.0, 1.0, 0.1));
    let call = |event| Message::Call(event);
    let (mic, mic_ink) = if voice.muted {
        (Icon::MicOff, theme.danger)
    } else {
        (Icon::Mic, ink)
    };
    let (ears, ears_ink) = if voice.deafened {
        (Icon::HeadphoneOff, theme.danger)
    } else {
        (Icon::Headphones, ink)
    };
    Some(
        row![
            ctx.look.glyph(Icon::Volume, 16, Color { a: 0.7, ..ink }),
            container(
                backcord_iced_kit::elide::elide(voice.name.as_str())
                    .size(13)
                    .font(BOLD)
                    .color(ink)
            )
            .width(120),
            round(
                ctx.look.glyph(mic, 18, mic_ink),
                32.0,
                wash,
                call(crate::call::Event::Mute),
            ),
            round(
                ctx.look.glyph(ears, 18, ears_ink),
                32.0,
                wash,
                call(crate::call::Event::Deafen),
            ),
            round(
                ctx.look.glyph(Icon::PhoneOff, 18, Color::WHITE),
                32.0,
                Some(theme.danger),
                call(crate::call::Event::Leave),
            ),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into(),
    )
}

/// Where the call is: its signal, its channel and server, and how it's
/// connected.
fn status<'a>(ctx: crate::ctx::Ctx<'a>, look: Look) -> Option<Element<'a, Message>> {
    let state = ctx.state;
    let voice = &state.voice;
    let room = voice.room.as_deref()?;
    let ink = look.ink;
    let server = crate::profile::look::room_server(state, room)
        .map(|server| server.name.clone())
        .unwrap_or_else(|| backcord_iced_kit::i18n::tr("Direct Messages").to_owned());
    let phase = match voice.phase {
        Phase::Connected => backcord_iced_kit::i18n::tr("Voice connected"),
        Phase::Joining => backcord_iced_kit::i18n::tr("Joining voice..."),
        Phase::Reconnecting => backcord_iced_kit::i18n::tr("Reconnecting voice..."),
        Phase::Leaving => backcord_iced_kit::i18n::tr("Leaving voice..."),
        Phase::Disconnected => backcord_iced_kit::i18n::tr("Voice disconnected"),
    };
    let theme = ctx.look.theme;
    let signal = ctx.look.glyph(
        crate::call::signal_icon(voice.link_quality),
        18,
        crate::call::signal_ink(theme, voice.link_quality),
    );
    Some(
        row![
            signal,
            column![
                backcord_iced_kit::elide::elide(server)
                    .size(13)
                    .font(BOLD)
                    .color(ink),
                backcord_iced_kit::elide::elide(voice.name.as_str())
                    .size(12)
                    .font(MEDIUM)
                    .color(Color { a: 0.6, ..ink }),
            ]
            .width(170),
            text(phase)
                .size(12)
                .font(MEDIUM)
                .color(theme.live)
                .wrapping(text::Wrapping::None),
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .into(),
    )
}

/// The tabs' dots under the pill, the one shown in the glow's colour.
fn dots<'a>(tabs: &[Tab], shown: Tab, look: Look) -> Element<'a, Message> {
    let mut dots = row![].spacing(6).align_y(Alignment::Center);
    for tab in tabs {
        let lit = *tab == shown;
        let ink = if lit {
            look.glow
        } else {
            Color::from_rgba(1.0, 1.0, 1.0, 0.35)
        };
        dots = dots.push(
            button(Space::new().width(6).height(6))
                .padding(0)
                .on_press(Message::Island(Event::Tab(*tab)))
                .style(move |_, _| button::Style {
                    background: Some(ink.into()),
                    border: Border::default().rounded(3.0),
                    ..Default::default()
                }),
        );
    }
    dots.into()
}

/// The glow running round the pill's edge: two tails of light, brighter
/// along its foot, a lap each 12 s at the slowest and 1.5 s at the
/// fastest; still, at the foot's middle, at speed 0. It only draws, so the
/// pill's buttons take their clicks.
struct Comet {
    look: Look,
}

#[derive(Default)]
struct CometState {
    began: Option<Instant>,
    now: Option<Instant>,
}

impl Hooks<Message> for Comet {
    type State = CometState;

    fn update(
        &mut self,
        state: &mut CometState,
        event: &iced::Event,
        _: At,
        shell: &mut Shell<'_, Message>,
        child: &mut Inner<'_, '_, Message>,
    ) {
        if self.look.intensity > 0.0
            && self.look.speed > 0.0
            && let iced::Event::Window(window::Event::RedrawRequested(now)) = event
        {
            state.began.get_or_insert(*now);
            state.now = Some(*now);
            shell.request_redraw_at(*now + FRAME);
        }
        child.update(shell);
    }

    fn draw(
        &self,
        state: &CometState,
        renderer: &mut Renderer,
        at: At,
        child: &mut dyn FnMut(&mut Renderer),
    ) {
        use iced::advanced::Renderer as _;
        child(renderer);
        let look = self.look;
        if look.intensity <= 0.0 {
            return;
        }
        let bounds = at.bounds;
        let elapsed = match (state.began, state.now) {
            (Some(began), Some(now)) => now.duration_since(began).as_secs_f32(),
            _ => 0.0,
        };
        let lap = 12.0 - 10.5 * look.speed;
        // At speed 0 both tails meet at the foot's middle.
        let phase = if look.speed > 0.0 {
            (elapsed / lap).fract()
        } else {
            0.0
        };
        let edge = Edge::new(bounds, look.radius);
        let foot = if look.speed > 0.0 {
            0.0
        } else {
            edge.foot_middle()
        };
        for head in [phase, phase + 0.5] {
            for dot in 0..TAIL {
                let behind = dot as f32 / TAIL as f32;
                let along = (foot + head - behind * 0.3).rem_euclid(1.0);
                let point = edge.at(along * edge.length);
                // The foot is the brightest: up to half again.
                let low = ((point.y - bounds.y) / bounds.height).clamp(0.0, 1.0);
                let fade = (1.0 - behind) * (1.0 - behind);
                let alpha = (look.intensity * fade * (0.7 + 0.5 * low)).min(1.0);
                let side = (look.border * 2.0 + 2.5) * (1.0 - 0.5 * behind);
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle {
                            x: point.x - side / 2.0,
                            y: point.y - side / 2.0,
                            width: side,
                            height: side,
                        },
                        border: Border::default().rounded(side / 2.0),
                        shadow: Shadow {
                            color: Color {
                                a: alpha * 0.8,
                                ..look.glow
                            },
                            offset: Vector::ZERO,
                            blur_radius: 4.0 + 10.0 * look.intensity,
                        },
                        ..Default::default()
                    },
                    Color {
                        a: alpha,
                        ..look.glow
                    },
                );
            }
        }
    }
}

/// A rounded rectangle's edge, walked clockwise from its top's left end.
struct Edge {
    bounds: Rectangle,
    radius: f32,
    length: f32,
}

impl Edge {
    fn new(bounds: Rectangle, radius: f32) -> Self {
        let radius = radius
            .min(bounds.width / 2.0)
            .min(bounds.height / 2.0)
            .max(0.0);
        let straight = 2.0 * (bounds.width - 2.0 * radius) + 2.0 * (bounds.height - 2.0 * radius);
        Self {
            bounds,
            radius,
            length: straight + std::f32::consts::TAU * radius,
        }
    }

    /// Where the foot's middle is along it, from 0 to 1.
    fn foot_middle(&self) -> f32 {
        let (w, h, r) = (self.bounds.width, self.bounds.height, self.radius);
        let quarter = std::f32::consts::FRAC_PI_2 * r;
        ((w - 2.0 * r) + quarter + (h - 2.0 * r) + quarter + (w - 2.0 * r) / 2.0) / self.length
    }

    /// The point `s` along it.
    fn at(&self, s: f32) -> iced::Point {
        use std::f32::consts::FRAC_PI_2;
        let Rectangle {
            x,
            y,
            width: w,
            height: h,
        } = self.bounds;
        let r = self.radius;
        let (across, down, quarter) = (w - 2.0 * r, h - 2.0 * r, FRAC_PI_2 * r);
        let arc = |cx: f32, cy: f32, from: f32, left: f32| {
            let angle = from + if r > 0.0 { left / r } else { 0.0 };
            iced::Point::new(cx + r * angle.cos(), cy + r * angle.sin())
        };
        let mut s = s.rem_euclid(self.length.max(f32::EPSILON));
        if s < across {
            return iced::Point::new(x + r + s, y);
        }
        s -= across;
        if s < quarter {
            return arc(x + w - r, y + r, -FRAC_PI_2, s);
        }
        s -= quarter;
        if s < down {
            return iced::Point::new(x + w, y + r + s);
        }
        s -= down;
        if s < quarter {
            return arc(x + w - r, y + h - r, 0.0, s);
        }
        s -= quarter;
        if s < across {
            return iced::Point::new(x + w - r - s, y + h);
        }
        s -= across;
        if s < quarter {
            return arc(x + r, y + h - r, FRAC_PI_2, s);
        }
        s -= quarter;
        if s < down {
            return iced::Point::new(x, y + h - r - s);
        }
        s -= down;
        arc(x + r, y + r, std::f32::consts::PI, s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_edge_goes_round_the_pill_and_back() {
        let edge = Edge::new(
            Rectangle::new(iced::Point::new(10.0, 20.0), iced::Size::new(200.0, 44.0)),
            22.0,
        );
        let start = edge.at(0.0);
        assert_eq!((start.x, start.y), (32.0, 20.0), "the top's left end");
        let foot = edge.at(edge.foot_middle() * edge.length);
        assert!(
            (foot.x - 110.0).abs() < 0.01 && (foot.y - 64.0).abs() < 0.01,
            "{foot:?}"
        );
        let back = edge.at(edge.length);
        assert!((back.x - start.x).abs() < 0.01 && (back.y - start.y).abs() < 0.01);
        for step in 0..100 {
            let point = edge.at(edge.length * step as f32 / 100.0);
            assert!(point.x >= 9.99 && point.x <= 210.01 && point.y >= 19.99 && point.y <= 64.01);
        }
    }
}
