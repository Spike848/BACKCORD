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

/// A theme BACKCORD comes with: the id a layout stores and the name Appearance
/// shows.
pub struct BuiltIn {
    pub id: &'static str,
    pub name: &'static str,
    pub theme: Theme,
}

/// The theme BACKCORD comes with.
pub const BUILT_IN: [BuiltIn; 1] = [BuiltIn {
    id: "backdoor",
    name: "BACKDOOR",
    theme: BACKDOOR,
}];

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
