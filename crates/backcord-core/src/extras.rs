//! Extra features the core does itself, each switched by the window as
//! Settings' Extra Features say: atomics, read where they act. Each starts
//! off; the window turns on those its preferences keep on.

use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU16, Ordering};

use crate::relationship_alerts::Kinds;

static MESSAGE_LOGGER: AtomicBool = AtomicBool::new(false);
/// Split Long Messages: a message over Discord's limit goes as several.
static SPLIT_LONG: AtomicBool = AtomicBool::new(false);
/// Message Logger keeps this account's own messages too.
static LOG_OWN: AtomicBool = AtomicBool::new(false);
/// This account's Discord user id, as the channels last said.
static ME: std::sync::RwLock<String> = std::sync::RwLock::new(String::new());

/// Relationship Notifier's removals told of, as [`Kinds::bits`].
static RELATIONSHIP_ALERTS: AtomicU8 = AtomicU8::new(0);

/// Fake Nitro and its settings, as [`FakeNitro`] has them.
static FAKE_NITRO: AtomicBool = AtomicBool::new(false);
static FAKE_NITRO_SIZE: AtomicU16 = AtomicU16::new(crate::fake_nitro::DEFAULT_SIZE);
static FAKE_NITRO_MASKED: AtomicBool = AtomicBool::new(true);
static FAKE_NITRO_RECEIVED: AtomicBool = AtomicBool::new(true);

#[cfg(test)]
thread_local! {
    /// A test's own Message Logger, so it doesn't change the others'.
    static TEST_LOGGER: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
    /// A test's own Split Long Messages: off unless the test turns it on.
    static TEST_SPLIT_LONG: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// A test's own Relationship Notifier: off unless the test turns it on,
    /// so the lists the other tests change tell of nothing.
    static TEST_RELATIONSHIPS: std::cell::Cell<u8> = const { std::cell::Cell::new(0) };
    /// A test's own Fake Nitro: off unless the test turns it on.
    static TEST_FAKE_NITRO: std::cell::Cell<FakeNitro> = const { std::cell::Cell::new(FakeNitro::OFF) };
}

/// Auto Delete: this account's Discord messages, sent while it's on, are
/// deleted `after_ms` after they're sent; 0 turns it off and forgets those
/// waiting. See `backcord_discord::service::set_auto_delete`.
#[cfg(feature = "discord")]
pub fn set_auto_delete(after_ms: u64) {
    backcord_discord::service::set_auto_delete(after_ms);
}

/// Anti Group Call: a Discord group DM's call doesn't ring this account
/// (see [`crate::application::Action::AntiGroupCall`], which sets this).
static ANTI_GROUP_CALL: AtomicBool = AtomicBool::new(false);

pub fn set_anti_group_call(on: bool) {
    ANTI_GROUP_CALL.store(on, Ordering::Relaxed);
}

pub fn anti_group_call() -> bool {
    !cfg!(test) && ANTI_GROUP_CALL.load(Ordering::Relaxed)
}

/// Anti Nickname Change: someone else's change to this account's nickname
/// in a server, where it may change its own, is set back; see
/// `backcord_discord::service::set_keep_nickname`.
#[cfg(feature = "discord")]
pub fn set_keep_nickname(on: bool) {
    backcord_discord::service::set_keep_nickname(on);
}

/// Split Long Messages: a message over Discord's limit is sent as several
/// ([`crate::split`]) instead of refused.
pub fn set_split_long(on: bool) {
    SPLIT_LONG.store(on, Ordering::Relaxed);
}

pub fn split_long() -> bool {
    #[cfg(test)]
    return TEST_SPLIT_LONG.with(std::cell::Cell::get);
    #[cfg(not(test))]
    SPLIT_LONG.load(Ordering::Relaxed)
}

/// Turns this test's Split Long Messages on or off.
#[cfg(test)]
pub fn test_split_long(on: bool) {
    TEST_SPLIT_LONG.with(|split| split.set(on));
}

/// Message Logger: deleted messages stay in a loaded channel, marked, and
/// edited ones keep what they said before.
pub fn set_message_logger(on: bool) {
    MESSAGE_LOGGER.store(on, Ordering::Relaxed);
}

pub fn message_logger() -> bool {
    #[cfg(test)]
    if let Some(on) = TEST_LOGGER.with(std::cell::Cell::get) {
        return on;
    }
    MESSAGE_LOGGER.load(Ordering::Relaxed)
}

/// Message Logger keeps this account's own deleted and edited messages too.
pub fn set_log_own(on: bool) {
    LOG_OWN.store(on, Ordering::Relaxed);
}

pub fn log_own() -> bool {
    // Tests never log their own.
    !cfg!(test) && LOG_OWN.load(Ordering::Relaxed)
}

/// Notes this account's user id, for telling its own messages.
pub fn note_me(id: &str) {
    if id.is_empty() {
        return;
    }
    let known = ME.read().map(|me| *me == id).unwrap_or(true);
    if !known && let Ok(mut me) = ME.write() {
        id.clone_into(&mut me);
    }
}

/// Whether a message by `author` is this account's.
fn is_me(author: &str) -> bool {
    // In tests no message is this account's.
    !cfg!(test) && ME.read().is_ok_and(|me| !me.is_empty() && *me == author)
}

/// Message Logger keeps what became of a message by `author`.
pub fn logs(author: &str) -> bool {
    message_logger() && (log_own() || !is_me(author))
}

/// Relationship Notifier: which removals of friends, friend requests, group
/// DMs and servers are told of; none while it's off.
pub fn set_relationship_alerts(kinds: Kinds) {
    RELATIONSHIP_ALERTS.store(kinds.bits(), Ordering::Relaxed);
}

pub fn relationship_alerts() -> Kinds {
    Kinds::from_bits(relationship_bits())
}

#[cfg(not(test))]
fn relationship_bits() -> u8 {
    RELATIONSHIP_ALERTS.load(Ordering::Relaxed)
}

#[cfg(test)]
fn relationship_bits() -> u8 {
    TEST_RELATIONSHIPS.with(std::cell::Cell::get)
}

/// Relationship Notifier for this test's thread alone.
#[cfg(test)]
pub(crate) fn test_relationship_alerts(kinds: Kinds) {
    TEST_RELATIONSHIPS.with(|told| told.set(kinds.bits()));
}

/// Message Logger for this test's thread alone.
#[cfg(test)]
pub(crate) fn test_message_logger(on: bool) {
    TEST_LOGGER.with(|logger| logger.set(Some(on)));
}

/// Fake Nitro: the custom emoji and stickers this account may not use where
/// it writes go as links to their pictures, and such links that come show as
/// the emoji they are. See [`crate::fake_nitro`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FakeNitro {
    pub on: bool,
    /// The size, in pixels, an emoji's picture is linked at.
    pub size: u16,
    /// The links go masked, as `[name](link)`, rather than bare.
    pub masked: bool,
    /// Links to emoji pictures that come show as those emoji.
    pub received: bool,
}

impl FakeNitro {
    /// Off, with its settings as they start.
    pub const OFF: Self = Self {
        on: false,
        size: crate::fake_nitro::DEFAULT_SIZE,
        masked: true,
        received: true,
    };
}

impl Default for FakeNitro {
    fn default() -> Self {
        Self::OFF
    }
}

pub fn set_fake_nitro(settings: FakeNitro) {
    FAKE_NITRO_SIZE.store(settings.size, Ordering::Relaxed);
    FAKE_NITRO_MASKED.store(settings.masked, Ordering::Relaxed);
    FAKE_NITRO_RECEIVED.store(settings.received, Ordering::Relaxed);
    FAKE_NITRO.store(settings.on, Ordering::Relaxed);
}

#[cfg(not(test))]
pub fn fake_nitro() -> FakeNitro {
    FakeNitro {
        on: FAKE_NITRO.load(Ordering::Relaxed),
        size: FAKE_NITRO_SIZE.load(Ordering::Relaxed),
        masked: FAKE_NITRO_MASKED.load(Ordering::Relaxed),
        received: FAKE_NITRO_RECEIVED.load(Ordering::Relaxed),
    }
}

#[cfg(test)]
pub fn fake_nitro() -> FakeNitro {
    TEST_FAKE_NITRO.with(std::cell::Cell::get)
}

/// Fake Nitro for this test's thread alone.
#[cfg(test)]
pub(crate) fn test_fake_nitro(settings: FakeNitro) {
    TEST_FAKE_NITRO.with(|fake| fake.set(settings));
}
