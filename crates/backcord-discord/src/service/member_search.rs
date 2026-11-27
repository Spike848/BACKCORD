//! Member searches for mentions, and requests for members' server
//! profiles: each waits for the chunk of members Discord sends back with
//! its nonce.

use std::collections::HashMap;

use anyhow::Result;
use tokio::sync::{mpsc, oneshot};

use super::tracker::Tracker;
use crate::{
    event::Event,
    gateway::{Dispatch, Gateway},
};

/// The most members one search asks for.
const FOUND: u64 = 10;
/// The most servers one search for people asks of at once.
const SERVERS_AT_ONCE: usize = 100;

/// Who of each server, by its id, matched a search for people.
pub(crate) type PeopleFound = mpsc::UnboundedSender<Vec<(String, String)>>;

/// A search for people across servers, waiting for each server's chunk.
struct Spread {
    left: usize,
    reply: PeopleFound,
}

/// Member searches waiting for their answer, by nonce.
#[derive(Default)]
pub(super) struct MemberSearches {
    waiting: HashMap<String, oneshot::Sender<Vec<String>>>,
    /// Searches for people across servers, by nonce.
    spread: HashMap<String, Spread>,
    nonce: u64,
}

impl MemberSearches {
    /// Asks Discord for the members of `guild` whose names start with `query`.
    pub(super) async fn start(
        &mut self,
        gateway: &mut Gateway,
        guild: &str,
        query: &str,
        reply: oneshot::Sender<Vec<String>>,
    ) -> Result<()> {
        if gateway.members_held(guild) {
            let _ = reply.send(Vec::new());
            return Ok(());
        }
        let nonce = self.wait(reply);
        gateway.search_members(guild, query, FOUND, &nonce).await
    }

    /// Asks Discord for the members of every one of `guilds` whose names
    /// start with `query`, a hundred servers to a request; `reply` hears
    /// each server's as they come, and closes once all came.
    pub(super) async fn people(
        &mut self,
        gateway: &mut Gateway,
        guilds: &[String],
        query: &str,
        reply: PeopleFound,
    ) -> Result<()> {
        self.spread.retain(|_, spread| !spread.reply.is_closed());
        for guilds in guilds.chunks(SERVERS_AT_ONCE) {
            self.nonce += 1;
            let nonce = format!("backcord-people-{}", self.nonce);
            self.spread.insert(
                nonce.clone(),
                Spread {
                    left: guilds.len(),
                    reply: reply.clone(),
                },
            );
            gateway
                .search_members_in(guilds, query, FOUND, &nonce)
                .await?;
        }
        Ok(())
    }

    /// Asks Discord for these members of `guild`, up to 100; `reply` hears
    /// which it found once their profiles are learned.
    pub(super) async fn members(
        &mut self,
        gateway: &mut Gateway,
        guild: &str,
        users: &[String],
        reply: oneshot::Sender<Vec<String>>,
    ) -> Result<()> {
        if gateway.members_held(guild) {
            let _ = reply.send(Vec::new());
            return Ok(());
        }
        let nonce = self.wait(reply);
        gateway.request_members(guild, users, Some(&nonce)).await
    }

    /// [`Self::answer`] for a search for people: a server's chunk, or the
    /// request dropped.
    fn answer_people(&mut self, tracker: &mut Tracker, event: &Dispatch) -> Option<bool> {
        if event.event == Event::RateLimited {
            let nonce = event.data["meta"]["nonce"].as_str()?;
            self.spread.remove(nonce)?;
            return Some(false);
        }
        let nonce = event.data["nonce"].as_str()?;
        let spread = self.spread.get_mut(nonce)?;
        let changed = tracker.apply(&event.event, &event.data);
        let guild = event.data["guild_id"].as_str().unwrap_or_default();
        let found = event.data["members"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|member| member["user"]["id"].as_str())
            .map(|user| (guild.to_owned(), user.to_owned()))
            .collect();
        let _ = spread.reply.send(found);
        // A server's members come in one chunk, a search being small.
        let last = event.data["chunk_index"].as_u64().unwrap_or(0) + 1
            >= event.data["chunk_count"].as_u64().unwrap_or(1);
        if last {
            spread.left = spread.left.saturating_sub(1);
            if spread.left == 0 {
                self.spread.remove(nonce);
            }
        }
        Some(changed)
    }

    /// A new nonce, whose chunk `reply` waits for.
    fn wait(&mut self, reply: oneshot::Sender<Vec<String>>) -> String {
        self.nonce += 1;
        let nonce = format!("backcord-search-{}", self.nonce);
        // Unanswered searches are dropped, so their callers stop waiting.
        self.waiting.retain(|_, reply| !reply.is_closed());
        self.waiting.insert(nonce.clone(), reply);
        nonce
    }

    /// Answers the search a chunk of members is for, if any. Its members are
    /// learned first, so their names are known; then this returns whether
    /// that changed the tracker, and the chunk needn't be applied again.
    pub(super) fn answer(&mut self, tracker: &mut Tracker, event: &Dispatch) -> Option<bool> {
        if let Some(changed) = self.answer_people(tracker, event) {
            return Some(changed);
        }
        if event.event == Event::RateLimited {
            // Discord dropped the request this nonce was for.
            let reply = event.data["meta"]["nonce"]
                .as_str()
                .and_then(|nonce| self.waiting.remove(nonce))?;
            let _ = reply.send(Vec::new());
            return Some(false);
        }
        let reply = event.data["nonce"]
            .as_str()
            .and_then(|nonce| self.waiting.remove(nonce))?;
        let changed = tracker.apply(&event.event, &event.data);
        let found = event.data["members"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|member| member["user"]["id"].as_str())
            .map(str::to_owned)
            .collect();
        let _ = reply.send(found);
        Some(changed)
    }
}
