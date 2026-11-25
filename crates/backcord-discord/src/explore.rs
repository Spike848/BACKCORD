//! Italian Discord servers listed on DISBOARD, for Explore servers: its
//! pages read as a browser reads them, and the server's public invite,
//! when its widget offers one, to join it in BACKCORD. DISBOARD's own
//! invites wait behind a human check, which BACKCORD leaves to the browser.
//! See "Exploring Italian servers" in docs/discord/servers.md.

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Mutex, OnceLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, bail};
use reqwest::Client;
use serde_json::Value;

/// DISBOARD's pages BACKCORD reads, in order: the Italian servers with
/// the most members, the latest bumped, then those tagged as Italian. Each holds about two dozen.
pub const SOURCES: [&str; 7] = [
    "https://disboard.org/it/servers?fl=it&sort=-member_count",
    "https://disboard.org/it/servers?fl=it",
    "https://disboard.org/it/servers/tag/italia",
    "https://disboard.org/it/servers/tag/italiano",
    "https://disboard.org/it/servers/tag/ita",
    "https://disboard.org/it/servers/tag/italian",
    "https://disboard.org/it/servers/tag/italy",
];
const TIMEOUT: Duration = Duration::from_secs(15);
/// What [`page`] fails with when DISBOARD asks to slow down.
pub const SLOW_DOWN: &str = "DISBOARD asks to slow down";
/// How much of a description the list keeps.
const DESCRIPTION: usize = 3000;

/// A server as DISBOARD lists it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Listed {
    /// Its Discord id.
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub description: String,
    pub tags: Vec<String>,
    /// How many of its members are online, as DISBOARD last counted.
    pub online: Option<u64>,
}

/// How long a page read from DISBOARD is used before it's read again.
const PAGE_FRESH: u64 = 30 * 60;

/// What BACKCORD read from DISBOARD, kept across restarts so that it asks
/// again only when it's old, and has something to show while DISBOARD asks
/// to slow down.
#[derive(Default, serde::Serialize, serde::Deserialize)]
struct Saved {
    /// Each page, by its index into [`SOURCES`], and when it was read, in
    /// Unix seconds.
    #[serde(default)]
    pages: HashMap<usize, (u64, Vec<Listed>)>,
}

static FILE: OnceLock<PathBuf> = OnceLock::new();

/// Keeps what's read from DISBOARD in `path` across restarts.
pub fn keep_in(path: PathBuf) {
    let _ = FILE.set(path);
}

fn saved() -> &'static Mutex<Saved> {
    static SAVED: OnceLock<Mutex<Saved>> = OnceLock::new();
    SAVED.get_or_init(|| {
        let kept = FILE
            .get()
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Mutex::new(kept)
    })
}

fn store(saved: &Saved) {
    let Some(path) = FILE.get() else {
        return;
    };
    let Ok(bytes) = serde_json::to_vec(saved) else {
        return;
    };
    let partial = path.with_extension("json.part");
    if let Err(error) =
        std::fs::write(&partial, bytes).and_then(|()| std::fs::rename(&partial, path))
    {
        tracing::warn!(%error, "could not keep DISBOARD's servers");
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// A client of its own, so that Discord's login never reaches DISBOARD.
fn http() -> reqwest::Result<Client> {
    crate::net::configure(Client::builder())
        .user_agent(crate::client::user_agent())
        .connect_timeout(TIMEOUT)
        .timeout(TIMEOUT)
        .build()
}

/// The servers on DISBOARD's page `source` (an index into [`SOURCES`]),
/// and whether they're the ones kept from before because DISBOARD didn't
/// answer: read lately, they're used as they are.
pub async fn page(source: usize) -> Result<(Vec<Listed>, bool)> {
    let kept = saved()
        .lock()
        .expect("DISBOARD's servers")
        .pages
        .get(&source)
        .cloned();
    if let Some((at, listed)) = &kept
        && now().saturating_sub(*at) < PAGE_FRESH
    {
        return Ok((listed.clone(), false));
    }
    match read_page(source).await {
        Ok(listed) => {
            let mut saved = saved().lock().expect("DISBOARD's servers");
            saved.pages.insert(source, (now(), listed.clone()));
            store(&saved);
            Ok((listed, false))
        }
        Err(error) => match kept {
            Some((_, listed)) => {
                tracing::info!(error = format!("{error:#}"), "DISBOARD's page as kept");
                Ok((listed, true))
            }
            None => Err(error),
        },
    }
}

async fn read_page(source: usize) -> Result<Vec<Listed>> {
    let Some(url) = SOURCES.get(source) else {
        return Ok(vec![]);
    };
    let response = http()?
        .get(*url)
        .send()
        .await
        .context("DISBOARD didn't answer")?;
    let status = response.status();
    if status.as_u16() == 429 {
        bail!(SLOW_DOWN);
    }
    if !status.is_success() {
        bail!("DISBOARD answered {status}");
    }
    let html = response
        .text()
        .await
        .context("couldn't read DISBOARD's page")?;
    let listed = parse(&html);
    if listed.is_empty() && html.len() < 200 && html.contains("1015") {
        bail!(SLOW_DOWN);
    }
    if listed.is_empty() && html.contains("challenge-platform") {
        bail!("DISBOARD asked for a human check");
    }
    Ok(listed)
}

/// The invite code of a server's public widget, if it offers one; asked
/// without this account's login, as anyone may.
pub async fn widget_invite(guild: &str) -> Result<Option<String>> {
    let url = format!("https://discord.com/api/v9/guilds/{guild}/widget.json");
    let response = http()?
        .get(url)
        .send()
        .await
        .context("Discord didn't answer")?;
    // 403 and 404: the widget is off, or there's no such server.
    if matches!(response.status().as_u16(), 403 | 404) {
        return Ok(None);
    }
    let body = response
        .bytes()
        .await
        .context("couldn't read Discord's answer")?;
    let answer: Value =
        serde_json::from_slice(&body).context("couldn't understand Discord's answer")?;
    Ok(invite_of(&answer))
}

/// The invite code in a widget's `instant_invite`.
pub fn invite_of(widget: &Value) -> Option<String> {
    let link = widget["instant_invite"].as_str()?;
    crate::invite::typed_code(link).map(str::to_owned)
}

/// Reads the servers of one of DISBOARD's listing pages.
pub fn parse(html: &str) -> Vec<Listed> {
    let mut listed: Vec<Listed> = vec![];
    let mut starts: Vec<usize> = html
        .match_indices("listing-card")
        .map(|(at, _)| at)
        .collect();
    starts.push(html.len());
    for pair in starts.windows(2) {
        let card = &html[pair[0]..pair[1]];
        let Some(id) = card_id(card) else {
            continue;
        };
        if listed.iter().any(|known| known.id == id) {
            continue;
        }
        let name = after(card, "class=\"server-name\"")
            .and_then(|rest| attribute(rest, "title"))
            .map(|name| unescape(&name))
            .unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        let icon = after(card, "class=\"server-icon\"")
            .and_then(|rest| attribute(rest, "data-src").or_else(|| attribute(rest, "src")))
            .filter(|url| url.starts_with("https://cdn.discordapp.com/"))
            .map(|url| match url.split_once('?') {
                Some((bare, _)) => format!("{bare}?size=96"),
                None => format!("{url}?size=96"),
            });
        let online = after(card, "class=\"server-online\">")
            .and_then(|rest| rest.split('<').next())
            .and_then(|count| count.trim().replace(['.', ','], "").parse().ok());
        let tags = after(card, "class=\"server-tags\"")
            .map(|rest| rest.split("</ul>").next().unwrap_or(rest))
            .map(|block| {
                block
                    .split("class=\"tag\"")
                    .skip(1)
                    .filter_map(|tag| attribute(tag, "title"))
                    .map(|tag| unescape(&tag))
                    .collect()
            })
            .unwrap_or_default();
        let description = after(card, "elastic-text-inner\">")
            .and_then(|rest| rest.split("</div>").next())
            .map(plain)
            .unwrap_or_default();
        listed.push(Listed {
            id,
            name,
            icon,
            description,
            tags,
            online,
        });
    }
    listed
}

/// The id in a card's `server-{id}` class.
fn card_id(card: &str) -> Option<String> {
    let head = card.get(..card.len().min(200))?;
    let rest = head.split("server-").nth(1)?;
    let id: String = rest.chars().take_while(char::is_ascii_digit).collect();
    (id.len() >= 15).then_some(id)
}

fn after<'a>(text: &'a str, marker: &str) -> Option<&'a str> {
    text.find(marker).map(|at| &text[at + marker.len()..])
}

/// The value of the first `name="..."` in `text`.
fn attribute(text: &str, name: &str) -> Option<String> {
    let marker = format!(" {name}=\"");
    let rest = after(text, &marker)?;
    Some(rest.split('"').next()?.to_owned())
}

/// A description's HTML as plain text: its line breaks kept, its spaces
/// gathered, cut at [`DESCRIPTION`] characters.
fn plain(html: &str) -> String {
    let html = html.replace("<br />", "\n").replace("<br>", "\n");
    let mut text = String::new();
    let mut inside = false;
    for character in html.chars() {
        match character {
            '<' => inside = true,
            '>' => inside = false,
            _ if !inside => text.push(character),
            _ => {}
        }
    }
    let text = unescape(&text);
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let joined = lines.join("\n");
    if joined.chars().count() > DESCRIPTION {
        let cut: String = joined.chars().take(DESCRIPTION).collect();
        format!("{}…", cut.trim_end())
    } else {
        joined
    }
}

/// HTML's entities as their characters. DISBOARD escapes some names
/// twice, so what's left after one pass is read once more.
fn unescape(text: &str) -> String {
    let once = unescape_once(text);
    if once.contains('&') {
        unescape_once(&once)
    } else {
        once
    }
}

fn unescape_once(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let Some(end) = rest.find(';').filter(|end| *end <= 10) else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let entity = &rest[1..end];
        let character = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "nbsp" => Some(' '),
            _ => entity.strip_prefix('#').and_then(|number| {
                match number.strip_prefix(['x', 'X']) {
                    Some(hex) => u32::from_str_radix(hex, 16).ok(),
                    None => number.parse().ok(),
                }
                .and_then(char::from_u32)
            }),
        };
        match character {
            Some(character) => {
                out.push(character);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const CARD: &str = r#"<div class="listing-page" data-page="1">
<div
  class="
    listing-card
        server-853596936241283082    has-review      "
    data-rank="1">
<div class="server-header">
    <div class="server-icon" >
        <a href="/it/server/853596936241283082" target="_blank">
            <img  data-src="https://cdn.discordapp.com/icons/853596936241283082/834b.jpg" alt="x" width="60" class="lazyload">
        </a>
    </div>
    <div class="server-info">
        <div class="server-misc" >
            <span class="server-online">60</span>
        </div>
        <div class="server-name">
            <a href="/it/server/853596936241283082" target="_blank" title="Rucus&amp;amp;Oscar | Chill">
        </div>
    </div>
</div>
<div class="server-tags">
<ul class="tags">
  <li><a class="tag" href="/it/servers/tag/community" title="community" data-id="6"></a></li>
  <li><a class="tag" href="/it/servers/tag/ita" title="ita" data-id="535"></a></li>
</ul>
</div>
<div class="server-description is-elastic-text">
  <div class="elastic-text-inner">
    Siamo simpatici &amp; attivi ❤️<br />
<br />
Qui l&#039;importante è divertirsi
  </div>
</div>
<div
  class="
    listing-card
        server-1505523806754570282"
    data-rank="2">
</div>"#;

    #[test]
    fn reads_a_listing() {
        let listed = parse(CARD);
        assert_eq!(listed.len(), 1, "a card without a name is left out");
        let server = &listed[0];
        assert_eq!(server.id, "853596936241283082");
        assert_eq!(server.name, "Rucus&Oscar | Chill");
        assert_eq!(
            server.icon.as_deref(),
            Some("https://cdn.discordapp.com/icons/853596936241283082/834b.jpg?size=96")
        );
        assert_eq!(server.online, Some(60));
        assert_eq!(server.tags, ["community", "ita"]);
        assert_eq!(
            server.description,
            "Siamo simpatici & attivi ❤️\nQui l'importante è divertirsi"
        );
    }

    #[test]
    fn reads_a_widgets_invite() {
        let widget = serde_json::json!({"instant_invite": "https://discord.com/invite/abcDEF"});
        assert_eq!(invite_of(&widget).as_deref(), Some("abcDEF"));
        assert_eq!(
            invite_of(&serde_json::json!({"instant_invite": null})),
            None
        );
    }
}
