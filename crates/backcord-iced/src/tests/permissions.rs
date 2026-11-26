//! Permissions Viewer: My Permissions in a server's and a channel's menus,
//! Permissions in a member's menu and on a role's card, its window, which
//! Escape and its X close, and its switch in Extra Features.

use backcord_core::{
    application::{Action, PermissionSource},
    backcord_discord::ready::permission,
};
use iced::Point;

use super::{interact, server::discord_server, shows};
use crate::app::{App, Message};

const SERVER: &str = "discord:10";
const GENERAL: &str = "discord:10:11";

/// The open menu's labels.
fn labels(app: &App) -> Vec<String> {
    app.win
        .menu
        .as_ref()
        .expect("a menu")
        .rows
        .iter()
        .map(|row| row.label.to_string())
        .collect()
}

/// Chooses the open menu's row reading `label`.
fn choose(app: &mut App, label: &str) {
    let at = labels(app)
        .iter()
        .position(|row| row == label)
        .unwrap_or_else(|| panic!("no {label:?} in {:?}", labels(app)));
    let _ = app.update(Message::Menu(crate::menus::Event::Choose(at)));
}

/// The window on the server, with #general open.
fn in_general(extra: u64) -> (App, tempfile::TempDir) {
    let (mut app, folder) = discord_server(extra, |server, _| {
        server.put_role(
            &serde_json::json!({"id": "20", "name": "Mods", "position": 2,
            "color": 0x3498db, "permissions": permission::KICK_MEMBERS.to_string()}),
        );
    });
    app.client.app.state.selected = Some(GENERAL.into());
    let _ = app.sync();
    (app, folder)
}

#[test]
fn my_permissions_opens_from_a_servers_menu_and_escape_closes_it() {
    let (mut app, _folder) = in_general(0);
    let _ = crate::place_menus::open_place_menu(&mut app, SERVER, Point::new(80.0, 44.0), false);
    choose(&mut app, "My Permissions");
    let shown = app.client.state().permissions.as_ref().expect("the window");
    assert_eq!(shown.place, "Test");
    assert!(shows(&app, "Permissions"));
    assert!(shows(&app, "View Channels"));
    assert!(shows(&app, "Given by @everyone"));
    assert!(shows(&app, "Manage Channels"));
    assert!(shows(&app, "No role gives it"));
    let _ = app.update(Message::Key(
        iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
        Default::default(),
    ));
    assert!(app.client.state().permissions.is_none());
    // A channel's menu shows its own.
    let _ = crate::place_menus::open_place_menu(&mut app, GENERAL, Point::new(80.0, 120.0), false);
    choose(&mut app, "My Permissions");
    let shown = app.client.state().permissions.as_ref().expect("the window");
    assert_eq!(shown.place, "general");
    assert!(shows(&app, "View Channel"));
    let _ = app.update(Message::Permissions(crate::permissions::Event::Close));
    assert!(app.client.state().permissions.is_none());
}

#[test]
fn a_members_menu_shows_their_permissions_where_it_opens() {
    let (mut app, _folder) = in_general(0);
    let _ = crate::person_menu::open_person_menu(
        &mut app,
        "99",
        Point::new(300.0, 200.0),
        crate::person_menu::CONTEXT_MENU,
    );
    choose(&mut app, "Permissions");
    let shown = app.client.state().permissions.as_ref().expect("the window");
    assert_eq!(shown.everything, Some(PermissionSource::Owner));
    assert_eq!(shown.place, "general");
    assert!(shows(
        &app,
        "Owns the server: every permission, in every channel."
    ));
    assert!(shows(&app, "Server owner"));
}

#[test]
fn a_roles_card_shows_what_the_role_gives() {
    let (mut app, _folder) = in_general(0);
    let _ = app.show_role("20".into(), Point::new(300.0, 200.0));
    app.client.app.state.role.as_mut().expect("the card").name = "Mods".into();
    interact(&mut app, |ui| {
        ui.click("Permissions").expect("the card's Permissions");
    });
    assert!(app.client.state().role.is_none(), "in place of the card");
    let shown = app.client.state().permissions.as_ref().expect("the window");
    assert_eq!(shown.name, "@Mods");
    let kick = shown
        .groups
        .iter()
        .flat_map(|group| &group.rows)
        .find(|row| row.bit == 1)
        .expect("Kick Members");
    assert!(kick.held);
    assert_eq!(kick.source, PermissionSource::Role("Mods".into()));
    assert!(shows(&app, "@Mods"));
}

#[test]
fn turned_off_in_extra_features_no_menu_offers_it() {
    let (mut app, _folder) = in_general(0);
    app.layout.prefs.permissions_viewer = Some(false);
    let _ = crate::place_menus::open_place_menu(&mut app, SERVER, Point::new(80.0, 44.0), false);
    assert!(!labels(&app).iter().any(|label| label == "My Permissions"));
    let _ = crate::person_menu::open_person_menu(
        &mut app,
        "99",
        Point::new(300.0, 200.0),
        crate::person_menu::CONTEXT_MENU,
    );
    assert!(!labels(&app).iter().any(|label| label == "Permissions"));
    app.client.dispatch(Action::ShowRole("20".into()));
    let _ = app.show_role("20".into(), Point::new(300.0, 200.0));
    assert!(!shows(&app, "Permissions"), "nor the role's card");
}
