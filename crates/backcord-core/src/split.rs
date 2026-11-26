//! Split Long Messages: a message longer than Discord takes goes as
//! several, each cut where the text breaks best: at a blank line, then a
//! line's end, then a space, and only inside a word when nothing else
//! fits. A code block cut in two is closed at the end of one part and
//! opened again, with its language, at the start of the next; a mention,
//! emoji or channel (`<...>`) and a link are never cut.

/// How many characters a code block's repair can add to a part: the
/// closing fence, and the opening one with a language.
const FENCE_ROOM: usize = 24;

/// `body` in parts of at most `limit` characters, in order. A body that
/// fits is its one part.
pub fn split_message(body: &str, limit: usize) -> Vec<String> {
    let limit = limit.max(FENCE_ROOM * 2);
    if body.chars().count() <= limit {
        return vec![body.to_owned()];
    }
    let mut parts = Vec::new();
    let mut rest = body.trim_start_matches('\n').to_owned();
    // The language of a code block the last part left open.
    let mut reopen: Option<String> = None;
    while !rest.is_empty() {
        let prefix = reopen
            .as_ref()
            .map(|language| format!("```{language}\n"))
            .unwrap_or_default();
        let text = format!("{prefix}{rest}");
        if text.chars().count() <= limit {
            parts.push(text);
            break;
        }
        let window = limit - FENCE_ROOM;
        let end = byte_at(&text, window);
        let cut = cut_at(&text, end, prefix.len());
        let (part, after) = text.split_at(cut);
        let mut part = part.trim_end().to_owned();
        let open = open_fence(&part);
        if open.is_some() {
            part.push_str("\n```");
        }
        if !part.trim().is_empty() {
            parts.push(part);
        }
        reopen = open;
        after.trim_start_matches([' ', '\n']).clone_into(&mut rest);
    }
    parts
}

/// The byte where the `chars`th character starts, or the text's end.
fn byte_at(text: &str, chars: usize) -> usize {
    text.char_indices()
        .nth(chars)
        .map_or(text.len(), |(at, _)| at)
}

/// Where to cut `text` at or before `end`, past `from`: the best break
/// in its second half, else in all of it, else `end`, moved back out of a
/// `<...>` or a link.
fn cut_at(text: &str, end: usize, from: usize) -> usize {
    let head = &text[..end];
    let half = from + (end - from) / 2;
    let found = |pattern: &str, after: usize| {
        head[after..]
            .rfind(pattern)
            .map(|at| after + at + pattern.len())
            .filter(|at| *at > from)
    };
    let cut = [half, from]
        .into_iter()
        .find_map(|after| {
            found("\n\n", after)
                .or_else(|| found("\n", after))
                .or_else(|| found(" ", after))
        })
        .unwrap_or(end);
    unbroken(text, cut, from)
}

/// `cut` moved back before a `<...>` or a link it falls inside, unless
/// that leaves nothing before it.
fn unbroken(text: &str, cut: usize, from: usize) -> usize {
    let head = &text[..cut];
    let word = head
        .rfind(|c: char| c.is_whitespace())
        .map_or(0, |at| at + 1);
    let mut start = cut;
    if let Some(open) = head.rfind('<')
        && !head[open..].contains('>')
    {
        start = start.min(open);
    }
    if head[word..].contains("://") {
        start = start.min(word);
    }
    if start > from { start } else { cut }
}

/// The language of the code block `text` leaves open, as its opening
/// fence names it (empty for none); none when every block is closed.
fn open_fence(text: &str) -> Option<String> {
    let mut open: Option<String> = None;
    let mut rest = text;
    while let Some(at) = rest.find("```") {
        let after = &rest[at + 3..];
        open = match open {
            Some(_) => None,
            None => Some(
                after
                    .split(|c: char| c.is_whitespace())
                    .next()
                    .filter(|word| {
                        word.chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-')
                    })
                    .unwrap_or_default()
                    .to_owned(),
            ),
        };
        rest = after;
    }
    open
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fits(parts: &[String], limit: usize) {
        for part in parts {
            assert!(
                part.chars().count() <= limit,
                "{} > {limit}",
                part.chars().count()
            );
            assert!(!part.trim().is_empty());
        }
    }

    #[test]
    fn a_message_that_fits_is_one_part() {
        assert_eq!(split_message("hello", 2000), ["hello"]);
        let exact = "a".repeat(2000);
        assert_eq!(split_message(&exact, 2000), [exact]);
    }

    #[test]
    fn parts_break_at_lines_then_spaces() {
        let line = "word ".repeat(30);
        let body = format!("{line}\n").repeat(40);
        let parts = split_message(&body, 2000);
        assert!(parts.len() > 1);
        fits(&parts, 2000);
        // Each part but the last ends where a line did.
        for part in &parts[..parts.len() - 1] {
            assert!(part.ends_with("word"));
        }
        let words: usize = parts
            .iter()
            .map(|part| part.split_whitespace().count())
            .sum();
        assert_eq!(words, 30 * 40, "no word lost or cut");
    }

    #[test]
    fn one_huge_word_is_cut_anyway() {
        let body = "x".repeat(4500);
        let parts = split_message(&body, 2000);
        fits(&parts, 2000);
        assert_eq!(parts.concat(), body);
    }

    #[test]
    fn a_code_block_cut_in_two_is_closed_and_opened_again() {
        let code = "let x = 1;\n".repeat(300);
        let body = format!("Look:\n```rust\n{code}```\nDone.");
        let parts = split_message(&body, 2000);
        assert!(parts.len() > 1);
        fits(&parts, 2000);
        for part in &parts {
            assert_eq!(part.matches("```").count() % 2, 0, "balanced: {part}");
        }
        assert!(parts[1].starts_with("```rust\n"));
    }

    #[test]
    fn mentions_and_links_are_not_cut() {
        let filler = "a".repeat(1990);
        let body = format!("{filler} <@123456789012345678> https://example.com/a/long/path rest");
        let parts = split_message(&body, 2000);
        fits(&parts, 2000);
        assert!(
            parts
                .iter()
                .any(|part| part.contains("<@123456789012345678>"))
        );
        assert!(
            parts
                .iter()
                .any(|part| part.contains("https://example.com/a/long/path"))
        );
    }
}
