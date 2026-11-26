//! Appearance: the themes as small windows drawn in their own colours (a
//! click shows one at once and keeps it), New theme and each custom theme's
//! pencil opening the editor, whose changes show at once (Save keeps them
//! among the person's own themes, Cancel puts the theme from before back),
//! and the profile switches. See docs/themes.md.

use backcord_core::{
    application::Action,
    theme::{self as palette, Custom},
};
use iced::{
    Alignment, Border, Color, Element, Length,
    widget::{Space, button, column, container, row, stack, text},
};

use super::{Drop, Event as SettingsEvent, Page, Settings};
use crate::{
    app::Message,
    controls::{Kind, action, lights, placed},
    ctx::{Cx, Pref as Shown, Request},
    fonts::{BOLD, MEDIUM},
    form::{self, switch},
    pictures::Icon,
    themes::color,
    widgets::{ColourPad, Hsv, hsv_rgb, rgb_hsv},
};

/// A theme's tile: at least this wide, this tall, this far apart.
const TILE: f32 = 150.0;
const TILE_HEIGHT: f32 = 124.0;
const TILE_GAP: f32 = 12.0;
/// The editor's colours: two columns from this width, each row this tall.
const TWO_COLUMNS: f32 = 520.0;
const COLOUR_GAP: f32 = 24.0;
/// The name field.
const NAME: &str = "theme-name";
/// The open palette's code field.
const THEME_HEX: &str = "theme-colour-code";

/// The theme editor while it's open: the theme as edited, each colour's
/// text and whether it's wrong, its name, and what it started from.
pub struct Editor {
    /// The custom theme edited, by id; none for a new one.
    pub editing: Option<String>,
    pub base: &'static str,
    pub theme: palette::Theme,
    pub name: String,
    /// Each colour's field, in `Theme::NAMES`' order: its name as
    /// `layout.json` has it, its text and whether it's a colour.
    pub fields: Vec<(String, String, bool)>,
    /// The theme a new one started as, among `palette::starts`.
    pub start: usize,
    /// Delete was pressed once; the next press deletes.
    pub confirming: bool,
    /// The colour whose palette is open under its swatch, and where its
    /// pad stands.
    pub picking: Option<usize>,
    pub hsv: Hsv,
}

impl Editor {
    fn new(
        editing: Option<String>,
        base: &'static str,
        theme: palette::Theme,
        name: String,
    ) -> Self {
        let mut editor = Self {
            editing,
            base,
            theme,
            name,
            fields: Vec::new(),
            start: 0,
            confirming: false,
            picking: None,
            hsv: [0.0; 3],
        };
        editor.fill();
        editor
    }

    /// Every field's text from the theme's colours.
    fn fill(&mut self) {
        self.fields = palette::Theme::NAMES
            .iter()
            .enumerate()
            .filter_map(|(index, name)| {
                Some((
                    palette::token(name),
                    palette::hex(self.theme.colour(index)?),
                    false,
                ))
            })
            .collect();
    }

    /// Takes a colour's text as typed: the theme takes it if it's a colour.
    /// Returns whether the theme changed.
    fn edit(&mut self, index: usize, text: String) -> bool {
        let parsed = palette::parse(&text);
        let Some(field) = self.fields.get_mut(index) else {
            return false;
        };
        (field.1, field.2) = (text, parsed.is_none());
        match (parsed, self.theme.colour_mut(index)) {
            (Some(colour), Some(slot)) if *slot != colour => {
                *slot = colour;
                true
            }
            _ => false,
        }
    }

    fn valid(&self) -> bool {
        self.fields.iter().all(|(_, _, wrong)| !wrong)
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    Choose(String),
    New,
    Edit(String),
    Start(usize),
    Colour(usize, String),
    /// A colour's swatch clicked: its palette opens, or, for none, closes.
    Pick(Option<usize>),
    /// The open palette's pad moved.
    Hsv(Hsv),
    Dark(bool),
    Name(String),
    Save,
    Cancel,
    Delete,
    Profile(Cosmetic, bool),
    /// A theme file: picked to import, its text (none when no file was
    /// picked); the chosen theme to export, and what came of it.
    Import,
    /// A file picked to import: its name and its text.
    Imported(Option<(String, String)>),
    Export,
    Exported(Option<String>),
    /// Themes' colours: a swatch clicked (none closes its palette), the
    /// pad moved, the code typed.
    QuickPick(Option<usize>),
    QuickHsv(Hsv),
    QuickCode(String),
}

/// Themes' colours, each a part of the window anyone can point at: its
/// colour's name in `Theme::NAMES`, what it's called and what it colours.
const QUICK: [(&str, &str, &str); 14] = [
    (
        "rail",
        "Server list",
        "The column of server icons on the far left.",
    ),
    (
        "sidebar",
        "Channels and direct messages",
        "The list beside the servers.",
    ),
    ("chat", "Chat background", "Behind the messages."),
    (
        "panel",
        "Side panels",
        "The member list, the profile beside a DM, and Settings.",
    ),
    (
        "raised",
        "Cards and boxes",
        "Embeds, cards and what stands over the background.",
    ),
    (
        "field",
        "Message box and fields",
        "Where you write, and the search fields.",
    ),
    (
        "hover",
        "Highlight under the pointer",
        "A row or button the pointer is over.",
    ),
    ("text", "Text", "Names and messages."),
    ("muted", "Secondary text", "Times, hints and descriptions."),
    (
        "accent",
        "Main colour",
        "Main buttons, switches, sliders and what's chosen.",
    ),
    ("link", "Links", "Web addresses in messages."),
    (
        "live",
        "Online and connected",
        "The online dot, and voice connected.",
    ),
    ("danger", "Danger", "Delete, leave, and errors."),
    ("line", "Lines", "The lines between the window's parts."),
];

/// Where `name` is among a theme's colours.
fn quick_index(name: &str) -> Option<usize> {
    palette::Theme::NAMES
        .iter()
        .position(|known| *known == name)
}

/// Sets the chosen theme's colour `index` and keeps it: a custom theme
/// changes; a built-in one is copied first as the person's own, named
/// after it, and that copy is chosen.
fn set_quick(cx: &mut Cx<'_>, index: usize, colour: palette::Colour) {
    let (id, theme) = palette::chosen(cx.themes.chosen.as_deref(), &cx.themes.custom);
    let id = id.to_owned();
    if theme.colour(index) == Some(colour) {
        return;
    }
    let target = if cx.themes.custom.iter().any(|custom| custom.id == id) {
        id
    } else {
        let built = palette::BUILT_IN
            .iter()
            .find(|built| built.id == id)
            .unwrap_or(&palette::BUILT_IN[0]);
        let new = palette::new_id(&cx.themes.custom);
        cx.themes.custom.push(Custom {
            id: new.clone(),
            name: format!("{} ({})", built.name, backcord_iced_kit::i18n::tr("mine")),
            base: built.id,
            theme,
        });
        new
    };
    if let Some(slot) = cx
        .themes
        .custom
        .iter_mut()
        .find(|custom| custom.id == target)
        .and_then(|custom| custom.theme.colour_mut(index))
    {
        *slot = colour;
    }
    cx.keep("custom_themes", &cx.themes.custom);
    choose(cx, &target);
}

/// Appearance's profile switches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cosmetic {
    Effects,
    Decorations,
    Nameplates,
    Moving,
    NameStyles,
    ServerTags,
    Frames,
}

fn theme_message(event: Event) -> Message {
    Message::Settings(SettingsEvent::Theme(event))
}

impl Settings {
    pub(super) fn theme_event(&mut self, event: Event, cx: &mut Cx<'_>) -> iced::Task<Message> {
        match event {
            Event::Import => return import_theme(),
            Event::Export => return export_theme(cx),
            Event::Imported(text) => return self.imported_theme(text, cx),
            Event::Exported(note) => {
                if note.is_some() {
                    self.theme_note = note;
                }
            }
            event => self.theme_change(event, cx),
        }
        iced::Task::none()
    }

    /// Adds the themes in an imported file among the person's own, and
    /// chooses the last: one theme as Export writes it, a list of them, or
    /// a whole `layout.json`.
    fn imported_theme(
        &mut self,
        text: Option<(String, String)>,
        cx: &mut Cx<'_>,
    ) -> iced::Task<Message> {
        let Some(text) = text else {
            return iced::Task::none();
        };
        let (file_name, text) = text;
        let themes = palette::import(&text, &file_name);
        if themes.is_empty() {
            self.theme_note =
                Some(backcord_iced_kit::i18n::tr("This file is not a BACKCORD theme.").to_owned());
            return iced::Task::none();
        }
        let mut names = Vec::new();
        let mut last = String::new();
        for mut theme in themes {
            theme.id = palette::new_id(&cx.themes.custom);
            theme.name = theme
                .name
                .chars()
                .filter(|c| !c.is_control() && !backcord_core::links::hidden(*c))
                .take(64)
                .collect();
            if theme.name.trim().is_empty() {
                backcord_iced_kit::i18n::tr("Imported theme").clone_into(&mut theme.name);
            }
            names.push(theme.name.clone());
            last.clone_from(&theme.id);
            cx.themes.custom.push(theme);
        }
        cx.keep("custom_themes", &cx.themes.custom);
        choose(cx, &last);
        self.theme_note = Some(format!(
            "{} {}",
            backcord_iced_kit::i18n::tr("Theme imported:"),
            names.join(", ")
        ));
        // The look Export wrote beside the colours, if the file has it.
        let look = serde_json::from_str::<serde_json::Value>(text.trim_start_matches('\u{feff}'))
            .ok()
            .and_then(|mut value| value.get_mut("appearance").map(serde_json::Value::take));
        let Some(look) = look else {
            return iced::Task::none();
        };
        let number = |key: &str| look.get(key).and_then(serde_json::Value::as_u64);
        let prefs = &mut cx.layout.prefs;
        if let Some(value) = number("neon_glow") {
            let percent = value.min(100) as u8;
            prefs.neon_glow = Some(percent);
            backcord_iced_kit::style::set_glow(percent);
            cx.keep("neon_glow", percent);
        }
        let prefs = &mut cx.layout.prefs;
        if let Some(value) = number("corners") {
            let percent = value.min(200) as u8;
            prefs.corners = Some(percent);
            backcord_iced_kit::renderer::set_corners(u32::from(percent));
            cx.keep("corners", percent);
        }
        let prefs = &mut cx.layout.prefs;
        if let Some(value) = number("round") {
            let percent = value.min(100) as u8;
            prefs.round = Some(percent);
            backcord_iced_kit::renderer::set_rounds(u32::from(percent));
            cx.keep("round", percent);
        }
        crate::backdrop::look_import(cx.client.root(), &look)
    }

    fn theme_change(&mut self, event: Event, cx: &mut Cx<'_>) {
        match event {
            Event::Import | Event::Imported(_) | Event::Export | Event::Exported(_) => {}
            Event::Choose(id) => choose(cx, &id),
            Event::New => self.new_theme(cx),
            Event::Edit(id) => {
                let Some(custom) = cx.themes.custom.iter().find(|custom| custom.id == id) else {
                    return;
                };
                let editor = Editor::new(
                    Some(custom.id.clone()),
                    custom.base,
                    custom.theme,
                    custom.name.clone(),
                );
                self.open_editor(editor, cx);
            }
            Event::Start(index) => {
                let starts: Vec<_> = palette::starts(&cx.themes.custom)
                    .map(|start| (start.base, start.theme, copy_name(start.name)))
                    .collect();
                let Some(editor) = self.editor.as_mut() else {
                    return;
                };
                let Some((base, theme, name)) = starts.get(index).cloned() else {
                    return;
                };
                // It takes the new start's name too, unless the person
                // named it.
                if starts
                    .get(editor.start)
                    .is_some_and(|before| before.2 == editor.name)
                {
                    editor.name = name;
                }
                editor.base = base;
                editor.theme = theme;
                editor.start = index;
                editor.fill();
                self.open = None;
                self.show_edited(cx);
            }
            Event::Colour(index, text) => {
                let changed = self.editor.as_mut().is_some_and(|editor| {
                    // A code typed for the open palette moves its pad there.
                    if editor.picking == Some(index)
                        && let Some(colour) = palette::parse(&text)
                    {
                        editor.hsv = rgb_hsv(colour.rgb_part());
                    }
                    editor.edit(index, text)
                });
                if changed {
                    self.show_edited(cx);
                }
            }
            Event::QuickPick(index) => {
                let (_, chosen) = palette::chosen(cx.themes.chosen.as_deref(), &cx.themes.custom);
                self.quick = index.and_then(|index| {
                    let colour = chosen.colour(index)?;
                    Some((index, rgb_hsv(colour.rgb_part()), palette::hex(colour)))
                });
                self.open = None;
            }
            Event::QuickHsv(hsv) => {
                if let Some((index, pad, code)) = self.quick.as_mut() {
                    let index = *index;
                    *pad = hsv;
                    let (_, chosen) =
                        palette::chosen(cx.themes.chosen.as_deref(), &cx.themes.custom);
                    let alpha = chosen.colour(index).map_or(0xff, |colour| colour.alpha());
                    let colour = palette::Colour::rgba((hsv_rgb(hsv) << 8) | u32::from(alpha));
                    *code = palette::hex(colour);
                    set_quick(cx, index, colour);
                }
            }
            Event::QuickCode(text) => {
                if let Some((index, pad, code)) = self.quick.as_mut() {
                    let index = *index;
                    *code = text;
                    if let Some(colour) = palette::parse(code) {
                        *pad = rgb_hsv(colour.rgb_part());
                        set_quick(cx, index, colour);
                    }
                }
            }
            Event::Pick(index) => {
                if let Some(editor) = self.editor.as_mut() {
                    editor.picking = index;
                    if let Some(colour) = index.and_then(|index| editor.theme.colour(index)) {
                        editor.hsv = rgb_hsv(colour.rgb_part());
                    }
                }
                self.open = None;
            }
            Event::Hsv(hsv) => {
                let changed = self.editor.as_mut().is_some_and(|editor| {
                    let Some(index) = editor.picking else {
                        return false;
                    };
                    editor.hsv = hsv;
                    // The pad chooses the colour; its alpha stays.
                    let alpha = editor
                        .theme
                        .colour(index)
                        .map_or(0xff, |colour| colour.alpha());
                    let rgb = hsv_rgb(hsv);
                    let colour = palette::Colour::rgba((rgb << 8) | u32::from(alpha));
                    editor.edit(index, palette::hex(colour))
                });
                if changed {
                    self.show_edited(cx);
                }
            }
            Event::Dark(dark) => {
                if let Some(editor) = self.editor.as_mut() {
                    editor.theme.dark = dark;
                }
                self.show_edited(cx);
            }
            Event::Name(name) => {
                if let Some(editor) = self.editor.as_mut() {
                    editor.name = name;
                }
            }
            Event::Save => self.save_theme(cx),
            Event::Cancel => self.cancel_theme(cx),
            Event::Delete => self.delete_theme(cx),
            Event::Profile(cosmetic, on) => self::cosmetic(cx, cosmetic, on),
        }
    }

    /// Opens the editor on a copy of the theme shown, which can start from
    /// any other.
    fn new_theme(&mut self, cx: &mut Cx<'_>) {
        let current = &cx.theme.id;
        let starts: Vec<_> = palette::starts(&cx.themes.custom)
            .map(|start| {
                (
                    start.id.to_owned(),
                    start.base,
                    start.theme,
                    copy_name(start.name),
                )
            })
            .collect();
        let start = starts
            .iter()
            .position(|(id, ..)| id == current)
            .unwrap_or(0);
        let (_, base, theme, name) = starts[start].clone();
        let mut editor = Editor::new(None, base, theme, name);
        editor.start = start;
        self.open_editor(editor, cx);
    }

    fn open_editor(&mut self, editor: Editor, cx: &mut Cx<'_>) {
        self.editor = Some(editor);
        self.show_edited(cx);
    }

    /// The window in the theme as edited, without keeping it.
    fn show_edited(&mut self, cx: &mut Cx<'_>) {
        let Some(editor) = &self.editor else {
            return;
        };
        let id = editor.editing.as_deref().unwrap_or("editing");
        self.preview = Some(crate::themes::theme(id, &editor.theme));
        cx.request(Request::Changed(Shown::Theme));
    }

    /// Closes the editor: the window shows the theme chosen again.
    fn close_editor(&mut self, cx: &mut Cx<'_>) -> Option<Editor> {
        let editor = self.editor.take()?;
        if self.preview.take().is_some() {
            cx.request(Request::Changed(Shown::Theme));
        }
        Some(editor)
    }

    /// Keeps the theme as edited among the person's own, and chooses it.
    fn save_theme(&mut self, cx: &mut Cx<'_>) {
        if !self.editor.as_ref().is_some_and(Editor::valid) {
            return;
        }
        let Some(editor) = self.close_editor(cx) else {
            return;
        };
        let name = match editor.name.trim() {
            "" => backcord_iced_kit::i18n::tr("My theme").to_owned(),
            name => name.to_owned(),
        };
        let custom = &mut cx.themes.custom;
        let saved = editor
            .editing
            .as_ref()
            .and_then(|id| custom.iter_mut().find(|custom| custom.id == *id));
        let id = match saved {
            Some(saved) => {
                saved.name = name;
                saved.theme = editor.theme;
                saved.id.clone()
            }
            None => {
                let id = palette::new_id(custom);
                custom.push(Custom {
                    id: id.clone(),
                    name,
                    base: editor.base,
                    theme: editor.theme,
                });
                id
            }
        };
        cx.keep("custom_themes", &cx.themes.custom);
        choose(cx, &id);
    }

    /// Closes the editor and shows the theme from before it opened.
    pub(super) fn cancel_theme(&mut self, cx: &mut Cx<'_>) {
        self.close_editor(cx);
    }

    /// Deletes the theme edited; if it was the chosen one, BACKDOOR shows.
    fn delete_theme(&mut self, cx: &mut Cx<'_>) {
        let Some(editor) = self.editor.as_mut() else {
            return;
        };
        if !editor.confirming {
            editor.confirming = true;
            return;
        }
        let Some(editor) = self.close_editor(cx) else {
            return;
        };
        if let Some(id) = editor.editing {
            cx.themes.custom.retain(|custom| custom.id != id);
            cx.keep("custom_themes", &cx.themes.custom);
        }
        let (id, _) = palette::chosen(cx.themes.chosen.as_deref(), &cx.themes.custom);
        let id = id.to_owned();
        choose(cx, &id);
    }
}

/// Asks for a theme file to import; its text follows.
fn import_theme() -> iced::Task<Message> {
    iced::Task::perform(
        async {
            let file = rfd::AsyncFileDialog::new()
                .set_title(backcord_iced_kit::i18n::tr("Import theme"))
                .add_filter("BACKCORD", &["json"])
                .pick_file()
                .await?;
            // A theme is a few kilobytes: a bigger file isn't one.
            // With its background and icon pictures in it, a few dozen MiB.
            const MOST: usize = 160 << 20;
            let bytes = file.read().await;
            if bytes.len() > MOST {
                return Some((file.file_name(), String::new()));
            }
            Some((
                file.file_name(),
                String::from_utf8_lossy(&bytes).into_owned(),
            ))
        },
        |text| theme_message(Event::Imported(text)),
    )
}

/// Saves the chosen theme, built-in or the person's own, where they say:
/// its name, the theme it began as and every colour, as `layout.json`
/// keeps a custom theme.
fn export_theme(cx: &Cx<'_>) -> iced::Task<Message> {
    let (id, theme) = palette::chosen(cx.themes.chosen.as_deref(), &cx.themes.custom);
    let custom = match cx.themes.custom.iter().find(|custom| custom.id == id) {
        Some(custom) => custom.clone(),
        None => {
            let built_in = palette::find(id);
            Custom {
                id: "custom:1".to_owned(),
                name: built_in
                    .map_or("BACKDOOR", |built_in| built_in.name)
                    .to_owned(),
                base: built_in.map_or("backdoor", |built_in| built_in.id),
                theme,
            }
        }
    };
    // The theme's colours, and the rest of how the window looks with it:
    // roundness, neon, menus, the background and DM icon pictures.
    let Ok(mut value) = serde_json::to_value(&custom) else {
        return iced::Task::none();
    };
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "appearance".to_owned(),
            crate::backdrop::look_export(&cx.layout.prefs),
        );
    }
    let Ok(json) = serde_json::to_string_pretty(&value) else {
        return iced::Task::none();
    };
    let stem: String = custom
        .name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let stem = match stem.trim() {
        "" => "tema".to_owned(),
        stem => stem.to_owned(),
    };
    iced::Task::perform(
        async move {
            let file = rfd::AsyncFileDialog::new()
                .set_title(backcord_iced_kit::i18n::tr("Export theme"))
                .set_file_name(format!("{stem}.json"))
                .add_filter("BACKCORD", &["json"])
                .save_file()
                .await?;
            Some(match std::fs::write(file.path(), json) {
                Ok(()) => format!(
                    "{} {}",
                    backcord_iced_kit::i18n::tr("Theme exported to"),
                    file.path().display()
                ),
                Err(error) => format!(
                    "{}: {error}",
                    backcord_iced_kit::i18n::tr("Could not export the theme")
                ),
            })
        },
        |note| theme_message(Event::Exported(note)),
    )
}

/// Chooses the theme `id` and keeps the choice, if there's such a theme:
/// the window shows it.
fn choose(cx: &mut Cx<'_>, id: &str) {
    let Some(theme) = cx.themes.find(id) else {
        return;
    };
    cx.themes.choose(cx.client.root(), &theme.id);
    cx.request(Request::Changed(Shown::Theme));
}

/// A profile switch: kept, and the core told what may now show.
fn cosmetic(cx: &mut Cx<'_>, cosmetic: Cosmetic, on: bool) {
    let prefs = &mut cx.layout.prefs;
    let (slot, key) = match cosmetic {
        Cosmetic::Effects => (&mut prefs.profile_effects, "profile_effects"),
        Cosmetic::Decorations => (&mut prefs.decorations, "decorations"),
        Cosmetic::Nameplates => (&mut prefs.nameplates, "nameplates"),
        Cosmetic::Moving => (&mut prefs.moving_cosmetics, "moving_cosmetics"),
        Cosmetic::NameStyles => (&mut prefs.name_styles, "name_styles"),
        Cosmetic::ServerTags => (&mut prefs.server_tags, "server_tags"),
        Cosmetic::Frames => (&mut prefs.profile_frames, "profile_frames"),
    };
    *slot = Some(on);
    cx.keep(key, on);
    if cosmetic == Cosmetic::Nameplates {
        cx.dispatch(Action::HideNameplates(!on));
    } else {
        let cosmetics = cx.layout.prefs.cosmetics();
        cx.dispatch(Action::SetCosmetics(cosmetics));
    }
    cx.request(Request::Changed(Shown::Roster));
}

impl<'a> Page<'a> {
    pub(super) fn appearance_page(self) -> Element<'a, Message> {
        let settings = self.s;
        let theme = self.cx.look.theme;
        if let Some(editor) = &settings.editor {
            return self.editor_view(editor, settings.open == Some(Drop::Start));
        }
        let tiles = self.theme_tiles();
        let tiles: Element<'a, Message> = match &settings.theme_note {
            Some(note) => column![
                tiles,
                text(backcord_iced_kit::i18n::t(note))
                    .size(13)
                    .color(Color {
                        a: 0.7,
                        ..theme.text
                    }),
            ]
            .spacing(8)
            .into(),
            None => tiles,
        };
        let ink = crate::theme::rgb(theme.text);
        let theme_buttons = row![
            action(
                theme,
                backcord_iced_kit::i18n::tr("Import"),
                Kind::Plain,
                Some(theme_message(Event::Import))
            )
            .small()
            .icon(self.cx.look.icon(Icon::Download, 16, ink)),
            action(
                theme,
                backcord_iced_kit::i18n::tr("Export"),
                Kind::Plain,
                Some(theme_message(Event::Export))
            )
            .small()
            .icon(self.cx.look.icon(Icon::File, 16, ink)),
            action(
                theme,
                backcord_iced_kit::i18n::tr("New theme"),
                Kind::Plain,
                Some(theme_message(Event::New))
            )
            .small()
            .icon(self.cx.look.icon(Icon::Plus, 16, ink)),
        ]
        .spacing(8);
        let prefs = &self.cx.layout.prefs;
        let cosmetics = prefs.cosmetics();
        let profile = |label, description, on: bool, which| {
            switch(label, on, Some(theme_message(Event::Profile(which, !on))))
                .description(description)
                .view(self.cx.look)
        };
        if settings.section() == super::Section::Appearance {
            return form::page([
                form::title(theme, backcord_iced_kit::i18n::tr("Themes")),
                form::group([
                    form::beside(
                        form::heading(theme, backcord_iced_kit::i18n::tr("THEME")),
                        theme_buttons,
                    ),
                    tiles,
                ]),
                self.neon_glow_slider(),
                self.corners_slider(),
                self.round_slider(),
                crate::backdrop::icon_settings(self.cx),
                crate::backdrop::menu_settings(self.cx),
                crate::backdrop::settings(self.cx),
                self.island_settings(),
                self.quick_colours(),
            ]);
        }
        form::page([
            form::title(theme, backcord_iced_kit::i18n::tr("Performance")),
            self.title_bar_switch(),
            column![
                form::heading(theme, backcord_iced_kit::i18n::tr("PROFILES")),
                profile(
                    backcord_iced_kit::i18n::tr("Play profile effects"),
                    backcord_iced_kit::i18n::tr("The animations some people put over their profile cards."),
                    cosmetics.effects,
                    Cosmetic::Effects,
                ),
                profile(
                    backcord_iced_kit::i18n::tr("Show avatar decorations"),
                    backcord_iced_kit::i18n::tr("The pictures around people's avatars."),
                    cosmetics.decorations,
                    Cosmetic::Decorations,
                ),
                profile(
                    backcord_iced_kit::i18n::tr("Show nameplates"),
                    backcord_iced_kit::i18n::tr("The pictures Discord members equip behind their rows in the member list and on your account card."),
                    prefs.nameplates.unwrap_or(true),
                    Cosmetic::Nameplates,
                ),
                profile(
                    backcord_iced_kit::i18n::tr("Animate decorations and nameplates"),
                    backcord_iced_kit::i18n::tr("They play when pointed at, as in Discord. Off, they hold still."),
                    cosmetics.moving,
                    Cosmetic::Moving,
                ),
                profile(
                    backcord_iced_kit::i18n::tr("Show name styles"),
                    backcord_iced_kit::i18n::tr("The fonts and colours people give their display names."),
                    cosmetics.name_styles,
                    Cosmetic::NameStyles,
                ),
                profile(
                    backcord_iced_kit::i18n::tr("Show server tags"),
                    backcord_iced_kit::i18n::tr("The tag and its badge beside people's names."),
                    cosmetics.server_tags,
                    Cosmetic::ServerTags,
                ),
                profile(
                    backcord_iced_kit::i18n::tr("Show profile frames"),
                    backcord_iced_kit::i18n::tr("The frames some people put around their profile cards."),
                    cosmetics.frames,
                    Cosmetic::Frames,
                ),
            ]
            .spacing(12)
            .into(),
        ])
    }

    /// The themes as tiles, as many 150 px or wider across as fit.
    fn theme_tiles(self) -> Element<'a, Message> {
        iced::widget::responsive(move |size| {
            let columns = ((size.width + TILE_GAP) / (TILE + TILE_GAP))
                .floor()
                .max(1.0) as usize;
            let width = (size.width - (columns - 1) as f32 * TILE_GAP) / columns as f32;
            let cards: Vec<(String, String, bool, palette::Theme)> =
                palette::BUILT_IN
                    .iter()
                    .map(|built_in| {
                        (
                            built_in.id.to_owned(),
                            built_in.name.to_owned(),
                            false,
                            built_in.theme,
                        )
                    })
                    // The person's own after them, each with its pencil.
                    .chain(
                        self.cx.themes.custom.iter().map(|custom| {
                            (custom.id.clone(), custom.name.clone(), true, custom.theme)
                        }),
                    )
                    .collect();
            let mut rows = column![].spacing(TILE_GAP);
            for chunk in cards.chunks(columns) {
                let mut line = row![].spacing(TILE_GAP);
                for (id, name, custom, palette) in chunk {
                    line = line.push(self.theme_tile(id, name, *custom, palette, width));
                }
                rows = rows.push(line);
            }
            rows.into()
        })
        .height(Length::Shrink)
        .into()
    }

    /// A theme to choose: a small window in its colours, its name under it,
    /// an accent ring and a tick on the one shown, and a pencil on the
    /// person's own.
    fn theme_tile(
        self,
        id: &str,
        name: &str,
        custom: bool,
        palette: &palette::Theme,
        width: f32,
    ) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let current = self.cx.themes.chosen_id() == id;
        let preview = preview(palette, width - 16.0);
        let mut label = row![
            text(name.to_owned())
                .size(14)
                .font(if current { BOLD } else { MEDIUM })
                .color(if current { theme.text } else { theme.secondary })
                .wrapping(text::Wrapping::None)
                .width(Length::Fill)
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .height(24);
        if current {
            label = label.push(self.cx.look.glyph(Icon::Check, 16, theme.accent));
        }
        let (accent, faint, line, hover) = (theme.accent, theme.faint, theme.line, theme.hover);
        let face = button(
            column![preview, container(label).padding([0, 2])]
                .spacing(6)
                .padding(8),
        )
        .padding(0)
        .width(width)
        .height(TILE_HEIGHT)
        .on_press(theme_message(Event::Choose(id.to_owned())))
        .style(move |_, status| {
            let lit = lights(status);
            button::Style {
                background: lit.then(|| hover.into()),
                border: Border {
                    color: if current {
                        accent
                    } else if lit {
                        faint
                    } else {
                        line
                    },
                    width: if current { 2.0 } else { 1.0 },
                    radius: 10.0.into(),
                },
                ..Default::default()
            }
        });
        if !custom {
            return face.into();
        }
        let pencil = self
            .cx
            .look
            .icon_button(
                Icon::Pencil,
                backcord_iced_kit::i18n::tr("Edit"),
                theme_message(Event::Edit(id.to_owned())),
            )
            .size(24.0, 14);
        stack![face, placed(pencil, width - 8.0 - 24.0, 8.0 + 78.0 + 6.0)].into()
    }

    /// Themes' colours: each part of the window by a name anyone knows,
    /// with what it colours and its swatch, whose palette opens under it;
    /// a change shows at once and is kept.
    fn quick_colours(self) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let (_, chosen) = palette::chosen(self.cx.themes.chosen.as_deref(), &self.cx.themes.custom);
        let mut rows = column![].spacing(10);
        for (name, label, about) in QUICK {
            let Some(index) = quick_index(name) else {
                continue;
            };
            let shown = chosen.colour(index).map_or(Color::TRANSPARENT, color);
            let open = self.s.quick.as_ref().filter(|(at, _, _)| *at == index);
            let edge = theme.edge;
            let swatch = button(Space::new())
                .width(32)
                .height(32)
                .padding(0)
                .on_press(theme_message(Event::QuickPick(
                    open.is_none().then_some(index),
                )))
                .style(move |_, _| button::Style {
                    background: Some(shown.into()),
                    border: Border {
                        color: edge,
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                });
            rows = rows.push(
                row![
                    swatch,
                    column![
                        text(backcord_iced_kit::i18n::tr(label))
                            .size(14)
                            .font(MEDIUM)
                            .color(theme.text),
                        text(backcord_iced_kit::i18n::tr(about))
                            .size(12)
                            .color(theme.muted),
                    ]
                    .spacing(2)
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            );
            if let Some((_, pad, code)) = open {
                rows = rows.push(form::colour_card(
                    theme,
                    ColourPad::new(*pad, |hsv| theme_message(Event::QuickHsv(hsv))),
                    form::field(theme, THEME_HEX, "#rrggbb", code)
                        .on_input(|text| theme_message(Event::QuickCode(text)))
                        .on_submit(theme_message(Event::QuickPick(None))),
                ));
            }
        }
        form::group([
            form::heading(theme, backcord_iced_kit::i18n::tr("COLOURS")),
            form::note(
                theme,
                backcord_iced_kit::i18n::tr(
                    "Click a colour to open its palette. A change shows at once; a built-in theme is copied as your own first, so it's never lost.",
                ),
            ),
            rows.into(),
        ])
    }

    /// Settings' Performance: the title bar and the profiles' extras.
    pub(super) fn performance_page(self) -> Element<'a, Message> {
        self.appearance_page()
    }

    /// The editor: the name with Save and Cancel, what a new theme starts
    /// from, Dark, each colour's field, and Delete for a saved one.
    fn editor_view(self, editor: &'a Editor, starts_open: bool) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let mut parts = vec![
            form::title(theme, backcord_iced_kit::i18n::tr("Appearance")),
            form::group([
                form::heading(
                    theme,
                    if editor.editing.is_some() {
                        backcord_iced_kit::i18n::tr("EDIT THEME")
                    } else {
                        backcord_iced_kit::i18n::tr("NEW THEME")
                    },
                ),
                row![
                    form::field(
                        theme,
                        NAME,
                        backcord_iced_kit::i18n::tr("Name"),
                        &editor.name
                    )
                    .on_input(|name| theme_message(Event::Name(name)))
                    .on_submit(theme_message(Event::Save)),
                    action(
                        theme,
                        backcord_iced_kit::i18n::tr("Save"),
                        Kind::Primary,
                        editor.valid().then(|| theme_message(Event::Save))
                    ),
                    action(
                        theme,
                        backcord_iced_kit::i18n::tr("Cancel"),
                        Kind::Plain,
                        Some(theme_message(Event::Cancel))
                    )
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .into(),
            ]),
        ];
        if editor.editing.is_none() {
            let names: Vec<String> = palette::starts(&self.cx.themes.custom)
                .map(|start| start.name.to_owned())
                .collect();
            parts.push(form::group([
                form::heading(theme, backcord_iced_kit::i18n::tr("START FROM")),
                form::dropdown(
                    names,
                    Some(editor.start),
                    starts_open,
                    Message::Settings(SettingsEvent::Drop(Drop::Start)),
                    |index| theme_message(Event::Start(index)),
                )
                .view(self.cx.look),
            ]));
        }
        parts.push(
            switch(
                backcord_iced_kit::i18n::tr("Dark"),
                editor.theme.dark,
                Some(theme_message(Event::Dark(!editor.theme.dark))),
            )
            .description(backcord_iced_kit::i18n::tr(
                "Drop-down lists and scroll bars draw dark to match.",
            ))
            .view(self.cx.look),
        );
        let colours = iced::widget::responsive(move |size| {
            let columns = if size.width >= TWO_COLUMNS { 2 } else { 1 };
            let per = editor.fields.len().div_ceil(columns);
            let mut line = row![].spacing(COLOUR_GAP);
            for column_index in 0..columns {
                let mut cells = column![];
                for index in column_index * per..((column_index + 1) * per).min(editor.fields.len())
                {
                    let (name, hex, wrong) = &editor.fields[index];
                    let shown = editor.theme.colour(index).map_or(Color::TRANSPARENT, color);
                    let open = editor.picking == Some(index);
                    let card = open.then(|| {
                        form::colour_card(
                            theme,
                            ColourPad::new(editor.hsv, |hsv| theme_message(Event::Hsv(hsv))),
                            form::field(theme, THEME_HEX, "#rrggbb", hex)
                                .on_input(move |text| theme_message(Event::Colour(index, text)))
                                .on_submit(theme_message(Event::Pick(None))),
                        )
                    });
                    cells = cells.push(form::colour_row(
                        theme,
                        name,
                        shown,
                        hex,
                        *wrong,
                        move |text| theme_message(Event::Colour(index, text)),
                        theme_message(Event::Pick((!open).then_some(index))),
                        card,
                        theme_message(Event::Pick(None)),
                    ));
                }
                line = line.push(container(cells).width(Length::FillPortion(1)));
            }
            line.into()
        })
        .height(Length::Shrink);
        parts.push(form::group([
            form::heading(theme, backcord_iced_kit::i18n::tr("COLOURS")),
            form::note(
                theme,
                backcord_iced_kit::i18n::tr(
                    "Click a swatch to pick its colour, or type it as #rgb, #rrggbb or #rrggbbaa. The window shows a change as you make it.",
                ),
            ),
            colours.into(),
        ]));
        if editor.editing.is_some() {
            parts.push(form::buttons([action(
                theme,
                if editor.confirming {
                    backcord_iced_kit::i18n::tr("Delete it for good")
                } else {
                    backcord_iced_kit::i18n::tr("Delete theme")
                },
                Kind::Danger,
                Some(theme_message(Event::Delete)),
            )
            .small()
            .icon(
                self.cx
                    .look
                    .icon(Icon::Trash, 16, crate::theme::rgb(theme.on_danger)),
            )
            .into()]));
        }
        form::page(parts)
    }
}

/// What a theme copied from another is called until it's named.
fn copy_name(name: &str) -> String {
    format!("{name} copy")
}

/// A theme's small window, 78 px tall: the rail with three servers, the
/// sidebar's channels, two messages and the composer, in its colours.
fn preview<'a>(palette: &palette::Theme, width: f32) -> Element<'a, Message> {
    let c = |colour| color(colour);
    let (rail, sidebar, chat, raised, accent, ink, muted) = (
        c(palette.rail),
        c(palette.sidebar),
        c(palette.chat),
        c(palette.raised),
        c(palette.accent),
        c(palette.text),
        c(palette.muted),
    );
    let blob = |w: f32, h: f32, colour: Color, radius: f32| -> Element<'a, Message> {
        container(Space::new())
            .width(w)
            .height(h)
            .style(move |_| crate::style::rounded(colour, radius))
            .into()
    };
    let mut layers = stack![blob(width, 78.0, chat, 6.0)];
    layers = layers.push(blob(16.0, 78.0, rail, 0.0));
    for index in 0..3 {
        let (colour, radius) = if index == 0 {
            (accent, 3.0)
        } else {
            (raised, 5.0)
        };
        layers = layers.push(placed(
            blob(10.0, 10.0, colour, radius),
            3.0,
            6.0 + index as f32 * 13.0,
        ));
    }
    layers = layers.push(placed(blob(38.0, 78.0, sidebar, 0.0), 16.0, 0.0));
    for index in 0..4 {
        let w = if index == 1 {
            24.0
        } else {
            18.0 + (index % 2) as f32 * 4.0
        };
        let colour = if index == 1 { ink } else { muted };
        layers = layers.push(placed(
            blob(w, 4.0, colour, 2.0),
            22.0,
            8.0 + index as f32 * 11.0,
        ));
    }
    let room = width - 66.0;
    for index in 0..2 {
        let y = 8.0 + index as f32 * 22.0;
        let first = index == 0;
        layers = layers
            .push(placed(
                blob(10.0, 10.0, if first { accent } else { muted }, 5.0),
                60.0,
                y,
            ))
            .push(placed(
                blob(if first { 22.0 } else { 16.0 }, 4.0, ink, 2.0),
                74.0,
                y + 1.0,
            ))
            .push(placed(
                blob(
                    (room - 14.0).min(if first { 52.0 } else { 40.0 }),
                    3.0,
                    muted,
                    1.5,
                ),
                74.0,
                y + 8.0,
            ));
    }
    layers = layers
        .push(placed(blob(room, 12.0, raised, 4.0), 60.0, 78.0 - 18.0))
        .push(placed(
            blob(6.0, 6.0, accent, 3.0),
            60.0 + room - 10.0,
            78.0 - 18.0,
        ));
    container(layers.width(width).height(78))
        .clip(true)
        .style(|_| iced::widget::container::Style {
            border: crate::style::round(6),
            ..Default::default()
        })
        .into()
}

impl<'a> Page<'a> {
    /// Appearance's corner roundness: every window's corners, from square
    /// to rounder than drawn; circles and pills have `round_slider`.
    pub(super) fn corners_slider(self) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let value = u32::from(
            self.cx
                .layout
                .prefs
                .corners
                .unwrap_or(crate::layout::CORNERS),
        );
        let pref = |value, done| {
            Message::Settings(SettingsEvent::Pref(super::pages::Pref::Corners(
                value, done,
            )))
        };
        column![
            crate::controls::slider(
                backcord_iced_kit::i18n::tr("Corner roundness"),
                value,
                200,
                move |value| pref(value, false),
            )
            .on_release(pref(value, true))
            .typed(move |value| pref(value, true))
            .view(theme),
            form::note(
                theme,
                backcord_iced_kit::i18n::tr(
                    "How round the corners of every window, panel and button are: 0 square, 100 as BACKCORD draws them, 200 rounder. What's round has its own slider below.",
                ),
            ),
        ]
        .spacing(8)
        .into()
    }

    /// Appearance's roundness of what's round: avatars, status dots, pills,
    /// server icons in the list and in folders, from square to round.
    pub(super) fn round_slider(self) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let value = u32::from(self.cx.layout.prefs.round.unwrap_or(100));
        let pref = |value, done| {
            Message::Settings(SettingsEvent::Pref(super::pages::Pref::Round(value, done)))
        };
        column![
            crate::controls::slider(
                backcord_iced_kit::i18n::tr("Roundness of avatars and icons"),
                value,
                100,
                move |value| pref(value, false),
            )
            .on_release(pref(value, true))
            .typed(move |value| pref(value, true))
            .view(theme),
            form::note(
                theme,
                backcord_iced_kit::i18n::tr(
                    "Like the corners, for what's round: avatars, status dots, pills, switches, and server icons in the list and in folders. 0 square, 100 round.",
                ),
            ),
        ]
        .spacing(8)
        .into()
    }

    /// Appearance's "Neon ring intensity": how strong the soft glow around
    /// the speaking rings and the call's speaking tiles is, 0 for none.
    pub(super) fn neon_glow_slider(self) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let value = u32::from(
            self.cx
                .layout
                .prefs
                .neon_glow
                .unwrap_or(backcord_iced_kit::style::DEFAULT_GLOW),
        );
        let pref = |value, done| {
            Message::Settings(SettingsEvent::Pref(super::pages::Pref::NeonGlow(
                value, done,
            )))
        };
        column![
            crate::controls::slider(
                backcord_iced_kit::i18n::tr("Neon ring intensity"),
                value,
                100,
                move |value| pref(value, false),
            )
            .on_release(pref(value, true))
            .typed(move |value| pref(value, true))
            .view(theme),
            form::note(
                theme,
                backcord_iced_kit::i18n::tr(
                    "A soft neon glow around the rings of those speaking, soundboard sounds included, and around their tiles in a call.",
                ),
            ),
        ]
        .spacing(8)
        .into()
    }

    /// Appearance's Dynamic Island: its switch, and while on, its style,
    /// sliders and colours.
    pub(super) fn island_settings(self) -> Element<'a, Message> {
        use super::pages::Pref;
        use crate::island;
        let theme = self.cx.look.theme;
        let prefs = &self.cx.layout.prefs;
        let pref = |pref| Message::Settings(SettingsEvent::Pref(pref));
        let on = prefs.island == Some(true);
        let mut group = column![
            form::heading(theme, backcord_iced_kit::i18n::tr("DYNAMIC ISLAND")),
            switch(
                backcord_iced_kit::i18n::tr("Dynamic Island"),
                on,
                Some(pref(Pref::Island(!on))),
            )
            .description(backcord_iced_kit::i18n::tr(
                "A pill at the top of the window with Spotify's player and the call's buttons. It shows only with a Spotify linked to Discord, while it plays something or you're in a call.",
            ))
            .view(self.cx.look),
        ]
        .spacing(12);
        if !on {
            return group.into();
        }
        let slider = |label: &'static str,
                      value: Option<u8>,
                      default: u8,
                      most: u32,
                      make: fn(u32, bool) -> Pref| {
            let value = u32::from(value.unwrap_or(default));
            crate::controls::slider(label, value, most, move |value| pref(make(value, false)))
                .on_release(pref(make(value, true)))
                .typed(move |value| pref(make(value, true)))
                .view(theme)
        };
        let style = usize::from(prefs.island_style.unwrap_or(0).min(1));
        group = group.push(crate::controls::choices(
            theme,
            &[
                backcord_iced_kit::i18n::tr("Glass"),
                backcord_iced_kit::i18n::tr("OLED"),
            ],
            style,
            move |style| pref(Pref::IslandStyle(style)),
        ));
        if style == 0 {
            group = group.push(slider(
                backcord_iced_kit::i18n::tr("Glass opacity"),
                prefs.island_opacity,
                island::OPACITY,
                100,
                Pref::IslandOpacity,
            ));
        }
        group = group
            .push(slider(
                backcord_iced_kit::i18n::tr("Glow intensity"),
                prefs.island_glow,
                island::GLOW,
                100,
                Pref::IslandGlow,
            ))
            .push(slider(
                backcord_iced_kit::i18n::tr("Glow speed"),
                prefs.island_speed,
                island::SPEED,
                100,
                Pref::IslandSpeed,
            ))
            .push(slider(
                backcord_iced_kit::i18n::tr("Corner radius (px)"),
                prefs.island_radius,
                island::RADIUS,
                22,
                Pref::IslandRadius,
            ))
            .push(slider(
                backcord_iced_kit::i18n::tr("Border thickness (px)"),
                prefs.island_border,
                island::BORDER,
                4,
                Pref::IslandBorder,
            ))
            .push(swatches(
                theme,
                backcord_iced_kit::i18n::tr("Glow colour"),
                theme.accent,
                &island::GLOWS,
                prefs.island_color,
                |colour| pref(Pref::IslandColor(colour)),
            ))
            .push(swatches(
                theme,
                backcord_iced_kit::i18n::tr("Text and icon colour"),
                Color::WHITE,
                &island::INKS,
                prefs.island_ink,
                |colour| pref(Pref::IslandInk(colour)),
            ));
        group.into()
    }

    /// Appearance's "Use the system title bar", where BACKCORD can draw its
    /// own: the desktop's way until it's chosen.
    pub(super) fn title_bar_switch(self) -> Element<'a, Message> {
        if !crate::window_bar::offered() {
            return Space::new().into();
        }
        let system = !self.cx.own_title_bar;
        switch(
            backcord_iced_kit::i18n::tr("Use the system title bar"),
            system,
            Some(Message::Settings(SettingsEvent::Pref(
                super::pages::Pref::SystemTitleBar(!system),
            ))),
        )
        .description(backcord_iced_kit::i18n::tr("Your desktop draws the window's title bar and buttons, as on its other windows. Off, BACKCORD draws its own."))
        .view(self.cx.look)
    }
}

/// A row of colours to choose from, under its label: `first`, which is none
/// chosen, then each of `colours` (0xRRGGBB); the one chosen ringed.
fn swatches<'a>(
    theme: &backcord_iced_kit::theme::Theme,
    label: &'a str,
    first: Color,
    colours: &[u32],
    chosen: Option<u32>,
    choose: impl Fn(Option<u32>) -> Message,
) -> Element<'a, Message> {
    let ring = theme.text;
    let swatch = |fill: Color, lit: bool, message: Message| {
        button(Space::new().width(22).height(22))
            .padding(0)
            .on_press(message)
            .style(move |_, _| iced::widget::button::Style {
                background: Some(fill.into()),
                border: Border {
                    color: if lit {
                        ring
                    } else {
                        Color::from_rgba(0.5, 0.5, 0.5, 0.4)
                    },
                    width: if lit { 2.0 } else { 1.0 },
                    radius: 11.0.into(),
                },
                ..Default::default()
            })
    };
    let mut row = row![swatch(first, chosen.is_none(), choose(None))].spacing(8);
    for colour in colours {
        let fill = Color::from_rgb8((colour >> 16) as u8, (colour >> 8) as u8, *colour as u8);
        row = row.push(swatch(fill, chosen == Some(*colour), choose(Some(*colour))));
    }
    column![
        text(label).size(13).font(MEDIUM).color(theme.secondary),
        row
    ]
    .spacing(8)
    .into()
}
