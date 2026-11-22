//! Extra features the core does itself, each switched by the window as
//! Settings' Extra Features say: atomics, read where they act. Each starts
//! off; the window turns on those its preferences keep on.

use std::sync::atomic::{AtomicBool, Ordering};

static MESSAGE_LOGGER: AtomicBool = AtomicBool::new(false);
/// Message Logger keeps this account's own messages too.
static LOG_OWN: AtomicBool = AtomicBool::new(false);
/// This account's Discord user id, as the channels last said.
static ME: std::sync::RwLock<String> = std::sync::RwLock::new(String::new());

#[cfg(test)]
thread_local! {
    /// A test's own Message Logger, so it doesn't change the others'.
    static TEST_LOGGER: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
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

/// Message Logger for this test's thread alone.
#[cfg(test)]
pub(crate) fn test_message_logger(on: bool) {
    TEST_LOGGER.with(|logger| logger.set(Some(on)));
}
