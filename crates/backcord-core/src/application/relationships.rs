//! Relationship Notifier: friends, friend requests, group DMs and servers
//! this account lost by others' doing, told of and listed in the inbox's
//! Friend Alerts; see [`crate::relationship_alerts`].

use std::{collections::HashSet, path::Path};

use super::*;
use crate::relationship_alerts::{self as alerts, Kept, Known, RelationshipAlert};

/// What the app knows to compare each snapshot with.
#[derive(Default)]
pub(super) struct Watch {
    /// Where each account's lists and alerts are kept; none in the tests
    /// that don't give one.
    file: Option<std::path::PathBuf>,
    /// The account whose lists `known` are.
    account: Option<String>,
    known: Option<Known>,
    /// The people, groups and servers this account removed itself, by
    /// id, until each is seen gone.
    mine: HashSet<String>,
}

impl Application {
    /// Keeps each account's lists and alerts in `root` from now on, and
    /// compares the next start's with them.
    pub fn keep_relationship_alerts_in(&mut self, root: &Path) {
        self.relationships.file = Some(root.join(alerts::FILE));
    }

    /// Notes the friends, requests, group DMs and servers that `effects`
    /// remove by this account's own doing, which aren't told of.
    pub(super) fn note_own_removals(&mut self, effects: &[Effect]) {
        use chat::RelationshipChange::{Block, Unfriend};
        for effect in effects {
            let Effect::Backend(Command::Discord(command)) = effect else {
                continue;
            };
            let id = match command {
                DiscordCommand::Relationship {
                    user,
                    change: Unfriend | Block,
                    ..
                }
                | DiscordCommand::Friend { user, add: false } => Some(user.as_str()),
                DiscordCommand::LeaveServer { server }
                | DiscordCommand::DeleteServer { server } => crate::discord::parse_server(server),
                DiscordCommand::LeaveGroup { room, .. } | DiscordCommand::CloseDm { room } => {
                    crate::discord::parse_room(room).map(|(_, channel)| channel)
                }
                _ => None,
            };
            if let Some(id) = id {
                self.relationships.mine.insert(id.to_owned());
            }
        }
    }

    /// Compares an online snapshot's friends, friend requests, group DMs and
    /// servers with those known: what others took away is listed first in
    /// Friend Alerts and notified, unless this account is set to Do Not
    /// Disturb. An account's first snapshot is compared with what was kept
    /// as BACKCORD last saw it.
    #[cfg(feature = "discord")]
    pub(super) fn watch_relationships(
        &mut self,
        snapshot: &backcord_discord::service::Snapshot,
    ) -> Vec<Effect> {
        use backcord_discord::service::Status;
        if snapshot.status != Status::Online || self.state.preview {
            return vec![];
        }
        let Some(account) = snapshot.user_id.as_deref() else {
            return vec![];
        };
        let watch = &mut self.relationships;
        let starting = watch.account.as_deref() != Some(account);
        if starting {
            let kept = watch
                .file
                .as_deref()
                .map(|file| alerts::read(file, account))
                .unwrap_or_default();
            watch.account = Some(account.to_owned());
            watch.known = kept.known;
            self.state.relationship_alerts = kept.alerts;
        } else if watch
            .known
            .as_ref()
            .is_some_and(|known| known.same_as(snapshot))
        {
            return vec![];
        }
        let now = Known::of(snapshot);
        if watch.known.as_ref() == Some(&now) {
            return vec![];
        }
        let kinds = crate::extras::relationship_alerts();
        let mut fresh = watch.known.as_ref().map_or_else(Vec::new, |was| {
            alerts::removals(was, &now, &mut watch.mine, alerts::now_ms(), starting)
        });
        watch.known = Some(now);
        fresh.retain(|alert| kinds.tell(alert));
        let listed = &mut self.state.relationship_alerts;
        listed.splice(0..0, fresh.iter().cloned());
        listed.truncate(alerts::KEPT);
        self.keep_relationships();
        if self.state.do_not_disturb() {
            return vec![];
        }
        fresh
            .iter()
            .map(|alert| Effect::Notify(self.relationship_alert(alert, snapshot)))
            .collect()
    }

    /// An alert's notification: who or where, what happened, and their
    /// picture when BACKCORD has it. A click opens the DM with the person,
    /// when there is one.
    #[cfg(feature = "discord")]
    fn relationship_alert(
        &self,
        alert: &RelationshipAlert,
        snapshot: &backcord_discord::service::Snapshot,
    ) -> chat::Alert {
        let room = snapshot
            .conversations
            .iter()
            .filter(|_| alert.round())
            .find(|dm| !dm.group && dm.person.as_deref() == Some(alert.id.as_str()))
            .map(|dm| crate::discord::room_id(crate::discord::DM_GUILD, &dm.id))
            .unwrap_or_default();
        chat::Alert {
            room,
            title: alert.name.clone(),
            body: alert.says().to_owned(),
            picture: alert
                .avatar
                .as_ref()
                .and_then(|avatar| self.state.pictures.get(avatar).cloned()),
            call: false,
            mention: false,
            relationship: true,
        }
    }

    /// Empties Friend Alerts.
    pub(super) fn clear_relationship_alerts(&mut self) -> Vec<Effect> {
        if !self.state.relationship_alerts.is_empty() {
            self.state.relationship_alerts.clear();
            self.keep_relationships();
        }
        vec![]
    }

    /// Keeps the account's lists and alerts on disk, for the next start.
    fn keep_relationships(&self) {
        let watch = &self.relationships;
        let (Some(file), Some(account)) = (&watch.file, &watch.account) else {
            return;
        };
        let kept = Kept {
            known: watch.known.clone(),
            alerts: self.state.relationship_alerts.clone(),
        };
        if let Err(error) = alerts::write(file, account, &kept) {
            tracing::warn!(%error, "could not keep the relationship alerts");
        }
    }
}
