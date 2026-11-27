//! The DM home's folders and picks: DMs picked with Ctrl and a click, lit,
//! whose menu (a right click on any of them) pins them, unpins them, puts
//! them in a new folder or takes them out of theirs; a folder's heading,
//! which folds and opens it, renames it in place and opens its menu; and
//! where DMs dragged (`arrange.rs`) land: onto a folder, into it; onto a
//! DM in one, into that one; onto a pinned DM, pinned; onto another DM, in
//! a new folder with it; onto the "Direct Messages" heading, out of their
//! folders and unpinned. Folders and pins are the core's, kept in
//! `layout.json` (`person_menu::keep_pins`).

use backcord_core::application::{Action, DmFolderChange, MenuChoice};
use iced::{
    Alignment, Element, Length, Point, Task,
    widget::{Space, container, mouse_area, operation, row, text, text_input},
};

use crate::{
    app::{App, Message, NOBODY},
    arrange::DmTarget,
    controls::{ROW_COUNT, count_pill},
    elide::elide,
    fonts::SEMIBOLD,
    menus::{Menu, MenuAct, MenuRow},
    pictures::Icon,
    style::rounded,
    widgets::{Drags, RightPress},
};

use super::{DM_FOLDER, DM_ROW, Event, ROW_RADIUS, SideRow};

/// The field a folder is renamed in.
pub const RENAME: &str = "dm-folder-name";

impl<'a> super::View<'a> {
    /// A folder's heading: its chevron, folder and name (or the field it's
    /// renamed in), how many DMs it lists, and their mentions.
    pub(super) fn dm_folder_row(self, row: &'a SideRow) -> Element<'a, Message> {
        let SideRow::DmFolder {
            id,
            name,
            count,
            collapsed,
            unread,
            badge,
        } = row
        else {
            return Space::new().into();
        };
        let (id, name, count, collapsed, unread) =
            (id.as_str(), name.as_str(), *count, *collapsed, *unread);
        let theme = self.cx.look.theme;
        let pointed = self.cx.pointed.side.as_deref() == Some(id);
        let ink = if unread {
            theme.text
        } else if pointed {
            theme.secondary
        } else {
            theme.muted
        };
        let label: Element<'a, Message> = match &self.s.renaming {
            Some((folder, typed)) if folder == id => {
                let (ground, value, hint, selection) =
                    (theme.field, theme.text, theme.muted, theme.selection);
                text_input(backcord_iced_kit::i18n::tr("Folder name"), typed)
                    .id(RENAME)
                    .on_input(|typed| Message::Side(Event::FolderName(typed)))
                    .on_submit(Message::Side(Event::FolderRenamed))
                    .size(13)
                    .padding([3, 6])
                    .style(move |_, _| text_input::Style {
                        background: ground.into(),
                        border: crate::style::round(4),
                        icon: value,
                        placeholder: hint,
                        value,
                        selection,
                    })
                    .into()
            }
            _ => {
                let shown = if name.is_empty() {
                    backcord_iced_kit::i18n::tr("New folder")
                } else {
                    name
                };
                elide(shown).size(14).font(SEMIBOLD).color(ink).into()
            }
        };
        let mut face = row![
            self.cx.look.glyph(
                if collapsed {
                    Icon::ChevronRight
                } else {
                    Icon::ChevronDown
                },
                12,
                ink
            ),
            self.cx.look.glyph(Icon::Folder, 16, ink),
            container(label).width(Length::Fill),
            text(count.to_string()).size(12).color(theme.muted),
        ]
        .spacing(6)
        .align_y(Alignment::Center);
        if !badge.is_empty() {
            face = face.push(count_pill(theme, badge, ROW_COUNT, None));
        }
        let ground = if pointed {
            theme.hover
        } else {
            iced::Color::TRANSPARENT
        };
        let area = mouse_area(
            container(face)
                .padding([0, 8])
                .height(DM_FOLDER)
                .width(Length::Fill)
                .align_y(Alignment::Center)
                .style(move |_| rounded(ground, ROW_RADIUS)),
        )
        .interaction(iced::mouse::Interaction::Pointer)
        .on_enter(Message::Side(Event::Point(id.to_owned(), true)))
        .on_exit(Message::Side(Event::Point(id.to_owned(), false)));
        let (folder, spot) = (id.to_owned(), self.cx.pressed);
        RightPress::new(area, move |_| {
            Message::Side(Event::Menu(folder.clone(), spot.get()))
        })
        .into()
    }

    /// A row of the DM home that takes part in dragging DMs: a DM's,
    /// clicked or dragged; a folder's, clicked to fold; and the heading's.
    /// While DMs are dragged, each says when the pointer comes onto it.
    pub(super) fn dm_drags(
        self,
        row: &'a SideRow,
        inset: Element<'a, Message>,
    ) -> Element<'a, Message> {
        let dragging = self.s.dm_moving.is_some();
        let onto = move |target: DmTarget| {
            move |over| Message::Arrange(crate::arrange::Event::OntoDm(target.clone(), over))
        };
        match row {
            SideRow::Room(room) => {
                let id = room.id.clone();
                let drags = Drags::new(inset)
                    .click(Message::Side(Event::Select(room.id.clone())))
                    .head(DM_ROW)
                    .dropped(Message::Arrange(crate::arrange::Event::Drop))
                    .tracking(dragging, onto(DmTarget::Room(room.id.clone())));
                // With Ctrl held a press starts the list's selection box.
                if self.cx.modifiers.control() {
                    drags.into()
                } else {
                    drags
                        .drag(move |_| Message::Arrange(crate::arrange::Event::TakeDm(id.clone())))
                        .into()
                }
            }
            SideRow::DmFolder { id, .. } => Drags::new(inset)
                .click(Message::Side(Event::ToggleFolder(id.clone())))
                .tracking(dragging, onto(DmTarget::Folder(id.clone())))
                .into(),
            SideRow::DmHeading => Drags::new(inset)
                .tracking(dragging, onto(DmTarget::Loose))
                .into(),
            _ => inset,
        }
    }
}

/// What DMs dragged onto `target` become.
pub fn drop_dms(app: &mut App, rooms: Vec<String>, target: DmTarget) -> Task<Message> {
    let state = app.client.state();
    let mut actions = Vec::new();
    let mut named = false;
    match target {
        DmTarget::Folder(folder) => actions.push(Action::DmFolder(DmFolderChange::Add {
            rooms,
            folder: Some(folder),
        })),
        DmTarget::Room(onto) if rooms.contains(&onto) => return Task::none(),
        DmTarget::Room(onto) => {
            if let Some(folder) = state.dm_folder_of(&onto) {
                actions.push(Action::DmFolder(DmFolderChange::Add {
                    rooms,
                    folder: Some(folder.id.clone()),
                }));
            } else if state.pinned.contains(&onto) {
                actions.extend(
                    rooms
                        .into_iter()
                        .map(|room| Action::Pin { room, pinned: true }),
                );
            } else {
                // As servers dropped on each other: a new folder of both.
                let mut both = vec![onto];
                both.extend(rooms);
                actions.push(Action::DmFolder(DmFolderChange::Add {
                    rooms: both,
                    folder: None,
                }));
                named = true;
            }
        }
        DmTarget::Loose => {
            for room in rooms.iter().filter(|room| state.pinned.contains(room)) {
                actions.push(Action::Pin {
                    room: room.clone(),
                    pinned: false,
                });
            }
            actions.push(Action::DmFolder(DmFolderChange::Remove(rooms)));
        }
    }
    finish(app, actions, named)
}

/// Carries out `actions`, keeps what they changed in `layout.json`, ends
/// the picking, and, for a new folder, starts naming it.
fn finish(app: &mut App, actions: Vec<Action>, named: bool) -> Task<Message> {
    let before = app.client.state().dm_folders.len();
    for action in actions {
        app.client.dispatch(action);
    }
    app.sidebar.dm_picked.clear();
    crate::person_menu::keep_pins(app);
    let folders = &app.client.state().dm_folders;
    let task = match folders.last() {
        Some(folder) if named && folders.len() > before => {
            app.sidebar.renaming = Some((folder.id.clone(), String::new()));
            operation::focus(RENAME)
        }
        _ => Task::none(),
    };
    Task::batch([app.sync(), task])
}

/// A DM's or a folder's menu row about folders: a new folder for the DM,
/// the DM out of its folder, a folder renamed in place, or deleted.
pub fn choose(app: &mut App, place: String, choice: MenuChoice) -> Task<Message> {
    let action = match choice {
        MenuChoice::NewDmFolder => {
            let add = Action::DmFolder(DmFolderChange::Add {
                rooms: vec![place],
                folder: None,
            });
            return finish(app, vec![add], true);
        }
        MenuChoice::RemoveFromDmFolder => Action::DmFolder(DmFolderChange::Remove(vec![place])),
        MenuChoice::DeleteDmFolder => Action::DmFolder(DmFolderChange::Delete(place)),
        MenuChoice::RenameDmFolder => {
            let name = app
                .client
                .state()
                .dm_folders
                .iter()
                .find(|folder| folder.id == place)
                .map(|folder| folder.name.clone())
                .unwrap_or_default();
            app.sidebar.renaming = Some((place, name));
            return Task::batch([app.sync(), operation::focus(RENAME)]);
        }
        _ => return Task::none(),
    };
    finish(app, vec![action], false)
}

/// What's done to the folders from the sidebar: a folder folded or opened,
/// its name typed, and the name kept.
pub fn update(app: &mut App, event: &Event) -> Option<Task<Message>> {
    match event {
        Event::ToggleFolder(folder) => {
            app.client
                .dispatch(Action::DmFolder(DmFolderChange::Toggle(folder.clone())));
            crate::person_menu::keep_pins(app);
        }
        Event::FolderName(typed) => {
            if let Some((_, name)) = &mut app.sidebar.renaming {
                typed.clone_into(name);
            }
            return Some(Task::none());
        }
        Event::FolderRenamed => {
            if let Some((folder, name)) = app.sidebar.renaming.take() {
                app.client
                    .dispatch(Action::DmFolder(DmFolderChange::Rename { folder, name }));
                crate::person_menu::keep_pins(app);
            }
            return Some(Task::batch([app.sync(), operation::focus(NOBODY)]));
        }
        _ => return None,
    }
    Some(app.sync())
}

/// A click on a DM: with Ctrl, picks it or lets it go; without, the
/// picking ends. Whether the click picked.
pub fn click(app: &mut App, room: &str) -> bool {
    let state = app.client.state();
    let dm = state.navigation == backcord_core::application::Navigation::DirectMessages
        && state
            .rooms
            .iter()
            .any(|listed| listed.id == room && listed.direct);
    if dm && app.modifiers.control() {
        let picked = &mut app.sidebar.dm_picked;
        match picked.iter().position(|id| id == room) {
            Some(at) => {
                picked.remove(at);
            }
            None => picked.push(room.to_owned()),
        }
        return true;
    }
    app.sidebar.dm_picked.clear();
    false
}

/// A right click on a DM: on one of several picked, their menu; on any
/// other, the picking ends.
pub fn open_if_picked(app: &mut App, room: &str, at: Point) -> Option<Task<Message>> {
    let picked = &app.sidebar.dm_picked;
    if picked.len() >= 2 && picked.iter().any(|id| id == room) {
        return Some(open_menu(app, at));
    }
    app.sidebar.dm_picked.clear();
    None
}

/// The menu of the DMs picked: how many, then Mark as read, Pin, Unpin,
/// New folder and Remove from folder, as they fit.
fn open_menu(app: &mut App, at: Point) -> Task<Message> {
    let state = app.client.state();
    let picked = &app.sidebar.dm_picked;
    let listed = |room: &String| state.rooms.iter().find(|listed| listed.id == *room);
    let unread = picked
        .iter()
        .filter_map(listed)
        .any(|room| room.unread > 0 || room.mentions > 0);
    let pinned = picked
        .iter()
        .filter(|room| state.pinned.contains(room))
        .count();
    let foldered = picked.iter().any(|room| state.dm_folder_of(room).is_some());
    let mut choices = Vec::new();
    if unread {
        choices.push(MenuChoice::MarkRead);
    }
    if pinned < picked.len() {
        choices.push(MenuChoice::Pin);
    }
    if pinned > 0 {
        choices.push(MenuChoice::Unpin);
    }
    choices.push(MenuChoice::NewDmFolder);
    if foldered {
        choices.push(MenuChoice::RemoveFromDmFolder);
    }
    let mut rows = vec![MenuRow {
        disabled: true,
        ..MenuRow::plain(
            MenuAct::DmPicked(MenuChoice::CopyId),
            backcord_iced_kit::i18n::t(format!("{} conversations picked", picked.len())),
        )
    }];
    rows.extend(
        choices
            .into_iter()
            .enumerate()
            .map(|(index, choice)| MenuRow {
                icon: Some(crate::place_menus::icon(choice)),
                separated: index == 0,
                ..MenuRow::plain(MenuAct::DmPicked(choice), crate::place_menus::label(choice))
            }),
    );
    app.win.menu = Some(Menu::new(at, rows).width(crate::menus::FIELD_WIDTH));
    operation::focus(NOBODY)
}

/// A row of the picked DMs' menu: it acts on each, then the picking ends.
pub fn choose_picked(app: &mut App, choice: MenuChoice) -> Task<Message> {
    let picked = std::mem::take(&mut app.sidebar.dm_picked);
    let actions: Vec<Action> = match choice {
        MenuChoice::MarkRead => picked.into_iter().map(Action::MarkRead).collect(),
        MenuChoice::Pin | MenuChoice::Unpin => {
            let pinned = choice == MenuChoice::Pin;
            let state = app.client.state();
            picked
                .into_iter()
                .filter(|room| state.pinned.contains(room) != pinned)
                .map(|room| Action::Pin { room, pinned })
                .collect()
        }
        MenuChoice::NewDmFolder => {
            let add = Action::DmFolder(DmFolderChange::Add {
                rooms: picked,
                folder: None,
            });
            return finish(app, vec![add], true);
        }
        MenuChoice::RemoveFromDmFolder => vec![Action::DmFolder(DmFolderChange::Remove(picked))],
        _ => return Task::none(),
    };
    finish(app, actions, false)
}

/// The DMs picked that are still listed; the rest are let go.
pub fn keep_listed(app: &mut App) {
    if app.sidebar.dm_picked.is_empty() {
        return;
    }
    let rows = &app.sidebar.rows;
    app.sidebar.dm_picked.retain(|room| {
        rows.iter()
            .any(|row| matches!(row, SideRow::Room(listed) if listed.id == *room))
    });
}
