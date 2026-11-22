//! View Icons: an avatar, a banner or a server's icon shown large over the
//! window, at the format and size Extra Features chose, fitted beside the
//! bars as a message's picture is. Escape, Close or a click outside closes
//! it.

use std::path::PathBuf;

use backcord_core::application::Action;
use iced::{
    Alignment, Color, Element, Length, Padding, Rectangle, Size, Task,
    widget::{container, text},
};

use crate::{
    app::{App, Message},
    ctx::Ctx,
    pictures::Icon,
    viewer::{GROUND, Viewer},
    widgets::Pic,
};

/// Room kept beside and above and below the picture, with the bars.
const SIDES: f32 = 160.0;
const ENDS: f32 = 128.0;
/// The note's ink, on its pill.
const NOTE_INK: Color = Color::from_rgba(1.0, 1.0, 1.0, 0xd9 as f32 / 255.0);

/// The formats a still picture can be asked in, as Extra Features lists
/// them; a moving one is always an animated WebP, the format Discord keeps
/// them in: small and served at once, where a GIF is made on the fly.
pub const FORMATS: [&str; 3] = ["png", "webp", "jpg"];
/// The sizes Discord's CDN gives, as Extra Features lists them.
pub const SIZES: [u32; 4] = [128, 256, 512, 1024];
/// [`SIZES`] as Extra Features' choice writes them.
pub const SIZE_NAMES: [&str; 4] = ["128", "256", "512", "1024"];
/// 2048, as the plugin's default.
pub const DEFAULT_SIZE: usize = 3;

/// The picture shown: its address as asked for, what it is, and the
/// smaller ones the app already holds of it, shown at once until it comes.
pub struct Shown {
    pub url: String,
    pub title: String,
    pub fallback: Vec<String>,
}

/// What the app may already hold of the picture at `url`: a moving banner's
/// or avatar's moving version, then the address itself.
fn fallbacks(url: &str) -> Vec<String> {
    let mut held: Vec<String> = backcord_core::chat::moving_banner(url)
        .into_iter()
        .collect();
    held.push(url.to_owned());
    held
}

/// The large address for each of these pictures, at the chosen format and
/// size, as the viewer asks for them.
pub fn large_urls<'a>(
    urls: impl IntoIterator<Item = &'a str>,
    prefs: &crate::layout::Prefs,
) -> Vec<String> {
    urls.into_iter()
        .map(|url| {
            full_url(
                url,
                prefs.icon_format.unwrap_or(0),
                prefs.icon_size.unwrap_or(DEFAULT_SIZE),
            )
        })
        .collect()
}

#[derive(Clone, Debug)]
pub enum Event {
    /// A picture to show, by the address the app has for it, and what it is.
    Open {
        url: String,
        title: String,
    },
    Close,
    /// Save it: where, asked with the system's dialog.
    Download,
    Saved(Option<PathBuf>),
}

/// The file's name `url` suggests, as `abc.gif`.
fn file_name(url: &str) -> String {
    let url = url.split_once('#').map_or(url, |(url, _)| url);
    let base = url.split_once('?').map_or(url, |(base, _)| base);
    base.rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or("picture.png")
        .to_owned()
}

/// `url` at the chosen format and size, for a picture on Discord's CDN; any
/// other, or one of Discord's default avatars, as it is.
pub fn full_url(url: &str, format: usize, size: usize) -> String {
    let base = url.split_once('?').map_or(url, |(base, _)| base);
    let discord = base.contains("cdn.discordapp.com") || base.contains("media.discordapp.net");
    if !discord || base.contains("/embed/avatars/") {
        return format!("{url}{}", backcord_core::chat::VIEW_PICTURE);
    }
    let (stem, extension) = base
        .rsplit_once('.')
        .filter(|(_, extension)| !extension.contains('/'))
        .unwrap_or((base, ""));
    let file = stem.rsplit('/').next().unwrap_or_default();
    let moving = file.starts_with("a_") || extension == "gif";
    let size = SIZES[size.min(SIZES.len() - 1)];
    let view = backcord_core::chat::VIEW_PICTURE;
    if moving {
        // Kept frame by frame: 1024 is plenty, and stays light.
        let size = size.min(1024);
        return format!("{stem}.webp?size={size}&animated=true{view}");
    }
    let format = FORMATS[format.min(FORMATS.len() - 1)];
    format!("{stem}.{format}?size={size}{view}")
}

impl App {
    pub fn icon_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Open { url, title } => {
                let fallback = fallbacks(&url);
                let url = large_urls([url.as_str()], &self.layout.prefs).remove(0);
                self.icon = Some(Shown {
                    url,
                    title,
                    fallback,
                });
                self.sync()
            }
            Event::Close => {
                self.icon = None;
                Task::none()
            }
            Event::Download => {
                let Some(shown) = &self.icon else {
                    return Task::none();
                };
                let name = file_name(&shown.url);
                Task::perform(
                    async move {
                        rfd::AsyncFileDialog::new()
                            .set_title(backcord_iced_kit::i18n::tr("Save picture"))
                            .set_file_name(name)
                            .save_file()
                            .await
                            .map(|file| file.path().to_owned())
                    },
                    |path| Message::Icon(Event::Saved(path)),
                )
            }
            Event::Saved(path) => {
                if let (Some(path), Some(shown)) = (path, &self.icon) {
                    self.client.dispatch(Action::SaveUrl {
                        url: shown.url.clone(),
                        path,
                    });
                }
                self.sync()
            }
        }
    }
}

/// The picture being viewed, over everything.
pub fn view<'a>(shown: &'a Option<Shown>, ctx: Ctx<'a>) -> Option<Element<'a, Message>> {
    let shown = shown.as_ref()?;
    let full = ctx
        .state
        .pictures
        .get(&shown.url)
        .filter(|picture| picture.width > 0 && picture.height > 0);
    // Until it comes, the one the app already holds, at the same size.
    let held = shown.fallback.iter().find_map(|url| {
        ctx.state
            .pictures
            .get(url)
            .filter(|picture| picture.width > 0 && picture.height > 0)
            .map(|picture| (url.as_str(), picture))
    });
    let (shown_url, picture) = match (full, held) {
        (Some(full), _) => (shown.url.as_str(), Some(full)),
        (None, Some((url, picture))) => (url, Some(picture)),
        (None, None) => (shown.url.as_str(), None),
    };
    let room = Size::new(
        (ctx.look.window.width - SIDES).max(1.0),
        (ctx.look.window.height - ENDS).max(1.0),
    );
    // The size it opens at: the large one's, or a held one's shape at the
    // size asked for, so nothing moves as the large one comes.
    let asked = SIZES[ctx
        .layout
        .prefs
        .icon_size
        .unwrap_or(DEFAULT_SIZE)
        .min(SIZES.len() - 1)] as f32;
    let natural = match (full, picture) {
        (Some(full), _) => Size::new(full.width as f32, full.height as f32),
        (None, Some(held)) => {
            let wide = held.width as f32 / held.height as f32;
            if wide >= 1.0 {
                Size::new(asked, asked / wide)
            } else {
                Size::new(asked * wide, asked)
            }
        }
        (None, None) => Size::new(256.0, 256.0),
    };
    let scale = 1f32
        .min(room.width / natural.width)
        .min(room.height / natural.height);
    let size = Size::new(
        (natural.width * scale).round().max(1.0),
        (natural.height * scale).round().max(1.0),
    );
    // A moving one plays, as the profile's banner does.
    let moving = ctx.moving(shown_url, size, Rectangle::with_size(size), false);
    let content: Element<'a, Message> = match picture {
        Some(_) if moving.is_some() => moving.map(Element::from).unwrap_or_else(|| text("").into()),
        Some(picture) => Pic::new(crate::pictures::shared(picture), size).into(),
        _ => container(
            container(
                text(backcord_iced_kit::i18n::tr("Loading the full image..."))
                    .size(13)
                    .color(NOTE_INK),
            )
            .padding(Padding::from([6, 12]))
            .style(|_| crate::style::rounded(Color::from_rgba(0.0, 0.0, 0.0, 0.6), 12)),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .into(),
    };
    let download = ctx
        .look
        .face_button(
            ctx.look.glyph(Icon::Download, 20, Color::WHITE),
            32.0,
            backcord_iced_kit::i18n::tr("Save"),
            Message::Icon(Event::Download),
        )
        .below()
        .into();
    let close = ctx
        .look
        .face_button(
            ctx.look.glyph(Icon::X, 20, Color::WHITE),
            32.0,
            backcord_iced_kit::i18n::tr("Close"),
            Message::Icon(Event::Close),
        )
        .below()
        .into();
    Some(
        Viewer::new(GROUND, content, size, Message::Icon(Event::Close))
            .title(shown.title.clone(), vec![download, close])
            .view(ctx.look),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asks_discords_cdn_for_the_chosen_format_and_size() {
        let avatar = "https://cdn.discordapp.com/avatars/1/abc.webp?size=80";
        assert_eq!(
            full_url(avatar, 0, 4),
            "https://cdn.discordapp.com/avatars/1/abc.png?size=1024#view"
        );
        // A moving one stays moving, as an animated WebP, at most 1024.
        assert_eq!(
            full_url(
                "https://cdn.discordapp.com/avatars/1/a_abc.png?size=80",
                1,
                0
            ),
            "https://cdn.discordapp.com/avatars/1/a_abc.webp?size=128&animated=true#view"
        );
        assert_eq!(
            full_url(
                "https://cdn.discordapp.com/banners/1/a_abc.png?size=600",
                0,
                5
            ),
            "https://cdn.discordapp.com/banners/1/a_abc.webp?size=1024&animated=true#view"
        );
        assert_eq!(
            full_url("https://cdn.discordapp.com/icons/2/def", 2, 5),
            "https://cdn.discordapp.com/icons/2/def.jpg?size=1024#view"
        );
        // Default avatars and pictures elsewhere keep their address.
        let default = "https://cdn.discordapp.com/embed/avatars/3.png";
        assert_eq!(full_url(default, 0, 4), format!("{default}#view"));
        assert_eq!(
            file_name(
                "https://cdn.discordapp.com/avatars/1/a_abc.webp?size=1024&animated=true#view"
            ),
            "a_abc.webp"
        );
    }
}
