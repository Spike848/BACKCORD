//! The DM home's folders and picks: DMs picked with Ctrl, their menu, a
//! folder named in place, and DMs dragged into it and out.

use super::*;
use crate::arrange::{DmTarget, Event as Drag};
use crate::sidebar::{Event, SideRow};
use backcord_core::application::Navigation;

/// The preview's DM home, as an account's: pins and folders take.
fn dm_home() -> (App, tempfile::TempDir, Vec<String>) {
    let (mut app, folder) = preview();
    change(&mut app, |state| {
        state.preview = false;
        state.navigation = Navigation::DirectMessages;
    });
    let dms: Vec<String> = app
        .client
        .state()
        .rooms
        .iter()
        .filter(|room| room.direct && !room.request)
        .map(|room| room.id.clone())
        .collect();
    assert!(dms.len() >= 3, "{dms:?}");
    (app, folder, dms)
}

fn labels(app: &App) -> Vec<String> {
    app.win
        .menu
        .as_ref()
        .map(|menu| menu.rows.iter().map(|row| row.label.to_string()).collect())
        .unwrap_or_default()
}

fn choose(app: &mut App, label: &str) {
    let at = labels(app)
        .iter()
        .position(|shown| shown == label)
        .unwrap_or_else(|| panic!("{label} in {:?}", labels(app)));
    let _ = app.update(Message::Menu(crate::menus::Event::Choose(at)));
}

/// The rows' keys, from the first DM folder or DM on.
fn listed(app: &App) -> Vec<String> {
    app.sidebar
        .rows
        .iter()
        .filter(|row| matches!(row, SideRow::Room(_) | SideRow::DmFolder { .. }))
        .map(SideRow::key)
        .collect()
}

fn drop_on(app: &mut App, room: &str, onto: DmTarget) {
    let _ = app.update(Message::Arrange(Drag::TakeDm(room.into())));
    let over = Point::new(crate::rail::WIDTH + 100.0, app.shell.frame.height / 2.0);
    let _ = app.update(Message::Arrange(Drag::At(over)));
    let _ = app.update(Message::Arrange(Drag::OntoDm(onto, true)));
    let _ = app.update(Message::Arrange(Drag::Drop));
}

#[test]
fn dms_picked_with_ctrl_go_in_a_folder_named_in_place() {
    let (mut app, _folder, dms) = dm_home();
    let selected = app.client.state().selected.clone();
    app.modifiers = iced::keyboard::Modifiers::CTRL;
    for dm in &dms[..2] {
        let _ = app.update(Message::Side(Event::Select(dm.clone())));
    }
    app.modifiers = iced::keyboard::Modifiers::default();
    assert_eq!(app.sidebar.dm_picked, dms[..2]);
    assert_eq!(app.client.state().selected, selected, "picked, not opened");
    // A right click on one of them: their menu.
    let _ = app.update(Message::Side(Event::Menu(
        dms[1].clone(),
        Point::new(200.0, 200.0),
    )));
    let shown = labels(&app);
    assert_eq!(shown[0], "2 conversations picked");
    assert!(shown.contains(&"Pin".to_owned()), "{shown:?}");
    choose(&mut app, "New folder");
    let state = app.client.state();
    assert_eq!(state.dm_folders.len(), 1);
    let folder = state.dm_folders[0].clone();
    assert_eq!(folder.rooms, dms[..2]);
    assert!(app.sidebar.dm_picked.is_empty(), "the picking ends");
    // Its heading, then its DMs, set in, before the rest.
    let rows = listed(&app);
    assert_eq!(
        rows[..3],
        [folder.id.clone(), dms[0].clone(), dms[1].clone()]
    );
    // Named in place, and kept in layout.json.
    assert_eq!(
        app.sidebar.renaming,
        Some((folder.id.clone(), String::new()))
    );
    let _ = app.update(Message::Side(Event::FolderName("Amici".into())));
    let _ = app.update(Message::Side(Event::FolderRenamed));
    assert_eq!(app.client.state().dm_folders[0].name, "Amici");
    assert_eq!(app.layout.dm_folders, app.client.state().dm_folders);
    // A click folds it to its heading.
    let _ = app.update(Message::Side(Event::ToggleFolder(folder.id.clone())));
    assert!(
        !listed(&app).contains(&dms[0]) || app.client.state().selected.as_ref() == Some(&dms[0])
    );
    let _ = app.update(Message::Side(Event::ToggleFolder(folder.id.clone())));

    // Another DM dragged onto the folder goes in it; dragged onto the
    // heading, out again.
    drop_on(&mut app, &dms[2], DmTarget::Folder(folder.id));
    assert_eq!(app.client.state().dm_folders[0].rooms, dms[..3]);
    drop_on(&mut app, &dms[2], DmTarget::Loose);
    assert_eq!(app.client.state().dm_folders[0].rooms, dms[..2]);
    // Onto a DM out of any folder, a new folder of both.
    drop_on(&mut app, &dms[0], DmTarget::Room(dms[2].clone()));
    let folders = &app.client.state().dm_folders;
    assert_eq!(folders.len(), 2);
    assert_eq!(folders[1].rooms, [dms[2].clone(), dms[0].clone()]);
    assert_eq!(app.layout.dm_folders, *folders);
}

#[test]
fn a_dms_menu_pins_it_over_the_folders_and_starts_one() {
    let (mut app, _folder, dms) = dm_home();
    let menu = |app: &mut App, room: &str| {
        let _ = app.update(Message::Side(Event::Menu(
            room.into(),
            Point::new(200.0, 200.0),
        )));
    };
    menu(&mut app, &dms[0]);
    let shown = labels(&app);
    let pin = shown.iter().position(|label| label == "Pin").expect("Pin");
    assert_eq!(shown[pin + 1], "New folder");
    choose(&mut app, "New folder");
    let folder = app.client.state().dm_folders[0].id.clone();
    // Pinned, a DM goes over the folders.
    menu(&mut app, &dms[1]);
    choose(&mut app, "Pin");
    assert_eq!(
        listed(&app)[..3],
        [dms[1].clone(), folder.clone(), dms[0].clone()]
    );
    assert_eq!(app.layout.pinned, [dms[1].clone()]);
    // The folder's own menu renames it or deletes it.
    menu(&mut app, &folder);
    assert_eq!(labels(&app), ["Rename folder", "Delete folder"]);
    choose(&mut app, "Delete folder");
    assert!(app.client.state().dm_folders.is_empty());
    assert!(app.layout.dm_folders.is_empty());
}
