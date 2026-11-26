//! Export Chat: a Discord channel's, DM's or group's history saved to a
//! file, as readable text, a simple web page or Discord's own JSON. The
//! window asks for the format, how much and where; the backend
//! (`discord_text/export.rs`) fetches the messages a page at a time and
//! writes what [`render`] makes of them. See "Export Chat" in
//! docs/discord/messages.md.

use std::path::PathBuf;

use chrono::{Local, NaiveDate, TimeZone};

/// What the file holds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExportFormat {
    /// One line a message, `[dd/mm/yyyy hh:mm] Author: text`, its files
    /// under it.
    #[default]
    Text,
    /// A web page with a simple style, pictures as links.
    Html,
    /// Discord's message objects, oldest first.
    Json,
}

impl ExportFormat {
    pub const ALL: [ExportFormat; 3] = [Self::Text, Self::Html, Self::Json];

    /// The file's extension, without its dot.
    pub fn extension(self) -> &'static str {
        match self {
            Self::Text => "txt",
            Self::Html => "html",
            Self::Json => "json",
        }
    }
}

/// How much of the history goes in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportRange {
    All,
    /// The newest this many.
    Last(u32),
    /// From this time on, in Unix milliseconds.
    Since(u64),
}

/// An export the backend carries out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatExport {
    /// The channel's, DM's or group's room id.
    pub room: String,
    /// What the file's title says: the channel's or the DM's name.
    pub title: String,
    pub format: ExportFormat,
    pub range: ExportRange,
    /// Its files and pictures, as links.
    pub attachments: bool,
    /// Where the file goes.
    pub path: PathBuf,
}

/// How an export ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExportEnd {
    /// The file was written here.
    Saved(PathBuf),
    /// It was stopped, and wrote nothing.
    Stopped,
    /// It failed, and why.
    Failed(String),
}

/// The file's suggested name: the place's name, then the day, as
/// `general-2026-11-25.txt`, without what file systems refuse in one.
pub fn file_name(title: &str, format: ExportFormat, today: NaiveDate) -> String {
    let name: String = title
        .trim()
        .trim_start_matches('#')
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                c
            }
        })
        .collect();
    let name = name.trim().trim_end_matches('.');
    let name = if name.is_empty() { "chat" } else { name };
    format!("{name}-{}.{}", today.format("%Y-%m-%d"), format.extension())
}

/// A day typed as `dd/mm/yyyy`, `dd-mm-yyyy`, `dd.mm.yyyy` or
/// `yyyy-mm-dd`.
pub fn parse_day(text: &str) -> Option<NaiveDate> {
    let text = text.trim();
    ["%d/%m/%Y", "%d-%m-%Y", "%d.%m.%Y", "%Y-%m-%d"]
        .iter()
        .find_map(|format| NaiveDate::parse_from_str(text, format).ok())
}

/// The start of a day in the local time zone, in Unix milliseconds.
pub fn day_start_ms(day: NaiveDate) -> u64 {
    let midnight = day.and_hms_opt(0, 0, 0).unwrap_or_default();
    let at = Local.from_local_datetime(&midnight).earliest().map_or_else(
        || midnight.and_utc().timestamp_millis(),
        |at| at.timestamp_millis(),
    );
    u64::try_from(at).unwrap_or_default()
}

/// The folder files are saved to: the system's Downloads, else the home
/// folder.
pub fn downloads() -> Option<PathBuf> {
    let dirs = directories::UserDirs::new()?;
    Some(
        dirs.download_dir()
            .map_or_else(|| dirs.home_dir().to_owned(), ToOwned::to_owned),
    )
}

/// The file's contents for Discord's message objects, oldest first, with
/// times in the local time zone.
#[cfg(feature = "discord")]
pub fn render(
    messages: &[serde_json::Value],
    title: &str,
    format: ExportFormat,
    attachments: bool,
) -> String {
    render_in(messages, title, format, attachments, &Local)
}

/// [`render`], with times in `zone`.
#[cfg(feature = "discord")]
pub fn render_in<Tz: TimeZone>(
    messages: &[serde_json::Value],
    title: &str,
    format: ExportFormat,
    attachments: bool,
    zone: &Tz,
) -> String
where
    Tz::Offset: std::fmt::Display,
{
    match format {
        ExportFormat::Json => json(messages, attachments),
        ExportFormat::Text | ExportFormat::Html => {
            let names = names(messages);
            let lines: Vec<Line> = messages
                .iter()
                .filter_map(|value| line(value, &names, attachments, zone))
                .collect();
            if format == ExportFormat::Text {
                text(&lines)
            } else {
                html(title, &lines)
            }
        }
    }
}

/// A message as the text and the page show it.
#[cfg(feature = "discord")]
struct Line {
    when: String,
    author: String,
    /// What it says, or a notice's words after the author's name.
    text: String,
    notice: bool,
    /// Its files, by name and address.
    files: Vec<(String, String)>,
}

#[cfg(feature = "discord")]
fn line<Tz: TimeZone>(
    value: &serde_json::Value,
    names: &std::collections::HashMap<String, String>,
    attachments: bool,
    zone: &Tz,
) -> Option<Line>
where
    Tz::Offset: std::fmt::Display,
{
    use backcord_discord::message::{self, Shown};
    let parsed = message::parse(value)?;
    let ms = i64::try_from(message::snowflake_ms(&parsed.id)).unwrap_or_default();
    let when = zone
        .timestamp_millis_opt(ms)
        .single()
        .map(|at| at.format("%d/%m/%Y %H:%M").to_string())
        .unwrap_or_default();
    let author = backcord_discord::ready::display_name(&value["author"])
        .map_or_else(|| parsed.author.clone(), str::to_owned);
    let (text, notice) = match &parsed.system {
        Some(system) => (system.clone(), true),
        None => (readable(&parsed.content, names), false),
    };
    let files = if attachments {
        parsed
            .attachments
            .iter()
            .filter(|file| matches!(file.shown, Shown::Message | Shown::Sticker))
            .map(|file| (file.name.clone(), file.url.to_string()))
            .collect()
    } else {
        Vec::new()
    };
    Some(Line {
        when,
        author,
        text,
        notice,
        files,
    })
}

/// The names of the people the messages mention or are by, by user id.
#[cfg(feature = "discord")]
fn names(messages: &[serde_json::Value]) -> std::collections::HashMap<String, String> {
    let mut names = std::collections::HashMap::new();
    for value in messages {
        let people = value["mentions"]
            .as_array()
            .into_iter()
            .flatten()
            .chain(std::iter::once(&value["author"]));
        for person in people {
            if let (Some(id), Some(name)) = (
                person["id"].as_str(),
                backcord_discord::ready::display_name(person),
            ) {
                names.insert(id.to_owned(), name.to_owned());
            }
        }
    }
    names
}

/// Discord Markdown made readable: mentions as `@name`, custom emoji as
/// `:name:`, timestamps as times.
#[cfg(feature = "discord")]
fn readable(content: &str, names: &std::collections::HashMap<String, String>) -> String {
    let content = crate::clock::render_timestamps(content);
    let mut out = String::with_capacity(content.len());
    let mut rest = content.as_str();
    while let Some(at) = rest.find('<') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let Some(end) = rest.find('>') else {
            break;
        };
        let inside = &rest[1..end];
        let shown = if let Some(id) = inside
            .strip_prefix("@!")
            .or_else(|| inside.strip_prefix('@'))
            .filter(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()))
        {
            Some(format!("@{}", names.get(id).map_or(id, String::as_str)))
        } else {
            let emoji = inside.strip_prefix("a:").unwrap_or(inside);
            emoji
                .strip_prefix(':')
                .and_then(|emoji| emoji.split_once(':'))
                .filter(|(name, id)| {
                    !name.is_empty() && !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit())
                })
                .map(|(name, _)| format!(":{name}:"))
        };
        match shown {
            Some(shown) => {
                out.push_str(&shown);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('<');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(feature = "discord")]
fn text(lines: &[Line]) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    for line in lines {
        let between = if line.notice { " " } else { ": " };
        let _ = writeln!(out, "[{}] {}{between}{}", line.when, line.author, line.text);
        for (name, url) in &line.files {
            let _ = writeln!(out, "    {name}: {url}");
        }
    }
    out
}

/// Text made safe to put in a web page.
#[cfg(feature = "discord")]
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

#[cfg(feature = "discord")]
fn html(title: &str, lines: &[Line]) -> String {
    use std::fmt::Write;
    const STYLE: &str = "body{font-family:sans-serif;background:#313338;color:#dbdee1;margin:0;padding:24px}\
h1{font-size:20px;color:#f2f3f5}\
.m{padding:4px 0;line-height:1.4}\
.t{color:#949ba4;font-size:12px;margin-right:8px}\
.a{color:#f2f3f5;font-weight:600;margin-right:6px}\
.n{color:#949ba4;font-style:italic}\
.x{white-space:pre-wrap;word-wrap:break-word}\
.f{margin-left:24px;font-size:14px}\
a{color:#00a8fc}";
    let title = escape(title);
    let mut out = format!(
        "<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<title>{title}</title>\n<style>{STYLE}</style>\n</head>\n<body>\n<h1>{title}</h1>\n"
    );
    for line in lines {
        let class = if line.notice { "x n" } else { "x" };
        let _ = write!(
            out,
            "<div class=\"m\"><span class=\"t\">{}</span><span class=\"a\">{}</span><span class=\"{class}\">{}</span>",
            escape(&line.when),
            escape(&line.author),
            escape(&line.text)
        );
        for (name, url) in &line.files {
            let _ = write!(
                out,
                "<div class=\"f\"><a href=\"{}\">{}</a></div>",
                escape(url),
                escape(name)
            );
        }
        out.push_str("</div>\n");
    }
    out.push_str("</body>\n</html>\n");
    out
}

#[cfg(feature = "discord")]
fn json(messages: &[serde_json::Value], attachments: bool) -> String {
    let messages: Vec<serde_json::Value> = messages
        .iter()
        .map(|value| {
            let mut value = value.clone();
            if !attachments && let Some(object) = value.as_object_mut() {
                object.remove("attachments");
            }
            value
        })
        .collect();
    serde_json::to_string_pretty(&messages).unwrap_or_default() + "\n"
}

#[cfg(test)]
mod tests;
