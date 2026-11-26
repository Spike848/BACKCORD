//! Relationship Notifier: what others did to this account's friendships,
//! group DMs and servers: what they took away, and the friend requests
//! they sent or accepted. The app compares each snapshot's friends, friend
//! requests, group DMs and servers with those it knew, and what went that
//! this account didn't take away itself it tells of and lists in the
//! inbox's Friend Alerts. Each account's last known lists and its alerts
//! are kept in the data folder's [`FILE`], by user id, so a start tells of
//! what went while BACKCORD was closed.

use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    path::Path,
};

use serde::{Deserialize, Serialize};

/// The file in the data folder.
pub const FILE: &str = "relationship-alerts.json";
/// The most alerts kept for an account; the oldest go first.
pub const KEPT: usize = 200;

/// What went, or, for a friend request sent or accepted, what came.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Removal {
    /// A friend removed this account, or blocked it.
    Unfriended,
    /// A friend request this account sent was declined.
    RequestDeclined,
    /// A friend request sent to this account was withdrawn.
    RequestWithdrawn,
    /// This account was removed from a group DM.
    Group,
    /// This account was kicked or banned from a server, or it was deleted.
    Server,
    /// Someone sent this account a friend request.
    RequestReceived,
    /// Someone accepted the friend request this account sent.
    RequestAccepted,
}

/// One alert, as Friend Alerts lists it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationshipAlert {
    pub removal: Removal,
    /// The person's user id, the group DM's channel id or the server's id.
    pub id: String,
    pub name: String,
    /// Their avatar, or the group's or server's icon.
    #[serde(default)]
    pub avatar: Option<String>,
    /// When BACKCORD noticed it, in Unix milliseconds.
    pub at: u64,
    /// Noticed as BACKCORD started: it went while BACKCORD was closed.
    #[serde(default)]
    pub offline: bool,
}

impl RelationshipAlert {
    /// What happened, in English, under the name; the window translates it.
    pub fn says(&self) -> &'static str {
        match (self.removal, self.offline) {
            (Removal::Unfriended, false) => "Removed you as a friend.",
            (Removal::Unfriended, true) => "Is no longer your friend.",
            (Removal::RequestDeclined, _) => "Declined your friend request.",
            (Removal::RequestWithdrawn, _) => "Withdrew their friend request.",
            (Removal::Group, false) => "You were removed from this group.",
            (Removal::Group, true) => "You're no longer in this group.",
            (Removal::Server, false) => "You were removed from this server, or it was deleted.",
            (Removal::Server, true) => "You're no longer in this server.",
            (Removal::RequestReceived, _) => "Sent you a friend request.",
            (Removal::RequestAccepted, _) => "Accepted your friend request.",
        }
    }

    /// A person's or a group's picture is round, a server's a rounded square.
    pub fn round(&self) -> bool {
        self.removal != Removal::Server
    }
}

/// Which removals are told of, as Settings' Extra Features say.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Kinds {
    /// Friends lost, and friend requests accepted.
    pub friends: bool,
    /// Friend requests received, declined or withdrawn.
    pub requests: bool,
    pub groups: bool,
    pub servers: bool,
    /// What went while BACKCORD was closed, told as it starts.
    pub offline: bool,
}

impl Kinds {
    pub const ALL: Self = Self {
        friends: true,
        requests: true,
        groups: true,
        servers: true,
        offline: true,
    };
    pub const NONE: Self = Self {
        friends: false,
        requests: false,
        groups: false,
        servers: false,
        offline: false,
    };

    /// Whether `alert` is told of.
    pub fn tell(self, alert: &RelationshipAlert) -> bool {
        let kind = match alert.removal {
            Removal::Unfriended | Removal::RequestAccepted => self.friends,
            Removal::RequestDeclined | Removal::RequestWithdrawn | Removal::RequestReceived => {
                self.requests
            }
            Removal::Group => self.groups,
            Removal::Server => self.servers,
        };
        kind && (self.offline || !alert.offline)
    }

    pub fn bits(self) -> u8 {
        [
            self.friends,
            self.requests,
            self.groups,
            self.servers,
            self.offline,
        ]
        .iter()
        .enumerate()
        .fold(0, |bits, (at, on)| bits | (u8::from(*on) << at))
    }

    pub fn from_bits(bits: u8) -> Self {
        let on = |at: u8| bits & (1 << at) != 0;
        Self {
            friends: on(0),
            requests: on(1),
            groups: on(2),
            servers: on(3),
            offline: on(4),
        }
    }
}

/// A person, group DM or server as it was last known: its name and picture.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Who {
    pub name: String,
    #[serde(default)]
    pub avatar: Option<String>,
}

/// An account's friends, friend requests, group DMs and servers, by id.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Known {
    pub friends: BTreeMap<String, Who>,
    /// Friend requests this account sent, and those sent to it.
    pub sent: BTreeMap<String, Who>,
    pub received: BTreeMap<String, Who>,
    pub groups: BTreeMap<String, Who>,
    pub servers: BTreeMap<String, Who>,
    /// Those this account blocked: a friend blocked went by its own doing.
    pub blocked: BTreeSet<String>,
}

#[cfg(feature = "discord")]
impl Known {
    /// What `snapshot` lists.
    pub fn of(snapshot: &backcord_discord::service::Snapshot) -> Self {
        let who = |name: &str, avatar: &str| Who {
            name: name.to_owned(),
            avatar: Some(avatar.to_owned()).filter(|avatar| !avatar.is_empty()),
        };
        let mut known = Self {
            friends: snapshot
                .friend_list
                .iter()
                .map(|friend| (friend.user_id.clone(), who(&friend.name, &friend.avatar)))
                .collect(),
            blocked: snapshot.blocked.iter().cloned().collect(),
            ..Self::default()
        };
        // A friend the list doesn't name yet is still a friend.
        for friend in snapshot.friends.iter() {
            known.friends.entry(friend.clone()).or_default();
        }
        for request in snapshot.friend_requests.iter() {
            let requests = if request.incoming {
                &mut known.received
            } else {
                &mut known.sent
            };
            requests.insert(request.user_id.clone(), who(&request.name, &request.avatar));
        }
        for group in snapshot.conversations.iter().filter(|dm| dm.group) {
            known.groups.insert(
                group.id.clone(),
                who(&group.name, group.avatar.as_deref().unwrap_or_default()),
            );
        }
        for server in &snapshot.servers {
            known.servers.insert(
                server.id.clone(),
                who(&server.name, server.icon.as_deref().unwrap_or_default()),
            );
        }
        known
    }

    /// Whether `snapshot` lists the same people, groups and servers, by
    /// id: snapshots come with each presence, and this asks nothing new.
    pub fn same_as(&self, snapshot: &backcord_discord::service::Snapshot) -> bool {
        let friends = snapshot.friends.len().max(snapshot.friend_list.len());
        let (received, sent) = snapshot
            .friend_requests
            .iter()
            .partition::<Vec<_>, _>(|request| request.incoming);
        let groups = snapshot.conversations.iter().filter(|dm| dm.group);
        self.friends.len() == friends
            && snapshot
                .friend_list
                .iter()
                .all(|friend| self.friends.contains_key(&friend.user_id))
            && snapshot
                .friends
                .iter()
                .all(|friend| self.friends.contains_key(friend))
            && self.received.len() == received.len()
            && received
                .iter()
                .all(|request| self.received.contains_key(&request.user_id))
            && self.sent.len() == sent.len()
            && sent
                .iter()
                .all(|request| self.sent.contains_key(&request.user_id))
            && self.groups.len() == groups.clone().count()
            && groups
                .into_iter()
                .all(|group| self.groups.contains_key(&group.id))
            && self.servers.len() == snapshot.servers.len()
            && snapshot
                .servers
                .iter()
                .all(|server| self.servers.contains_key(&server.id))
            && self.blocked.len() == snapshot.blocked.len()
            && snapshot
                .blocked
                .iter()
                .all(|blocked| self.blocked.contains(blocked))
    }
}

/// What went between `was` and `now` that this account didn't take away:
/// ids in `mine` it did, and each is let go of as it's seen gone. Then the
/// friend requests others sent it, and those of its own they accepted.
/// Noticed at `at`, in Unix milliseconds; `offline` as BACKCORD starts.
pub fn removals(
    was: &Known,
    now: &Known,
    mine: &mut HashSet<String>,
    at: u64,
    offline: bool,
) -> Vec<RelationshipAlert> {
    let mut alerts = Vec::new();
    // Someone gone from a list who is a friend now, or whom this account
    // blocked, didn't go by someone else's doing.
    let stayed = |id: &str| now.friends.contains_key(id) || now.blocked.contains(id);
    let lists = [
        (&was.friends, &now.friends, Removal::Unfriended),
        (&was.sent, &now.sent, Removal::RequestDeclined),
        (&was.received, &now.received, Removal::RequestWithdrawn),
        (&was.groups, &now.groups, Removal::Group),
        (&was.servers, &now.servers, Removal::Server),
    ];
    for (before, after, removal) in lists {
        for (id, who) in before {
            if after.contains_key(id) {
                continue;
            }
            let person = matches!(
                removal,
                Removal::Unfriended | Removal::RequestDeclined | Removal::RequestWithdrawn
            );
            if mine.remove(id) || (person && stayed(id)) {
                continue;
            }
            alerts.push(RelationshipAlert {
                removal,
                id: id.clone(),
                name: who.name.clone(),
                avatar: who.avatar.clone(),
                at,
                offline,
            });
        }
    }
    // A request that came, from someone who wasn't a friend already.
    for (id, who) in &now.received {
        if was.received.contains_key(id) || was.friends.contains_key(id) {
            continue;
        }
        alerts.push(RelationshipAlert {
            removal: Removal::RequestReceived,
            id: id.clone(),
            name: who.name.clone(),
            avatar: who.avatar.clone(),
            at,
            offline,
        });
    }
    // A request this account sent that became a friendship: one it
    // received and accepted itself isn't told of.
    for (id, who) in &now.friends {
        if was.friends.contains_key(id) {
            continue;
        }
        let Some(sent) = was.sent.get(id) else {
            continue;
        };
        // The friend list may not name them yet; the request did.
        let who = if who.name.is_empty() { sent } else { who };
        alerts.push(RelationshipAlert {
            removal: Removal::RequestAccepted,
            id: id.clone(),
            name: who.name.clone(),
            avatar: who.avatar.clone(),
            at,
            offline,
        });
    }
    alerts
}

/// What [`FILE`] keeps for an account: its lists when BACKCORD last saw
/// them, and its alerts, the newest first.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Kept {
    pub known: Option<Known>,
    pub alerts: Vec<RelationshipAlert>,
}

/// What `file` keeps for `account`; nothing when it's missing or doesn't
/// read.
pub fn read(file: &Path, account: &str) -> Kept {
    read_all(file).remove(account).unwrap_or_default()
}

fn read_all(file: &Path) -> BTreeMap<String, Kept> {
    std::fs::read(file)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

/// Keeps `kept` as `account`'s in `file`, beside the other accounts': to
/// a file beside it, then over it, so a crash mid-write leaves the old one
/// whole.
pub fn write(file: &Path, account: &str, kept: &Kept) -> std::io::Result<()> {
    let mut all = read_all(file);
    all.insert(account.to_owned(), kept.clone());
    let bytes = serde_json::to_vec(&all).map_err(std::io::Error::other)?;
    if let Some(folder) = file.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let beside = file.with_extension("json.tmp");
    std::fs::write(&beside, bytes)?;
    std::fs::rename(&beside, file)
}

/// Now, in Unix milliseconds.
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn who(name: &str) -> Who {
        Who {
            name: name.into(),
            avatar: None,
        }
    }

    fn known(friends: &[&str], servers: &[&str]) -> Known {
        Known {
            friends: friends.iter().map(|id| ((*id).into(), who(id))).collect(),
            servers: servers.iter().map(|id| ((*id).into(), who(id))).collect(),
            ..Known::default()
        }
    }

    #[test]
    fn only_what_others_took_away_is_told() {
        let was = Known {
            sent: [("5".into(), who("Sent"))].into(),
            received: [("6".into(), who("Received"))].into(),
            groups: [("70".into(), who("Hikers"))].into(),
            ..known(&["2", "3", "4"], &["10", "11"])
        };
        // 2 unfriended this account; 3 it unfriended itself; 4 it blocked;
        // 5 declined its request; 6 withdrew theirs; the group and server
        // 10 went; it left server 11 itself.
        let now = Known {
            blocked: ["4".into()].into(),
            ..known(&[], &[])
        };
        let mut mine: HashSet<String> = ["3".into(), "11".into()].into();
        let alerts = removals(&was, &now, &mut mine, 7, false);
        let told: Vec<(Removal, &str)> = alerts
            .iter()
            .map(|alert| (alert.removal, alert.id.as_str()))
            .collect();
        assert_eq!(
            told,
            [
                (Removal::Unfriended, "2"),
                (Removal::RequestDeclined, "5"),
                (Removal::RequestWithdrawn, "6"),
                (Removal::Group, "70"),
                (Removal::Server, "10"),
            ]
        );
        assert!(mine.is_empty(), "each of its own is let go of once seen");
        assert!(alerts.iter().all(|alert| alert.at == 7 && !alert.offline));
        // A request that became a friendship went nowhere: it was
        // accepted.
        let was = Known {
            sent: [("5".into(), who("Sent"))].into(),
            ..Known::default()
        };
        let now = known(&["5"], &[]);
        let alerts = removals(&was, &now, &mut mine, 7, false);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].removal, Removal::RequestAccepted);
        assert_eq!(alerts[0].id, "5");
    }

    #[test]
    fn requests_that_come_and_are_accepted_are_told() {
        let was = Known {
            sent: [("5".into(), who("Sent"))].into(),
            received: [("6".into(), who("Old"))].into(),
            ..known(&["2"], &[])
        };
        // 7 sent a request; 6's was already there; 5 accepted this
        // account's; 6 too is a friend now, but this account accepted it.
        let now = Known {
            received: [("7".into(), who("New"))].into(),
            ..known(&["2", "5", "6"], &[])
        };
        let mut mine = HashSet::new();
        let told: Vec<(Removal, String)> = removals(&was, &now, &mut mine, 1, false)
            .into_iter()
            .map(|alert| (alert.removal, alert.id))
            .collect();
        assert_eq!(
            told,
            [
                (Removal::RequestReceived, "7".to_owned()),
                (Removal::RequestAccepted, "5".to_owned()),
            ]
        );
    }

    #[test]
    fn kinds_choose_what_is_told() {
        let alert = |removal, offline| RelationshipAlert {
            removal,
            id: "1".into(),
            name: "Ann".into(),
            avatar: None,
            at: 0,
            offline,
        };
        let kinds = Kinds {
            groups: false,
            offline: false,
            ..Kinds::ALL
        };
        assert!(kinds.tell(&alert(Removal::Unfriended, false)));
        assert!(!kinds.tell(&alert(Removal::Unfriended, true)));
        assert!(!kinds.tell(&alert(Removal::Group, false)));
        assert!(kinds.tell(&alert(Removal::RequestWithdrawn, false)));
        assert_eq!(Kinds::from_bits(kinds.bits()), kinds);
        assert_eq!(Kinds::from_bits(Kinds::ALL.bits()), Kinds::ALL);
        assert_eq!(Kinds::NONE.bits(), 0);
    }

    #[test]
    fn each_account_keeps_its_own() {
        let folder = tempfile::tempdir().expect("a folder");
        let file = folder.path().join(FILE);
        assert_eq!(read(&file, "1"), Kept::default());
        let first = Kept {
            known: Some(known(&["2"], &["10"])),
            alerts: vec![],
        };
        let second = Kept {
            known: Some(known(&["3"], &[])),
            alerts: vec![RelationshipAlert {
                removal: Removal::Server,
                id: "11".into(),
                name: "Old".into(),
                avatar: Some("icon.png".into()),
                at: 9,
                offline: true,
            }],
        };
        write(&file, "1", &first).expect("written");
        write(&file, "2", &second).expect("written");
        assert_eq!(read(&file, "1"), first);
        assert_eq!(read(&file, "2"), second);
        assert!(!file.with_extension("json.tmp").exists());
    }
}
