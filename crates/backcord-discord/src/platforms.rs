//! Where each person is online, as Discord's presences say in
//! `client_status`: on the desktop app, a phone, the web, a console
//! (`embedded`) or a VR headset, each with its own status. Presences say no
//! more: which desktop or phone system, or which console, is in nobody's
//! presence but this account's own sessions'.
//!
//! Kept by user id where presences are read (friends, DMs, member lists,
//! this account's sessions) and read where a person is drawn; someone who
//! goes offline everywhere leaves it.

use std::{
    collections::HashMap,
    sync::{LazyLock, PoisonError, RwLock},
};

use serde_json::Value;

/// One of the platforms `client_status` names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Platform {
    Desktop,
    Mobile,
    Web,
    /// A console: Xbox or PlayStation.
    Embedded,
    Vr,
}

impl Platform {
    pub const ALL: [Self; 5] = [
        Self::Desktop,
        Self::Mobile,
        Self::Web,
        Self::Embedded,
        Self::Vr,
    ];

    /// Its key in `client_status`, and a session's `client_info.client`.
    fn key(self) -> &'static str {
        match self {
            Self::Desktop => "desktop",
            Self::Mobile => "mobile",
            Self::Web => "web",
            Self::Embedded => "embedded",
            Self::Vr => "vr",
        }
    }

    fn shift(self) -> u16 {
        3 * self as u16
    }
}

/// A platform's status.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Status {
    Online,
    Idle,
    DoNotDisturb,
    /// Only this account's own sessions can say it.
    Invisible,
}

impl Status {
    fn parse(status: &str) -> Option<Self> {
        match status {
            "online" => Some(Self::Online),
            "idle" => Some(Self::Idle),
            "dnd" => Some(Self::DoNotDisturb),
            "invisible" => Some(Self::Invisible),
            _ => None,
        }
    }

    fn code(self) -> u16 {
        self as u16 + 1
    }
}

/// Each platform's status, three bits each.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Clients(u16);

impl Clients {
    /// From a presence's `client_status`.
    pub fn parse(client_status: &Value) -> Self {
        Platform::ALL
            .into_iter()
            .fold(Self::default(), |clients, platform| {
                match client_status[platform.key()]
                    .as_str()
                    .and_then(Status::parse)
                {
                    Some(status) => clients.with(platform, status),
                    None => clients,
                }
            })
    }

    /// From this account's sessions, as SESSIONS_REPLACE lists them: each
    /// session's client and status, the busiest status where several share
    /// a platform. `all` sums them up and says nothing of its own.
    pub fn of_sessions(sessions: &Value) -> Self {
        let mut clients = Self::default();
        for session in sessions.as_array().into_iter().flatten() {
            if session["session_id"] == "all" {
                continue;
            }
            let client = session["client_info"]["client"]
                .as_str()
                .unwrap_or_default();
            let Some(platform) = Platform::ALL
                .into_iter()
                .find(|platform| platform.key() == client)
            else {
                continue;
            };
            let Some(status) = session["status"].as_str().and_then(Status::parse) else {
                continue;
            };
            let busier = clients
                .status(platform)
                .is_none_or(|known| rank(status) > rank(known));
            if busier {
                clients = clients.with(platform, status);
            }
        }
        clients
    }

    #[must_use]
    pub fn with(self, platform: Platform, status: Status) -> Self {
        let shift = platform.shift();
        Self(self.0 & !(0b111 << shift) | status.code() << shift)
    }

    pub fn status(self, platform: Platform) -> Option<Status> {
        match (self.0 >> platform.shift()) & 0b111 {
            1 => Some(Status::Online),
            2 => Some(Status::Idle),
            3 => Some(Status::DoNotDisturb),
            4 => Some(Status::Invisible),
            _ => None,
        }
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The platforms they're on, in [`Platform::ALL`]'s order.
    pub fn iter(self) -> impl Iterator<Item = (Platform, Status)> {
        Platform::ALL
            .into_iter()
            .filter_map(move |platform| self.status(platform).map(|status| (platform, status)))
    }
}

/// Which of two statuses a platform several sessions share shows.
fn rank(status: Status) -> u8 {
    match status {
        Status::Invisible => 0,
        Status::Idle => 1,
        Status::DoNotDisturb => 2,
        Status::Online => 3,
    }
}

static KNOWN: LazyLock<RwLock<HashMap<Box<str>, Clients>>> = LazyLock::new(RwLock::default);

/// Learns where `user` is from a presence; returns what it says.
pub fn learn(user: &str, presence: &Value) -> Clients {
    let clients = Clients::parse(&presence["client_status"]);
    set(user, clients);
    clients
}

/// Where `user` is now; none forgets them.
pub fn set(user: &str, clients: Clients) {
    if of(user) == clients {
        return;
    }
    let mut known = KNOWN.write().unwrap_or_else(PoisonError::into_inner);
    if clients.is_empty() {
        known.remove(user);
    } else {
        known.insert(user.into(), clients);
    }
}

/// Where `user` is, as last learned.
pub fn of(user: &str) -> Clients {
    KNOWN
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .get(user)
        .copied()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn reads_each_platforms_status() {
        let clients = Clients::parse(&json!({"desktop": "dnd", "mobile": "online", "vr": "idle"}));
        assert_eq!(
            clients.iter().collect::<Vec<_>>(),
            [
                (Platform::Desktop, Status::DoNotDisturb),
                (Platform::Mobile, Status::Online),
                (Platform::Vr, Status::Idle)
            ]
        );
        assert!(Clients::parse(&json!({})).is_empty());
        assert!(Clients::parse(&Value::Null).is_empty());
    }

    #[test]
    fn this_accounts_sessions_say_its_own_platforms() {
        let sessions = json!([
            {"session_id": "all", "client_info": {"client": "unknown"}, "status": "online"},
            {"session_id": "a", "client_info": {"client": "web", "os": "linux"}, "status": "invisible"},
            {"session_id": "b", "client_info": {"client": "mobile", "os": "ios"}, "status": "idle"},
            {"session_id": "c", "client_info": {"client": "mobile", "os": "android"}, "status": "online"},
        ]);
        let clients = Clients::of_sessions(&sessions);
        assert_eq!(clients.status(Platform::Web), Some(Status::Invisible));
        assert_eq!(clients.status(Platform::Mobile), Some(Status::Online));
        assert_eq!(clients.status(Platform::Desktop), None);
    }

    #[test]
    fn someone_offline_everywhere_is_forgotten() {
        let user = "platforms-test-user";
        learn(user, &json!({"client_status": {"web": "online"}}));
        assert_eq!(of(user).status(Platform::Web), Some(Status::Online));
        learn(user, &json!({"status": "offline", "client_status": {}}));
        assert!(of(user).is_empty());
        assert!(!KNOWN.read().unwrap().contains_key(user));
    }
}
