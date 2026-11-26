//! The text formatting button ("Aa") beside every field Discord reads
//! Markdown in: the composers (a channel's, a voice channel's chat, a
//! thread's, a new thread's and forum post's, a message being edited) and
//! the profile's bio. Its panel puts Discord's Markdown around the
//! selection, or at the caret, and writes the selection, or the whole text
//! when nothing is selected, in other letters: superscript, small caps,
//! fullwidth, upside down and so on, which are Unicode and show anywhere.

use std::ops::Range;

use iced::{
    Alignment, Length, Point, Size,
    widget::{Space, column, container, operation, row, text, tooltip},
};

use crate::{
    app::{App, Message},
    composer::Writing,
    controls::{self, lit_style},
};

/// The field the panel writes in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Writing(Writing),
    Bio,
}

/// The panel, while open: for which field, and where its button was
/// pressed.
#[derive(Clone, Copy, Debug)]
pub struct Open {
    pub target: Target,
    pub at: Point,
}

#[derive(Clone, Debug)]
pub enum Event {
    Open(Target),
    Close,
    Apply(Style),
    /// The bio's emoji button: the picker, beside it.
    BioEmoji,
}

/// What the panel offers, in its order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Bold,
    Italic,
    Underline,
    Strike,
    Spoiler,
    InlineCode,
    CodeBlock,
    Quote,
    Bullets,
    Superscript,
    SmallCaps,
    Fullwidth,
    UpsideDown,
    VariedCaps,
    Leet,
    Thicc,
    Upper,
    Lower,
    FirstCaps,
}

impl Style {
    pub const ALL: [Style; 19] = [
        Style::Bold,
        Style::Italic,
        Style::Underline,
        Style::Strike,
        Style::Spoiler,
        Style::InlineCode,
        Style::CodeBlock,
        Style::Quote,
        Style::Bullets,
        Style::Superscript,
        Style::SmallCaps,
        Style::Fullwidth,
        Style::UpsideDown,
        Style::VariedCaps,
        Style::Leet,
        Style::Thicc,
        Style::Upper,
        Style::Lower,
        Style::FirstCaps,
    ];

    /// What its cell shows over its name.
    fn face(self) -> &'static str {
        match self {
            Style::Bold => "B",
            Style::Italic => "I",
            Style::Underline => "U",
            Style::Strike => "S\u{336}",
            Style::Spoiler => "!",
            Style::InlineCode => "<>",
            Style::CodeBlock => "{ }",
            Style::Quote => "»",
            Style::Bullets => "•",
            Style::Superscript => "x²",
            Style::SmallCaps => "SC",
            Style::Fullwidth => "F",
            Style::UpsideDown => "∩",
            Style::VariedCaps => "V",
            Style::Leet => "13",
            Style::Thicc => "T",
            Style::Upper => "AA",
            Style::Lower => "aa",
            Style::FirstCaps => "Aa",
        }
    }

    fn label(self) -> &'static str {
        backcord_iced_kit::i18n::tr(match self {
            Style::Bold => "Bold",
            Style::Italic => "Italic",
            Style::Underline => "Underline",
            Style::Strike => "Strikethrough",
            Style::Spoiler => "Spoiler",
            Style::InlineCode => "Inline Code",
            Style::CodeBlock => "Codeblock",
            Style::Quote => "Blockquote",
            Style::Bullets => "Bullet List",
            Style::Superscript => "Superscript",
            Style::SmallCaps => "Small Caps",
            Style::Fullwidth => "Fullwidth",
            Style::UpsideDown => "Upside Down",
            Style::VariedCaps => "Varied Caps",
            Style::Leet => "Leet Speak",
            Style::Thicc => "Extra Thicc",
            Style::Upper => "UPPERCASE",
            Style::Lower => "lowercase",
            Style::FirstCaps => "First Caps",
        })
    }

    /// Discord's Markdown around the text, for the styles that wrap it.
    fn marks(self) -> Option<(&'static str, &'static str)> {
        Some(match self {
            Style::Bold => ("**", "**"),
            Style::Italic => ("*", "*"),
            Style::Underline => ("__", "__"),
            Style::Strike => ("~~", "~~"),
            Style::Spoiler => ("||", "||"),
            Style::InlineCode => ("`", "`"),
            Style::CodeBlock => ("```\n", "\n```"),
            _ => return None,
        })
    }
}

/// `text` with `style` applied to `selected`, or at `caret` when nothing
/// is selected; and what to select afterwards (empty: the caret there).
/// Markdown already around the selection comes off again.
pub fn apply(
    style: Style,
    text: &str,
    selected: Option<Range<usize>>,
    caret: usize,
) -> (String, Range<usize>) {
    let caret = caret.min(text.len());
    if let Some((open, close)) = style.marks() {
        return wrap(text, selected.unwrap_or(caret..caret), open, close);
    }
    match style {
        Style::Quote => return prefix_lines(text, selected.unwrap_or(caret..caret), "> "),
        Style::Bullets => return prefix_lines(text, selected.unwrap_or(caret..caret), "- "),
        _ => {}
    }
    // Letters: the selection's, or else the whole text's.
    let whole = selected.is_none();
    let range = selected.unwrap_or(0..text.len());
    let changed = letters(style, &text[range.clone()]);
    let written = format!("{}{changed}{}", &text[..range.start], &text[range.end..]);
    let end = range.start + changed.len();
    if !whole {
        (written, range.start..end)
    } else {
        let len = written.len();
        (written, len..len)
    }
}

fn wrap(text: &str, range: Range<usize>, open: &str, close: &str) -> (String, Range<usize>) {
    let (before, inside, after) = (
        &text[..range.start],
        &text[range.clone()],
        &text[range.end..],
    );
    // Already wrapped, just outside the selection or at its ends: unwrapped.
    if !inside.is_empty() && before.ends_with(open) && after.starts_with(close) {
        let start = range.start - open.len();
        let written = format!("{}{inside}{}", &before[..start], &after[close.len()..]);
        return (written, start..start + inside.len());
    }
    if inside.len() >= open.len() + close.len()
        && inside.starts_with(open)
        && inside.ends_with(close)
    {
        let kept = &inside[open.len()..inside.len() - close.len()];
        let written = format!("{before}{kept}{after}");
        return (written, range.start..range.start + kept.len());
    }
    let written = format!("{before}{open}{inside}{close}{after}");
    let start = range.start + open.len();
    (written, start..start + inside.len())
}

/// The lines `range` touches, each with `prefix` before it, or without it
/// when they all have it already.
fn prefix_lines(text: &str, range: Range<usize>, prefix: &str) -> (String, Range<usize>) {
    let start = text[..range.start]
        .rfind('\n')
        .map_or(0, |newline| newline + 1);
    let end = text[range.end..]
        .find('\n')
        .map_or(text.len(), |newline| range.end + newline);
    let lines: Vec<&str> = text[start..end].split('\n').collect();
    let all = lines.iter().all(|line| line.starts_with(prefix));
    let changed: Vec<String> = lines
        .iter()
        .map(|line| {
            if all {
                line[prefix.len()..].to_owned()
            } else {
                format!("{prefix}{line}")
            }
        })
        .collect();
    let changed = changed.join("\n");
    let written = format!("{}{changed}{}", &text[..start], &text[end..]);
    if range.is_empty() && lines.len() == 1 {
        // The caret stays where it was in the line.
        let caret = if all {
            range.start.saturating_sub(prefix.len()).max(start)
        } else {
            range.start + prefix.len()
        };
        return (written, caret..caret);
    }
    (written, start..start + changed.len())
}

/// `text` in `style`'s letters.
fn letters(style: Style, text: &str) -> String {
    match style {
        Style::Superscript => text.chars().map(|c| superscript(c).unwrap_or(c)).collect(),
        Style::SmallCaps => text.chars().map(|c| small_cap(c).unwrap_or(c)).collect(),
        Style::Fullwidth => text
            .chars()
            .map(|c| match c {
                ' ' => '\u{3000}',
                '!'..='~' => char::from_u32(c as u32 + 0xfee0).unwrap_or(c),
                _ => c,
            })
            .collect(),
        Style::UpsideDown => text
            .chars()
            .rev()
            .map(|c| upside_down(c).unwrap_or(c))
            .collect(),
        Style::VariedCaps => {
            let mut upper = false;
            text.chars()
                .flat_map(|c| {
                    if c.is_alphabetic() {
                        upper = !upper;
                        if upper {
                            return c.to_lowercase().collect::<Vec<_>>();
                        }
                        return c.to_uppercase().collect();
                    }
                    vec![c]
                })
                .collect()
        }
        Style::Leet => text
            .chars()
            .map(|c| match c.to_ascii_lowercase() {
                'a' => '4',
                'e' => '3',
                'i' => '1',
                'o' => '0',
                's' => '5',
                't' => '7',
                'b' => '8',
                'g' => '9',
                _ => c,
            })
            .collect(),
        Style::Thicc => text
            .chars()
            .map(|c| {
                const THICC: [char; 26] = [
                    '卂', '乃', '匚', 'ᗪ', '乇', '千', 'Ꮆ', '卄', '丨', 'ﾌ', 'Ҝ', 'ㄥ', '爪', '几',
                    'ㄖ', '卩', 'Ɋ', '尺', '丂', 'ㄒ', 'ㄩ', 'ᐯ', '山', '乂', 'ㄚ', '乙',
                ];
                let lower = c.to_ascii_lowercase();
                if lower.is_ascii_lowercase() {
                    THICC[(lower as u8 - b'a') as usize]
                } else {
                    c
                }
            })
            .collect(),
        Style::Upper => text.to_uppercase(),
        Style::Lower => text.to_lowercase(),
        Style::FirstCaps => {
            let mut start = true;
            let mut out = String::with_capacity(text.len());
            for c in text.chars() {
                if c.is_alphanumeric() {
                    if start {
                        out.extend(c.to_uppercase());
                    } else {
                        out.extend(c.to_lowercase());
                    }
                    start = false;
                } else {
                    out.push(c);
                    start = c.is_whitespace() || c == '-';
                }
            }
            out
        }
        _ => text.to_owned(),
    }
}

fn superscript(c: char) -> Option<char> {
    const LOWER: &str = "ᵃᵇᶜᵈᵉᶠᵍʰⁱʲᵏˡᵐⁿᵒᵖᑫʳˢᵗᵘᵛʷˣʸᶻ";
    const UPPER: &str = "ᴬᴮᶜᴰᴱᶠᴳᴴᴵᴶᴷᴸᴹᴺᴼᴾᑫᴿˢᵀᵁⱽᵂˣʸᶻ";
    const DIGITS: &str = "⁰¹²³⁴⁵⁶⁷⁸⁹";
    match c {
        'a'..='z' => LOWER.chars().nth((c as u8 - b'a') as usize),
        'A'..='Z' => UPPER.chars().nth((c as u8 - b'A') as usize),
        '0'..='9' => DIGITS.chars().nth((c as u8 - b'0') as usize),
        '+' => Some('⁺'),
        '-' => Some('⁻'),
        '=' => Some('⁼'),
        '(' => Some('⁽'),
        ')' => Some('⁾'),
        _ => None,
    }
}

fn small_cap(c: char) -> Option<char> {
    const CAPS: &str = "ᴀʙᴄᴅᴇꜰɢʜɪᴊᴋʟᴍɴᴏᴘǫʀꜱᴛᴜᴠᴡxʏᴢ";
    let lower = c.to_ascii_lowercase();
    lower
        .is_ascii_lowercase()
        .then(|| CAPS.chars().nth((lower as u8 - b'a') as usize))
        .flatten()
}

fn upside_down(c: char) -> Option<char> {
    const FROM: &str =
        "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.,'!?()[]{}<>&_\"";
    const TO: &str =
        "ɐqɔpǝɟƃɥᴉɾʞlɯuodbɹsʇnʌʍxʎz∀ᗺƆᗡƎℲ⅁HIſʞ˥WNOԀΌᴚS⊥∩ΛMX⅄Z0ƖᄅƐㄣϛ9ㄥ86˙'‚¡¿)(][}{><⅋‾„";
    FROM.chars()
        .position(|from| from == c)
        .and_then(|at| TO.chars().nth(at))
}

/// The panel's cells: their size, the gap between them, and how many to a
/// row.
const CELL: Size = Size::new(62.0, 50.0);
const GAP: f32 = 4.0;
const COLUMNS: usize = 7;
const PAD: f32 = 10.0;

fn panel_size() -> Size {
    let rows = Style::ALL.len().div_ceil(COLUMNS) as f32;
    Size::new(
        COLUMNS as f32 * CELL.width + (COLUMNS - 1) as f32 * GAP + 2.0 * PAD,
        rows * CELL.height + (rows - 1.0) * GAP + 2.0 * PAD,
    )
}

/// The "Aa" button that opens the panel for `target`.
pub fn button(ctx: crate::ctx::Ctx<'_>, target: Target) -> iced::Element<'static, Message> {
    let theme = ctx.look.theme;
    let face = container(
        text("Aa")
            .size(16)
            .font(crate::fonts::BOLD)
            .color(theme.secondary),
    )
    .width(32)
    .height(32)
    .align_x(Alignment::Center)
    .align_y(Alignment::Center);
    let face = iced::widget::button(face)
        .padding(0)
        .width(32)
        .height(32)
        .on_press(Message::Format(Event::Open(target)))
        .style(lit_style(None, theme.wash(0.1), 6.0));
    tooltip(
        face,
        controls::tip(theme, backcord_iced_kit::i18n::tr("Text formatting")),
        tooltip::Position::Top,
    )
    .gap(6)
    .into()
}

/// The panel, over the window, by the button that opened it.
pub fn view<'a>(
    open: Option<&Open>,
    window: Size,
    ctx: crate::ctx::Ctx<'a>,
) -> Option<iced::Element<'a, Message>> {
    let open = open?;
    let theme = ctx.look.theme;
    let size = panel_size();
    let mut grid = column![].spacing(GAP);
    for styles in Style::ALL.chunks(COLUMNS) {
        let mut line = row![].spacing(GAP);
        for &style in styles {
            let cell = column![
                text(style.face())
                    .size(15)
                    .font(crate::fonts::BOLD)
                    .color(theme.text),
                backcord_iced_kit::elide::elide(style.label())
                    .size(11)
                    .color(theme.secondary),
            ]
            .spacing(3)
            .align_x(Alignment::Center);
            line = line.push(
                iced::widget::button(
                    container(cell)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .align_x(Alignment::Center)
                        .align_y(Alignment::Center),
                )
                .padding([0, 4])
                .width(CELL.width)
                .height(CELL.height)
                .on_press(Message::Format(Event::Apply(style)))
                .style(lit_style(None, theme.wash(0.1), 6.0)),
            );
        }
        grid = grid.push(line);
    }
    let (ground, edge) = (theme.raised, theme.line);
    // The frosted window under its see-through ground, as the menus'.
    let panel = backcord_iced_kit::widgets::frost::Frost::new(
        [8.0; 4],
        container(grid)
            .padding(PAD)
            .width(size.width)
            .height(size.height)
            .style(move |_| crate::style::outlined(ground, edge, 8.0)),
    );
    // Over the button, or under it when there's no room above.
    let top = if open.at.y - size.height - 16.0 >= 8.0 {
        open.at.y - size.height - 16.0
    } else {
        open.at.y + 20.0
    };
    let left =
        (open.at.x - size.width + 40.0).clamp(8.0, (window.width - size.width - 8.0).max(8.0));
    Some(
        column![
            Space::new().height(top),
            row![Space::new().width(left), panel]
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into(),
    )
}

pub fn update(app: &mut App, event: Event) -> iced::Task<Message> {
    match event {
        Event::Open(target) => {
            app.format = Some(Open {
                target,
                at: app.pressed.get(),
            });
            iced::Task::none()
        }
        Event::Close => {
            app.format = None;
            iced::Task::none()
        }
        Event::BioEmoji => {
            let at = app.pressed.get();
            crate::picker::open_picker(
                app,
                crate::picker::Target::Bio(at),
                crate::picker::Tab::Emoji,
            )
        }
        Event::Apply(style) => {
            let Some(open) = app.format.take() else {
                return iced::Task::none();
            };
            match open.target {
                Target::Bio => app.update(crate::profile::editor::format_bio(style)),
                Target::Writing(which) => {
                    let draft = crate::composer::writing(app.ctx(), which);
                    let (written, selected) =
                        apply(style, &draft.text(), draft.selected(), draft.caret());
                    draft.replace_selecting(&written, selected.clone());
                    crate::composer::keep_text(app, which, written.clone());
                    crate::suggest::caret_moved(app, which, &written, selected.end);
                    operation::focus(iced::widget::Id::new(crate::thread::field_id(which)))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_goes_around_the_selection_or_at_the_caret_and_comes_off_again() {
        assert_eq!(
            apply(Style::Bold, "say hi now", Some(4..6), 0),
            ("say **hi** now".to_owned(), 6..8)
        );
        assert_eq!(
            apply(Style::Bold, "say **hi** now", Some(6..8), 0),
            ("say hi now".to_owned(), 4..6)
        );
        assert_eq!(
            apply(Style::Spoiler, "ab", None, 1),
            ("a||||b".to_owned(), 3..3)
        );
        assert_eq!(
            apply(Style::CodeBlock, "x", Some(0..1), 0),
            ("```\nx\n```".to_owned(), 4..5)
        );
    }

    #[test]
    fn quotes_and_bullets_go_before_each_line_touched() {
        let (written, _) = apply(Style::Quote, "one\ntwo\nthree", Some(1..5), 0);
        assert_eq!(written, "> one\n> two\nthree");
        let (written, _) = apply(Style::Quote, &written, Some(3..9), 0);
        assert_eq!(written, "one\ntwo\nthree", "off again");
        assert_eq!(
            apply(Style::Bullets, "milk", None, 2),
            ("- milk".to_owned(), 4..4)
        );
    }

    #[test]
    fn letters_change_in_the_selection_or_the_whole_text() {
        assert_eq!(letters(Style::Superscript, "x2+a"), "ˣ²⁺ᵃ");
        assert_eq!(letters(Style::SmallCaps, "Hey"), "ʜᴇʏ");
        assert_eq!(letters(Style::Fullwidth, "Hi 1!"), "Ｈｉ\u{3000}１！");
        assert_eq!(letters(Style::UpsideDown, "hey!"), "¡ʎǝɥ");
        assert_eq!(letters(Style::VariedCaps, "hello"), "hElLo");
        assert_eq!(letters(Style::Leet, "Leet Speak"), "L337 5p34k");
        assert_eq!(letters(Style::Thicc, "Ab"), "卂乃");
        assert_eq!(letters(Style::FirstCaps, "hello WORLD"), "Hello World");
        assert_eq!(
            apply(Style::Upper, "say hi", Some(4..6), 0),
            ("say HI".to_owned(), 4..6)
        );
        assert_eq!(
            apply(Style::Lower, "ABC", None, 0),
            ("abc".to_owned(), 3..3)
        );
    }
}
