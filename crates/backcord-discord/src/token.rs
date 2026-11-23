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
/// The accounts whose logins BACKCORD keeps to switch to, as JSON of
/// [`Saved`]; each login is kept apart, under `SAVED_TOKEN` and its id.
const SAVED: &str = "discord-accounts";
const SAVED_TOKEN: &str = "discord-token-";
/// The most accounts kept: the list stays well within what one entry of
/// Windows' credential store holds.
const MOST_SAVED: usize = 8;

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

/// The accounts kept to switch to, the last used first.
pub fn saved() -> Vec<Saved> {
    match read(SAVED) {
        Ok(Some(list)) => serde_json::from_str(&list).unwrap_or_default(),
        Ok(None) => vec![],
        Err(error) => {
            tracing::warn!(%error, "could not read the saved Discord accounts");
            vec![]
        }
    }
}

fn save_list(list: &[Saved]) -> Result<()> {
    if list.is_empty() {
        return write(SAVED, None);
    }
    write(SAVED, Some(&serde_json::to_string(list)?))
}

/// Keeps the linked account's login as `account`'s, to switch back to.
pub fn remember(account: &Saved) -> Result<()> {
    let token = load()?;
    write(&format!("{SAVED_TOKEN}{}", account.id), Some(&token))?;
    let mut list = saved();
    list.retain(|kept| kept.id != account.id);
    list.insert(0, account.clone());
    for dropped in list.split_off(list.len().min(MOST_SAVED)) {
        write(&format!("{SAVED_TOKEN}{}", dropped.id), None)?;
    }
    save_list(&list)
}

/// Links the saved account `id` in place of the one linked, whose login
/// stays where [`remember`] put it. What the cache kept was the other's.
pub fn use_saved(id: &str) -> Result<()> {
    let token = read(&format!("{SAVED_TOKEN}{id}"))?
        .ok_or_else(|| anyhow!("BACKCORD no longer has that account's login"))?;
    forget_linked()?;
    store(&token)
}

/// Unlinks the account on this device without ending its session, for
/// another to sign in; a login [`remember`] kept stays.
pub fn park() -> Result<()> {
    forget_linked()
}

/// Forgets the saved account `id` and its login.
pub fn forget_saved(id: &str) -> Result<()> {
    write(&format!("{SAVED_TOKEN}{id}"), None)?;
    let mut list = saved();
    list.retain(|kept| kept.id != id);
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
        for kept in saved() {
            if read(&format!("{SAVED_TOKEN}{}", kept.id))
                .ok()
                .flatten()
                .is_some_and(|saved| saved == token)
            {
                forget_saved(&kept.id)?;
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
    use super::clean;

    #[test]
    fn cleans_copied_header_values() {
        assert_eq!(clean("  \"abc.def.ghi\"\n").unwrap(), "abc.def.ghi");
        assert!(clean("   ").is_err());
        assert!(clean("Bearer abc").is_err());
    }
}
