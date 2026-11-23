//! Discord's Markdown as its message field shows it while you write: the
//! marks stay, greyed, and what they mark takes its style. `**bold**` is
//! bold between its stars, `*italic*` and `_italic_` italic, `***both***`
//! both, `\`code\`` and fenced blocks in the monospace face, and a heading's
//! line (`# `, `## `, `### `) bold. Underline, strikethrough and spoilers
//! have their marks greyed only, as the editor draws neither line nor cover.

use std::ops::Range;

/// How a run of the text is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    /// A pill's key, in semibold.
    Key,
    /// Markdown's own characters, greyed.
    Syntax,
    /// Text a mark styles.
    Style {
        bold: bool,
        italic: bool,
        code: bool,
    },
}

const BOLD: u8 = 1;
const ITALIC: u8 = 2;
const CODE: u8 = 4;
const SYNTAX: u8 = 8;
/// A byte some mark already took, which no other mark may start or end on.
const TAKEN: u8 = 16;

/// The runs of `text` that Markdown styles, by byte offsets in it, in order.
pub fn runs(text: &str) -> Vec<(Range<usize>, Mark)> {
    if !text.contains(['*', '_', '~', '|', '`', '#', '>']) {
        return Vec::new();
    }
    let lines: Vec<(usize, &str)> = text
        .split('\n')
        .scan(0, |start, line| {
            let at = *start;
            *start += line.len() + 1;
            Some((at, line))
        })
        .collect();
    // Fences pair up in order; one left open fences nothing, as in Discord.
    let fences: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, (_, line))| line.trim_start().starts_with("```"))
        .map(|(index, _)| index)
        .collect();
    let mut fenced = vec![0u8; lines.len()];
    for pair in fences.chunks_exact(2) {
        fenced[pair[0]] = 1;
        fenced[pair[1]] = 1;
        for inside in &mut fenced[pair[0] + 1..pair[1]] {
            *inside = 2;
        }
    }
    let mut out = Vec::new();
    for ((start, line), fence) in lines.into_iter().zip(fenced) {
        let mut flags = vec![0u8; line.len()];
        match fence {
            1 => flags.fill(SYNTAX),
            2 => flags.fill(CODE),
            _ => line_flags(line, &mut flags),
        }
        push_runs(&flags, start, &mut out);
    }
    out
}

/// Marks one line outside a fenced block.
fn line_flags(line: &str, flags: &mut [u8]) {
    let bytes = line.as_bytes();
    let length = bytes.len();
    // A heading: its hashes greyed and the rest bold; a quote's `>` greyed.
    for (hashes, mark) in [
        ("### ", BOLD),
        ("## ", BOLD),
        ("# ", BOLD),
        ("-# ", 0),
        ("> ", 0),
    ] {
        if line.starts_with(hashes) {
            let marks = hashes.len() - 1;
            for flag in &mut flags[..marks] {
                *flag |= SYNTAX | TAKEN;
            }
            for flag in &mut flags[marks..] {
                *flag |= mark;
            }
            break;
        }
    }
    // Inline code first, between single backticks: nothing inside it is
    // Markdown. A fence left open is no code.
    let tick = |at: usize| {
        bytes[at] == b'`' && bytes.get(at + 1) != Some(&b'`') && (at == 0 || bytes[at - 1] != b'`')
    };
    let mut at = 0;
    while at < length {
        if tick(at)
            && flags[at] & TAKEN == 0
            && let Some(end) = (at + 2..length).find(|&end| tick(end))
        {
            flags[at] |= SYNTAX | TAKEN;
            flags[end] |= SYNTAX | TAKEN;
            for flag in &mut flags[at + 1..end] {
                *flag |= CODE | TAKEN;
            }
            at = end + 1;
        } else {
            at += 1;
        }
    }
    let free =
        |flags: &[u8], range: Range<usize>| flags[range].iter().all(|flag| flag & TAKEN == 0);
    let word = |at: usize| at < length && bytes[at].is_ascii_alphanumeric();
    for (mark, style) in [
        ("***", BOLD | ITALIC),
        ("**", BOLD),
        ("__", 0),
        ("~~", 0),
        ("||", 0),
        ("*", ITALIC),
        ("_", ITALIC),
    ] {
        let size = mark.len();
        let single = size == 1;
        let sign = mark.as_bytes()[0];
        // A single star or underscore isn't part of a longer run of them.
        let alone = |at: usize| {
            !single || (bytes.get(at + 1) != Some(&sign) && (at == 0 || bytes[at - 1] != sign))
        };
        let mut at = 0;
        while at + size <= length {
            let opens = &line[at..at + size] == mark
                && free(flags, at..at + size)
                && alone(at)
                && at + size < length
                && !bytes[at + size].is_ascii_whitespace()
                && !(sign == b'_' && single && at > 0 && word(at - 1));
            if !opens {
                at += 1;
                continue;
            }
            let close = (at + size + 1..=length - size).find(|&end| {
                &line[end..end + size] == mark
                    && free(flags, end..end + size)
                    && alone(end)
                    && !bytes[end - 1].is_ascii_whitespace()
                    && !(sign == b'_' && single && word(end + 1))
            });
            let Some(end) = close else {
                at += 1;
                continue;
            };
            for index in (at..at + size).chain(end..end + size) {
                flags[index] |= SYNTAX | TAKEN;
            }
            for flag in &mut flags[at + size..end] {
                *flag |= style;
            }
            at = end + size;
        }
    }
}

/// Joins a line's bytes that look the same into runs.
fn push_runs(flags: &[u8], start: usize, out: &mut Vec<(Range<usize>, Mark)>) {
    let mark = |flag: u8| {
        if flag & SYNTAX != 0 {
            Some(Mark::Syntax)
        } else if flag & (BOLD | ITALIC | CODE) != 0 {
            Some(Mark::Style {
                bold: flag & BOLD != 0,
                italic: flag & ITALIC != 0,
                code: flag & CODE != 0,
            })
        } else {
            None
        }
    };
    let mut at = 0;
    while at < flags.len() {
        let here = mark(flags[at]);
        let mut end = at + 1;
        while end < flags.len() && mark(flags[end]) == here {
            end += 1;
        }
        if let Some(here) = here {
            out.push((start + at..start + end, here));
        }
        at = end;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The text of each run and how it's drawn.
    fn shown(text: &str) -> Vec<(&str, Mark)> {
        runs(text)
            .into_iter()
            .map(|(range, mark)| (&text[range], mark))
            .collect()
    }

    const fn style(bold: bool, italic: bool, code: bool) -> Mark {
        Mark::Style { bold, italic, code }
    }

    #[test]
    fn bold_keeps_its_stars_greyed() {
        assert_eq!(
            shown("**ciao** tu"),
            [
                ("**", Mark::Syntax),
                ("ciao", style(true, false, false)),
                ("**", Mark::Syntax)
            ]
        );
        assert!(runs("ciao").is_empty() && runs("** ciao**").is_empty());
    }

    #[test]
    fn italic_both_and_code() {
        assert_eq!(shown("*a*")[1], ("a", style(false, true, false)));
        assert_eq!(shown("_a_")[1], ("a", style(false, true, false)));
        assert_eq!(shown("***a***")[1], ("a", style(true, true, false)));
        assert_eq!(
            shown("`**x**`"),
            [
                ("`", Mark::Syntax),
                ("**x**", style(false, false, true)),
                ("`", Mark::Syntax)
            ]
        );
        assert!(
            runs("snake_case_name").is_empty(),
            "underscores inside a word"
        );
    }

    #[test]
    fn underline_strike_and_spoiler_grey_their_marks() {
        for text in ["__a__", "~~a~~", "||a||"] {
            assert_eq!(
                shown(text),
                [(&text[..2], Mark::Syntax), (&text[3..], Mark::Syntax)]
            );
        }
    }

    #[test]
    fn a_heading_and_a_fenced_block() {
        assert_eq!(
            shown("# Hi"),
            [("#", Mark::Syntax), (" Hi", style(true, false, false))]
        );
        let text = "```\nlet a = 1;\n```";
        assert_eq!(
            shown(text),
            [
                ("```", Mark::Syntax),
                ("let a = 1;", style(false, false, true)),
                ("```", Mark::Syntax)
            ]
        );
        assert!(
            shown("```\nopen")
                .iter()
                .all(|(_, mark)| *mark != style(false, false, true))
        );
    }
}
