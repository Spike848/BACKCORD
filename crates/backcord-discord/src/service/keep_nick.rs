//! Anti Nickname Change: someone else changing this account's nickname in
//! a server where it may change its own is undone, back to what it was.
//! BACKCORD's own changes, which [`expect`] notes before they go, stay; one
//! made on another client looks like anyone else's, and is undone too.

use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

static ON: AtomicBool = AtomicBool::new(false);
static KEEPER: Mutex<Keeper> = Mutex::new(Keeper::new());

/// Turns Anti Nickname Change on or off.
pub fn set_keep_nickname(on: bool) {
    ON.store(on, Ordering::Relaxed);
}

/// BACKCORD is setting this account's nickname in `guild` to `nick`, none or
/// empty resetting it: its GUILD_MEMBER_UPDATE is this account's own.
pub(super) fn expect(guild: &str, nick: Option<&str>) {
    keeper().expect(guild, nick);
}

/// What [`expect`] noted for `guild` didn't go through.
pub(super) fn forget(guild: &str) {
    keeper().forget(guild);
}

/// What to set this account's nickname in `guild` back to, after a
/// GUILD_MEMBER_UPDATE took it from `before` to `after`; none to leave it.
/// `may` is whether it may change its own nickname there.
pub(super) fn restore(
    guild: &str,
    before: Option<&str>,
    after: Option<&str>,
    may: bool,
) -> Option<Option<String>> {
    let on = ON.load(Ordering::Relaxed);
    keeper().restore(guild, before, after, on && may, super::now_ms())
}

fn keeper() -> std::sync::MutexGuard<'static, Keeper> {
    KEEPER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// A nickname, none for an empty one, as Discord resets with either.
fn nick(nick: Option<&str>) -> Option<&str> {
    nick.filter(|nick| !nick.is_empty())
}

/// The nicknames BACKCORD is setting, and when it last restored one.
pub(super) struct Keeper {
    /// By server: the nickname on its way, until Discord shows it.
    expected: Vec<(String, Option<String>)>,
    /// When it restored a nickname lately, in ms: it stops after a few in a
    /// minute, as a bot that keeps renaming this account would.
    restores: Vec<u64>,
}

impl Keeper {
    const TIMES: usize = 5;
    const MINUTE: u64 = 60_000;

    pub(super) const fn new() -> Self {
        Self {
            expected: Vec::new(),
            restores: Vec::new(),
        }
    }

    pub(super) fn expect(&mut self, guild: &str, wanted: Option<&str>) {
        self.forget(guild);
        self.expected
            .push((guild.to_owned(), nick(wanted).map(str::to_owned)));
    }

    pub(super) fn forget(&mut self, guild: &str) {
        self.expected.retain(|(known, _)| known != guild);
    }

    /// See [`restore`]; `on` is whether it's on and may, `now` in ms.
    pub(super) fn restore(
        &mut self,
        guild: &str,
        before: Option<&str>,
        after: Option<&str>,
        on: bool,
        now: u64,
    ) -> Option<Option<String>> {
        let (before, after) = (nick(before), nick(after));
        if before == after {
            return None;
        }
        let own = self
            .expected
            .iter()
            .position(|(known, wanted)| known == guild && wanted.as_deref() == after);
        if let Some(own) = own {
            self.expected.remove(own);
            return None;
        }
        if !on {
            return None;
        }
        self.restores
            .retain(|at| now.saturating_sub(*at) < Self::MINUTE);
        if self.restores.len() >= Self::TIMES {
            return None;
        }
        self.restores.push(now);
        // Its own change back, when Discord shows it.
        self.expect(guild, before);
        Some(before.map(str::to_owned))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn someone_elses_change_goes_back_and_its_own_stays() {
        let mut keeper = Keeper::new();
        assert_eq!(
            keeper.restore("1", Some("Al"), Some("Clown"), true, 0),
            Some(Some("Al".into()))
        );
        // The change back is this account's own.
        assert_eq!(
            keeper.restore("1", Some("Clown"), Some("Al"), true, 1),
            None
        );
        // So is one BACKCORD makes.
        keeper.expect("1", Some("Alice"));
        assert_eq!(
            keeper.restore("1", Some("Al"), Some("Alice"), true, 2),
            None
        );
        // A nickname taken away comes back, and one given where there was
        // none goes.
        assert_eq!(
            keeper.restore("1", Some("Alice"), None, true, 3),
            Some(Some("Alice".into()))
        );
        assert_eq!(
            keeper.restore("2", None, Some("Clown"), true, 4),
            Some(None)
        );
        assert_eq!(
            keeper.restore("2", Some(""), None, true, 5),
            None,
            "the same"
        );
    }

    #[test]
    fn nothing_goes_back_while_off_or_not_allowed() {
        let mut keeper = Keeper::new();
        assert_eq!(
            keeper.restore("1", Some("Al"), Some("Clown"), false, 0),
            None
        );
    }

    #[test]
    fn it_gives_up_after_five_times_in_a_minute() {
        let mut keeper = Keeper::new();
        let restored = (0..8)
            .filter(|at| {
                keeper
                    .restore("1", Some("Al"), Some("Clown"), true, *at)
                    .is_some()
            })
            .count();
        assert_eq!(restored, 5);
        assert!(
            keeper
                .restore("1", Some("Al"), Some("Clown"), true, 61_000)
                .is_some(),
            "a minute later"
        );
    }
}
