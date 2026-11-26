//! Export Chat, from a Discord channel's, DM's or group's menu: its dialog
//! asks for the format, how much of the history and whether files go in
//! as links; the window then asks where to save it, and the backend
//! fetches the messages a page at a time, with how many came so far in the
//! dialog, which stops it. See "Export Chat" in docs/discord/messages.md.

use std::path::PathBuf;

use super::*;
use crate::chat_export::{ChatExport, ExportEnd, ExportFormat, ExportRange};

/// How much of the history the dialog asks for, before the number or the
/// day typed for it is read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExportSpan {
    #[default]
    All,
    /// The newest so many, as typed in `count`.
    Last,
    /// From the day typed in `since` on.
    Since,
}

/// The export dialog, while it's open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportDialog {
    /// The channel's, DM's or group's room id, and its name.
    pub room: String,
    pub name: String,
    pub format: ExportFormat,
    pub span: ExportSpan,
    /// How many of the newest, as typed.
    pub count: String,
    /// The first day, as typed (`dd/mm/yyyy`).
    pub since: String,
    /// Files and pictures go in as links.
    pub attachments: bool,
    /// Why what's typed can't be exported, when it can't.
    pub error: Option<String>,
    /// How the last export from this dialog ended.
    pub ended: Option<ExportEnd>,
    /// How many messages that export wrote.
    pub written: u32,
}

/// The export going on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Exporting {
    pub room: String,
    pub name: String,
    /// How many messages came so far.
    pub messages: u32,
    /// Asked to stop, which it does after the page going on.
    pub stopping: bool,
}

/// How many of the newest the dialog suggests.
pub const EXPORT_COUNT: u32 = 1000;

impl ExportDialog {
    /// The file's suggested name, for `today`.
    pub fn file_name(&self, today: chrono::NaiveDate) -> String {
        crate::chat_export::file_name(&self.name, self.format, today)
    }

    /// What's typed, as an export to `path`, or why it can't be.
    fn export(&self, path: PathBuf) -> Result<ChatExport, &'static str> {
        let range = match self.span {
            ExportSpan::All => ExportRange::All,
            ExportSpan::Last => match self.count.trim().parse::<u32>() {
                Ok(count) if count > 0 => ExportRange::Last(count),
                _ => return Err("Type how many messages to export."),
            },
            ExportSpan::Since => match crate::chat_export::parse_day(&self.since) {
                Some(day) => ExportRange::Since(crate::chat_export::day_start_ms(day)),
                None => return Err("Type the first day as dd/mm/yyyy."),
            },
        };
        Ok(ChatExport {
            room: self.room.clone(),
            title: self.name.clone(),
            format: self.format,
            range,
            attachments: self.attachments,
            path,
        })
    }
}

impl AppState {
    /// Whether a conversation's menu offers Export Chat: a listed Discord
    /// channel with messages, DM or group, while signed in to Discord.
    pub fn may_export_chat(&self, id: &str) -> bool {
        if self.preview || self.discord_user.is_none() {
            return false;
        }
        self.rooms.iter().any(|room| {
            room.id == id && room.network == Network::Discord && room.has_text() && !room.locked
        })
    }
}

impl Application {
    pub(super) fn ask_export_chat(&mut self, room: String) -> Vec<Effect> {
        if !self.state.may_export_chat(&room) {
            return vec![];
        }
        let name = self
            .state
            .rooms
            .iter()
            .find(|known| known.id == room)
            .map(|known| known.name.clone())
            .unwrap_or_default();
        self.state.chat_export = Some(ExportDialog {
            room,
            name,
            format: ExportFormat::Text,
            span: ExportSpan::All,
            count: EXPORT_COUNT.to_string(),
            since: String::new(),
            attachments: true,
            error: None,
            ended: None,
            written: 0,
        });
        vec![]
    }

    /// Changes what the open dialog asks for.
    pub(super) fn change_export(&mut self, change: impl FnOnce(&mut ExportDialog)) -> Vec<Effect> {
        if let Some(dialog) = &mut self.state.chat_export {
            change(dialog);
            dialog.error = None;
            dialog.ended = None;
        }
        vec![]
    }

    /// Starts the export the dialog asks for, to the file chosen.
    pub(super) fn start_export_chat(&mut self, path: PathBuf) -> Vec<Effect> {
        let open = self
            .state
            .chat_export
            .as_ref()
            .map(|dialog| dialog.room.as_str());
        if self.state.exporting.is_some()
            || !open.is_some_and(|room| self.state.may_export_chat(room))
        {
            return vec![];
        }
        let Some(dialog) = &mut self.state.chat_export else {
            return vec![];
        };
        let export = match dialog.export(path) {
            Ok(export) => export,
            Err(error) => {
                dialog.error = Some(error.to_owned());
                return vec![];
            }
        };
        dialog.error = None;
        dialog.ended = None;
        self.state.exporting = Some(Exporting {
            room: export.room.clone(),
            name: export.title.clone(),
            messages: 0,
            stopping: false,
        });
        vec![Effect::Backend(Command::Discord(
            DiscordCommand::ExportChat(Box::new(export)),
        ))]
    }

    pub(super) fn stop_export_chat(&mut self) -> Vec<Effect> {
        let Some(going) = self
            .state
            .exporting
            .as_mut()
            .filter(|going| !going.stopping)
        else {
            return vec![];
        };
        going.stopping = true;
        vec![Effect::Backend(Command::Discord(
            DiscordCommand::StopExport {
                room: going.room.clone(),
            },
        ))]
    }

    pub(super) fn chat_exported(&mut self, room: String, messages: u32, end: Option<ExportEnd>) {
        let Some(going) = self
            .state
            .exporting
            .as_mut()
            .filter(|going| going.room == room)
        else {
            return;
        };
        going.messages = going.messages.max(messages);
        let Some(end) = end else {
            return;
        };
        let going = self
            .state
            .exporting
            .take()
            .unwrap_or_else(|| unreachable!());
        let count = if messages > 0 {
            messages
        } else {
            going.messages
        };
        let shown = self
            .state
            .chat_export
            .as_mut()
            .filter(|dialog| dialog.room == room);
        match shown {
            Some(dialog) => {
                dialog.written = count;
                dialog.ended = Some(end);
            }
            // With its dialog closed, the notice tells how it ended.
            None => {
                self.state.notice = Some(match end {
                    ExportEnd::Saved(path) => format!(
                        "Exported {count} messages from {} to {}.",
                        going.name,
                        path.display()
                    ),
                    ExportEnd::Stopped => format!("Export of {} stopped.", going.name),
                    ExportEnd::Failed(error) => {
                        format!("Couldn't export {}: {error}", going.name)
                    }
                });
            }
        }
    }
}
