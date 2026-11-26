//! Folder colour: a Discord server folder's name and colour, from its menu
//! on the rail, as Discord's Folder Settings has them: the name's field,
//! Discord's preset colours as swatches, the kit's colour pad with the
//! colour's code under it, and Reset, which goes back to Discord's default
//! colour. Save hands them to the core (`Action::SetServerFolder`), which
//! has Discord keep them in the account's server list; the rail shows them
//! at once. What's being chosen is the window's, from Open to Save or
//! Close; Escape or a click outside closes it.

use backcord_core::application::Action;
use backcord_iced_kit::i18n::tr;
use iced::{
    Border, Element, Length, Task,
    widget::{Space, button, column, container, operation, row},
};

use crate::{
    app::{App, Message},
    controls::{Kind, action, lights},
    ctx::Ctx,
    dialog::buttons,
    form,
    style::hex,
    widgets::{ColourPad, Hsv, hsv_rgb, rgb_hsv},
};

/// The dialog's width.
const WIDTH: f32 = 400.0;
/// A preset's swatch, and the room between them: ten to a row.
const SWATCH: f32 = 28.0;
const SWATCH_GAP: f32 = 8.0;
/// The fields' ids.
const NAME_FIELD: &str = "folder-name";
const CODE_FIELD: &str = "folder-colour-code";
/// Discord's colour for a folder that wasn't given one, as the rail draws it.
pub const DEFAULT_COLOUR: u32 = 0x5865f2;
/// The colours Discord's colour picker offers, a bright row then a dark one.
pub const PRESETS: [u32; 20] = [
    0x1abc9c, 0x2ecc71, 0x3498db, 0x9b59b6, 0xe91e63, 0xf1c40f, 0xe67e22, 0xe74c3c, 0x95a5a6,
    0x607d8b, 0x11806a, 0x1f8b4c, 0x206694, 0x71368a, 0xad1457, 0xc27c0e, 0xa84300, 0x992d22,
    0x979c9f, 0x546e7a,
];

/// The dialog while it's open: the folder, its name and colour as chosen
/// (none for Discord's default), as they were, and the pad's colour and
/// its code as written.
#[derive(Debug, PartialEq)]
pub struct Shown {
    pub folder: String,
    pub name: String,
    pub colour: Option<u32>,
    start: (String, Option<u32>),
    pub hsv: Hsv,
    pub code: String,
}

#[derive(Clone, Debug)]
pub enum Event {
    /// The folder's menu chose Folder colour: by the folder's id.
    Open(String),
    Close,
    Name(String),
    Preset(u32),
    /// Discord's default colour.
    Reset,
    Hsv(Hsv),
    Code(String),
    Save,
}

fn message(event: Event) -> Message {
    Message::FolderColour(event)
}

/// A colour's code as six hex digits, with or without its `#`.
fn parse(code: &str) -> Option<u32> {
    let hex = code.trim().trim_start_matches('#');
    (hex.len() == 6)
        .then(|| u32::from_str_radix(hex, 16).ok())
        .flatten()
}

impl Shown {
    pub fn new(folder: String, name: Option<String>, colour: Option<u32>) -> Self {
        let name = name.unwrap_or_default();
        let shown = colour.unwrap_or(DEFAULT_COLOUR);
        Self {
            folder,
            start: (name.clone(), colour),
            name,
            colour,
            hsv: rgb_hsv(shown),
            code: format!("#{shown:06x}"),
        }
    }

    /// Whether Save would change anything.
    pub fn changed(&self) -> bool {
        let (name, colour) = &self.start;
        self.name.trim() != name.trim() || self.colour != *colour
    }

    /// The colour the pad and the swatches show.
    pub fn shown(&self) -> u32 {
        self.colour.unwrap_or(DEFAULT_COLOUR)
    }

    fn pick(&mut self, colour: Option<u32>) {
        self.colour = colour;
        let shown = self.shown();
        self.hsv = rgb_hsv(shown);
        self.code = format!("#{shown:06x}");
    }

    pub fn update(&mut self, event: Event) {
        match event {
            Event::Name(name) => self.name = name,
            Event::Preset(colour) => self.pick(Some(colour)),
            Event::Reset => self.pick(None),
            Event::Hsv(hsv) => {
                self.hsv = hsv;
                let colour = hsv_rgb(hsv);
                self.colour = Some(colour);
                self.code = format!("#{colour:06x}");
            }
            Event::Code(code) => {
                // A code that isn't one yet is kept as written.
                if let Some(colour) = parse(&code) {
                    self.colour = Some(colour);
                    self.hsv = rgb_hsv(colour);
                }
                self.code = code;
            }
            Event::Open(_) | Event::Close | Event::Save => {}
        }
    }

    /// What Save hands the core.
    pub fn action(&self) -> Action {
        let name = self.name.trim();
        Action::SetServerFolder {
            folder: self.folder.clone(),
            name: (!name.is_empty()).then(|| name.to_owned()),
            color: self.colour,
        }
    }
}

impl App {
    pub fn folder_colour_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Open(folder) => {
                let known = self
                    .client
                    .state()
                    .servers
                    .iter()
                    .filter_map(|server| server.folder.as_ref())
                    .find(|each| each.id == folder);
                let Some(known) = known else {
                    return Task::none();
                };
                self.folder_colour = Some(Shown::new(folder, known.name.clone(), known.color));
                operation::focus(NAME_FIELD)
            }
            Event::Close => {
                self.folder_colour = None;
                Task::none()
            }
            Event::Save => {
                let Some(shown) = self.folder_colour.take() else {
                    return Task::none();
                };
                if shown.changed() {
                    self.client.dispatch(shown.action());
                }
                self.sync()
            }
            event => {
                if let Some(shown) = &mut self.folder_colour {
                    shown.update(event);
                }
                Task::none()
            }
        }
    }
}

/// The dialog, while it's open.
pub fn view<'a>(shown: Option<&'a Shown>, ctx: Ctx<'a>) -> Option<Element<'a, Message>> {
    let shown = shown?;
    let theme = ctx.look.theme;
    let ink = theme.text;
    let chosen = shown.colour;
    let swatch = |colour: u32| -> Element<'a, Message> {
        let picked = chosen == Some(colour);
        let fill = hex(colour);
        button(Space::new())
            .width(SWATCH)
            .height(SWATCH)
            .on_press(message(Event::Preset(colour)))
            .style(move |_, status| button::Style {
                background: Some(fill.into()),
                border: Border {
                    color: ink,
                    width: if picked {
                        2.0
                    } else if lights(status) {
                        1.0
                    } else {
                        0.0
                    },
                    radius: 6.0.into(),
                },
                ..Default::default()
            })
            .into()
    };
    let swatches = column(PRESETS.chunks(10).map(|line| {
        row(line.iter().map(|&colour| swatch(colour)))
            .spacing(SWATCH_GAP)
            .into()
    }))
    .spacing(SWATCH_GAP);
    let pad = container(form::colour_card(
        theme,
        ColourPad::new(shown.hsv, |hsv| message(Event::Hsv(hsv))),
        form::field(theme, CODE_FIELD, "#rrggbb", &shown.code)
            .on_input(|code| message(Event::Code(code)))
            .on_submit(message(Event::Save)),
    ))
    .width(form::COLOUR_CARD);
    // How the folder shows on the rail, beside the pad.
    let tile_fill = hex(shown.shown());
    let tile = container(
        ctx.look
            .glyph(crate::pictures::Icon::Folder, 24, tile_fill)
            .boxed(48),
    )
    .style(move |_| {
        crate::style::rounded(
            iced::Color {
                a: 0.4,
                ..tile_fill
            },
            16,
        )
    });
    let body = vec![
        ctx.look
            .dialog_head(tr("Folder colour"), message(Event::Close)),
        form::heading(theme, tr("Folder name")),
        form::field(theme, NAME_FIELD, "", &shown.name)
            .padding([8.0, 10.0])
            .on_input(|name| message(Event::Name(name)))
            .on_submit(message(Event::Save))
            .into(),
        form::heading(theme, tr("Folder colour")),
        swatches.into(),
        row![pad, container(tile).center_x(Length::Fill)]
            .spacing(16)
            .into(),
        buttons([
            action(
                theme,
                tr("Reset"),
                Kind::Ghost,
                chosen.is_some().then(|| message(Event::Reset)),
            )
            .into(),
            action(
                theme,
                tr("Cancel"),
                Kind::Plain,
                Some(message(Event::Close)),
            )
            .into(),
            action(
                theme,
                tr("Save"),
                Kind::Primary,
                shown.changed().then(|| message(Event::Save)),
            )
            .into(),
        ]),
    ];
    Some(ctx.look.framed(body, WIDTH, message(Event::Close)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_on_the_folders_colour_or_discords_default() {
        let shown = Shown::new("discord-folder:7".into(), None, None);
        assert_eq!(
            (shown.shown(), shown.code.as_str()),
            (DEFAULT_COLOUR, "#5865f2")
        );
        assert!(!shown.changed());
        let shown = Shown::new(
            "discord-folder:7".into(),
            Some("Games".into()),
            Some(0xe74c3c),
        );
        assert_eq!(shown.code, "#e74c3c");
        assert_eq!(hsv_rgb(shown.hsv), 0xe74c3c);
    }

    #[test]
    fn a_preset_the_pad_or_a_code_colours_it_and_reset_goes_back() {
        let mut shown = Shown::new("discord-folder:7".into(), Some("Games".into()), None);
        shown.update(Event::Preset(0x1abc9c));
        assert_eq!(
            (shown.colour, shown.code.as_str()),
            (Some(0x1abc9c), "#1abc9c")
        );
        shown.update(Event::Hsv([0.0, 1.0, 1.0]));
        assert_eq!(
            (shown.colour, shown.code.as_str()),
            (Some(0xff0000), "#ff0000")
        );
        // A code half typed changes nothing until it's one.
        shown.update(Event::Code("#00f".into()));
        assert_eq!(
            (shown.colour, shown.code.as_str()),
            (Some(0xff0000), "#00f")
        );
        shown.update(Event::Code("0000ff".into()));
        assert_eq!(shown.colour, Some(0x0000ff));
        assert!(shown.changed());
        shown.update(Event::Reset);
        assert_eq!((shown.colour, shown.code.as_str()), (None, "#5865f2"));
        assert!(!shown.changed(), "back as it opened");
    }

    #[test]
    fn save_names_it_or_leaves_it_unnamed() {
        let mut shown = Shown::new("discord-folder:7".into(), Some("Games".into()), None);
        shown.update(Event::Name("  ".into()));
        shown.update(Event::Preset(0x2ecc71));
        assert!(matches!(
            shown.action(),
            Action::SetServerFolder { folder, name: None, color: Some(0x2ecc71) }
                if folder == "discord-folder:7"
        ));
        shown.update(Event::Name(" Work ".into()));
        assert!(matches!(
            shown.action(),
            Action::SetServerFolder { name: Some(name), .. } if name == "Work"
        ));
    }
}
