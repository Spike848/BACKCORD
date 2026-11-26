//! Auto Delete: this account's messages, sent anywhere while it's on, are
//! deleted once they're as old as it says. The messages waiting go to disk
//! as they come (`cache::auto_deletes`), encrypted for the login, so those
//! due while BACKCORD was closed go as it next connects. Turning it off
//! forgets those waiting: nothing more is deleted.

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Notify;

use crate::api::Api;

/// How old a message gets before it goes, in ms; 0 while off.
static AFTER: AtomicU64 = AtomicU64::new(0);
static QUEUE: Mutex<Queue> = Mutex::new(Queue::new());
/// Wakes the deleter: a message came, or the setting changed.
static WAKE: Notify = Notify::const_new();

/// A failed delete is tried again this much later, and given up after
/// [`TRIES`].
const RETRY_MS: u64 = 60_000;
const TRIES: u8 = 3;

/// Turns Auto Delete on, deleting messages `after_ms` after they're sent,
/// or off with 0, which forgets those waiting.
pub fn set_auto_delete(after_ms: u64) {
    AFTER.store(after_ms, Ordering::Relaxed);
    if after_ms == 0 {
        let mut queue = queue();
        if queue.loaded && !queue.waiting.is_empty() {
            queue.waiting.clear();
            queue.changed = true;
        }
    }
    WAKE.notify_one();
}

/// A message waiting to go.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Waiting {
    pub channel: String,
    pub message: String,
    /// When it goes, in Unix ms.
    pub due: u64,
    #[serde(default)]
    pub tries: u8,
}

struct Queue {
    waiting: Vec<Waiting>,
    /// The kept ones were read from disk.
    loaded: bool,
    /// Changed since it was last written.
    changed: bool,
}

impl Queue {
    const fn new() -> Self {
        Self {
            waiting: Vec::new(),
            loaded: false,
            changed: false,
        }
    }
}

fn queue() -> std::sync::MutexGuard<'static, Queue> {
    QUEUE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The message a MESSAGE_CREATE brings, to delete in time, when it's this
/// account's own (`me`) and Auto Delete is on: a plain message or a reply,
/// not an ephemeral one nor a call's or a pin's notice, which aren't
/// deleted as messages are.
pub(super) fn waiting_for(data: &Value, me: &str, after_ms: u64, now: u64) -> Option<Waiting> {
    const EPHEMERAL: u64 = 1 << 6;
    if after_ms == 0 || me.is_empty() || data["author"]["id"].as_str() != Some(me) {
        return None;
    }
    if !matches!(data["type"].as_u64(), Some(0 | 19))
        || data["flags"].as_u64().unwrap_or(0) & EPHEMERAL != 0
    {
        return None;
    }
    Some(Waiting {
        channel: data["channel_id"].as_str()?.to_owned(),
        message: data["id"].as_str()?.to_owned(),
        due: now.saturating_add(after_ms),
        tries: 0,
    })
}

/// Takes in a MESSAGE_CREATE, as the session sees it.
pub(super) fn note(data: &Value, me: &str) {
    let after = AFTER.load(Ordering::Relaxed);
    let Some(waiting) = waiting_for(data, me, after, super::now_ms()) else {
        return;
    };
    let mut queue = queue();
    if !queue
        .waiting
        .iter()
        .any(|known| known.message == waiting.message)
    {
        queue.waiting.push(waiting);
        queue.changed = true;
        drop(queue);
        WAKE.notify_one();
    }
}

/// The messages of `waiting` due by `now`, taken out of it.
pub(super) fn take_due(waiting: &mut Vec<Waiting>, now: u64) -> Vec<Waiting> {
    let (due, later) = std::mem::take(waiting)
        .into_iter()
        .partition(|known| known.due <= now);
    *waiting = later;
    due
}

/// Deletes the messages waiting as they fall due, for as long as the
/// service runs; one at a time, as Discord's limits on deleting allow.
pub(super) async fn run(api: Arc<Api>) {
    let kept = tokio::task::spawn_blocking(crate::cache::auto_deletes)
        .await
        .unwrap_or_default();
    {
        let mut queue = queue();
        if AFTER.load(Ordering::Relaxed) != 0 {
            for waiting in kept {
                if !queue
                    .waiting
                    .iter()
                    .any(|known| known.message == waiting.message)
                {
                    queue.waiting.push(waiting);
                }
            }
        } else {
            // Turned off since: nothing more goes.
            queue.changed = true;
        }
        queue.loaded = true;
    }
    loop {
        let due = take_due(&mut queue().waiting, super::now_ms());
        for mut waiting in due {
            if let Err(error) = api.delete(&waiting.channel, &waiting.message).await {
                waiting.tries += 1;
                if waiting.tries < TRIES && AFTER.load(Ordering::Relaxed) != 0 {
                    tracing::warn!(%error, "Auto Delete: could not delete a message; trying again later");
                    waiting.due = super::now_ms() + RETRY_MS;
                    queue().waiting.push(waiting);
                } else {
                    tracing::warn!(%error, "Auto Delete: could not delete a message; giving up");
                }
            }
            queue().changed = true;
        }
        save().await;
        let next = queue().waiting.iter().map(|known| known.due).min();
        let wait = next.map(|due| due.saturating_sub(super::now_ms()));
        match wait {
            Some(wait) => {
                tokio::select! {
                    () = tokio::time::sleep(std::time::Duration::from_millis(wait)) => {}
                    () = WAKE.notified() => {}
                }
            }
            None => WAKE.notified().await,
        }
    }
}

/// Writes the queue to disk when it changed.
async fn save() {
    let waiting = {
        let mut queue = queue();
        if !queue.changed {
            return;
        }
        queue.changed = false;
        queue.waiting.clone()
    };
    let _ = tokio::task::spawn_blocking(move || crate::cache::keep_auto_deletes(&waiting)).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn only_this_accounts_plain_messages_and_replies_wait_while_on() {
        let message = |author: &str, kind: u64, flags: u64| json!({"id": "9", "channel_id": "5", "author": {"id": author}, "type": kind, "flags": flags});
        assert_eq!(
            waiting_for(&message("1", 0, 0), "1", 30_000, 1_000),
            Some(Waiting {
                channel: "5".into(),
                message: "9".into(),
                due: 31_000,
                tries: 0
            })
        );
        assert!(
            waiting_for(&message("1", 19, 0), "1", 30_000, 0).is_some(),
            "a reply"
        );
        assert!(
            waiting_for(&message("2", 0, 0), "1", 30_000, 0).is_none(),
            "someone else's"
        );
        assert!(waiting_for(&message("1", 0, 0), "1", 0, 0).is_none(), "off");
        assert!(
            waiting_for(&message("1", 6, 0), "1", 30_000, 0).is_none(),
            "a pin's notice"
        );
        assert!(
            waiting_for(&message("1", 0, 64), "1", 30_000, 0).is_none(),
            "ephemeral"
        );
    }

    #[test]
    fn those_due_are_taken_and_the_rest_wait() {
        let waiting = |message: &str, due: u64| Waiting {
            channel: "5".into(),
            message: message.into(),
            due,
            tries: 0,
        };
        let mut queue = vec![waiting("a", 10), waiting("b", 30), waiting("c", 20)];
        let due = take_due(&mut queue, 20);
        assert_eq!(due, [waiting("a", 10), waiting("c", 20)]);
        assert_eq!(queue, [waiting("b", 30)]);
    }
}
