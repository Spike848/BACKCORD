//! The title bar BACKCORD draws in place of the desktop's, with its Discord
//! look: Back and Forward, where BACKCORD is in the middle, Inbox and Help,
//! then minimize, maximize and close. Pressing and moving it moves the
//! window, a double click maximizes it, and its edges resize it. The Inbox
//! opens its popout under it: the channels and conversations with unread
//! messages, or those that mention this account.
//!
//! Whether BACKCORD draws it is the desktop's way (`frame.rs`) until
//! Appearance's "Use the system title bar" chooses. A window is made with
//! the frame or without; changing it makes the window again.

use crate::widgets::scroll_view;
use backcord_core::application::DmScreen;
use iced::{
    Alignment, Element, Length, Point, Size, Task,
    mouse::Interaction,
    widget::{Space, button, column, container, mouse_area, row, stack, text, tooltip},
    window,
};

use crate::{
    app::{App, Message},
    controls::{self, lit_style},
    elide::elide,
    fonts::BOLD,
    pictures::Icon,
    style::{faded, scrollbar},
    widgets::Hover,
};

/// The bar's height, which moves the window's content down.
pub const HEIGHT: f32 = 32.0;

/// A caption button's width, and its glyph's size: Discord's frame draws
/// them as 1 px strokes in a 10 px square.
const CAPTION: f32 = 36.0;
const CAPTION_GLYPH: u32 = 10;

/// The inbox popout's width and tallest height, as Discord's.
const INBOX_WIDTH: f32 = 600.0;
const INBOX_MOST: f32 = 720.0;
/// The most channels the inbox shows at once.
const INBOX_CARDS: usize = 30;
/// The widest and tallest a message's picture shows in the inbox.
const INBOX_PICTURE: (f32, f32) = (360.0, 220.0);

/// Where Discord's help opens.
const HELP: &str = "https://support.discord.com";

/// How far the pointer moves with the button down before the window moves,
/// so a double click still maximizes it.
const DRAG_AFTER: f32 = 3.0;

/// The edges' and corners' widths that resize a window without a frame.
const EDGE: f32 = 5.0;
const CORNER: f32 = 8.0;

/// Whether BACKCORD draws the title bar, as `system` (Appearance's choice)
/// or else the desktop's way has it: on Linux where the desktop frames no
/// windows, and on Windows, where Discord's look draws its own.
pub fn own(system: Option<bool>) -> bool {
    offered() && !system.unwrap_or_else(system_by_default)
}

/// Where BACKCORD can draw its own title bar.
pub fn offered() -> bool {
    cfg!(any(target_os = "linux", windows))
}

/// Whether the desktop draws the title bar unless the person chose: on a
/// desktop not known to give windows one, it's the desktop's, which may be
/// none at all, as a tiling compositor's. The tests lay the window out
/// without a bar wherever they run, unless one asks for it.
fn system_by_default() -> bool {
    cfg!(test) || (!cfg!(windows) && !crate::frame::desktop_frames())
}

#[derive(Clone, Debug)]
pub enum Event {
    Back,
    Forward,
    /// Mark everything read, in every server and DM.
    MarkAllRead,
    Inbox,
    InboxTab(Tab),
    InboxOpen(String),
    InboxScrolled(crate::sync::Scroll),
    /// A channel's or conversation's check: marked read, out of the inbox.
    InboxRead(String),
    /// Ask Discord for the tab's messages again.
    InboxReload,
    CloseInbox,
    Help,
    /// The bar pressed, the pointer moved on it, the button let go.
    Press,
    Moved(Point),
    Release,
    Maximize,
    Minimize,
    Close,
    /// An edge or corner pressed, by the direction it resizes.
    Resize(window::Direction),
    /// Whether the window is maximized now, and full screen (as a kiosk
    /// makes it), read at the size `Shell::sized` counts so.
    State(bool, bool, u32),
}

/// The inbox's tabs, as Discord's first two.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Unreads,
    Mentions,
}

/// The inbox popout while it's open: its tab, its channels, and their
/// messages' text laid out.
#[derive(Default)]
pub struct Inbox {
    pub tab: Tab,
    pub rows: Vec<InboxRow>,
    /// Its list's viewport.
    pub scroll: crate::sync::Scroll,
    /// The messages shown, as the core had them when laid out, and each
    /// one's text, by its id.
    pub messages: Vec<backcord_core::chat::InboxMessage>,
    pub bodies: std::collections::HashMap<String, crate::markdown::Body>,
}

/// One line of the inbox: a channel or conversation with something waiting.
#[derive(Clone, Debug, PartialEq)]
pub struct InboxRow {
    pub id: String,
    /// As the row shows it (a channel's with its `#`), and the name its
    /// avatar's initial and tint are taken from.
    pub name: String,
    pub avatar_name: String,
    pub place: String,
    pub picture: Option<String>,
    pub mentions: u64,
    /// A conversation's picture is round, a server's a rounded square.
    pub direct: bool,
}

/// The press that may move the window, and where it was.
#[derive(Default)]
pub struct Drag {
    pub armed: Option<Option<Point>>,
}

/// Alt+Left and Alt+Right, and the mouse's back and forward buttons, go
/// back and forward through the conversations visited, as in Discord and
/// whichever bar the window has.
pub fn history_keys() -> iced::Subscription<Message> {
    use iced::keyboard::{self, Key, Modifiers, key::Named};
    iced::event::listen_with(|event, _, _| {
        let back = match event {
            iced::Event::Keyboard(keyboard::Event::KeyPressed {
                key: Key::Named(named @ (Named::ArrowLeft | Named::ArrowRight)),
                modifiers: Modifiers::ALT,
                repeat: false,
                ..
            }) => named == Named::ArrowLeft,
            iced::Event::Mouse(iced::mouse::Event::ButtonPressed(button)) => match button {
                iced::mouse::Button::Back => true,
                iced::mouse::Button::Forward => false,
                _ => return None,
            },
            _ => return None,
        };
        Some(Message::Bar(if back {
            Event::Back
        } else {
            Event::Forward
        }))
    })
}

/// While the bar is pressed, the pointer's moves and the button let go,
/// anywhere in the window: a quick drag leaves the bar's few pixels at
/// once, and heard only over the bar, the window didn't move, and the
/// press stayed armed for a move with the button up.
pub fn drag_subscription() -> iced::Subscription<Message> {
    iced::event::listen_with(|event, _, _| match event {
        iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
            Some(Message::Bar(Event::Moved(position)))
        }
        iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)) => {
            Some(Message::Bar(Event::Release))
        }
        _ => None,
    })
}

/// The room under the title bar is what the window's content lays out
/// in (`Below`), so it's the window's size to everything under it.
pub fn fit_window(app: &mut App) {
    let frame = app.shell.frame;
    let room = Size::new(frame.width, (frame.height - bar_height(app)).max(0.0));
    if room != app.window {
        app.window = room;
    }
}

/// The room the title bar takes at the window's top.
pub fn bar_height(app: &App) -> f32 {
    if app.shell.own_title_bar && !full_screen(app) {
        HEIGHT
    } else {
        0.0
    }
}

/// Something full screen covers the window and the bar, or the window
/// is full screen itself.
fn full_screen(app: &App) -> bool {
    app.shell.full_screen || app.playback.clip_full || crate::call::stage::full_shown(app)
}

/// Asks the window whether it's maximized or full screen, as its size
/// changed.
pub fn read_window_state(app: &App) -> Task<Message> {
    let Some(id) = app.shell.window else {
        return Task::none();
    };
    let sized = app.shell.sized;
    window::is_maximized(id).then(move |maximized| {
        window::mode(id).map(move |mode| {
            Message::Bar(Event::State(
                maximized,
                mode == window::Mode::Fullscreen,
                sized,
            ))
        })
    })
}

pub fn update(app: &mut App, event: Event) -> Task<Message> {
    let id = app.shell.window;
    match event {
        Event::Back | Event::Forward
            if !crate::signin::workspace(app) || app.win.settings.is_open() =>
        {
            Task::none()
        }
        Event::Back | Event::Forward => {
            app.client.dispatch(match event {
                Event::Back => backcord_core::application::Action::NavigateBack,
                _ => backcord_core::application::Action::NavigateForward,
            });
            app.sync()
        }
        Event::MarkAllRead => {
            if crate::signin::workspace(app) && !app.win.settings.is_open() {
                app.client
                    .dispatch(backcord_core::application::Action::MarkAllRead);
            }
            app.sync()
        }
        Event::Inbox => {
            app.inbox = match app.inbox.take() {
                Some(_) => None,
                None => Some(Inbox {
                    scroll: crate::sync::Scroll::new(INBOX_MOST),
                    ..Inbox::default()
                }),
            };
            inbox_sync(app);
            load_inbox(app);
            app.sync()
        }
        Event::InboxTab(tab) => {
            if let Some(inbox) = &mut app.inbox {
                inbox.tab = tab;
            }
            inbox_sync(app);
            load_inbox(app);
            app.sync()
        }
        Event::InboxReload => {
            load_inbox(app);
            app.sync()
        }
        Event::InboxRead(room) => {
            app.client
                .dispatch(backcord_core::application::Action::MarkRead(room));
            app.sync()
        }
        Event::InboxOpen(room) => {
            app.inbox = None;
            app.client
                .dispatch(backcord_core::application::Action::SwitchTo(room));
            app.sync()
        }
        Event::InboxScrolled(scroll) => {
            if let Some(inbox) = &mut app.inbox {
                inbox.scroll = scroll;
            }
            Task::none()
        }
        Event::CloseInbox => {
            app.inbox = None;
            Task::none()
        }
        Event::Help => {
            crate::browser::open(HELP);
            Task::none()
        }
        Event::Press => {
            app.bar_drag.armed = Some(None);
            Task::none()
        }
        Event::Moved(at) => {
            let Some(armed) = &mut app.bar_drag.armed else {
                return Task::none();
            };
            let Some(from) = *armed else {
                *armed = Some(at);
                return Task::none();
            };
            if (at.x - from.x).abs() > DRAG_AFTER || (at.y - from.y).abs() > DRAG_AFTER {
                // The desktop takes the pointer for the move, and the
                // window may not hear the button let go.
                app.bar_drag.armed = None;
                return id.map_or_else(Task::none, window::drag);
            }
            Task::none()
        }
        Event::Release => {
            app.bar_drag.armed = None;
            Task::none()
        }
        Event::Maximize => {
            app.bar_drag.armed = None;
            id.map_or_else(Task::none, window::toggle_maximize)
        }
        Event::Minimize => id.map_or_else(Task::none, |id| window::minimize(id, true)),
        // As the desktop's close button: to the tray, or quit.
        Event::Close => app.close_requested(),
        Event::Resize(direction) => {
            id.map_or_else(Task::none, |id| window::drag_resize(id, direction))
        }
        Event::State(maximized, full, sized) => {
            // Read before the window took the size it has now, as one
            // asked while it opened and answered once it maximized: it
            // would keep the maximized size as the one to go back to. The
            // read that size asked for follows.
            if sized != app.shell.sized {
                return Task::none();
            }
            app.shell.maximized = maximized;
            app.shell.full_screen = full;
            app.keep_placement();
            Task::none()
        }
    }
}

/// Appearance's switch: the window is made again with the desktop's
/// frame or without it, where it was.
pub fn choose_title_bar(app: &mut App, system: bool) -> Task<Message> {
    let own = own(Some(system));
    if own == app.shell.own_title_bar {
        return Task::none();
    }
    app.shell.own_title_bar = own;
    app.inbox = None;
    if app.shell.window.is_none() {
        return Task::none();
    }
    // iced could take the frame off the window as it is, but where the
    // desktop draws none winit draws its own, and a maximized window
    // taking it or losing it grows or shrinks past the screen.
    // Settings, which the switch is in, stay open as they are.
    let closed = app.drop_window_keeping(crate::app::Keep { settings: true });
    let opened = app.open_window();
    Task::batch([closed, opened])
}

/// Asks the core for the open inbox's messages, for its tab.
fn load_inbox(app: &mut App) {
    let Some(inbox) = &app.inbox else {
        return;
    };
    let mentions = inbox.tab == Tab::Mentions;
    app.client
        .dispatch(backcord_core::application::Action::LoadInbox { mentions });
}

/// The pictures the open inbox shows: its channels' servers' icons and
/// its conversations' pictures, and its messages' authors and pictures.
pub fn inbox_pictures(app: &App) -> Vec<String> {
    let Some(inbox) = &app.inbox else {
        return Vec::new();
    };
    let mut urls: Vec<String> = inbox
        .rows
        .iter()
        .take(INBOX_CARDS)
        .filter_map(|row| row.picture.clone())
        .collect();
    for message in &inbox.messages {
        urls.extend(message.avatar.clone());
        urls.extend(message.picture.as_ref().map(|(url, _, _)| url.clone()));
    }
    urls
}

/// The inbox's channels for its tab, from what the client already knows:
/// those with unread messages, muted ones only for a mention, or those its
/// mentions are in; and its messages' text laid out.
pub fn inbox_sync(app: &mut App) {
    let Some(inbox) = &mut app.inbox else {
        return;
    };
    let state = app.client.state();
    let mentions_only = inbox.tab == Tab::Mentions;
    // Each channel's server, found once.
    let servers: std::collections::HashMap<&str, &backcord_core::chat::ChatServer> = state
        .servers
        .iter()
        .flat_map(|server| {
            server
                .channels
                .iter()
                .map(move |channel| (channel.as_str(), &**server))
        })
        .collect();
    let row = |room: &backcord_core::chat::ChatRoom| {
        let server = servers.get(room.id.as_str());
        InboxRow {
            id: room.id.clone(),
            name: if room.direct {
                room.name.clone()
            } else {
                format!("#{}", room.name)
            },
            avatar_name: server.map_or(room.name.clone(), |server| server.name.clone()),
            place: server.map(|server| server.name.clone()).unwrap_or_default(),
            picture: if room.direct {
                room.avatar.clone()
            } else {
                server.and_then(|server| server.icon.clone())
            },
            mentions: room.mentions,
            direct: room.direct,
        }
    };
    let feed = &state.inbox;
    let messages: Vec<backcord_core::chat::InboxMessage> = if feed.mentions == mentions_only {
        feed.messages.clone()
    } else {
        Vec::new()
    };
    let rows: Vec<InboxRow> = if mentions_only {
        // The conversations the mentions are in, newest first.
        let mut seen = std::collections::HashSet::new();
        messages
            .iter()
            .filter(|message| seen.insert(message.room.as_str()))
            .filter_map(|message| state.rooms.iter().find(|room| room.id == message.room))
            .map(|room| row(room))
            .collect()
    } else {
        // As many as the core asks about, picked as it picks them.
        state
            .rooms
            .iter()
            .filter(|room| !room.request && !room.unlisted)
            .filter(|room| room.mentions > 0 || (room.unread > 0 && !room.muted))
            .take(backcord_core::chat::INBOX_ROOMS)
            .map(|room| row(room))
            .collect()
    };
    if rows != inbox.rows {
        inbox.rows = rows;
    }
    if messages != inbox.messages {
        let link = crate::theme::rgb(app.theme.mention);
        inbox.bodies = messages
            .iter()
            .map(|message| {
                (
                    message.id.clone(),
                    crate::markdown::parse(&message.text, &[], link, &message.id, |_| false),
                )
            })
            .collect();
        inbox.messages = messages;
    }
}

/// The bar across the window's top, when BACKCORD draws it.
pub fn window_bar(app: &App) -> Option<Element<'_, Message>> {
    if bar_height(app) == 0.0 {
        return None;
    }
    let theme = &app.theme;
    let workspace = crate::signin::workspace(app);
    let state = app.client.state();
    let settings_open = app.win.settings.is_open();
    let place = if !workspace {
        backcord_iced_kit::i18n::tr("BACKCORD").to_owned()
    } else if app.sidebar.header.server.is_none() {
        backcord_iced_kit::i18n::tr("Direct Messages").to_owned()
    } else {
        app.sidebar.header.name.clone()
    };
    let ink = if !app.shell.focused {
        theme.faint
    } else if theme.dark {
        theme.nav_open
    } else {
        theme.secondary
    };
    // The Friends page names the bar "Friends", as Discord's does.
    let friends = workspace
        && app.sidebar.header.server.is_none()
        && crate::friends::dm_screen(app) == Some(DmScreen::Friends);
    let place = if friends {
        backcord_iced_kit::i18n::tr("Friends").to_owned()
    } else {
        place
    };
    let middle = row![
        app.look()
            .glyph(
                if friends { Icon::Users } else { Icon::Dm },
                16,
                theme.secondary
            )
            .boxed(16.0),
        elide(place).size(12).font(BOLD).color(ink).shrink()
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let back = workspace && !settings_open && state.can_go_back();
    let forward = workspace && !settings_open && state.can_go_forward();
    let left = row![
        bar_button(
            app,
            Icon::ArrowLeft,
            backcord_iced_kit::i18n::tr("Back"),
            back.then_some(Event::Back),
            false,
            BarSize::Arrow
        ),
        bar_button(
            app,
            Icon::ArrowRight,
            backcord_iced_kit::i18n::tr("Forward"),
            forward.then_some(Event::Forward),
            false,
            BarSize::Arrow
        ),
        bar_button(
            app,
            Icon::Check,
            backcord_iced_kit::i18n::tr("Mark everything as read"),
            (workspace && !settings_open).then_some(Event::MarkAllRead),
            false,
            BarSize::Arrow
        ),
    ]
    .padding(iced::Padding {
        left: 12.0,
        ..iced::Padding::ZERO
    })
    .align_y(Alignment::Center);
    let mut right = row![].align_y(Alignment::Center);
    if workspace {
        right = right.push(
            row![
                bar_button(
                    app,
                    Icon::Inbox,
                    backcord_iced_kit::i18n::tr("Inbox"),
                    Some(Event::Inbox),
                    app.inbox.is_some(),
                    BarSize::Tool
                ),
                bar_button(
                    app,
                    Icon::Help,
                    backcord_iced_kit::i18n::tr("Help"),
                    Some(Event::Help),
                    false,
                    BarSize::Tool
                ),
            ]
            .spacing(4)
            .padding(iced::Padding {
                right: 11.5,
                ..iced::Padding::ZERO
            }),
        );
    }
    let maximized = app.shell.maximized;
    right = right
        .push(caption_button(
            app,
            Icon::CaptionMin,
            Event::Minimize,
            false,
        ))
        .push(caption_button(
            app,
            if maximized {
                Icon::CaptionRestore
            } else {
                Icon::CaptionMax
            },
            Event::Maximize,
            false,
        ))
        .push(caption_button(app, Icon::CaptionClose, Event::Close, true));
    // The bar's ground moves the window; its buttons sit over it. Once
    // pressed, the pointer's moves come from `drag_subscription`, over
    // the whole window: a pointer passing over it sends none.
    let ground = mouse_area(container(Space::new()).width(Length::Fill).height(HEIGHT))
        .interaction(Interaction::Idle)
        .on_press(Message::Bar(Event::Press))
        .on_release(Message::Bar(Event::Release))
        .on_double_click(Message::Bar(Event::Maximize));
    let rail = theme.rail;
    let bar = stack![
        ground,
        container(middle)
            .center_x(Length::Fill)
            .height(HEIGHT)
            .align_y(Alignment::Center)
            .padding([0, 230]),
        row![left, Space::new().width(Length::Fill), right]
            .height(HEIGHT)
            .align_y(Alignment::Center),
    ];
    Some(
        container(bar)
            .width(Length::Fill)
            .height(HEIGHT)
            .style(move |_| crate::style::fill(rail))
            .into(),
    )
}

/// The bar's buttons: Discord draws the back and forward arrows 16 px in
/// a 24 px box and the inbox and help 18 px in a 32 px one.
#[derive(Clone, Copy)]
enum BarSize {
    Arrow,
    Tool,
}

impl BarSize {
    fn width(self) -> f32 {
        match self {
            BarSize::Arrow => 24.0,
            BarSize::Tool => 32.0,
        }
    }

    fn glyph(self) -> u32 {
        match self {
            BarSize::Arrow => 16,
            BarSize::Tool => 18,
        }
    }
}

/// A button in the bar's own row: a glyph on a 4 px rounded ground, its
/// label in a tip below.
fn bar_button<'a>(
    app: &'a App,
    icon: Icon,
    label: &'static str,
    event: Option<Event>,
    active: bool,
    size: BarSize,
) -> Element<'a, Message> {
    let theme = &app.theme;
    let enabled = event.is_some();
    let rest_ink = if !enabled {
        faded(theme.secondary, 0.3)
    } else if active {
        theme.text
    } else {
        theme.secondary
    };
    let glyph = size.glyph();
    let box_size = glyph as f32;
    let base = app.look().glyph(icon, glyph, rest_ink).boxed(box_size);
    let face: Element<'_, Message> = if enabled && !active {
        Hover::new(
            base,
            app.look().glyph(icon, glyph, theme.text).boxed(box_size),
        )
        .into()
    } else {
        base.into()
    };
    let rest = active.then(|| theme.wash(0.12));
    let lit = if !enabled {
        iced::Color::TRANSPARENT
    } else if active {
        theme.wash(0.12)
    } else {
        theme.wash(0.08)
    };
    let face = button(container(face).center(Length::Fill))
        .padding(0)
        .width(size.width())
        .height(24)
        .on_press_maybe(event.map(Message::Bar))
        .style(lit_style(rest, lit, 4.0));
    // What the click opens opens under the tip, which goes while it's open.
    if active {
        return face.into();
    }
    tooltip(face, controls::tip(theme, label), tooltip::Position::Bottom)
        .gap(6)
        .into()
}

/// The window's minimize, maximize and close: close lights red. The three
/// sit 2 px nearer the edge than their 36 px boxes put them, as Discord's
/// do, so the close box is 2 px narrower and its glyph keeps its place.
fn caption_button(app: &App, icon: Icon, event: Event, close: bool) -> Element<'_, Message> {
    let theme = &app.theme;
    let size = CAPTION_GLYPH as f32;
    let base = app
        .look()
        .glyph(icon, CAPTION_GLYPH, theme.secondary)
        .boxed(size);
    let face: Element<'_, Message> = if close {
        Hover::new(
            base,
            app.look()
                .glyph(icon, CAPTION_GLYPH, theme.on_danger)
                .boxed(size),
        )
        .into()
    } else {
        base.into()
    };
    let lit = if close { theme.danger } else { theme.hover };
    let (width, inset) = if close {
        (CAPTION - 2.0, 2.0)
    } else {
        (CAPTION, 0.0)
    };
    button(container(face).center(Length::Fill).padding(iced::Padding {
        left: inset,
        ..iced::Padding::ZERO
    }))
    .padding(0)
    .width(width)
    .height(HEIGHT)
    .on_press(Message::Bar(event))
    .style(lit_style(None, lit, 0.0))
    .into()
}

/// The edges and corners that resize a window without the desktop's
/// frame, while it isn't maximized: a layer over the window that only
/// they take the pointer in.
pub fn resize_edges(app: &App) -> Option<Element<'_, Message>> {
    if bar_height(app) == 0.0 || app.shell.maximized {
        return None;
    }
    use window::Direction;
    let grip = |direction: Direction, width: Length, height: Length| -> Element<'_, Message> {
        let cursor = match direction {
            Direction::North | Direction::South => Interaction::ResizingVertically,
            Direction::East | Direction::West => Interaction::ResizingHorizontally,
            Direction::NorthWest | Direction::SouthEast => Interaction::ResizingDiagonallyDown,
            Direction::NorthEast | Direction::SouthWest => Interaction::ResizingDiagonallyUp,
        };
        mouse_area(container(Space::new()).width(width).height(height))
            .interaction(cursor)
            .on_press(Message::Bar(Event::Resize(direction)))
            .into()
    };
    let fill = Length::Fill;
    let top = row![
        grip(
            Direction::NorthWest,
            Length::Fixed(CORNER),
            Length::Fixed(CORNER)
        ),
        column![
            grip(Direction::North, fill, Length::Fixed(EDGE)),
            Space::new().height(CORNER - EDGE)
        ]
        .width(fill),
        grip(
            Direction::NorthEast,
            Length::Fixed(CORNER),
            Length::Fixed(CORNER)
        ),
    ]
    .height(CORNER);
    let sides = row![
        grip(Direction::West, Length::Fixed(EDGE), fill),
        Space::new().width(fill).height(fill),
        grip(Direction::East, Length::Fixed(EDGE), fill),
    ]
    .height(fill);
    let bottom = row![
        grip(
            Direction::SouthWest,
            Length::Fixed(CORNER),
            Length::Fixed(CORNER)
        ),
        column![
            Space::new().height(CORNER - EDGE),
            grip(Direction::South, fill, Length::Fixed(EDGE))
        ]
        .width(fill),
        grip(
            Direction::SouthEast,
            Length::Fixed(CORNER),
            Length::Fixed(CORNER)
        ),
    ]
    .height(CORNER);
    Some(column![top, sides, bottom].into())
}

/// The inbox popout under the bar's Inbox button, as Discord's: Unreads
/// and Mentions, a card for each channel or conversation with its newest
/// messages, a check that marks it read, a click going there.
pub fn inbox_view(app: &App) -> Option<Element<'_, Message>> {
    let inbox = app.inbox.as_ref()?;
    if bar_height(app) == 0.0 || !crate::signin::workspace(app) {
        return None;
    }
    let theme = &app.theme;
    let feed = &app.client.state().inbox;
    let accent = theme.accent;
    let tab = |label: &'static str, which: Tab| -> Element<'_, Message> {
        let selected = inbox.tab == which;
        let words = text(label).size(14).font(BOLD).color(if selected {
            theme.text
        } else {
            theme.secondary
        });
        let underline = container(Space::new().width(Length::Fill).height(2)).style(move |_| {
            crate::style::fill(if selected {
                accent
            } else {
                iced::Color::TRANSPARENT
            })
        });
        button(
            column![
                container(words)
                    .center_x(Length::Fill)
                    .height(32)
                    .align_y(Alignment::Center),
                underline
            ]
            .width(Length::Fill),
        )
        .padding(0)
        .width(Length::Fill)
        .on_press(Message::Bar(Event::InboxTab(which)))
        .style(lit_style(None, theme.wash(0.06), 4.0))
        .into()
    };
    let tool = |icon: Icon, label: &'static str, event: Event| -> Element<'_, Message> {
        tooltip(
            button(
                container(app.look().glyph(icon, 16, theme.secondary).boxed(16.0))
                    .center(Length::Fill),
            )
            .padding(0)
            .width(32)
            .height(32)
            .on_press(Message::Bar(event))
            .style(lit_style(Some(theme.wash(0.06)), theme.wash(0.12), 8.0)),
            controls::tip(theme, label),
            tooltip::Position::Bottom,
        )
        .gap(4)
        .into()
    };
    let head = row![
        app.look().glyph(Icon::Inbox, 20, theme.text).boxed(20.0),
        text(backcord_iced_kit::i18n::tr("Inbox"))
            .size(18)
            .font(BOLD)
            .color(theme.text),
        Space::new().width(Length::Fill),
        tool(Icon::Refresh, "Aggiorna", Event::InboxReload),
        tool(
            Icon::Check,
            backcord_iced_kit::i18n::tr("Mark everything as read"),
            Event::MarkAllRead
        ),
    ]
    .spacing(10)
    .padding([0, 16])
    .height(56)
    .align_y(Alignment::Center);
    let tabs = row![
        tab(backcord_iced_kit::i18n::tr("Unreads"), Tab::Unreads),
        tab(backcord_iced_kit::i18n::tr("Mentions"), Tab::Mentions),
    ]
    .spacing(4)
    .padding([0, 12]);
    let note = |words: String| -> Element<'_, Message> {
        container(
            text(words)
                .size(14)
                .color(theme.muted)
                .align_x(Alignment::Center)
                .width(Length::Fill),
        )
        .padding(16)
        .height(136)
        .align_y(Alignment::Center)
        .into()
    };
    let body: Element<'_, Message> = if inbox.rows.is_empty() {
        if feed.loading && inbox.tab == Tab::Mentions {
            note("Caricamento…".to_owned())
        } else if let Some(error) = feed.error.as_ref().filter(|_| inbox.tab == Tab::Mentions) {
            note(error.clone())
        } else {
            note(
                match inbox.tab {
                    Tab::Unreads => backcord_iced_kit::i18n::tr(
                        "Unread messages from your unmuted channels show up here.",
                    ),
                    Tab::Mentions => backcord_iced_kit::i18n::tr("Nothing mentions you right now."),
                }
                .to_owned(),
            )
        }
    } else {
        let mut cards = column![].spacing(12).padding(12);
        for entry in inbox.rows.iter().take(INBOX_CARDS) {
            cards = cards.push(inbox_card(app, inbox, entry, feed.loading));
        }
        scroll_view(cards)
            .on_scroll(|viewport| {
                Message::Bar(Event::InboxScrolled(crate::sync::Scroll::of(viewport)))
            })
            .style(scrollbar(theme))
            .height(Length::Fill)
            .into()
    };
    let (raised, line) = (theme.raised, theme.line);
    let panel = container(column![
        head,
        tabs,
        controls::bar(theme.line, Length::Fill, 1),
        body
    ])
    .width(INBOX_WIDTH)
    // `window` is the room under the bar already: the panel starts 4 px
    // under it and stops 12 px over the window's foot.
    .height((app.window.height - 4.0 - 12.0).min(INBOX_MOST))
    .style(move |_| crate::style::panel(raised, line, 8.0));
    let at = Point::new(
        (app.window.width - INBOX_WIDTH - 8.0).max(0.0),
        HEIGHT + 4.0,
    );
    Some(
        stack![
            controls::outside(Message::Bar(Event::CloseInbox), false),
            controls::placed(mouse_area(panel).on_press(Message::Nothing), at.x, at.y),
        ]
        .into(),
    )
}

/// A channel's or conversation's card: its picture, name and place, its
/// mentions and its check, then its newest messages.
fn inbox_card<'a>(
    app: &'a App,
    inbox: &'a Inbox,
    entry: &'a InboxRow,
    loading: bool,
) -> Element<'a, Message> {
    let theme = &app.theme;
    let mut names = column![
        elide(entry.name.as_str())
            .size(15)
            .font(BOLD)
            .color(theme.text)
    ];
    if !entry.place.is_empty() {
        names = names.push(elide(entry.place.as_str()).size(12).color(theme.muted));
    }
    let radius = if entry.direct { 16.0 } else { 10.0 };
    let mut top = row![
        app.ctx().avatar_tinted(
            entry.picture.as_deref(),
            &entry.avatar_name,
            &entry.id,
            32.0,
            radius,
            1.0
        ),
        container(names.spacing(2)).width(Length::Fill).clip(true)
    ]
    .spacing(12)
    .align_y(Alignment::Center);
    if entry.mentions > 0 {
        top = top.push(controls::count_pill(
            theme,
            &controls::count_badge(entry.mentions),
            controls::INBOX_COUNT,
            None,
        ));
    }
    let top = button(top)
        .padding(4)
        .width(Length::Fill)
        .on_press(Message::Bar(Event::InboxOpen(entry.id.clone())))
        .style(lit_style(None, theme.wash(0.06), 6.0));
    let read: Element<'_, Message> = if inbox.tab == Tab::Unreads {
        tooltip(
            button(
                container(
                    app.look()
                        .glyph(Icon::Check, 16, theme.secondary)
                        .boxed(16.0),
                )
                .center(Length::Fill),
            )
            .padding(0)
            .width(32)
            .height(32)
            .on_press(Message::Bar(Event::InboxRead(entry.id.clone())))
            .style(lit_style(Some(theme.wash(0.06)), theme.wash(0.12), 8.0)),
            controls::tip(theme, backcord_iced_kit::i18n::tr("Mark as read")),
            tooltip::Position::Bottom,
        )
        .gap(4)
        .into()
    } else {
        Space::new().width(0).into()
    };
    let mut card = column![row![top, read].spacing(8).align_y(Alignment::Center)].spacing(4);
    let mut shown = 0;
    for message in inbox
        .messages
        .iter()
        .filter(|message| message.room == entry.id)
    {
        card = card.push(inbox_message(app, inbox, message));
        shown += 1;
    }
    if shown == 0 && loading {
        card =
            card.push(container(text("Caricamento…").size(12).color(theme.muted)).padding([4, 48]));
    }
    let (ground, edge) = (theme.wash(0.03), theme.line);
    container(card)
        .padding(8)
        .width(Length::Fill)
        .style(move |_| crate::style::outlined(ground, edge, 10.0))
        .into()
}

/// A message in the inbox: its author's picture and name, when, its text
/// formatted as in the chat, and its first picture.
fn inbox_message<'a>(
    app: &'a App,
    inbox: &'a Inbox,
    message: &'a backcord_core::chat::InboxMessage,
) -> Element<'a, Message> {
    let theme = &app.theme;
    let ctx = app.ctx();
    let when = crate::place_menus::until(message.at / 1000, app.layout.prefs.twelve_hour_clock);
    let mut words = column![
        row![
            elide(message.author.as_str())
                .size(14)
                .font(BOLD)
                .color(theme.text)
                .shrink(),
            text(when).size(11).color(theme.muted),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
    ]
    .spacing(2)
    .width(Length::Fill);
    if let Some(body) = inbox.bodies.get(&message.id) {
        for part in &body.parts {
            let size = match part.shape {
                crate::markdown::Shape::Heading(1) => 20.0,
                crate::markdown::Shape::Heading(2) => 18.0,
                crate::markdown::Shape::Heading(_) => 16.0,
                crate::markdown::Shape::Subtext => 12.0,
                _ => 14.0,
            };
            let ink = if part.shape == crate::markdown::Shape::Subtext {
                theme.muted
            } else {
                theme.text
            };
            let (sunken, link) = (theme.sunken, theme.link);
            let spans: Vec<_> = part
                .runs
                .iter()
                .map(|run| inbox_span(run, ink, sunken, link))
                .collect();
            let line: Element<'_, Message> = iced::widget::rich_text::<(), _, _, _>(spans)
                .size(size)
                .color(ink)
                .width(Length::Fill)
                .into();
            let line: Element<'_, Message> = if part.quoted {
                let bar_ink = theme.faint;
                row![
                    container(Space::new().width(4).height(Length::Fill))
                        .style(move |_| crate::style::rounded(bar_ink, 2)),
                    line
                ]
                .spacing(8)
                .height(Length::Shrink)
                .into()
            } else if part.shape == crate::markdown::Shape::Code {
                let code_ground = theme.sunken;
                container(line)
                    .padding(8)
                    .width(Length::Fill)
                    .style(move |_| crate::style::rounded(code_ground, 6))
                    .into()
            } else {
                line
            };
            words = words.push(line);
        }
    }
    if let Some((url, width, height)) = &message.picture {
        let scale = (INBOX_PICTURE.0 / *width as f32)
            .min(INBOX_PICTURE.1 / *height as f32)
            .min(1.0);
        let size = iced::Size::new(
            (*width as f32 * scale).round().max(16.0),
            (*height as f32 * scale).round().max(16.0),
        );
        if let Some(picture) = ctx.picture(url, size.width as u32, size.height as u32, true) {
            words = words.push(crate::widgets::Pic::new(picture, size).radius(8.0));
        } else {
            let ground = theme.sunken;
            words = words.push(
                container(Space::new())
                    .width(size.width)
                    .height(size.height)
                    .style(move |_| crate::style::rounded(ground, 8)),
            );
        }
    }
    if message.files > 0 {
        words = words.push(
            text(format!("📎 {} allegati", message.files))
                .size(12)
                .color(theme.muted),
        );
    }
    let line = row![
        ctx.avatar(message.avatar.as_deref(), &message.author, 32.0, 16.0, 1.0),
        words
    ]
    .spacing(12);
    button(line)
        .padding([6, 8])
        .width(Length::Fill)
        .on_press(Message::Bar(Event::InboxOpen(message.room.clone())))
        .style(lit_style(None, theme.wash(0.04), 6.0))
        .into()
}

/// A run of a message's text as the inbox draws it: its weight, slant,
/// lines and colour, code on its ground, a spoiler covered.
fn inbox_span(
    run: &crate::markdown::Run,
    ink: iced::Color,
    sunken: iced::Color,
    link: iced::Color,
) -> iced::widget::text::Span<'_, (), iced::Font> {
    use iced::{Font, font::Style};
    let font = if run.code {
        Font::MONOSPACE
    } else {
        Font {
            style: if run.italic {
                Style::Italic
            } else {
                Style::Normal
            },
            ..if run.bold {
                BOLD
            } else {
                crate::fonts::REGULAR
            }
        }
    };
    let piece = iced::widget::span(run.text.as_str())
        .font(font)
        .strikethrough(run.strike)
        .underline(run.underline);
    if run.hidden {
        return piece.color(sunken).background(sunken);
    }
    if let Some(rgb) = run.color {
        piece.color(crate::style::hex(rgb))
    } else if run.link.is_some() {
        piece.color(link)
    } else if run.code {
        piece.color(ink).background(sunken)
    } else {
        piece
    }
}
