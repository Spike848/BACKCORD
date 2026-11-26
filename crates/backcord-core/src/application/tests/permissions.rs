//! Permissions Viewer: what a member, this account or a role may do in a
//! Discord server or one of its channels, each permission with what gives
//! or takes it.

use super::*;
use backcord_discord::ready::{Timeout, permission};

const SERVER: &str = "discord:10";

/// The open window's row titled `title`.
fn row<'a>(app: &'a Application, title: &str) -> &'a PermissionRow {
    app.state
        .permissions
        .as_ref()
        .expect("the window")
        .groups
        .iter()
        .flat_map(|group| &group.rows)
        .find(|row| row.title == title)
        .unwrap_or_else(|| panic!("no {title:?}"))
}

fn held(held: bool, source: PermissionSource) -> (bool, PermissionSource) {
    (held, source)
}

fn verdict(app: &Application, title: &str) -> (bool, PermissionSource) {
    let row = row(app, title);
    (row.held, row.source.clone())
}

#[test]
fn permissions_viewer_offers_itself_only_in_a_servers_places() {
    let app = discord_server();
    assert!(app.offers_permissions(SERVER));
    assert!(app.offers_permissions(&crate::discord::room_id("10", "12")));
    assert!(
        app.offers_permissions(&crate::discord::room_id("10", "4")),
        "a category"
    );
    assert!(!app.offers_permissions("discord:77"));
    assert!(!app.offers_permissions("general"), "Matrix's");
}

#[test]
fn ones_own_permissions_in_a_server_name_the_role_that_gives_each() {
    let mut app = discord_server();
    assert!(
        app.dispatch(Action::ShowPermissions(PermissionsOf::Mine(SERVER.into())))
            .is_empty()
    );
    let shown = app.state.permissions.as_ref().expect("the window");
    assert_eq!(shown.place, "Test");
    assert_eq!(shown.everything, None);
    assert_eq!(shown.groups[0].heading, "General Server Permissions");
    let mods = || PermissionSource::Role("Mods".into());
    let everyone = || PermissionSource::Role("@everyone".into());
    assert_eq!(verdict(&app, "View Channels"), held(true, everyone()));
    assert_eq!(verdict(&app, "Manage Roles"), held(true, mods()));
    assert_eq!(
        verdict(&app, "Kick, Approve, and Reject Members"),
        held(true, mods())
    );
    assert_eq!(
        verdict(&app, "Ban Members"),
        held(false, PermissionSource::Nothing)
    );
    assert!(
        !shown
            .groups
            .iter()
            .flat_map(|group| &group.rows)
            .any(|row| row.title == "View Server Insights"),
        "only a Community server lists it"
    );
    app.dispatch(Action::ClosePermissions);
    assert!(app.state.permissions.is_none());
}

#[test]
fn in_a_channel_its_overrides_decide_and_its_page_lists_them() {
    let mut app = discord_server();
    let news = crate::discord::room_id("10", "12");
    app.dispatch(Action::ShowPermissions(PermissionsOf::Mine(news)));
    let shown = app.state.permissions.as_ref().expect("the window");
    assert_eq!(shown.place, "news");
    assert_eq!(shown.groups[0].heading, "General Channel Permissions");
    assert_eq!(
        verdict(&app, "Send Messages"),
        held(false, PermissionSource::RoleOverwrite("@everyone".into())),
        "#news denies @everyone sending"
    );
    assert_eq!(
        verdict(&app, "View Channel"),
        held(true, PermissionSource::Role("@everyone".into()))
    );
    // A member's own override has the last word.
    change_server(&mut app, |server| {
        let channel = server
            .text_channels
            .iter_mut()
            .find(|channel| channel.id == "12")
            .unwrap();
        let mut overwrites = channel.overwrites.to_vec();
        overwrites.push(backcord_discord::ready::Overwrite {
            id: "1".into(),
            member: true,
            allow: permission::SEND_MESSAGES,
            deny: 0,
        });
        channel.overwrites = overwrites.into();
    });
    app.dispatch(Action::ShowPermissions(PermissionsOf::Mine(
        crate::discord::room_id("10", "12"),
    )));
    assert_eq!(
        verdict(&app, "Send Messages"),
        held(true, PermissionSource::MemberOverwrite)
    );
}

#[test]
fn the_owner_and_administrators_hold_everything() {
    let mut app = discord_server();
    app.dispatch(Action::ShowPermissions(PermissionsOf::Member {
        place: crate::discord::room_id("10", "12"),
        user: "99".into(),
    }));
    let shown = app.state.permissions.as_ref().expect("the window");
    assert_eq!(shown.everything, Some(PermissionSource::Owner));
    assert!(
        shown
            .groups
            .iter()
            .flat_map(|group| &group.rows)
            .all(|row| row.held && row.source == PermissionSource::Owner),
        "even what #news denies"
    );
    change_server(&mut app, |server| {
        server.put_role(
            &serde_json::json!({"id": "20", "name": "Mods", "position": 2,
            "permissions": permission::ADMINISTRATOR.to_string()}),
        );
    });
    app.dispatch(Action::ShowPermissions(PermissionsOf::Mine(SERVER.into())));
    let admin = PermissionSource::Administrator("Mods".into());
    let shown = app.state.permissions.as_ref().expect("the window");
    assert_eq!(shown.everything, Some(admin.clone()));
    assert_eq!(verdict(&app, "Ban Members"), held(true, admin));
}

#[test]
fn a_timeout_leaves_only_viewing_and_reading() {
    let mut app = discord_server();
    change_server(&mut app, |server| {
        server.my_timeout = Some(Timeout {
            user: "1".into(),
            until: u64::MAX,
        });
    });
    app.dispatch(Action::ShowPermissions(PermissionsOf::Mine(SERVER.into())));
    assert_eq!(
        verdict(&app, "Kick, Approve, and Reject Members"),
        held(false, PermissionSource::Timeout)
    );
    assert_eq!(
        verdict(&app, "View Channels"),
        held(true, PermissionSource::Role("@everyone".into()))
    );
}

#[test]
fn a_role_shows_what_it_gives_on_its_own() {
    let mut app = discord_server();
    app.dispatch(Action::Navigate(Navigation::Server(SERVER.into())));
    app.dispatch(Action::ShowPermissions(PermissionsOf::Role {
        place: crate::discord::room_id("10", "12"),
        role: "20".into(),
    }));
    let shown = app.state.permissions.as_ref().expect("the window");
    assert_eq!(shown.name, "@Mods");
    assert_eq!(shown.color, Some(0x3498db));
    assert_eq!(shown.place, "Test", "in the server as a whole");
    assert_eq!(
        verdict(&app, "Kick, Approve, and Reject Members"),
        held(true, PermissionSource::Role("Mods".into()))
    );
    assert_eq!(
        verdict(&app, "View Channels"),
        held(false, PermissionSource::Nothing),
        "@everyone's isn't the role's"
    );
    // Opening another channel closes it.
    app.dispatch(Action::SelectRoom(crate::discord::room_id("10", "11")));
    assert!(app.state.permissions.is_none());
}
