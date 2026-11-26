//! Appearance's custom background: a picture chosen from disk, copied
//! into BACKCORD's data folder, drawn under the whole window, whose grounds
//! (rail, lists, panels, chat, stage) let it through as its opacity says
//! while text, menus and controls stay solid. It takes no clicks. Its blur is made once, off the UI
//! thread, as the picture loads or the blur's slider is let go of; where
//! it's cut to fill the window is a point dragged in its preview in
//! Appearance. See "Custom background" in docs/themes.md.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use backcord_iced_kit::widgets::frost::Glass;

use iced::{
    Alignment, Border, ContentFit, Element, Length, Point, Rectangle, Size, Task,
    advanced::image::Handle,
    mouse,
    widget::{column, container, image, mouse_area, responsive, row, stack, text},
};

use crate::{
    app::{App, Message},
    controls::{Kind, action},
    form,
    pictures::Icon,
};

/// Its defaults: the grounds a third see-through, no blur, cut at its
/// middle.
pub const OPACITY: u8 = 35;
pub const BLUR: u8 = 0;
pub const FOCUS: u8 = 50;
/// The preview's width in Appearance; its height has the window's shape.
pub const PREVIEW: f32 = 420.0;
/// The largest it's decoded at, and at with a blur, which hides detail.
const SHARP: (u32, u32) = (2560, 1440);
const BLURRED: (u32, u32) = (1280, 720);
/// The file's name in the data folder, before its extension.
const FILE: &str = "backdrop";

/// The picture as made, and the drag in its preview.
#[derive(Default)]
pub struct Backdrop {
    shown: Option<Shown>,
    /// The pointer over the preview, and where a drag started with the
    /// focus then.
    pointer: Option<Point>,
    grab: Option<(Point, (f32, f32))>,
    /// The blurred copy frosted menus show, and how many were made.
    glass: Option<Arc<Glass>>,
    made_glass: u64,
    /// The window's size the glass was taken at, in the window's units.
    glass_window: Size,
    /// The window taken, small and sharp, so a new blur is made from it
    /// without taking it again: its size and pixels.
    sample: Option<(u32, u32, Arc<Vec<u8>>)>,
    /// The window is being drawn without what's over it, for the glass.
    pub capturing: bool,
    /// Something was open over the window at the last message.
    pub covered: bool,
}

/// The picture as made: its file and blur, and its size.
#[derive(Clone, Debug)]
pub struct Shown {
    file: String,
    blur: u8,
    width: u32,
    height: u32,
    handle: Handle,
}

#[derive(Clone, Debug)]
pub enum Event {
    /// Choose image, and the file chosen copied in, or why it wasn't.
    Pick,
    Picked(Option<Result<String, String>>),
    Remove,
    /// The picture made for a file and blur, or none when it can't be read.
    Made(String, u8, Option<Shown>),
    /// The sliders, and whether let go of.
    Blur(u32, bool),
    Opacity(u32, bool),
    /// The pointer over the preview, pressed on it, let go or gone.
    Move(Point),
    Press,
    Release,
    /// Menus' and dialogs' opacity and blur, and whether let go of; the
    /// frosted copy made for a file and blur.
    MenuOpacity(u32, bool),
    MenuBlur(u32, bool),
    Glassed(u8, Option<Made>, Size),
    /// The window as drawn without what's over it, for the glass.
    Shot(iced::window::Screenshot),
    /// The DM home's picture: Choose image, the file copied in, Reset.
    PickIcon,
    IconPicked(Option<Result<String, String>>),
    ResetIcon,
    /// An imported look's background point, after its picture.
    Focus(Option<u8>, Option<u8>),
}

fn message(event: Event) -> Message {
    Message::Backdrop(event)
}

/// What Appearance chose: the file, its focus (0 to 1 across and down),
/// blur and opacity (0 to 100).
struct Chosen<'a> {
    file: &'a str,
    focus: (f32, f32),
    blur: u8,
    opacity: u8,
}

fn chosen(prefs: &crate::layout::Prefs) -> Option<Chosen<'_>> {
    let file = prefs.backdrop.as_deref().filter(|file| !file.is_empty())?;
    let unit = |value: Option<u8>| f32::from(value.unwrap_or(FOCUS).min(100)) / 100.0;
    Some(Chosen {
        file,
        focus: (unit(prefs.backdrop_x), unit(prefs.backdrop_y)),
        blur: prefs.backdrop_blur.unwrap_or(BLUR).min(100),
        opacity: prefs.backdrop_opacity.unwrap_or(OPACITY).min(100),
    })
}

/// The picture chosen, made as BACKCORD starts.
pub fn load(app: &App) -> Task<Message> {
    match chosen(&app.layout.prefs) {
        Some(chosen) => make(chosen.file.to_owned(), chosen.blur),
        None => Task::none(),
    }
}

/// The menus' blur, 0 for none.
fn menu_blur(prefs: &crate::layout::Prefs) -> u8 {
    prefs.menu_blur.unwrap_or(0).min(100)
}

/// Whether menus show the window frosted under them: a blur, and menus
/// not fully opaque.
fn frosted(prefs: &crate::layout::Prefs) -> bool {
    menu_blur(prefs) > 0 && prefs.menu_opacity.unwrap_or(100) < 100
}

/// After each message: as something first opens over the window (a menu,
/// Settings, a popup), the window under it is taken, without it, and
/// blurred, for the glass menus show under their see-through grounds.
pub fn follow_layers(app: &mut App) -> Task<Message> {
    let covered = app.layers().next().is_some();
    let opened = covered && !app.backdrop.covered;
    app.backdrop.covered = covered;
    if !covered {
        app.backdrop.glass = None;
        app.backdrop.sample = None;
    }
    // Settings is taken even without a blur, so turning it on there shows
    // at once, with nothing taken again.
    if opened && (frosted(&app.layout.prefs) || app.win.settings.is_open()) {
        return capture(app);
    }
    Task::none()
}

/// A window taken: its size, then its small sharp copy and the copy
/// blurred.
type Made = (u32, u32, Arc<Vec<u8>>, Vec<u8>);

/// `sample` blurred by `blur`.
fn blurred(width: u32, height: u32, sample: &[u8], blur: u8) -> Vec<u8> {
    let mut pixels = sample.to_vec();
    let radius = blur_radius(width, height, blur);
    if radius > 0 {
        box_blur(&mut pixels, width as usize, height as usize, radius);
    }
    pixels
}

/// The kept sample blurred anew, off the UI thread: the glass shown stays
/// until it's made, so nothing over the window flickers.
fn reblur(app: &App, blur: u8) -> Task<Message> {
    let Some((width, height, sample)) = app.backdrop.sample.clone() else {
        return Task::none();
    };
    let window = app.backdrop.glass_window;
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                let pixels = blurred(width, height, &sample, blur);
                (width, height, sample, pixels)
            })
            .await
            .ok()
        },
        move |made| message(Event::Glassed(blur, made, window)),
    )
}

/// Takes the window without what's over it: one frame drawn without it.
fn capture(app: &mut App) -> Task<Message> {
    if app.backdrop.capturing {
        return Task::none();
    }
    app.backdrop.capturing = true;
    iced::window::latest()
        .and_then(iced::window::screenshot)
        .map(|shot| message(Event::Shot(shot)))
}

/// The window taken, made small and blurred off the UI thread.
fn glass(shot: iced::window::Screenshot, blur: u8) -> Task<Message> {
    let scale = shot.scale_factor.max(0.1);
    let window = Size::new(
        shot.size.width as f32 / scale,
        shot.size.height as f32 / scale,
    );
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                let (width, height) = (shot.size.width as usize, shot.size.height as usize);
                if width == 0 || height == 0 || shot.rgba.len() < width * height * 4 {
                    return None;
                }
                // A small copy: blurred, it loses nothing, and it's quick.
                let step = (width.max(height) / 640).max(1);
                let (small_width, small_height) = (width / step, height / step);
                let mut pixels = Vec::with_capacity(small_width * small_height * 4);
                for y in 0..small_height {
                    for x in 0..small_width {
                        let mut sum = [0u32; 4];
                        for dy in 0..step {
                            for dx in 0..step {
                                let at = ((y * step + dy) * width + x * step + dx) * 4;
                                for (channel, total) in sum.iter_mut().enumerate() {
                                    *total += u32::from(shot.rgba[at + channel]);
                                }
                            }
                        }
                        let count = (step * step) as u32;
                        pixels.extend(sum.map(|total| (total / count) as u8));
                    }
                }
                // Opaque: what's under the window doesn't show through it.
                for alpha in pixels.iter_mut().skip(3).step_by(4) {
                    *alpha = 255;
                }
                let (width, height) = (small_width as u32, small_height as u32);
                let shown = blurred(width, height, &pixels, blur);
                Some((width, height, Arc::new(pixels), shown))
            })
            .await
            .ok()
            .flatten()
        },
        move |made| message(Event::Glassed(blur, made, window)),
    )
}

/// `theme` as what opens over the window draws it: its grounds as
/// see-through as Appearance's menu opacity says; text and controls solid.
pub fn menus(
    app: &App,
    mut theme: backcord_iced_kit::theme::Theme,
) -> backcord_iced_kit::theme::Theme {
    let opacity = f32::from(app.layout.prefs.menu_opacity.unwrap_or(100).min(100)) / 100.0;
    if opacity >= 1.0 {
        return theme;
    }
    for ground in [
        &mut theme.menu,
        &mut theme.chat,
        &mut theme.panel,
        &mut theme.sidebar,
        &mut theme.rail,
        &mut theme.stage,
        &mut theme.raised,
    ] {
        ground.a *= opacity;
    }
    theme
}

/// Makes `file`'s picture with `blur`, off the UI thread.
fn make(file: String, blur: u8) -> Task<Message> {
    Task::perform(
        async move {
            let path = PathBuf::from(&file);
            let made = tokio::task::spawn_blocking(move || picture(&path, blur))
                .await
                .ok()
                .flatten();
            (file, blur, made)
        },
        |(file, blur, made)| {
            let made = made.map(|(width, height, pixels)| Shown {
                file: file.clone(),
                blur,
                width,
                height,
                handle: Handle::from_rgba(width, height, pixels),
            });
            message(Event::Made(file, blur, made))
        },
    )
}

/// `path`'s picture (an animation's first frame) in the renderer's
/// pixels, blurred: its size and pixels.
fn picture(path: &Path, blur: u8) -> Option<(u32, u32, Vec<u8>)> {
    let (width, height) = if blur > 0 { BLURRED } else { SHARP };
    let picture = backcord_core::chat::decode_picture_at(path, width, height)?;
    let mut pixels = picture.rgba.to_vec();
    let radius = blur_radius(picture.width, picture.height, blur);
    if radius > 0 {
        box_blur(
            &mut pixels,
            picture.width as usize,
            picture.height as usize,
            radius,
        );
    }
    Some((picture.width, picture.height, pixels))
}

/// The blur's radius in pixels: up to a 24th of the picture's long side.
fn blur_radius(width: u32, height: u32, blur: u8) -> usize {
    (width.max(height) as f32 / 24.0 * f32::from(blur.min(100)) / 100.0).round() as usize
}

/// Three box blurs one way then the other, which come near a Gaussian.
/// Each channel alike, whatever their order.
fn box_blur(pixels: &mut [u8], width: usize, height: usize, radius: usize) {
    let radius = radius.max(1);
    let mut line = Vec::new();
    for _ in 0..3 {
        for y in 0..height {
            pass(pixels, y * width * 4, 4, width, radius, &mut line);
        }
        for x in 0..width {
            pass(pixels, x * 4, width * 4, height, radius, &mut line);
        }
    }
}

/// One box blur along `count` pixels from `start`, `step` bytes apart,
/// the edges held past the ends.
fn pass(
    pixels: &mut [u8],
    start: usize,
    step: usize,
    count: usize,
    radius: usize,
    line: &mut Vec<u8>,
) {
    line.clear();
    line.extend((0..count).flat_map(|i| {
        let at = start + i * step;
        [pixels[at], pixels[at + 1], pixels[at + 2], pixels[at + 3]]
    }));
    let span = (2 * radius + 1) as u32;
    let get = |i: isize, channel: usize| {
        u32::from(line[(i.clamp(0, count as isize - 1) as usize) * 4 + channel])
    };
    for channel in 0..4 {
        let mut sum: u32 = (-(radius as isize)..=radius as isize)
            .map(|i| get(i, channel))
            .sum();
        for i in 0..count {
            pixels[start + i * step + channel] = (sum / span) as u8;
            let i = i as isize;
            sum = sum + get(i + radius as isize + 1, channel) - get(i - radius as isize, channel);
        }
    }
}

/// The part of a `width` by `height` picture that fills a box of `size`,
/// cut round `focus`.
fn crop(width: u32, height: u32, size: Size, focus: (f32, f32)) -> Rectangle<u32> {
    let (w, h) = (width as f32, height as f32);
    let shape = (size.width / size.height.max(1.0)).max(0.01);
    let (cut_w, cut_h) = if w / h > shape {
        (h * shape, h)
    } else {
        (w, w / shape)
    };
    let x = (w - cut_w) * focus.0.clamp(0.0, 1.0);
    let y = (h - cut_h) * focus.1.clamp(0.0, 1.0);
    Rectangle {
        x: x.round() as u32,
        y: y.round() as u32,
        width: (cut_w.round() as u32).clamp(1, width),
        height: (cut_h.round() as u32).clamp(1, height),
    }
}

/// The picture filling a box, cut round `focus`, as see-through as
/// `opacity` says.
fn drawn<'a>(shown: &Shown, focus: (f32, f32), opacity: f32) -> Element<'a, Message> {
    let shown = shown.clone();
    responsive(move |size| {
        image(shown.handle.clone())
            .crop(crop(shown.width, shown.height, size, focus))
            .content_fit(ContentFit::Fill)
            .width(Length::Fill)
            .height(Length::Fill)
            .opacity(opacity)
            .into()
    })
    .into()
}

/// The picture over the main window, while one is chosen and made.
pub fn view(app: &App) -> Option<Element<'_, Message>> {
    // The frosted window, for what's open over it.
    backcord_iced_kit::widgets::frost::set_glass(
        app.backdrop
            .glass
            .clone()
            .filter(|_| !app.backdrop.capturing && frosted(&app.layout.prefs))
            .map(|glass| {
                let all = Rectangle::new(
                    Point::ORIGIN,
                    Size::new(glass.width as f32, glass.height as f32),
                );
                (glass, all, app.backdrop.glass_window)
            }),
    );
    let chosen = chosen(&app.layout.prefs)?;
    let shown = app
        .backdrop
        .shown
        .as_ref()
        .filter(|shown| shown.file == chosen.file)?;
    // One veil of the theme's chat ground over all of it, as the opacity
    // says: the chat itself has no ground of its own to dim it.
    let mut veil = app.theme_solid.chat;
    veil.a = 1.0 - f32::from(chosen.opacity.min(100)) / 100.0;
    Some(
        iced::widget::stack![
            drawn(shown, chosen.focus, 1.0),
            iced::widget::container(iced::widget::Space::new())
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_| iced::widget::container::Style {
                    background: Some(veil.into()),
                    ..Default::default()
                })
        ]
        .into(),
    )
}

/// `theme` as the window shows it: with a custom background shown, the
/// grounds of the rail, the lists, the panels, the chat and the stage let
/// it through as its opacity says. Text, menus, fields and buttons stay
/// solid, so all over it reads as before.
pub fn see_through(
    app: &App,
    mut theme: backcord_iced_kit::theme::Theme,
) -> backcord_iced_kit::theme::Theme {
    let shown = chosen(&app.layout.prefs).filter(|chosen| {
        app.backdrop
            .shown
            .as_ref()
            .is_some_and(|shown| shown.file == chosen.file)
    });
    let Some(chosen) = shown else {
        return theme;
    };
    // The veil under them already dims the picture as the opacity says;
    // the chat lets it all through, the other grounds dim it half again
    // so the window's parts still tell apart.
    let solid = 1.0 - f32::from(chosen.opacity.min(100)) / 100.0;
    theme.chat.a = 0.0;
    // A pointed or chosen row, a pressed button: a see-through wash, still
    // plain to see, rather than a solid block over the picture.
    for wash in [
        &mut theme.hover,
        &mut theme.active,
        &mut theme.row_hover,
        &mut theme.button_default_hover,
    ] {
        wash.a *= 0.45;
    }
    for ground in [
        &mut theme.rail,
        &mut theme.sidebar,
        &mut theme.panel,
        &mut theme.stage,
    ] {
        ground.a *= solid * 0.5;
    }
    theme
}

/// The window's theme again, after the background or its opacity changed.
fn refresh(app: &mut App) {
    let solid = app.theme_solid.clone();
    app.show_theme(solid);
}

pub fn update(app: &mut App, event: Event) -> Task<Message> {
    let root = app.client.root().to_owned();
    let prefs = &mut app.layout.prefs;
    match event {
        Event::Pick => {
            return Task::perform(
                async move {
                    let file = rfd::AsyncFileDialog::new()
                        .set_title(backcord_iced_kit::i18n::tr("Choose a background"))
                        .add_filter(
                            backcord_iced_kit::i18n::tr("Pictures"),
                            &["png", "jpg", "jpeg", "webp", "gif"],
                        )
                        .pick_file()
                        .await?;
                    let path = file.path().to_owned();
                    Some(
                        tokio::task::spawn_blocking(move || copy_in(&root, &path))
                            .await
                            .unwrap_or_else(|error| Err(error.to_string())),
                    )
                },
                |picked| message(Event::Picked(picked)),
            );
        }
        Event::MenuOpacity(value, done) => {
            let opacity = value.min(100) as u8;
            prefs.menu_opacity = Some(opacity);
            if done {
                crate::layout::store(&root, "menu_opacity", opacity);
            }
            refresh(app);
        }
        Event::MenuBlur(value, done) => {
            let blur = value.min(100) as u8;
            prefs.menu_blur = Some(blur);
            if done {
                crate::layout::store(&root, "menu_blur", blur);
            }
            // Made again from what was taken, as the slider moves.
            if app.backdrop.covered && blur > 0 {
                return reblur(app, blur);
            }
        }
        Event::Shot(shot) => {
            app.backdrop.capturing = false;
            if app.backdrop.covered {
                return glass(shot, menu_blur(prefs));
            }
        }
        Event::Glassed(blur, made, window) => {
            let Some((width, height, sample, pixels)) = made.filter(|_| app.backdrop.covered)
            else {
                return Task::none();
            };
            app.backdrop.sample = Some((width, height, sample));
            app.backdrop.glass_window = window;
            // The slider moved on while it was made: made again for where
            // it is now, the glass shown kept till then.
            let now = menu_blur(prefs);
            if now != blur {
                return reblur(app, now);
            }
            app.backdrop.made_glass += 1;
            app.backdrop.glass = Some(Arc::new(Glass {
                pixels: Arc::new(pixels),
                width,
                height,
                generation: app.backdrop.made_glass,
            }));
        }
        Event::PickIcon => {
            return Task::perform(
                async move {
                    let file = rfd::AsyncFileDialog::new()
                        .set_title(backcord_iced_kit::i18n::tr("Choose an icon"))
                        .add_filter(
                            backcord_iced_kit::i18n::tr("Pictures"),
                            &["png", "jpg", "jpeg", "webp", "gif"],
                        )
                        .pick_file()
                        .await?;
                    let path = file.path().to_owned();
                    Some(
                        tokio::task::spawn_blocking(move || copy_as(&root, &path, ICON_FILE))
                            .await
                            .unwrap_or_else(|error| Err(error.to_string())),
                    )
                },
                |picked| message(Event::IconPicked(picked)),
            );
        }
        Event::Focus(x, y) => {
            prefs.backdrop_x = x.map(|x| x.min(100));
            prefs.backdrop_y = y.map(|y| y.min(100));
            crate::layout::store(&root, "backdrop_x", prefs.backdrop_x);
            crate::layout::store(&root, "backdrop_y", prefs.backdrop_y);
            refresh(app);
        }
        Event::IconPicked(None) => {}
        Event::IconPicked(Some(Err(error))) => {
            tracing::warn!(%error, "could not take the icon");
        }
        Event::IconPicked(Some(Ok(file))) => {
            crate::layout::store(&root, "home_icon", &file);
            prefs.home_icon = Some(file);
            crate::rail::forget_home_icon();
        }
        Event::ResetIcon => {
            if let Some(file) = prefs.home_icon.take()
                && let Err(error) = std::fs::remove_file(file)
            {
                tracing::debug!(%error, "could not remove the icon");
            }
            crate::layout::store(&root, "home_icon", None::<String>);
            crate::rail::forget_home_icon();
        }
        Event::Picked(None) => {}
        Event::Picked(Some(Err(error))) => {
            tracing::warn!(%error, "could not take the background");
        }
        Event::Picked(Some(Ok(file))) => {
            prefs.backdrop = Some(file.clone());
            prefs.backdrop_x = None;
            prefs.backdrop_y = None;
            crate::layout::store(&root, "backdrop", &file);
            crate::layout::store(&root, "backdrop_x", None::<u8>);
            crate::layout::store(&root, "backdrop_y", None::<u8>);
            // The old picture goes: the new one has the same name.
            app.backdrop.shown = None;
            let blur = prefs.backdrop_blur.unwrap_or(BLUR);
            return make(file, blur);
        }
        Event::Remove => {
            if let Some(file) = prefs.backdrop.take()
                && let Err(error) = std::fs::remove_file(file)
            {
                tracing::debug!(%error, "could not remove the background");
            }
            crate::layout::store(&root, "backdrop", None::<String>);
            app.backdrop.shown = None;
            refresh(app);
        }
        Event::Made(file, blur, made) => {
            if prefs.backdrop.as_deref() == Some(file.as_str())
                && prefs.backdrop_blur.unwrap_or(BLUR) == blur
            {
                if made.is_none() {
                    tracing::warn!("could not read the background picture");
                }
                app.backdrop.shown = made;
                refresh(app);
            }
        }
        Event::Blur(value, done) => {
            let blur = value.min(100) as u8;
            prefs.backdrop_blur = Some(blur);
            if done {
                crate::layout::store(&root, "backdrop_blur", blur);
                if let Some(file) = prefs.backdrop.clone()
                    && app
                        .backdrop
                        .shown
                        .as_ref()
                        .is_none_or(|shown| shown.blur != blur)
                {
                    return make(file, blur);
                }
            }
        }
        Event::Opacity(value, done) => {
            let opacity = value.min(100) as u8;
            prefs.backdrop_opacity = Some(opacity);
            if done {
                crate::layout::store(&root, "backdrop_opacity", opacity);
            }
            refresh(app);
        }
        Event::Move(point) => {
            app.backdrop.pointer = Some(point);
            if let Some((start, focus)) = app.backdrop.grab
                && let Some(shown) = &app.backdrop.shown
            {
                let window = app.window;
                let preview = Size::new(PREVIEW, preview_height(window));
                let cut = crop(shown.width, shown.height, preview, (0.5, 0.5));
                // How far the picture can move in the preview, each way.
                let room_x = (shown.width as f32 / cut.width as f32 - 1.0) * preview.width;
                let room_y = (shown.height as f32 / cut.height as f32 - 1.0) * preview.height;
                let along = |room: f32, moved: f32, from: f32| {
                    if room > 0.5 {
                        (from - moved / room).clamp(0.0, 1.0)
                    } else {
                        from
                    }
                };
                let x = along(room_x, point.x - start.x, focus.0);
                let y = along(room_y, point.y - start.y, focus.1);
                prefs.backdrop_x = Some((x * 100.0).round() as u8);
                prefs.backdrop_y = Some((y * 100.0).round() as u8);
            }
        }
        Event::Press => {
            if let (Some(pointer), Some(chosen)) = (app.backdrop.pointer, chosen(prefs)) {
                app.backdrop.grab = Some((pointer, chosen.focus));
            }
        }
        Event::Release => {
            if app.backdrop.grab.take().is_some() {
                crate::layout::store(&root, "backdrop_x", prefs.backdrop_x);
                crate::layout::store(&root, "backdrop_y", prefs.backdrop_y);
            }
        }
    }
    Task::none()
}

/// `path` chosen as the background, as Choose image's dialog would.
pub fn chosen_file(app: &mut App, path: &Path) -> Task<Message> {
    let picked = copy_in(app.client.root(), path);
    update(app, Event::Picked(Some(picked)))
}

/// Copies `path` into the data folder as the background, in place of the
/// one before; returns where it went.
fn copy_in(root: &Path, path: &Path) -> Result<String, String> {
    copy_as(root, path, FILE)
}

/// The DM home's own picture's name in the data folder, before its
/// extension.
const ICON_FILE: &str = "home-icon";

/// `path` chosen as the DM home's picture, as its dialog would.
pub fn chosen_icon(app: &mut App, path: &Path) -> Task<Message> {
    let picked = copy_as(app.client.root(), path, ICON_FILE);
    update(app, Event::IconPicked(Some(picked)))
}

/// Copies `path` into the data folder as `name` with its extension, in
/// place of one before; the copy's path.
fn copy_as(root: &Path, path: &Path, name: &str) -> Result<String, String> {
    let extension = picture_extension(path)?;
    let size = std::fs::metadata(path)
        .map_err(|error| error.to_string())?
        .len();
    if size > MOST_PICTURE {
        return Err("larger than 64 MiB".to_owned());
    }
    let target = clear_as(root, name, &extension);
    std::fs::copy(path, &target).map_err(|error| error.to_string())?;
    Ok(target.to_string_lossy().into_owned())
}

/// The largest picture taken in.
const MOST_PICTURE: u64 = 64 * 1024 * 1024;

/// `path`'s extension, lower case, if it's a picture BACKCORD reads.
fn picture_extension(path: &Path) -> Result<String, String> {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .filter(|extension| ["png", "jpg", "jpeg", "webp", "gif"].contains(&extension.as_str()))
        .ok_or_else(|| "not a picture BACKCORD reads".to_owned())
}

/// Removes the data folder's `name.*` and says where `name.extension` goes.
fn clear_as(root: &Path, name: &str, extension: &str) -> PathBuf {
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            if entry
                .file_name()
                .to_str()
                .is_some_and(|file| file.starts_with(&format!("{name}.")))
                && let Err(error) = std::fs::remove_file(entry.path())
            {
                tracing::debug!(%error, "could not remove the old background");
            }
        }
    }
    root.join(format!("{name}.{extension}"))
}

/// The look beside a theme's colours, as Export writes it: neon, both
/// roundnesses, the background's sliders and point, the menus', and the
/// background and DM icon pictures themselves, in base64.
pub fn look_export(prefs: &crate::layout::Prefs) -> serde_json::Value {
    let picture = |file: &Option<String>| {
        let path = Path::new(file.as_deref()?);
        let extension = picture_extension(path).ok()?;
        let bytes = std::fs::read(path).ok()?;
        Some(serde_json::json!({ "type": extension, "data": base64(&bytes) }))
    };
    serde_json::json!({
        "neon_glow": prefs.neon_glow,
        "corners": prefs.corners,
        "round": prefs.round,
        "backdrop_blur": prefs.backdrop_blur,
        "backdrop_opacity": prefs.backdrop_opacity,
        "backdrop_x": prefs.backdrop_x,
        "backdrop_y": prefs.backdrop_y,
        "menu_opacity": prefs.menu_opacity,
        "menu_blur": prefs.menu_blur,
        "backdrop": picture(&prefs.backdrop),
        "home_icon": picture(&prefs.home_icon),
    })
}

/// What an imported look sets that the background holds: its pictures
/// written to the data folder, and the messages that show them and its
/// sliders. Neon and the roundnesses are Settings' own.
pub fn look_import(root: &Path, look: &serde_json::Value) -> Task<Message> {
    let number = |key: &str| {
        look.get(key)
            .and_then(serde_json::Value::as_u64)
            .map(|value| value.min(200) as u32)
    };
    let picture = |key: &str, name: &str| -> Option<Result<String, String>> {
        let item = look.get(key)?;
        let extension = item.get("type")?.as_str()?.to_ascii_lowercase();
        if !["png", "jpg", "jpeg", "webp", "gif"].contains(&extension.as_str()) {
            return None;
        }
        let bytes = unbase64(item.get("data")?.as_str()?)?;
        if bytes.len() as u64 > MOST_PICTURE {
            return None;
        }
        let target = clear_as(root, name, &extension);
        Some(
            std::fs::write(&target, bytes)
                .map(|()| target.to_string_lossy().into_owned())
                .map_err(|error| error.to_string()),
        )
    };
    let mut tasks = Vec::new();
    // The picture first: taking it puts its point back to the middle.
    if let Some(picked) = picture("backdrop", FILE) {
        tasks.push(Task::done(message(Event::Picked(Some(picked)))));
        let point = |key: &str| number(key).map(|value| value.min(100) as u8);
        tasks.push(Task::done(message(Event::Focus(
            point("backdrop_x"),
            point("backdrop_y"),
        ))));
    }
    if let Some(picked) = picture("home_icon", ICON_FILE) {
        tasks.push(Task::done(message(Event::IconPicked(Some(picked)))));
    }
    for (key, event) in [
        ("backdrop_opacity", Event::Opacity as fn(u32, bool) -> Event),
        ("backdrop_blur", Event::Blur),
        ("menu_opacity", Event::MenuOpacity),
        ("menu_blur", Event::MenuBlur),
    ] {
        if let Some(value) = number(key) {
            tasks.push(Task::done(message(event(value.min(100), true))));
        }
    }
    Task::batch(tasks)
}

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// `bytes` in standard base64, padded.
fn base64(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let word = (u32::from(chunk[0]) << 16)
            | (u32::from(chunk.get(1).copied().unwrap_or(0)) << 8)
            | u32::from(chunk.get(2).copied().unwrap_or(0));
        for at in 0..4 {
            if at <= chunk.len() {
                out.push(char::from(BASE64[(word >> (18 - 6 * at) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Standard base64, padded or not, white space skipped; none if it isn't.
fn unbase64(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let (mut word, mut bits) = (0u32, 0u32);
    for byte in text.bytes() {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' | b' ' | b'\n' | b'\r' | b'\t' => continue,
            _ => return None,
        };
        word = (word << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((word >> bits) as u8);
            word &= (1 << bits) - 1;
        }
    }
    Some(out)
}

#[cfg(test)]
mod base64_tests {
    #[test]
    fn base64_goes_there_and_back() {
        for bytes in [
            &b""[..],
            b"f",
            b"fo",
            b"foo",
            b"foob",
            b"fooba",
            b"foobar",
            &[0, 255, 128, 7],
        ] {
            let text = super::base64(bytes);
            assert_eq!(super::unbase64(&text).as_deref(), Some(bytes), "{text}");
        }
        assert_eq!(super::base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(super::base64(b"fo"), "Zm8=");
    }
}

/// The preview's height: the window's shape at [`PREVIEW`] wide.
fn preview_height(window: Size) -> f32 {
    (PREVIEW * window.height / window.width.max(1.0)).clamp(120.0, 320.0)
}

/// Appearance's "Custom background": Choose image and Remove, the preview
/// to drag the picture in, and the blur's and opacity's sliders.
/// Appearance's menus and dialogs: their opacity, and the blur of the
/// background under them.
pub fn menu_settings<'a>(ctx: crate::ctx::Ctx<'a>) -> Element<'a, Message> {
    let theme = ctx.look.theme;
    let prefs = &ctx.layout.prefs;
    let opacity = u32::from(prefs.menu_opacity.unwrap_or(100).min(100));
    let blur = u32::from(menu_blur(prefs));
    column![
        form::heading(theme, backcord_iced_kit::i18n::tr("MENUS AND DIALOGS")),
        crate::controls::slider(
            backcord_iced_kit::i18n::tr("Menu opacity"),
            opacity,
            100,
            |value| message(Event::MenuOpacity(value, false)),
        )
        .mark(false)
        .on_release(message(Event::MenuOpacity(opacity, true)))
        .typed(|value| message(Event::MenuOpacity(value, true)))
        .view(theme),
        crate::controls::slider(
            backcord_iced_kit::i18n::tr("Menu blur (0 off)"),
            blur,
            100,
            |value| message(Event::MenuBlur(value, false)),
        )
        .mark(false)
        .on_release(message(Event::MenuBlur(blur, true)))
        .typed(|value| message(Event::MenuBlur(value, true)))
        .view(theme),
        form::note(
            theme,
            backcord_iced_kit::i18n::tr(
                "Settings, the soundboard, the call's menus and right-click menus: how see-through they are, and how blurred what's behind them shows. The blur shows when they're not fully opaque.",
            ),
        ),
    ]
    .spacing(12)
    .into()
}

/// Appearance's DM home picture: the one shown, Choose image and Reset.
pub fn icon_settings<'a>(ctx: crate::ctx::Ctx<'a>) -> Element<'a, Message> {
    let theme = ctx.look.theme;
    let ink = crate::theme::rgb(theme.text);
    let custom = ctx.layout.prefs.home_icon.is_some();
    let mut buttons = row![
        action(
            theme,
            backcord_iced_kit::i18n::tr("Choose image"),
            Kind::Plain,
            Some(message(Event::PickIcon)),
        )
        .small()
        .icon(ctx.look.icon(Icon::Image, 16, ink)),
    ]
    .spacing(8);
    if custom {
        buttons = buttons.push(
            action(
                theme,
                backcord_iced_kit::i18n::tr("Reset"),
                Kind::Plain,
                Some(message(Event::ResetIcon)),
            )
            .small()
            .icon(ctx.look.icon(Icon::Trash, 16, ink)),
        );
    }
    let side = crate::rail::TILE as u32;
    let face: Element<'a, Message> =
        match crate::rail::home_face(&ctx.layout.prefs, ctx.themes.chosen_id(), side) {
            Some(handle) => image(handle)
                .width(crate::rail::TILE)
                .height(crate::rail::TILE)
                .into(),
            None => iced::widget::Space::new().width(crate::rail::TILE).into(),
        };
    column![
        form::beside(
            form::heading(theme, backcord_iced_kit::i18n::tr("DIRECT MESSAGES ICON")),
            buttons,
        ),
        row![
            face,
            form::note(
                theme,
                backcord_iced_kit::i18n::tr(
                    "The picture at the top left, over your direct messages: any picture, cut square and fitted to its size.",
                ),
            )
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    ]
    .spacing(12)
    .into()
}

pub fn settings<'a>(ctx: crate::ctx::Ctx<'a>) -> Element<'a, Message> {
    let theme = ctx.look.theme;
    let prefs = &ctx.layout.prefs;
    let ink = crate::theme::rgb(theme.text);
    let chosen = chosen(prefs);
    let mut buttons = row![
        action(
            theme,
            backcord_iced_kit::i18n::tr("Choose image"),
            Kind::Plain,
            Some(message(Event::Pick)),
        )
        .small()
        .icon(ctx.look.icon(Icon::Image, 16, ink)),
    ]
    .spacing(8);
    if chosen.is_some() {
        buttons = buttons.push(
            action(
                theme,
                backcord_iced_kit::i18n::tr("Remove"),
                Kind::Plain,
                Some(message(Event::Remove)),
            )
            .small()
            .icon(ctx.look.icon(Icon::Trash, 16, ink)),
        );
    }
    let mut group = column![form::beside(
        form::heading(theme, backcord_iced_kit::i18n::tr("CUSTOM BACKGROUND")),
        buttons,
    )]
    .spacing(12);
    let Some(chosen) = chosen else {
        return group
            .push(form::note(
                theme,
                backcord_iced_kit::i18n::tr(
                    "A picture of your own behind BACKCORD, see-through as you choose.",
                ),
            ))
            .into();
    };
    let height = preview_height(ctx.look.window);
    let ground = theme.chat;
    let edge = theme.edge;
    let shown = ctx
        .backdrop
        .shown
        .as_ref()
        .filter(|shown| shown.file == chosen.file);
    // The window's ground under it, as the window has it.
    let face: Element<'a, Message> = match shown {
        Some(shown) => stack![
            container(text(""))
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_| crate::style::fill(ground)),
            drawn(shown, chosen.focus, 1.0),
        ]
        .into(),
        None => container(
            text(backcord_iced_kit::i18n::tr("Loading..."))
                .size(13)
                .color(theme.muted),
        )
        .center(Length::Fill)
        .into(),
    };
    let preview = container(face)
        .width(PREVIEW)
        .height(height)
        .clip(true)
        .style(move |_| container::Style {
            border: Border {
                color: edge,
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        });
    let preview = mouse_area(preview)
        .interaction(if ctx.backdrop.grab.is_some() {
            mouse::Interaction::Grabbing
        } else {
            mouse::Interaction::Grab
        })
        .on_move(|point| message(Event::Move(point)))
        .on_press(message(Event::Press))
        .on_release(message(Event::Release))
        .on_exit(message(Event::Release));
    let blur = u32::from(chosen.blur);
    let opacity_value = u32::from(chosen.opacity);
    group = group
        .push(
            column![
                preview,
                form::note(
                    theme,
                    backcord_iced_kit::i18n::tr(
                        "Drag the preview to choose which part of the picture shows.",
                    ),
                ),
            ]
            .spacing(6)
            .align_x(Alignment::Start),
        )
        .push(
            crate::controls::slider(
                backcord_iced_kit::i18n::tr("Background blur (0 off)"),
                blur,
                100,
                |value| message(Event::Blur(value, false)),
            )
            .on_release(message(Event::Blur(blur, true)))
            .typed(|value| message(Event::Blur(value, true)))
            .view(theme),
        )
        .push(
            crate::controls::slider(
                backcord_iced_kit::i18n::tr("Background opacity"),
                opacity_value,
                100,
                |value| message(Event::Opacity(value, false)),
            )
            .on_release(message(Event::Opacity(opacity_value, true)))
            .typed(|value| message(Event::Opacity(value, true)))
            .view(theme),
        );
    group.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cut_fills_the_box_round_its_focus() {
        // A wide picture in a square box: cut at its left, middle, right.
        let size = Size::new(100.0, 100.0);
        assert_eq!(
            crop(400, 200, size, (0.0, 0.5)),
            Rectangle {
                x: 0,
                y: 0,
                width: 200,
                height: 200
            }
        );
        assert_eq!(crop(400, 200, size, (0.5, 0.5)).x, 100);
        assert_eq!(crop(400, 200, size, (1.0, 0.5)).x, 200);
        // A tall one in a wide box: cut down it.
        let wide = Size::new(200.0, 100.0);
        assert_eq!(
            crop(200, 400, wide, (0.5, 1.0)),
            Rectangle {
                x: 0,
                y: 300,
                width: 200,
                height: 100
            }
        );
    }

    #[test]
    fn the_blur_smooths_and_keeps_a_flat_picture() {
        let (width, height) = (16, 8);
        let mut flat = vec![120u8; width * height * 4];
        box_blur(&mut flat, width, height, 3);
        assert!(flat.iter().all(|byte| *byte == 120));
        let mut edge = vec![0u8; width * height * 4];
        for y in 0..height {
            for x in width / 2..width {
                edge[(y * width + x) * 4] = 255;
            }
        }
        box_blur(&mut edge, width, height, 2);
        let red = |x: usize| edge[(4 * width + x) * 4];
        assert!(
            red(0) < red(width / 2 - 1)
                && red(width / 2 - 1) < red(width / 2)
                && red(width / 2) < red(width - 1)
        );
        assert_eq!(blur_radius(1280, 720, 0), 0);
        assert_eq!(blur_radius(1200, 720, 100), 50);
    }
}
