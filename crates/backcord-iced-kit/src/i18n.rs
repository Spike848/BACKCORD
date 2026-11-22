//! The interface's Italian: every label the screens draw goes through
//! `tr`, which looks its English text up in the table beside this file
//! and gives back the Italian. A text with no entry stays as written.

use std::collections::HashMap;
use std::sync::OnceLock;

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

/// The Italian for an English label, or the label itself when unknown (or
/// with the `english` feature, as the window's tests build it).
pub fn tr(english: &'static str) -> &'static str {
    if cfg!(feature = "english") {
        return english;
    }
    table().get(english).copied().unwrap_or(english)
}
