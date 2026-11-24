//! The DM list's folders, kept on this device as pinned DMs are: each a
//! name and the DMs in it, which the DM home lists under its heading,
//! after the pinned DMs and before the rest. A DM is in one place at a
//! time: pinned, in one folder, or loose. Dropped onto another DM, or
//! picked and put in a new folder, DMs make one; a folder with none left
//! goes.

use super::*;

/// A folder of DMs.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DmFolder {
    /// `dm-folder:N`, which its menu and its heading go by.
    pub id: String,
    /// As named; empty for one not named yet.
    #[serde(default)]
    pub name: String,
    /// Its DMs, by room id, in order.
    #[serde(default)]
    pub rooms: Vec<String>,
    /// Folded: its heading alone shows.
    #[serde(default)]
    pub collapsed: bool,
}

/// A change to the DM list's folders.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DmFolderChange {
    /// DMs put in a folder, by its id, or in a new one: they leave where
    /// they were, pinned or in another folder.
    Add {
        rooms: Vec<String>,
        folder: Option<String>,
    },
    /// DMs taken out of their folders.
    Remove(Vec<String>),
    Rename {
        folder: String,
        name: String,
    },
    /// A folder folds, or opens.
    Toggle(String),
    /// A folder goes; its DMs stay, loose.
    Delete(String),
}

/// The longest name a folder takes, in characters.
pub const DM_FOLDER_NAME: usize = 100;

impl AppState {
    /// The folder a DM is in.
    pub fn dm_folder_of(&self, room: &str) -> Option<&DmFolder> {
        self.dm_folders
            .iter()
            .find(|folder| folder.rooms.iter().any(|id| id == room))
    }

    /// Whether an id is a DM folder's.
    pub fn is_dm_folder(&self, id: &str) -> bool {
        self.dm_folders.iter().any(|folder| folder.id == id)
    }

    /// A folder's menu: rename it, or delete it.
    pub(super) fn dm_folder_choices(&self) -> Vec<MenuChoice> {
        if self.preview {
            return vec![];
        }
        vec![MenuChoice::RenameDmFolder, MenuChoice::DeleteDmFolder]
    }

    /// A listed DM that isn't a request, which folders may hold.
    fn foldable(&self, room: &str) -> bool {
        self.rooms
            .iter()
            .any(|listed| listed.id == room && listed.direct && !listed.request)
    }

    /// The DMs leave their folders; a folder with none left goes.
    pub(super) fn leave_dm_folders(&mut self, rooms: &[String]) {
        for folder in &mut self.dm_folders {
            folder.rooms.retain(|room| !rooms.contains(room));
        }
        self.dm_folders.retain(|folder| !folder.rooms.is_empty());
    }

    /// The id a new folder takes.
    fn new_dm_folder_id(&self) -> String {
        let next = self
            .dm_folders
            .iter()
            .filter_map(|folder| folder.id.strip_prefix("dm-folder:")?.parse::<u64>().ok())
            .max()
            .map_or(1, |most| most + 1);
        format!("dm-folder:{next}")
    }
}

impl Application {
    pub(super) fn change_dm_folders(&mut self, change: DmFolderChange) -> Vec<Effect> {
        let state = &mut self.state;
        if state.preview {
            return vec![];
        }
        match change {
            DmFolderChange::Add { rooms, folder } => {
                let mut moved: Vec<String> = Vec::with_capacity(rooms.len());
                for room in rooms {
                    if state.foldable(&room) && !moved.contains(&room) {
                        moved.push(room);
                    }
                }
                if moved.is_empty() || folder.as_ref().is_some_and(|id| !state.is_dm_folder(id)) {
                    return vec![];
                }
                state.pinned.retain(|id| !moved.contains(id));
                for each in &mut state.dm_folders {
                    each.rooms.retain(|room| !moved.contains(room));
                }
                match folder.and_then(|id| state.dm_folders.iter_mut().find(|each| each.id == id)) {
                    Some(into) => into.rooms.extend(moved),
                    None => {
                        let id = state.new_dm_folder_id();
                        state.dm_folders.push(DmFolder {
                            id,
                            rooms: moved,
                            ..Default::default()
                        });
                    }
                }
                state.dm_folders.retain(|each| !each.rooms.is_empty());
            }
            DmFolderChange::Remove(rooms) => state.leave_dm_folders(&rooms),
            DmFolderChange::Rename { folder, name } => {
                let Some(folder) = state.dm_folders.iter_mut().find(|each| each.id == folder)
                else {
                    return vec![];
                };
                folder.name = name.trim().chars().take(DM_FOLDER_NAME).collect();
            }
            DmFolderChange::Toggle(folder) => {
                let Some(folder) = state.dm_folders.iter_mut().find(|each| each.id == folder)
                else {
                    return vec![];
                };
                folder.collapsed = !folder.collapsed;
            }
            DmFolderChange::Delete(folder) => state.dm_folders.retain(|each| each.id != folder),
        }
        state.people_changed();
        vec![]
    }
}
