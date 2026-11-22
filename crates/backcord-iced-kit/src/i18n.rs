//! The interface's Italian: every label the screens draw goes through
//! `tr`, which looks its English text up in the table beside this file
//! and gives back the Italian. A text with no entry stays as written.
//!
//! Text made at run time (a count, a name, a time, an error the core or
//! Discord wrote) goes through [`t`]: an entry whose English has `{}` in
//! it is a template, matched against the whole text, each `{}` taking the
//! words in its place, which go into the Italian's `{}` in order, or by
//! number as `{0}`, `{1}`, each translated in turn.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

const TABLE: &str = include_str!("i18n.tsv");

fn table() -> &'static HashMap<&'static str, &'static str> {
    static TABLE_MAP: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    TABLE_MAP.get_or_init(|| {
        TABLE
            .lines()
            .filter_map(|line| line.split_once('\t'))
            .collect()
    })
}

/// The table's templates: the English's pieces around its `{}`s, and the
/// Italian, longest English first, so the most specific matches first.
fn templates() -> &'static [(Vec<&'static str>, &'static str)] {
    static TEMPLATES: OnceLock<Vec<(Vec<&'static str>, &'static str)>> = OnceLock::new();
    TEMPLATES.get_or_init(|| {
        let mut templates: Vec<(Vec<&'static str>, &'static str)> = table()
            .iter()
            .filter(|(english, _)| english.contains("{}"))
            .map(|(english, italian)| (english.split("{}").collect(), *italian))
            .collect();
        templates.sort_by_key(|(pieces, _)| {
            std::cmp::Reverse(pieces.iter().map(|piece| piece.len()).sum::<usize>())
        });
        templates
    })
}

/// The Italian for an English label, or the label itself when unknown (or
/// with the `english` feature, as the window's tests build it).
pub fn tr(english: &'static str) -> &'static str {
    if cfg!(feature = "english") {
        return english;
    }
    table().get(english).copied().unwrap_or(english)
}

/// The Italian for any English text, by the table's entry for it or a
/// template it fits; the text as it is when none does (or with the
/// `english` feature).
pub fn t(english: impl AsRef<str>) -> String {
    let english = english.as_ref();
    if cfg!(feature = "english") || english.is_empty() {
        return english.to_owned();
    }
    static KNOWN: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    let known = KNOWN.get_or_init(Mutex::default);
    if let Some(italian) = known
        .lock()
        .ok()
        .and_then(|known| known.get(english).cloned())
    {
        return italian;
    }
    let italian = translate(english, 0);
    if let Ok(mut known) = known.lock() {
        if known.len() > 20_000 {
            known.clear();
        }
        known.insert(english.to_owned(), italian.clone());
    }
    italian
}

fn translate(english: &str, depth: u8) -> String {
    if let Some(italian) = table().get(english) {
        return (*italian).to_owned();
    }
    if depth > 2 {
        return english.to_owned();
    }
    for (pieces, italian) in templates() {
        if let Some(words) = fit(pieces, english) {
            let words: Vec<String> = words
                .into_iter()
                .map(|word| translate(word, depth + 1))
                .collect();
            return fill(italian, &words);
        }
    }
    english.to_owned()
}

/// The words in place of each `{}` when `text` fits the template cut into
/// `pieces`: each piece found in turn, the first at the start and the last
/// at the end, and no word empty.
fn fit<'a>(pieces: &[&str], text: &'a str) -> Option<Vec<&'a str>> {
    let (first, rest) = pieces.split_first()?;
    let mut left = text.strip_prefix(first)?;
    let mut words = Vec::with_capacity(rest.len());
    for (index, piece) in rest.iter().enumerate() {
        let last = index + 1 == rest.len();
        let at = if last {
            if !left.ends_with(piece) {
                return None;
            }
            left.len() - piece.len()
        } else if piece.is_empty() {
            return None;
        } else {
            // The first that leaves a word before it.
            left.get(1..)?.find(piece)? + 1
        };
        let word = &left[..at];
        if word.is_empty() {
            return None;
        }
        words.push(word);
        left = &left[at + piece.len()..];
    }
    Some(words)
}

/// `italian` with its `{}`s, or `{0}`, `{1}` and on, filled with `words`.
fn fill(italian: &str, words: &[String]) -> String {
    let mut out = String::with_capacity(italian.len() + 16);
    let mut next = 0;
    let mut rest = italian;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else {
            out.push_str(&rest[open..]);
            return out;
        };
        let inside = &after[..close];
        let index = if inside.is_empty() {
            next += 1;
            Some(next - 1)
        } else {
            inside.parse::<usize>().ok()
        };
        match index.and_then(|index| words.get(index)) {
            Some(word) => out.push_str(word),
            None => out.push_str(&rest[open..open + close + 2]),
        }
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_template_takes_the_words_in_its_place() {
        let pieces: Vec<&str> = "For {} hours".split("{}").collect();
        assert_eq!(fit(&pieces, "For 3 hours"), Some(vec!["3"]));
        assert_eq!(fit(&pieces, "For hours"), None);
        let two: Vec<&str> = "{} and {} are typing...".split("{}").collect();
        assert_eq!(
            fit(&two, "Ann and Bo are typing..."),
            Some(vec!["Ann", "Bo"])
        );
        assert_eq!(
            fill("{1} prima di {0}", &["a".into(), "b".into()]),
            "b prima di a"
        );
        assert_eq!(fill("Per {} ore", &["3".into()]), "Per 3 ore");
    }
}
