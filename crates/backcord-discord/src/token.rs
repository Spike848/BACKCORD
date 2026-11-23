//! The Discord token lives in the OS credential store, next to BACKCORD's Matrix
//! session, and never in Matrix. So does the key [`crate::cache`] encrypts
//! its files with.

use std::sync::{Mutex, OnceLock};

use anyhow::{Result, anyhow, bail};
use keyring::Entry;

const SERVICE: &str = "io.github.spike848.BACKCORD";
/// The linked Discord account, set by linking by QR code or by the developer
/// probe's store-token command.
const ACCOUNT: &str = "discord-token";
/// Where the first probe kept the token; moved to `ACCOUNT` when read.
const EARLIER_ACCOUNT: &str = "discord-probe-token";
/// The key of the files [`crate::cache`] keeps, in hex.
const CACHE_KEY: &str = "discord-cache-key";
/// The accounts whose logins BACKCORD keeps to switch to, as JSON of their
/// ids, the last used first. Each one's login, name and avatar are kept
/// apart, as [`Kept`] under `SAVED_TOKEN` and its id, so no entry nears the
/// 2,560 bytes (1,280 UTF-16 characters) one entry of Windows' credential
/// store holds, however long the names and avatars.
const SAVED: &str = "discord-accounts";
const SAVED_TOKEN: &str = "discord-token-";
/// The most accounts kept.
const MOST_SAVED: usize = 8;

/// What is kept of a saved account under its own entry.
#[derive(serde::Serialize, serde::Deserialize)]
struct Kept {
    token: String,
    name: String,
    #[serde(default)]
    avatar: Option<String>,
}

/// An account BACKCORD keeps a login for, to switch to without signing in
/// again: its id, name and avatar as last seen.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Saved {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub avatar: Option<String>,
}

/// With the test server, what the credential store would keep, while
/// BACKCORD runs.
fn test_store() -> &'static Mutex<std::collections::HashMap<String, String>> {
    static STORE: OnceLock<Mutex<std::collections::HashMap<String, String>>> = OnceLock::new();
    STORE.get_or_init(Mutex::default)
}

fn read(account: &str) -> Result<Option<String>> {
    if crate::client::test_server().is_some() {
        return Ok(test_store()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(account)
            .cloned());
    }
    match named(account)?.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(_) => bail!("could not read a saved account from the OS credential store"),
    }
}

fn write(account: &str, value: Option<&str>) -> Result<()> {
    if crate::client::test_server().is_some() {
        let mut store = test_store()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match value {
            Some(value) => store.insert(account.to_owned(), value.to_owned()),
            None => store.remove(account),
        };
        return Ok(());
    }
    let entry = named(account)?;
    match value {
        Some(value) => entry
            .set_password(value)
            .map_err(|_| anyhow!("could not save an account in the OS credential store")),
        None => match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => bail!("could not remove an account from the OS credential store"),
        },
    }
}

/// The ids of the accounts kept, the last used first.
fn ids() -> Vec<String> {
    match read(SAVED) {
        Ok(Some(list)) => serde_json::from_str(&list).unwrap_or_default(),
        Ok(None) => vec![],
        Err(error) => {
            tracing::warn!(%error, "could not read the saved Discord accounts");
            vec![]
        }
    }
}

/// What is kept of the saved account `id`, if anything.
fn kept(id: &str) -> Result<Option<Kept>> {
    Ok(read(&format!("{SAVED_TOKEN}{id}"))?.and_then(|kept| serde_json::from_str(&kept).ok()))
}

/// The accounts kept to switch to, the last used first.
pub fn saved() -> Vec<Saved> {
    ids()
        .into_iter()
        .filter_map(|id| {
            let kept = kept(&id).ok().flatten()?;
            Some(Saved {
                id,
                name: kept.name,
                avatar: kept.avatar,
            })
        })
        .collect()
}

fn save_list(list: &[String]) -> Result<()> {
    if list.is_empty() {
        return write(SAVED, None);
    }
    write(SAVED, Some(&serde_json::to_string(list)?))
}

/// Puts `id` first in `list` and trims it to `MOST_SAVED`, the least
/// recently used going first but never `keep`; returns those dropped.
fn put_first(list: &mut Vec<String>, id: &str, keep: Option<&str>) -> Vec<String> {
    list.retain(|kept| kept != id);
    list.insert(0, id.to_owned());
    let mut dropped = Vec::new();
    while list.len() > MOST_SAVED {
        let Some(at) = list.iter().rposition(|kept| Some(kept.as_str()) != keep) else {
            break;
        };
        dropped.push(list.remove(at));
    }
    dropped
}

/// Keeps the linked account's login as `account`'s, to switch back to. The
/// account `keep`, about to be switched to, is never dropped to make room.
pub fn remember(account: &Saved, keep: Option<&str>) -> Result<()> {
    let kept = Kept {
        token: load()?,
        name: account.name.clone(),
        avatar: account.avatar.clone(),
    };
    write(
        &format!("{SAVED_TOKEN}{}", account.id),
        Some(&serde_json::to_string(&kept)?),
    )?;
    let mut list = ids();
    for dropped in put_first(&mut list, &account.id, keep) {
        write(&format!("{SAVED_TOKEN}{dropped}"), None)?;
    }
    save_list(&list)
}

/// Links the saved account `id` in place of the one linked, whose login
/// stays where [`remember`] put it. What the cache kept was the other's.
pub fn use_saved(id: &str) -> Result<()> {
    let kept = kept(id)?.ok_or_else(|| anyhow!("BACKCORD no longer has that account's login"))?;
    forget_linked()?;
    store(&kept.token)
}

/// Unlinks the account on this device without ending its session, for
/// another to sign in; a login [`remember`] kept stays.
pub fn park() -> Result<()> {
    forget_linked()
}

/// Forgets the saved account `id` and its login.
pub fn forget_saved(id: &str) -> Result<()> {
    write(&format!("{SAVED_TOKEN}{id}"), None)?;
    let mut list = ids();
    list.retain(|kept| kept != id);
    save_list(&list)
}

/// The linked login and the cache, but not the saved accounts.
fn forget_linked() -> Result<()> {
    crate::cache::forget();
    if crate::client::test_server().is_some() {
        *test_link()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
        return Ok(());
    }
    for account in [ACCOUNT, EARLIER_ACCOUNT, CACHE_KEY] {
        match named(account)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(_) => bail!("could not remove the token from the OS credential store"),
        }
    }
    Ok(())
}

fn entry() -> Result<Entry> {
    named(ACCOUNT)
}

fn named(account: &str) -> Result<Entry> {
    Entry::new(SERVICE, account).map_err(|_| anyhow!("could not access the OS credential store"))
}

/// With the test server, the link lasts while BACKCORD runs. It starts linked,
/// with [`crate::client::TEST_TOKEN`], unless `BACKCORD_DISCORD_TEST_UNLINKED` is
/// set, which shows linking against the fake.
fn test_link() -> &'static Mutex<Option<String>> {
    static LINK: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    LINK.get_or_init(|| {
        let unlinked = std::env::var_os("BACKCORD_DISCORD_TEST_UNLINKED").is_some();
        Mutex::new((!unlinked).then(|| crate::client::TEST_TOKEN.to_owned()))
    })
}

/// No Discord account is linked on this device, which [`load`] says rather
/// than failing.
#[derive(Debug)]
pub struct NotLinked;

impl std::fmt::Display for NotLinked {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("no Discord account is linked")
    }
}

impl std::error::Error for NotLinked {}

pub fn store(token: &str) -> Result<()> {
    let token = clean(token)?;
    if crate::client::test_server().is_some() {
        *test_link()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(token);
        return Ok(());
    }
    entry()?
        .set_password(&token)
        .map_err(|_| anyhow!("could not save the token in the OS credential store"))
}

pub fn load() -> Result<String> {
    if crate::client::test_server().is_some() {
        return test_link()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
            .ok_or_else(|| NotLinked.into());
    }
    match entry()?.get_password() {
        Ok(token) => return Ok(token),
        Err(keyring::Error::NoEntry) => {}
        Err(_) => bail!("could not read the token from the OS credential store"),
    }
    match named(EARLIER_ACCOUNT)?.get_password() {
        Ok(token) => {
            store(&token)?;
            let _ = named(EARLIER_ACCOUNT)?.delete_credential();
            Ok(token)
        }
        Err(keyring::Error::NoEntry) => Err(NotLinked.into()),
        Err(_) => bail!("could not read the token from the OS credential store"),
    }
}

/// The key [`crate::cache`] encrypts its files with, made the first time.
/// With the test server, always the same one, so what a run kept shows in
/// the next, as with Discord.
pub(crate) fn cache_key() -> Result<crate::cache::Key> {
    if crate::client::test_server().is_some() {
        return Ok([7; 32]);
    }
    let entry = named(CACHE_KEY)?;
    match entry.get_password() {
        Ok(hex) => {
            if let Some(key) = crate::cache::read_key(&hex) {
                return Ok(key);
            }
        }
        Err(keyring::Error::NoEntry) => {}
        Err(_) => bail!("could not read the cache key from the OS credential store"),
    }
    // None yet, or one that isn't a key: what it encrypted can't be read.
    let key = crate::cache::new_key();
    entry
        .set_password(&crate::cache::write_key(&key))
        .map_err(|_| anyhow!("could not save the cache key in the OS credential store"))?;
    Ok(key)
}

/// Forgets the linked account, and its saved login, which logging out
/// ends on Discord.
pub fn forget() -> Result<()> {
    if let Ok(token) = load() {
        for id in ids() {
            if kept(&id)
                .ok()
                .flatten()
                .is_some_and(|kept| kept.token == token)
            {
                forget_saved(&id)?;
            }
        }
    }
    forget_linked()
}

/// Accept the header value as copied from browser developer tools, with or
/// without surrounding quotes or whitespace.
pub fn clean(token: &str) -> Result<String> {
    let token = token.trim().trim_matches(['"', '\'']).trim();
    if token.is_empty() || token.contains(char::is_whitespace) {
        bail!("That doesn't look like a Discord token.");
    }
    Ok(token.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{MOST_SAVED, clean, put_first};

    #[test]
    fn the_account_switched_to_is_never_dropped_to_make_room() {
        let mut list: Vec<String> = (0..MOST_SAVED).map(|n| n.to_string()).collect();
        let oldest = list.last().unwrap().clone();
        // Linked by Add an Account, so not yet in the list.
        let dropped = put_first(&mut list, "new", Some(&oldest));
        assert_eq!(list.len(), MOST_SAVED);
        assert_eq!(list[0], "new");
        assert!(list.contains(&oldest));
        assert_eq!(dropped, [(MOST_SAVED - 2).to_string()]);
        // Without one to keep, the oldest goes.
        let dropped = put_first(&mut list, "newer", None);
        assert_eq!(dropped, [oldest]);
        // One already listed moves to the front, and none is dropped.
        assert!(put_first(&mut list, "3", None).is_empty());
        assert_eq!(list[0], "3");
    }

    #[test]
    fn cleans_copied_header_values() {
        assert_eq!(clean("  \"abc.def.ghi\"\n").unwrap(), "abc.def.ghi");
        assert!(clean("   ").is_err());
        assert!(clean("Bearer abc").is_err());
    }
}
