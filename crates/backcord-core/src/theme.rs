//! The window's colours as data: a full set of them, the themes BACKCORD
//! comes with, and the person's own as `layout.json` keeps them. The window
//! paints with them in its own colour type; see docs/themes.md.

/// A colour as `0xaarrggbb`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Colour(u32);

impl Colour {
    /// Opaque `#rrggbb`.
    pub const fn rgb(hex: u32) -> Self {
        Self(0xff00_0000 | hex)
    }

    /// `#rrggbbaa`: a colour with its alpha last.
    pub const fn rgba(hex: u32) -> Self {
        Self(hex.rotate_right(8))
    }

    pub const fn argb(self) -> u32 {
        self.0
    }

    /// `0xrrggbb`, without the alpha.
    pub const fn rgb_part(self) -> u32 {
        self.0 & 0xff_ffff
    }

    pub const fn red(self) -> u8 {
        (self.0 >> 16) as u8
    }

    pub const fn green(self) -> u8 {
        (self.0 >> 8) as u8
    }

    pub const fn blue(self) -> u8 {
        self.0 as u8
    }

    pub const fn alpha(self) -> u8 {
        (self.0 >> 24) as u8
    }
}

const fn rgb(hex: u32) -> Colour {
    Colour::rgb(hex)
}

const fn rgba(hex: u32) -> Colour {
    Colour::rgba(hex)
}

/// Declares `Theme` with one field per colour.
macro_rules! theme {
    ($($name:ident,)*) => {
        /// A full set of the window's colours, and whether they make a dark
        /// theme.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct Theme {
            pub dark: bool,
            $(pub $name: Colour,)*
        }

        impl Theme {
            /// Each colour's name, as `layout.json` has it but with `_` for
            /// `-`, in the order of [`Self::colour`].
            pub const NAMES: &[&str] = &[$(stringify!($name),)*];

            pub fn colour(&self, index: usize) -> Option<Colour> {
                [$(self.$name,)*].get(index).copied()
            }

            pub fn colour_mut(&mut self, index: usize) -> Option<&mut Colour> {
                [$(&mut self.$name,)*].into_iter().nth(index)
            }
        }
    };
}

theme! {
    rail,
    sidebar,
    panel,
    chat,
    stage,
    raised,
    field,
    sunken,
    hover,
    active,
    row_hover,
    line,
    edge,
    outline,
    outline_chat,
    outline_field,
    text,
    secondary,
    muted,
    faint,
    accent,
    accent_hover,
    on_accent,
    accent_soft,
    danger,
    danger_hover,
    on_danger,
    live,
    loud,
    link,
    button_default,
    button_default_hover,
    overlay,
    shimmer,
    reply_spine,
    tooltip,
    avatar,
}

/// BACKCORD's own colours: black grounds with a red accent.
pub const BACKDOOR: Theme = Theme {
    dark: true,
    rail: rgb(0x000000),
    sidebar: rgb(0x050505),
    panel: rgb(0x050505),
    chat: rgb(0x000000),
    stage: rgb(0x000000),
    raised: rgb(0x080808),
    field: rgb(0x050505),
    sunken: rgb(0x000000),
    hover: rgb(0x181818),
    active: rgb(0x241010),
    row_hover: rgb(0x121212),
    line: rgb(0x1a1a1a),
    outline: rgb(0x262626),
    outline_chat: rgb(0x1a1a1a),
    outline_field: rgb(0x262626),
    edge: rgb(0x111111),
    text: rgb(0xffffff),
    secondary: rgb(0xb5bac1),
    muted: rgb(0x6d6f78),
    faint: rgb(0x4e5058),
    accent: rgb(0xe02424),
    accent_hover: rgb(0xff3b3b),
    on_accent: rgb(0xffffff),
    accent_soft: rgba(0xe0242426),
    danger: rgb(0xf23f43),
    danger_hover: rgb(0xff5c5f),
    on_danger: rgb(0xffffff),
    live: rgb(0x23a55a),
    loud: rgb(0x23a55a),
    link: rgb(0xff4d4d),
    button_default: rgb(0x0a0a0a),
    button_default_hover: rgb(0x2e2e2e),
    overlay: rgb(0xffffff),
    shimmer: rgba(0xffffff10),
    reply_spine: rgb(0x4e5058),
    tooltip: rgb(0x111214),
    avatar: rgb(0x1f1f1f),
};

/// Discord's own dark greys and blurple.
pub const DISCORD: Theme = Theme {
    dark: true,
    rail: rgb(0x08080b),
    sidebar: rgb(0x101116),
    panel: rgb(0x101116),
    chat: rgb(0x0b0b0f),
    stage: rgb(0x09090d),
    raised: rgb(0x191920),
    field: rgb(0x101116),
    sunken: rgb(0x09090d),
    hover: rgb(0x1c1c24),
    active: rgb(0x202447),
    row_hover: rgb(0x191920),
    line: rgb(0x202027),
    edge: rgb(0x292930),
    outline: rgb(0x303039),
    outline_chat: rgb(0x202027),
    outline_field: rgb(0x303039),
    text: rgb(0xffffff),
    secondary: rgb(0xb5bac1),
    muted: rgb(0x6d6f78),
    faint: rgb(0x4e5058),
    accent: rgb(0x5865f2),
    accent_hover: rgb(0x4752c4),
    accent_soft: rgb(0x202447),
    link: rgb(0x7289da),
    danger: rgb(0xed4245),
    danger_hover: rgb(0xff5c60),
    on_accent: rgb(0xffffff),
    on_danger: rgb(0xffffff),
    live: rgb(0x23a559),
    loud: rgb(0x23a559),
    button_default: rgb(0x121217),
    button_default_hover: rgb(0x292930),
    overlay: rgb(0xffffff),
    shimmer: rgba(0xffffff10),
    reply_spine: rgb(0x4e5058),
    tooltip: rgb(0x111116),
    avatar: rgb(0x1c1c22),
};

/// Violet greys with a purple accent and pink links.
pub const VENCORD: Theme = Theme {
    dark: true,
    rail: rgb(0x08070b),
    sidebar: rgb(0x100d14),
    panel: rgb(0x100d14),
    chat: rgb(0x0a090d),
    stage: rgb(0x08070b),
    raised: rgb(0x1b1521),
    field: rgb(0x100d14),
    sunken: rgb(0x08070b),
    hover: rgb(0x21182b),
    active: rgb(0x291a3d),
    row_hover: rgb(0x1b1521),
    line: rgb(0x241c2b),
    edge: rgb(0x2b2233),
    outline: rgb(0x36283f),
    outline_chat: rgb(0x241c2b),
    outline_field: rgb(0x36283f),
    text: rgb(0xfff9ff),
    secondary: rgb(0xc0b4ca),
    muted: rgb(0x81758d),
    faint: rgb(0x594c65),
    accent: rgb(0x9b59ff),
    accent_hover: rgb(0xb477ff),
    accent_soft: rgb(0x291a3d),
    link: rgb(0xff72b6),
    danger: rgb(0xf23f43),
    danger_hover: rgb(0xff5c70),
    on_accent: rgb(0xffffff),
    on_danger: rgb(0xffffff),
    live: rgb(0x23a55a),
    loud: rgb(0x23a55a),
    button_default: rgb(0x120d16),
    button_default_hover: rgb(0x2b2234),
    overlay: rgb(0xffffff),
    shimmer: rgba(0xffffff10),
    reply_spine: rgb(0x594c65),
    tooltip: rgb(0x110d15),
    avatar: rgb(0x1d1623),
};

/// Warm browns with an orange accent.
pub const TESTCORD: Theme = Theme {
    dark: true,
    rail: rgb(0x080705),
    sidebar: rgb(0x100d09),
    panel: rgb(0x100d09),
    chat: rgb(0x0a0805),
    stage: rgb(0x080705),
    raised: rgb(0x1b140d),
    field: rgb(0x100d09),
    sunken: rgb(0x080705),
    hover: rgb(0x21180e),
    active: rgb(0x2b1c0d),
    row_hover: rgb(0x1b140d),
    line: rgb(0x241b12),
    edge: rgb(0x2b2117),
    outline: rgb(0x35281b),
    outline_chat: rgb(0x241b12),
    outline_field: rgb(0x35281b),
    text: rgb(0xfff9f1),
    secondary: rgb(0xc1b6a8),
    muted: rgb(0x827568),
    faint: rgb(0x594d40),
    accent: rgb(0xf08c28),
    accent_hover: rgb(0xffab48),
    accent_soft: rgb(0x2b1c0d),
    link: rgb(0xffb454),
    danger: rgb(0xe5484d),
    danger_hover: rgb(0xff6266),
    on_accent: rgb(0x171009),
    on_danger: rgb(0xffffff),
    live: rgb(0x35b86b),
    loud: rgb(0x35b86b),
    button_default: rgb(0x120e09),
    button_default_hover: rgb(0x2a2016),
    overlay: rgb(0xffffff),
    shimmer: rgba(0xffffff10),
    reply_spine: rgb(0x594d40),
    tooltip: rgb(0x110d09),
    avatar: rgb(0x1d160f),
};

/// A theme BACKCORD comes with: the id a layout stores and the name Appearance
/// shows.
pub struct BuiltIn {
    pub id: &'static str,
    pub name: &'static str,
    pub theme: Theme,
}

/// The themes BACKCORD comes with, BACKDOOR first: the one shown when a
/// layout chose none.
pub const BUILT_IN: [BuiltIn; 4] = [
    BuiltIn {
        id: "backdoor",
        name: "BACKDOOR",
        theme: BACKDOOR,
    },
    BuiltIn {
        id: "discord",
        name: "Discord",
        theme: DISCORD,
    },
    BuiltIn {
        id: "vencord",
        name: "Vencord",
        theme: VENCORD,
    },
    BuiltIn {
        id: "testcord",
        name: "Testcord",
        theme: TESTCORD,
    },
];

pub fn find(id: &str) -> Option<&'static BuiltIn> {
    BUILT_IN.iter().find(|theme| theme.id == id)
}

/// A theme Appearance offers, built in or the person's own, and its id.
pub fn lookup<'a>(id: &str, custom: &'a [Custom]) -> Option<(&'a str, Theme)> {
    find(id)
        .map(|built_in| (built_in.id, built_in.theme))
        .or_else(|| {
            custom
                .iter()
                .find(|custom| custom.id == id)
                .map(|custom| (custom.id.as_str(), custom.theme))
        })
}

/// The theme a layout chose, and its id: BACKDOOR when it chose none, or
/// one this version doesn't have or the person deleted.
pub fn chosen<'a>(id: Option<&str>, custom: &'a [Custom]) -> (&'a str, Theme) {
    id.and_then(|id| lookup(id, custom))
        .unwrap_or((BUILT_IN[0].id, BACKDOOR))
}

/// A colour's name as `layout.json` has it.
pub fn token(name: &str) -> String {
    name.replace('_', "-")
}

/// Where a colour named as `layout.json` has it is in [`Theme::colour`].
fn token_index(token: &str) -> Option<usize> {
    Theme::NAMES.iter().position(|name| {
        name.bytes()
            .map(|byte| if byte == b'_' { b'-' } else { byte })
            .eq(token.bytes())
    })
}

/// A colour written `#rgb`, `#rrggbb` or `#rrggbbaa`.
pub fn parse(text: &str) -> Option<Colour> {
    let digits = text.trim().strip_prefix('#')?;
    if !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let value = u32::from_str_radix(digits, 16).ok()?;
    match digits.len() {
        3 => {
            let nibble = |shift: u32| ((value >> shift) & 0xf) * 0x11;
            Some(rgb((nibble(8) << 16) | (nibble(4) << 8) | nibble(0)))
        }
        6 => Some(rgb(value)),
        8 => Some(rgba(value)),
        _ => None,
    }
}

/// A colour as `#rrggbb`, or `#rrggbbaa` when it isn't opaque.
pub fn hex(colour: Colour) -> String {
    let rgb = colour.rgb_part();
    match colour.alpha() {
        0xff => format!("#{rgb:06x}"),
        alpha => format!("#{rgb:06x}{alpha:02x}"),
    }
}

/// One of the person's own themes, as a layout keeps it.
#[derive(Clone, Debug, PartialEq)]
pub struct Custom {
    /// `custom:` and a number: no built-in id has a colon. See [`tidy`].
    pub id: String,
    pub name: String,
    /// The built-in theme it began as a copy of, whose colours stand in for
    /// any it doesn't name, such as one added since it was saved.
    pub base: &'static str,
    pub theme: Theme,
}

/// A custom theme in `layout.json`: each colour as text, by its name as
/// [`token`] gives it.
#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct Stored {
    id: String,
    name: String,
    base: String,
    dark: Option<bool>,
    colours: std::collections::BTreeMap<String, String>,
}

impl From<Stored> for Custom {
    fn from(stored: Stored) -> Self {
        let base = find(&stored.base).unwrap_or(&BUILT_IN[0]);
        let mut theme = base.theme;
        theme.dark = stored.dark.unwrap_or(theme.dark);
        for (token, text) in &stored.colours {
            if let (Some(slot), Some(colour)) = (
                token_index(token).and_then(|index| theme.colour_mut(index)),
                parse(text),
            ) {
                *slot = colour;
            }
        }
        Self {
            id: stored.id,
            name: stored.name,
            base: base.id,
            theme,
        }
    }
}

impl From<&Custom> for Stored {
    fn from(custom: &Custom) -> Self {
        let colours = Theme::NAMES
            .iter()
            .enumerate()
            .filter_map(|(index, name)| Some((token(name), hex(custom.theme.colour(index)?))))
            .collect();
        Self {
            id: custom.id.clone(),
            name: custom.name.clone(),
            base: custom.base.to_owned(),
            dark: Some(custom.theme.dark),
            colours,
        }
    }
}

impl serde::Serialize for Custom {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        Stored::from(self).serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for Custom {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Stored::deserialize(deserializer).map(Self::from)
    }
}

/// The themes in a file to import: one theme as BACKCORD exports it, a
/// list of them, a whole `layout.json`, or just colours by name, as a theme
/// written by hand often is. Colour names may use `_` or capitals, the
/// colours may sit under `colors`, and a file Notepad saved with a BOM
/// reads the same. A theme with no name takes `file_name`'s; one that
/// doesn't say whether it's dark is as dark as its chat's colour.
pub fn import(text: &str, file_name: &str) -> Vec<Custom> {
    let text = text.trim_start_matches('\u{feff}');
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return Vec::new();
    };
    let items = match value {
        serde_json::Value::Array(items) => items,
        serde_json::Value::Object(ref object) => match object.get("custom_themes") {
            Some(serde_json::Value::Array(items)) => items.clone(),
            _ => vec![value],
        },
        _ => Vec::new(),
    };
    let name = file_name.rsplit(['/', '\\']).next().unwrap_or_default();
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    items
        .iter()
        .filter_map(|item| imported(item.as_object()?, stem))
        .take(32)
        .collect()
}

/// One theme to import, if it names a colour BACKCORD has.
fn imported(item: &serde_json::Map<String, serde_json::Value>, stem: &str) -> Option<Custom> {
    let colours = ["colours", "colors"]
        .iter()
        .find_map(|key| item.get(*key)?.as_object())
        .unwrap_or(item);
    let colours: std::collections::BTreeMap<String, String> = colours
        .iter()
        .filter_map(|(name, value)| {
            let token = name.trim().to_ascii_lowercase().replace('_', "-");
            token_index(&token)?;
            let text = value.as_str()?.trim();
            let text = if text.starts_with('#') {
                text.to_owned()
            } else {
                format!("#{text}")
            };
            parse(&text)?;
            Some((token, text))
        })
        .collect();
    if colours.is_empty() {
        return None;
    }
    let text = |key: &str| {
        item.get(key)
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
    };
    let name = match text("name").trim() {
        "" => stem.to_owned(),
        name => name.to_owned(),
    };
    let dark = item.get("dark").and_then(serde_json::Value::as_bool);
    let chat = colours.get("chat").and_then(|chat| parse(chat));
    let mut custom = Custom::from(Stored {
        id: String::new(),
        name,
        base: text("base").to_owned(),
        dark,
        colours,
    });
    if dark.is_none()
        && let Some(chat) = chat
    {
        let rgb = chat.rgb_part();
        let [r, g, b] = [(rgb >> 16) & 0xff, (rgb >> 8) & 0xff, rgb & 0xff].map(f64::from);
        custom.theme.dark = 0.2126 * r + 0.7152 * g + 0.0722 * b < 128.0;
    }
    Some(custom)
}

/// The number in a custom theme's id, if it's one BACKCORD gives.
fn custom_number(id: &str) -> Option<u32> {
    id.strip_prefix("custom:")?.parse().ok()
}

/// An id for a new custom theme, after those there are.
pub fn new_id(custom: &[Custom]) -> String {
    let last = custom
        .iter()
        .filter_map(|custom| custom_number(&custom.id))
        .max()
        .unwrap_or(0);
    format!("custom:{}", last + 1)
}

/// Gives a new id to each custom theme from `layout.json` whose id BACKCORD
/// wouldn't have given, or that another has, so that ids stay apart from
/// the built-in ones and from each other.
pub fn tidy(custom: &mut [Custom]) {
    let mut seen = std::collections::HashSet::new();
    let unfit: Vec<usize> = (0..custom.len())
        .filter(|&index| {
            custom_number(&custom[index].id).is_none() || !seen.insert(custom[index].id.clone())
        })
        .collect();
    for index in unfit {
        custom[index].id = new_id(custom);
    }
}

/// A theme a new one can start as.
pub struct Start<'a> {
    pub id: &'a str,
    /// The built-in theme it is or began as.
    pub base: &'static str,
    pub name: &'a str,
    pub theme: Theme,
}

/// What a new theme can start as: every theme Appearance offers, in its
/// order.
pub fn starts(custom: &[Custom]) -> impl Iterator<Item = Start<'_>> {
    BUILT_IN
        .iter()
        .map(|built_in| Start {
            id: built_in.id,
            base: built_in.id,
            name: built_in.name,
            theme: built_in.theme,
        })
        .chain(custom.iter().map(|custom| Start {
            id: &custom.id,
            base: custom.base,
            name: &custom.name,
            theme: custom.theme,
        }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_a_theme_of_just_colours_as_well_as_an_exported_one() {
        // Colours alone, as a hand-written theme is, with a BOM.
        let flat =
            "\u{feff}{\"rail\": \"#08080B\", \"chat\": \"#0B0B0F\", \"row-hover\": \"#191920\",
            \"Accent_Hover\": \"4752C4\", \"shimmer\": \"#FFFFFF10\", \"unknown\": \"#123456\"}";
        let themes = import(flat, "C:\\Users\\x\\discord.json");
        assert_eq!(themes.len(), 1);
        let theme = &themes[0];
        assert_eq!(theme.name, "discord");
        assert_eq!(hex(theme.theme.rail), "#08080b");
        assert_eq!(hex(theme.theme.row_hover), "#191920");
        assert_eq!(hex(theme.theme.accent_hover), "#4752c4");
        assert_eq!(hex(theme.theme.shimmer), "#ffffff10");
        assert!(theme.theme.dark, "a dark chat makes a dark theme");
        let light = import(
            r##"{"name": "Day", "colors": {"chat": "#ffffff"}}"##,
            "x.json",
        );
        assert_eq!(light[0].name, "Day");
        assert!(!light[0].theme.dark);
        // As BACKCORD exports one, and back.
        let exported = serde_json::to_string(&themes[0]).unwrap();
        assert_eq!(import(&exported, "other.json")[0].theme, themes[0].theme);
        // Not a theme.
        assert!(import("{\"hello\": 1}", "x.json").is_empty());
        assert!(import("not json", "x.json").is_empty());
    }

    #[test]
    fn colours_read_and_write_as_hex() {
        assert_eq!(parse("#ffbf47"), Some(Colour::rgb(0xffbf47)));
        assert_eq!(parse("#fff"), Some(Colour::rgb(0xffffff)));
        assert_eq!(parse("#ffbf4726").map(Colour::alpha), Some(0x26));
        assert_eq!(parse("ffbf47"), None);
        assert_eq!(hex(Colour::rgba(0xffffff10)), "#ffffff10");
        assert_eq!(hex(BACKDOOR.accent), "#e02424");
    }

    #[test]
    fn a_custom_theme_takes_its_bases_colour_for_any_it_lacks() {
        // A base this version doesn't have: BACKDOOR's colours stand in.
        let stored = r##"{"id": "custom:1", "name": "Plum", "base": "nord",
            "colours": {"chat": "#2a1a3a", "nonsense": "#000", "rail": "blue"}}"##;
        let custom: Custom = serde_json::from_str(stored).unwrap();
        assert_eq!(custom.base, "backdoor");
        assert_eq!(custom.theme.chat, Colour::rgb(0x2a1a3a));
        assert_eq!(custom.theme.rail, BACKDOOR.rail, "not a colour");
        assert_eq!(custom.theme.dark, BACKDOOR.dark);
        assert_eq!(
            chosen(Some("custom:1"), std::slice::from_ref(&custom)).1,
            custom.theme
        );
        assert_eq!(chosen(Some("gone"), &[custom]).0, "backdoor");
    }
}
