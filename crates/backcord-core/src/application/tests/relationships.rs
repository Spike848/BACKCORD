//! Relationship Notifier: friends, friend requests, group DMs and servers
//! others took away from this account, told of, listed and kept.

use super::*;
use crate::relationship_alerts::{Kinds, Removal};
use backcord_discord::service::{Conversation, Friend, FriendRequest, Snapshot, Status};

fn friend(id: &str, name: &str) -> Friend {
    Friend {
        decoration: None,
        user_id: id.into(),
        name: name.into(),
        avatar: format!("{id}.png"),
        nickname: None,
        since: None,
        presence: None,
        activity: None,
        activities: Default::default(),
    }
}

fn server(id: &str, name: &str) -> MarkedServer {
    let server = backcord_discord::ready::server(&serde_json::json!({
        "id": id, "properties": {"name": name}, "roles": [], "channels": [],
    }))
    .expect("a server");
    MarkedServer::marked(server, ServerMarks::default(), |_| ChannelMarks::default())
}

/// Alice, "1": friends with Annie and Ben, a request sent to Dave and one
/// from Erin, a DM with Annie, the group Hikers, and the servers Test and
/// Other.
fn everyone() -> Snapshot {
    Snapshot {
        status: Status::Online,
        user_id: Some("1".into()),
        friends: vec!["2".into(), "3".into()].into(),
        friend_list: vec![friend("2", "Annie"), friend("3", "Ben")].into(),
        friend_requests: vec![
            FriendRequest {
                user_id: "4".into(),
                name: "Erin".into(),
                username: None,
                avatar: String::new(),
                incoming: true,
                spam: false,
            },
            FriendRequest {
                user_id: "5".into(),
                name: "Dave".into(),
                username: None,
                avatar: String::new(),
                incoming: false,
                spam: false,
            },
        ]
        .into(),
        conversations: vec![
            Arc::new(Conversation {
                id: "30".into(),
                name: "Annie".into(),
                person: Some("2".into()),
                people: 2,
                ..Default::default()
            }),
            Arc::new(Conversation {
                id: "31".into(),
                name: "Hikers".into(),
                group: true,
                people: 3,
                ..Default::default()
            }),
        ],
        servers: vec![server("10", "Test"), server("11", "Other")],
        ..Default::default()
    }
}

fn notified(effects: &[Effect]) -> Vec<(String, String, String)> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            Effect::Notify(alert) if alert.relationship => {
                Some((alert.room.clone(), alert.title.clone(), alert.body.clone()))
            }
            _ => None,
        })
        .collect()
}

fn listed(app: &Application) -> Vec<(Removal, &str, bool)> {
    app.state
        .relationship_alerts
        .iter()
        .map(|alert| (alert.removal, alert.name.as_str(), alert.offline))
        .collect()
}

#[test]
fn what_others_take_away_is_told_and_listed_and_what_this_account_does_is_not() {
    crate::extras::test_relationship_alerts(Kinds::ALL);
    let folder = tempfile::tempdir().expect("a folder");
    let mut app = Application::default();
    app.keep_relationship_alerts_in(folder.path());
    // The first start knows of nothing earlier to tell of.
    assert!(notified(&app.apply(discord_event(everyone()))).is_empty());
    assert!(app.state.relationship_alerts.is_empty());
    // A presence, or a snapshot of the same lists, tells of nothing.
    assert!(notified(&app.apply(discord_event(everyone()))).is_empty());

    // Annie removes Alice: told, with her DM to open, and listed.
    let mut snapshot = everyone();
    snapshot.friends = vec!["3".into()].into();
    snapshot.friend_list = vec![friend("3", "Ben")].into();
    let effects = app.apply(discord_event(snapshot.clone()));
    assert_eq!(
        notified(&effects),
        [(
            "discord:@me:30".into(),
            "Annie".into(),
            "Removed you as a friend.".into()
        )]
    );
    assert_eq!(listed(&app), [(Removal::Unfriended, "Annie", false)]);
    assert_eq!(
        app.state.relationship_alerts[0].avatar.as_deref(),
        Some("2.png")
    );

    // Alice cancels her request to Dave and leaves Other herself: nothing.
    app.dispatch(Action::RemoveFriendRequest("5".into()));
    app.dispatch(Action::AskLeaveServer("discord:11".into()));
    assert!(matches!(
        app.dispatch(Action::Leave).as_slice(),
        [Effect::Backend(Command::Discord(
            DiscordCommand::LeaveServer { .. }
        ))]
    ));
    snapshot.friend_requests = vec![snapshot.friend_requests[0].clone()].into();
    snapshot.servers = vec![server("10", "Test")];
    assert!(notified(&app.apply(discord_event(snapshot.clone()))).is_empty());
    assert_eq!(app.state.relationship_alerts.len(), 1);

    // Erin withdraws hers, Alice is removed from Hikers and from Test.
    snapshot.friend_requests = Vec::new().into();
    snapshot.conversations.truncate(1);
    snapshot.servers = Vec::new();
    let effects = app.apply(discord_event(snapshot.clone()));
    assert_eq!(notified(&effects).len(), 3);
    assert_eq!(
        listed(&app),
        [
            (Removal::RequestWithdrawn, "Erin", false),
            (Removal::Group, "Hikers", false),
            (Removal::Server, "Test", false),
            (Removal::Unfriended, "Annie", false),
        ]
    );

    // Off for groups, and set to Do Not Disturb: listed, not notified.
    crate::extras::test_relationship_alerts(Kinds {
        groups: false,
        ..Kinds::ALL
    });
    let mut with_group = snapshot.clone();
    with_group.conversations = everyone().conversations;
    with_group.friends = vec!["2".into(), "3".into()].into();
    with_group.friend_list = everyone().friend_list;
    app.apply(discord_event(with_group));
    snapshot.presence = "dnd".into();
    let effects = app.apply(discord_event(snapshot));
    assert!(
        notified(&effects).is_empty(),
        "Do Not Disturb holds it back"
    );
    assert_eq!(
        &listed(&app)[..2],
        [
            (Removal::Unfriended, "Annie", false),
            (Removal::RequestWithdrawn, "Erin", false)
        ],
        "the group isn't told of"
    );

    // Cleared, Friend Alerts are empty, on disk too.
    app.dispatch(Action::ClearRelationshipAlerts);
    assert!(app.state.relationship_alerts.is_empty());
    let kept = crate::relationship_alerts::read(
        &folder.path().join(crate::relationship_alerts::FILE),
        "1",
    );
    assert!(kept.alerts.is_empty() && kept.known.is_some());
}

#[test]
fn a_start_tells_of_what_went_while_backcord_was_closed() {
    crate::extras::test_relationship_alerts(Kinds::ALL);
    let folder = tempfile::tempdir().expect("a folder");
    let mut app = Application::default();
    app.keep_relationship_alerts_in(folder.path());
    app.apply(discord_event(everyone()));
    drop(app);
    // Next start: Ben and the server Other are gone.
    let mut snapshot = everyone();
    snapshot.friends = vec!["2".into()].into();
    snapshot.friend_list = vec![friend("2", "Annie")].into();
    snapshot.servers = vec![server("10", "Test")];
    let mut app = Application::default();
    app.keep_relationship_alerts_in(folder.path());
    // What shows while connecting isn't compared.
    let connecting = Snapshot {
        status: Status::Connecting,
        ..Snapshot::default()
    };
    assert!(notified(&app.apply(discord_event(connecting))).is_empty());
    let effects = app.apply(discord_event(snapshot.clone()));
    assert_eq!(
        notified(&effects),
        [
            (
                String::new(),
                "Ben".into(),
                "Is no longer your friend.".into()
            ),
            (
                String::new(),
                "Other".into(),
                "You're no longer in this server.".into()
            ),
        ]
    );
    assert_eq!(
        listed(&app),
        [
            (Removal::Unfriended, "Ben", true),
            (Removal::Server, "Other", true)
        ]
    );
    // Another start lists them again, and tells of nothing new.
    let mut app = Application::default();
    app.keep_relationship_alerts_in(folder.path());
    assert!(notified(&app.apply(discord_event(snapshot.clone()))).is_empty());
    assert_eq!(app.state.relationship_alerts.len(), 2);
    // Another account keeps its own.
    let mut app = Application::default();
    app.keep_relationship_alerts_in(folder.path());
    snapshot.user_id = Some("9".into());
    app.apply(discord_event(snapshot));
    assert!(app.state.relationship_alerts.is_empty());

    // Without While offline, a start lists nothing that went meanwhile.
    crate::extras::test_relationship_alerts(Kinds {
        offline: false,
        ..Kinds::ALL
    });
    let mut app = Application::default();
    app.keep_relationship_alerts_in(folder.path());
    let effects = app.apply(discord_event(Snapshot {
        servers: Vec::new(),
        ..everyone()
    }));
    assert!(notified(&effects).is_empty());
}
