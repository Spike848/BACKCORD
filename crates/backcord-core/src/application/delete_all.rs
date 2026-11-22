//! Cancella tutto, from a Discord DM's or server's menu: it asks first,
//! then deletes every message this account sent there, one at a time,
//! with how many went so far in the notice and in the menu, which stops
//! it. See "Cancella tutto" in docs/discord/messages.md.

use super::*;

/// Asks before deleting every message this account sent in a DM or server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeleteAllDialog {
    /// The DM's room id or the server's id, and its name.
    pub place: String,
    pub name: String,
    pub server: bool,
}

/// Cancella tutto going on in a DM or server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeletingAll {
    pub place: String,
    pub name: String,
    /// How many of this account's messages it deleted so far.
    pub deleted: u32,
    /// Asked to stop, which it does after the deletion going on.
    pub stopping: bool,
}

impl AppState {
    /// Whether a DM's or server's menu offers Cancella tutto: a listed
    /// Discord DM (not a message request) or server, while Discord is
    /// connected.
    pub fn may_delete_all(&self, id: &str) -> bool {
        if self.preview || self.discord_user.is_none() || !self.discord_online {
            return false;
        }
        self.servers
            .iter()
            .any(|server| server.discord && server.id == id)
            || self.rooms.iter().any(|room| {
                room.id == id && room.direct && !room.request && room.network == Network::Discord
            })
    }

    /// Cancella tutto going on in a DM or server, if it is.
    pub fn deleting_all(&self, id: &str) -> Option<&DeletingAll> {
        self.deleting_all.iter().find(|going| going.place == id)
    }

    /// A DM's or server's name, as its menu has it.
    fn place_name(&self, id: &str) -> String {
        self.servers
            .iter()
            .find(|server| server.id == id)
            .map(|server| server.name.clone())
            .or_else(|| {
                self.rooms
                    .iter()
                    .find(|room| room.id == id)
                    .map(|room| room.name.clone())
            })
            .unwrap_or_default()
    }
}

impl Application {
    pub(super) fn ask_delete_all(&mut self, place: String) -> Vec<Effect> {
        if !self.state.may_delete_all(&place) || self.state.deleting_all(&place).is_some() {
            return vec![];
        }
        self.state.delete_all = Some(DeleteAllDialog {
            name: self.state.place_name(&place),
            server: self.state.servers.iter().any(|server| server.id == place),
            place,
        });
        vec![]
    }

    /// Starts deleting where the dialog asked about.
    pub(super) fn delete_all(&mut self) -> Vec<Effect> {
        let Some(dialog) = self.state.delete_all.take() else {
            return vec![];
        };
        if !self.state.may_delete_all(&dialog.place)
            || self.state.deleting_all(&dialog.place).is_some()
        {
            return vec![];
        }
        self.state.notice = Some(format!("Cancella tutto in {}: avvio…", dialog.name));
        self.state.deleting_all.push(DeletingAll {
            place: dialog.place.clone(),
            name: dialog.name,
            deleted: 0,
            stopping: false,
        });
        vec![Effect::Backend(Command::Discord(
            DiscordCommand::DeleteAll {
                place: dialog.place,
            },
        ))]
    }

    pub(super) fn stop_delete_all(&mut self, place: String) -> Vec<Effect> {
        let Some(going) = self
            .state
            .deleting_all
            .iter_mut()
            .find(|going| going.place == place && !going.stopping)
        else {
            return vec![];
        };
        going.stopping = true;
        vec![Effect::Backend(Command::Discord(
            DiscordCommand::StopDeleteAll { place },
        ))]
    }

    pub(super) fn deleted_all(
        &mut self,
        place: String,
        deleted: u32,
        ended: bool,
        error: Option<String>,
    ) {
        let Some(at) = self
            .state
            .deleting_all
            .iter()
            .position(|going| going.place == place)
        else {
            return;
        };
        let going = &mut self.state.deleting_all[at];
        going.deleted = going.deleted.max(deleted);
        let (name, count, stopping) = (going.name.clone(), going.deleted, going.stopping);
        let messages = if count == 1 { "messaggio" } else { "messaggi" };
        if !ended {
            self.state.notice = Some(format!(
                "Cancella tutto in {name}: {count} {messages} cancellati…"
            ));
            return;
        }
        self.state.deleting_all.remove(at);
        self.state.notice = Some(match error {
            Some(error) => {
                format!("Cancella tutto in {name} si è fermato dopo {count} {messages}: {error}")
            }
            None if stopping => {
                format!("Cancella tutto in {name} fermato: {count} {messages} cancellati.")
            }
            None => format!("Cancella tutto in {name} finito: {count} {messages} cancellati."),
        });
    }
}
