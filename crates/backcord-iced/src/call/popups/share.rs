//! Go Live's window, as Discord's: tabs for the applications' windows,
//! the screens and the devices, each a card with its picture; under them
//! the stream's mode, a gear for its resolution and frame rate, and Go Live
//! (or, while live, Switch). And the menu the share button opens while
//! live ([`live_menu`]): another window, the stream's quality, stopping.

use crate::widgets::scroll_view;
use std::sync::Arc;

use backcord_core::{
    stream_worker::Source,
    voice::{Command, Resolution, StreamQuality},
};
use iced::{
    Alignment, Border, Element, Length, Padding, Task,
    advanced::image::Handle,
    widget::{Space, button, column, container, operation, row, stack, text},
};

use crate::call::{Event, Popup};
use crate::{
    app::{App, Message, NOBODY},
    controls::{self, Check, heading, menu_panel},
    fonts::{BOLD, SEMIBOLD},
    pictures::Icon,
};
use backcord_iced_kit::i18n::tr;

/// A screen's or window's picture in the share window, in pixels.
pub(super) const GLANCE: (u32, u32) = (480, 270);
/// The gear's menu's width, and the live menu's.
const GEAR_WIDTH: f32 = 290.0;
pub(super) const LIVE_WIDTH: f32 = 270.0;

/// The share window's tabs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShareTab {
    Applications,
    Screens,
    Devices,
}

/// What the gear's menu shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GearPage {
    Main,
}

/// The stream's mode, as Discord's gear names them: two kept qualities,
/// and the person's own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamMode {
    Gaming,
    Screenshare,
    Custom,
}

impl StreamMode {
    fn of(quality: StreamQuality) -> Self {
        match (quality.resolution, quality.fps) {
            (Resolution::P1080, 60) => Self::Gaming,
            (Resolution::Source, 15) => Self::Screenshare,
            _ => Self::Custom,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Gaming => tr("Gaming"),
            Self::Screenshare => tr("Screenshare"),
            Self::Custom => tr("Custom"),
        }
    }
}

use super::quality::resolution_name;

/// The share window's sources and choices.
pub struct Share {
    /// The screens and the windows with their pictures once taken, the
    /// cameras; then every source in the window's order (screens, windows,
    /// the system's dialog, cameras).
    pub screens: Vec<(String, Option<Handle>)>,
    pub windows: Vec<(String, Option<Handle>)>,
    pub cameras: Vec<String>,
    pub sources: Vec<Source>,
    /// The system's dialog picks a screen or window.
    pub dialog: bool,
    pub pick: usize,
    pub quality: StreamQuality,
    /// It switches what's shared, at the stream's quality.
    pub switching: bool,
    pub tab: ShareTab,
    /// The gear's menu, open on this page.
    pub gear: Option<GearPage>,
}

impl Share {
    /// Switching where the system's dialog is the only choice, as no
    /// screen, window or camera is listed: the dialog opens with no menu.
    pub fn dialog_alone(&self) -> bool {
        self.switching
            && self.dialog
            && self.screens.is_empty()
            && self.windows.is_empty()
            && self.cameras.is_empty()
    }

    /// Where `tab`'s choices are among the sources.
    fn range(&self, tab: ShareTab) -> std::ops::Range<usize> {
        let screens = self.screens.len();
        let windows = screens + self.windows.len();
        match tab {
            ShareTab::Screens => 0..screens,
            ShareTab::Applications => screens..windows,
            ShareTab::Devices => windows..self.sources.len(),
        }
    }
}

/// What's listed to share, as it comes from the thread that lists it.
pub struct Listing(pub Vec<backcord_core::stream_worker::Shareable>);

impl std::fmt::Debug for Listing {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} shareable", self.0.len())
    }
}

/// What's done in the share window.
#[derive(Clone, Debug)]
pub enum ShareEvent {
    /// The sources listed, by the listing's count.
    Listed(u64, Arc<Listing>),
    /// A screen's or window's picture, by its id and whether it's a
    /// window.
    Picture(u64, bool, Handle),
    Pick(usize),
    Tab(ShareTab),
    Gear(Option<GearPage>),
    Mode(StreamMode),
    Resolution(Resolution),
    Fps(u32),
    HighQuality,
    Go,
}

/// Lists what can be shared beside the window, as listing starts a
/// capturer for screens and one for windows and looks at each camera; the
/// menu opens when it's done.
pub(super) fn list_sources(count: u64) -> Task<Message> {
    Task::done(()).then(move |()| {
        let (sender, receiver) = iced::futures::channel::oneshot::channel();
        std::thread::spawn(move || {
            sender.send(backcord_core::stream_worker::shareable()).ok();
        });
        Task::perform(receiver, move |listed| {
            Message::Call(Event::Share(ShareEvent::Listed(
                count,
                Arc::new(Listing(listed.unwrap_or_default())),
            )))
        })
    })
}

/// Pictures of the screens to share, from a worker, so capturing them
/// takes memory only until they're in.
pub(super) fn glance(ids: Vec<u64>, windows: Vec<u64>) -> Task<Message> {
    if ids.is_empty() && windows.is_empty() {
        return Task::none();
    }
    // The worker starts as the task runs, not as it's made.
    Task::done((ids, windows)).then(|(ids, windows)| {
        let (sender, receiver) = iced::futures::channel::mpsc::unbounded();
        std::thread::spawn(move || {
            use backcord_core::stream_worker::{Report, Worker};
            let (_worker, mut reports) = match Worker::glance(ids, windows, GLANCE.0, GLANCE.1) {
                Ok(started) => started,
                Err(error) => {
                    tracing::warn!("no pictures of the screens to share: {error:#}");
                    return;
                }
            };
            while let Some(report) = reports.blocking_recv() {
                match report {
                    Report::Screen {
                        id,
                        window,
                        width,
                        height,
                        rgba,
                    } => {
                        let handle = Handle::from_rgba(width, height, rgba);
                        if sender.unbounded_send((id, window, handle)).is_err() {
                            break;
                        }
                    }
                    Report::Ended { error } => {
                        if let Some(error) = error {
                            tracing::warn!("no pictures of the screens to share: {error}");
                        }
                        break;
                    }
                    _ => {}
                }
            }
        });
        Task::run(receiver, |(id, window, handle)| {
            Message::Call(Event::Share(ShareEvent::Picture(id, window, handle)))
        })
    })
}

pub(in crate::call) fn share_event(app: &mut App, event: ShareEvent) -> Task<Message> {
    match event {
        ShareEvent::Listed(count, listing) => {
            // Only the listing the window waits for, in a call.
            if app.call.share_listing != Some(count) || !app.client.state().voice.active() {
                return Task::none();
            }
            app.call.share_listing = None;
            let (mut screens, mut windows, mut cameras) = (vec![], vec![], vec![]);
            for listed in &listing.0 {
                match listed.source {
                    Source::Display(_) => screens.push(listed),
                    Source::Camera(_) => cameras.push(listed),
                    _ => windows.push(listed),
                }
            }
            let id_of = |listed: &&backcord_core::stream_worker::Shareable| match listed.source {
                Source::Display(id) | Source::Window(id) => Some(id),
                _ => None,
            };
            let screen_ids: Vec<u64> = screens.iter().filter_map(id_of).collect();
            let window_ids: Vec<u64> = windows.iter().filter_map(id_of).collect();
            // Where the system's dialog picks a screen or window, it's a
            // device's choice, before the cameras.
            let dialog = backcord_core::stream_worker::portal_picks();
            let sources = screens
                .iter()
                .chain(&windows)
                .map(|listed| listed.source.clone())
                .chain(dialog.then_some(Source::Screen))
                .chain(cameras.iter().map(|listed| listed.source.clone()))
                .collect();
            let voice = &app.client.state().voice;
            let mut share = Share {
                screens: screens
                    .iter()
                    .map(|listed| (listed.title.clone(), None))
                    .collect(),
                windows: windows
                    .iter()
                    .map(|listed| (listed.title.clone(), None))
                    .collect(),
                cameras: cameras.iter().map(|listed| listed.title.clone()).collect(),
                sources,
                dialog,
                pick: 0,
                quality: voice.quality,
                switching: voice.sharing.is_some(),
                tab: ShareTab::Applications,
                gear: None,
            };
            // The first tab with something in it, its first chosen.
            share.tab = [ShareTab::Applications, ShareTab::Screens, ShareTab::Devices]
                .into_iter()
                .find(|tab| !share.range(*tab).is_empty())
                .unwrap_or(ShareTab::Screens);
            share.pick = share.range(share.tab).start;
            let alone = share.dialog_alone();
            app.call.share = Some(share);
            if alone {
                return share_event(app, ShareEvent::Go);
            }
            app.call.popup = Some(Popup::Share);
            return Task::batch([glance(screen_ids, window_ids), operation::focus(NOBODY)]);
        }
        ShareEvent::Picture(id, window, handle) => {
            if let Some(share) = &mut app.call.share {
                let source = if window {
                    Source::Window(id)
                } else {
                    Source::Display(id)
                };
                let at = share.sources.iter().position(|listed| *listed == source);
                let screens = share.screens.len();
                let picture = match at {
                    Some(at) if at < screens => share.screens.get_mut(at),
                    Some(at) => share.windows.get_mut(at - screens),
                    None => None,
                };
                if let Some((_, picture)) = picture {
                    *picture = Some(handle);
                }
            }
        }
        ShareEvent::Pick(pick) => {
            if let Some(share) = &mut app.call.share {
                share.pick = pick;
            }
        }
        ShareEvent::Tab(tab) => {
            if let Some(share) = &mut app.call.share {
                share.tab = tab;
                let range = share.range(tab);
                if !range.contains(&share.pick) && !range.is_empty() {
                    share.pick = range.start;
                }
            }
        }
        ShareEvent::Gear(page) => {
            if let Some(share) = &mut app.call.share {
                share.gear = page;
            }
        }
        ShareEvent::Mode(mode) => {
            if let Some(share) = &mut app.call.share {
                match mode {
                    StreamMode::Gaming => {
                        share.quality.resolution = Resolution::P1080;
                        share.quality.fps = 60;
                    }
                    StreamMode::Screenshare => {
                        share.quality.resolution = Resolution::Source;
                        share.quality.fps = 15;
                    }
                    // Its own resolution and rate, chosen below.
                    StreamMode::Custom => {}
                }
            }
        }
        ShareEvent::Resolution(resolution) => {
            if let Some(share) = &mut app.call.share {
                share.quality.resolution = resolution;
                share.gear = Some(GearPage::Main);
            }
        }
        ShareEvent::Fps(fps) => {
            if let Some(share) = &mut app.call.share {
                share.quality.fps = fps;
                share.gear = Some(GearPage::Main);
            }
        }
        ShareEvent::HighQuality => {
            if let Some(share) = &mut app.call.share {
                share.quality.high_quality = !share.quality.high_quality;
            }
        }
        ShareEvent::Go => {
            let Some(share) = app.call.share.take() else {
                return Task::none();
            };
            let picked = share.sources.get(share.pick).cloned();
            let command = if share.switching {
                if let Some(picked) = picked {
                    backcord_core::stream_worker::pick(picked);
                }
                Command::SwitchLive
            } else {
                backcord_core::stream_worker::pick(picked.unwrap_or(Source::Screen));
                Command::GoLive(share.quality)
            };
            crate::call::popups::close_popup(app);
            app.client
                .dispatch(backcord_core::application::Action::Voice(command));
            return Task::batch([app.sync(), crate::composer::refocus(app)]);
        }
    }
    Task::none()
}

fn share_message(event: ShareEvent) -> Message {
    Message::Call(Event::Share(event))
}

/// The share window: its tabs, the chosen tab's cards two to a row, and
/// under them the stream's mode, the gear and Go Live; the gear's menu
/// over its corner while open.
pub(super) fn share_menu<'a>(ctx: crate::ctx::Ctx<'a>, share: &'a Share) -> Element<'a, Message> {
    let theme = ctx.look.theme;
    let window = ctx.look.window;
    let width = (window.width - 64.0).clamp(360.0, 900.0);
    let height = (window.height - 64.0).clamp(320.0, 680.0);
    // The tabs, as Discord's: a strip, the chosen one lifted.
    let (raised, sunken, accent, secondary, hover) = (
        theme.raised,
        theme.sunken,
        theme.accent,
        theme.secondary,
        theme.hover,
    );
    let tab_button = |tab: ShareTab, label: &'static str, icon: Icon| -> Element<'a, Message> {
        let chosen = share.tab == tab;
        let ink = if chosen { accent } else { secondary };
        button(
            container(
                row![
                    ctx.look.glyph(icon, 16, ink),
                    text(label).size(14).font(SEMIBOLD).color(ink)
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .center_x(Length::Fill),
        )
        .padding([8, 12])
        .width(Length::Fill)
        .on_press(share_message(ShareEvent::Tab(tab)))
        .style(move |_, status| button::Style {
            background: if chosen {
                Some(raised.into())
            } else if controls::lights(status) {
                Some(hover.into())
            } else {
                None
            },
            border: crate::style::round(8),
            ..Default::default()
        })
        .into()
    };
    let tabs = container(
        row![
            tab_button(ShareTab::Applications, tr("Applications"), Icon::Gamepad),
            tab_button(ShareTab::Screens, tr("Entire Screen"), Icon::ScreenShare),
            tab_button(ShareTab::Devices, tr("Devices"), Icon::Video),
        ]
        .spacing(4),
    )
    .padding(4)
    .style(move |_| crate::style::rounded(sunken, 10));
    // The cards.
    let inner = width - 32.0;
    let card = ((inner - 12.0) / 2.0).floor();
    let mut cards: Vec<Element<'a, Message>> = Vec::new();
    for at in share.range(share.tab) {
        let pick = share_message(ShareEvent::Pick(at));
        let chosen = share.pick == at;
        let screens = share.screens.len();
        let windows = screens + share.windows.len();
        let shown = if at < screens {
            let (title, picture) = &share.screens[at];
            source_card(
                ctx,
                title,
                picture.as_ref(),
                Icon::ScreenShare,
                card,
                chosen,
                pick,
            )
        } else if at < windows {
            let (title, picture) = &share.windows[at - screens];
            source_card(
                ctx,
                title,
                picture.as_ref(),
                Icon::Gamepad,
                card,
                chosen,
                pick,
            )
        } else if share.dialog && at == windows {
            source_card(
                ctx,
                tr("Screen or window"),
                None,
                Icon::ScreenShare,
                card,
                chosen,
                pick,
            )
        } else {
            let camera = at - windows - usize::from(share.dialog);
            let title = share.cameras.get(camera).map_or("", String::as_str);
            source_card(ctx, title, None, Icon::Video, card, chosen, pick)
        };
        cards.push(shown);
    }
    let mut grid = column![].spacing(12);
    if cards.is_empty() {
        grid = grid.push(
            container(
                text(tr("Nothing to share here."))
                    .size(14)
                    .color(theme.muted),
            )
            .center_x(Length::Fill)
            .padding(40),
        );
    }
    let mut cards = cards.into_iter();
    while let Some(first) = cards.next() {
        grid = grid.push(match cards.next() {
            Some(second) => row![first, second].spacing(12),
            None => row![first],
        });
    }
    let grid = scroll_view(grid)
        .height(Length::Fill)
        .style(crate::style::scrollbar(theme));
    // The foot: the stream's mode and quality, the gear, Go Live.
    let mut foot = row![].spacing(12).align_y(Alignment::Center);
    if share.switching {
        foot = foot.push(Space::new().width(Length::Fill));
    } else {
        let quality = share.quality;
        foot = foot.push(
            column![
                text(StreamMode::of(quality).name())
                    .size(16)
                    .font(BOLD)
                    .color(theme.text),
                text(backcord_iced_kit::i18n::t(format!(
                    "{} \u{b7} {} fps",
                    resolution_name(quality.resolution),
                    quality.fps
                )))
                .size(13)
                .color(theme.muted),
            ]
            .spacing(2)
            .width(Length::Fill),
        );
        // The stream's sound, shared or not, beside the gear.
        let sound = !ctx.layout.voice.stream_without_sound;
        let (sound_lit, sound_hover) = (theme.raised, theme.hover);
        foot = foot.push(
            button(
                row![
                    ctx.look.glyph(
                        if sound { Icon::Volume } else { Icon::VolumeOff },
                        18,
                        theme.secondary,
                    ),
                    text(if sound {
                        tr("Sound on")
                    } else {
                        tr("Sound off")
                    })
                    .size(14)
                    .font(SEMIBOLD)
                    .color(theme.secondary),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .padding([0, 12])
            .height(40)
            .on_press(Message::Call(Event::StreamSound(!sound)))
            .style(move |_, status| button::Style {
                background: Some(
                    if controls::lights(status) {
                        sound_hover
                    } else {
                        sound_lit
                    }
                    .into(),
                ),
                border: crate::style::round(8),
                ..Default::default()
            }),
        );
        let gear_open = share.gear.is_some();
        let lit = if gear_open {
            theme.active
        } else {
            theme.raised
        };
        let hover = theme.hover;
        foot = foot.push(
            button(
                container(ctx.look.glyph(Icon::Settings, 20, theme.secondary)).center(Length::Fill),
            )
            .padding(0)
            .width(40)
            .height(40)
            .on_press(share_message(ShareEvent::Gear(
                (!gear_open).then_some(GearPage::Main),
            )))
            .style(move |_, status| button::Style {
                background: Some(if controls::lights(status) { hover } else { lit }.into()),
                border: crate::style::round(8),
                ..Default::default()
            }),
        );
    }
    let label = if share.switching {
        tr("Switch")
    } else {
        tr("Go Live")
    };
    let can_go = share.range(share.tab).contains(&share.pick);
    foot = foot.push(
        controls::action(
            theme,
            label,
            controls::Kind::Primary,
            can_go.then(|| share_message(ShareEvent::Go)),
        )
        .icon(
            ctx.look
                .icon(Icon::ScreenShare, 18, crate::theme::rgb(theme.on_accent)),
        ),
    );
    let body = column![tabs, grid, foot]
        .spacing(16)
        .padding(16)
        .width(width)
        .height(height);
    let panel = theme.panel;
    let edge = theme.line;
    let body = container(body).style(move |_| crate::style::panel(panel, edge, 12.0));
    let Some(page) = share.gear.filter(|_| !share.switching) else {
        return body.into();
    };
    stack![
        body,
        container(iced::widget::opaque(gear_menu(ctx, share.quality, page)))
            .width(width)
            .height(height)
            .padding(Padding {
                bottom: 72.0,
                right: 16.0,
                ..Padding::ZERO
            })
            .align_x(Alignment::End)
            .align_y(Alignment::End),
    ]
    .into()
}

/// The gear's menu: the stream's mode, its resolution and frame rate (each
/// a page of its own), and the encoder's effort.
fn gear_menu<'a>(
    ctx: crate::ctx::Ctx<'a>,
    quality: StreamQuality,
    page: GearPage,
) -> Element<'a, Message> {
    let theme = ctx.look.theme;
    let mut rows = column![].spacing(2);
    match page {
        GearPage::Main => {
            rows = rows.push(heading(theme, tr("STREAM MODE"), 4.0));
            let mode = StreamMode::of(quality);
            for (each, words) in [
                (StreamMode::Gaming, tr("Smoother video (1080p, 60fps)")),
                (StreamMode::Screenshare, tr("Clearer text (Source, 15fps)")),
                (StreamMode::Custom, tr("Your own resolution and frame rate")),
            ] {
                rows = rows.push(crate::call::popups::popup_row(
                    ctx,
                    each.name(),
                    Some(words.into()),
                    None,
                    if mode == each {
                        Check::Current
                    } else {
                        Check::None
                    },
                    Some(share_message(ShareEvent::Mode(each))),
                ));
            }
            rows = rows.push(controls::menu_separator(theme));
            rows = rows.push(super::quality::quality_picker(
                ctx,
                quality.resolution,
                quality.fps,
                |resolution| share_message(ShareEvent::Resolution(resolution)),
                |fps| share_message(ShareEvent::Fps(fps)),
            ));
            rows = rows.push(crate::call::popups::popup_row(
                ctx,
                tr("Higher quality"),
                Some(tr("Sharper, at twice the CPU").into()),
                None,
                Check::Box(quality.high_quality),
                Some(share_message(ShareEvent::HighQuality)),
            ));
        }
    }
    // The frosted window under its see-through ground, as the menus'.
    backcord_iced_kit::widgets::frost::Frost::new(
        [8.0; 4],
        container(rows)
            .padding(6)
            .width(GEAR_WIDTH)
            .style(menu_panel(theme)),
    )
    .into()
}

/// A screen, window or device in the share window: its picture (or its
/// glyph till one comes, or for a device), ringed when chosen, over its
/// name.
fn source_card<'a>(
    ctx: crate::ctx::Ctx<'a>,
    title: &'a str,
    picture: Option<&Handle>,
    glyph: Icon,
    width: f32,
    chosen: bool,
    press: Message,
) -> Element<'a, Message> {
    let theme = ctx.look.theme;
    let inner = width - 16.0;
    let height = (inner * 9.0 / 16.0).round();
    let shown: Element<'a, Message> = match picture {
        Some(handle) => iced::widget::image(handle.clone())
            .width(Length::Fill)
            .height(Length::Fill)
            .content_fit(iced::ContentFit::Contain)
            .into(),
        None => container(ctx.look.glyph(glyph, 32, theme.faint))
            .center(Length::Fill)
            .into(),
    };
    let (sunken, edge) = (
        theme.sunken,
        if chosen {
            theme.accent
        } else {
            iced::Color::TRANSPARENT
        },
    );
    let frame = container(shown)
        .width(inner)
        .height(height)
        .clip(true)
        .style(move |_| container::Style {
            background: Some(sunken.into()),
            border: Border {
                color: edge,
                width: if chosen { 3.0 } else { 0.0 },
                radius: 8.0.into(),
            },
            ..Default::default()
        });
    let ink = if chosen { theme.text } else { theme.secondary };
    let name = row![
        ctx.look.glyph(glyph, 16, ink),
        text(title)
            .size(14)
            .font(if chosen {
                SEMIBOLD
            } else {
                crate::fonts::REGULAR
            })
            .color(ink)
            .wrapping(text::Wrapping::None)
            .width(Length::Fill),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let (active, hover) = (theme.active, theme.hover);
    button(column![frame, name].spacing(8))
        .padding(8)
        .width(width)
        .on_press(press)
        .style(move |_, status| button::Style {
            background: if chosen {
                Some(active.into())
            } else if controls::lights(status) {
                Some(hover.into())
            } else {
                None
            },
            border: crate::style::round(10),
            ..Default::default()
        })
        .into()
}

/// What the share button opens while live: another window to share, the
/// stream's resolution and frame rate and the encoder's effort (which
/// change at once, the stream going on), and stopping it.
pub(super) fn live_menu<'a>(ctx: crate::ctx::Ctx<'a>) -> Element<'a, Message> {
    let theme = ctx.look.theme;
    let quality = ctx.state.voice.quality;
    let live = |quality: StreamQuality| Some(Message::Call(Event::LiveQuality(quality)));
    let mut rows = column![].spacing(2);
    rows = rows.push(crate::call::popups::popup_row(
        ctx,
        tr("Change windows"),
        None,
        Some(Icon::ScreenShare),
        Check::None,
        Some(Message::Call(Event::Open(Popup::Share))),
    ));
    rows = rows.push(super::quality::quality_picker(
        ctx,
        quality.resolution,
        quality.fps,
        |resolution| {
            Message::Call(Event::LiveQuality(StreamQuality {
                resolution,
                ..quality
            }))
        },
        |fps| Message::Call(Event::LiveQuality(StreamQuality { fps, ..quality })),
    ));
    let sound = !ctx.layout.voice.stream_without_sound;
    rows = rows.push(crate::call::popups::popup_row(
        ctx,
        tr("Share sound"),
        Some(tr("Others hear your computer's sound with the stream").into()),
        None,
        Check::Box(sound),
        Some(Message::Call(Event::StreamSound(!sound))),
    ));
    rows = rows.push(crate::call::popups::popup_row(
        ctx,
        tr("Higher quality"),
        Some(tr("Sharper, at twice the CPU").into()),
        None,
        Check::Box(quality.high_quality),
        live(StreamQuality {
            high_quality: !quality.high_quality,
            ..quality
        }),
    ));
    let stop: Element<'a, Message> = controls::action(
        theme,
        tr("Stop streaming"),
        controls::Kind::Danger,
        Some(Message::Call(Event::StopSharing)),
    )
    .icon(
        ctx.look
            .icon(Icon::ScreenShareOff, 18, crate::theme::rgb(theme.on_danger)),
    )
    .wide(36.0)
    .into();
    column![
        rows,
        container(stop).padding(Padding {
            top: 10.0,
            left: 4.0,
            right: 4.0,
            bottom: 4.0,
        }),
    ]
    .padding(6)
    .into()
}
