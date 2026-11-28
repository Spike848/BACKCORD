//! A Discord server's Events dialog, which its "N Events" row opens, as
//! Discord's client has it: the events that haven't ended, each with its
//! Interested button, who's interested and its link; and, for those who may,
//! making an event, changing it, starting or ending it, cancelling it and
//! deleting it. The events themselves are the server's
//! ([`ServerEvent`]s, from the snapshot), so the gateway's changes show at
//! once; the dialog keeps only its page, the form and what's on its way.

use super::*;
use crate::chat::{
    EventDraft, EventPerson, EventPreview, EventsAnswer, EventsCommand, Permission, ServerEvent,
};

/// The longest an event's name, location and description may be, as
/// Discord takes them.
pub const EVENT_NAME_LIMIT: usize = 100;
pub const EVENT_LOCATION_LIMIT: usize = 100;
pub const EVENT_DESCRIPTION_LIMIT: usize = 1000;
/// Where an event happens, as `entity_type` says.
pub const EVENT_STAGE: u8 = 1;
pub const EVENT_VOICE: u8 = 2;
pub const EVENT_EXTERNAL: u8 = 3;

/// The Events dialog, while it's open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventsDialog {
    /// The server, by its id.
    pub server: String,
    pub page: EventsPage,
    /// The list is being asked of Discord, for the counts.
    pub loading: bool,
    /// A change is on its way.
    pub busy: bool,
    /// The event whose Interested button was pressed, until Discord answers.
    pub marking: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventsPage {
    List,
    /// Who's interested in an event, once Discord answered.
    Users {
        event: String,
        people: Option<Vec<EventPerson>>,
    },
    Form(Box<EventForm>),
    /// Asking before deleting an event.
    Delete(String),
}

/// The event form, as Discord's has it, with its times as written: a date
/// `YYYY-MM-DD` and a time `HH:MM`, on this computer's clock.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventForm {
    /// The event changed, none for a new one.
    pub editing: Option<String>,
    pub kind: u8,
    /// The voice or stage channel, by its room id.
    pub room: Option<String>,
    pub location: String,
    pub name: String,
    pub description: String,
    pub start_date: String,
    pub start_time: String,
    pub end_date: String,
    pub end_time: String,
    /// A picture file chosen as its cover; whether it has one already.
    pub cover: Option<String>,
    pub has_cover: bool,
}

/// What's done in the Events dialog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventsAction {
    /// Open it for a server, by its id.
    Open(String),
    Close,
    /// Back to the list, from the form, the people or a question.
    Back,
    ToggleInterest(String),
    Users(String),
    New,
    Edit(String),
    Field(EventField),
    Save,
    /// Start a scheduled event now, end an active one, cancel one.
    Start(String),
    End(String),
    Cancel(String),
    AskDelete(String),
    Delete,
}

/// A change to one of the form's fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventField {
    Kind(u8),
    Room(String),
    Location(String),
    Name(String),
    Description(String),
    StartDate(String),
    StartTime(String),
    EndDate(String),
    EndTime(String),
    Cover(Option<String>),
}

impl EventsDialog {
    /// The server's Discord id.
    pub fn guild(&self) -> &str {
        self.server.strip_prefix("discord:").unwrap_or(&self.server)
    }
}

impl Application {
    /// A server's events that haven't ended, soonest first.
    pub fn server_events(&self, server: &str) -> &[ServerEvent] {
        self.state
            .servers
            .iter()
            .find(|known| known.id == server)
            .map_or(&[], |known| known.events.as_slice())
    }

    /// Whether this account may make events in a server.
    pub fn may_create_events(&self, server: &str) -> bool {
        self.may(server, Permission::CreateEvents)
    }

    /// Whether this account may change, start, end, cancel and delete an
    /// event: anyone's with Manage Events, its own with Create Events.
    pub fn may_manage_event(&self, server: &str, event: &ServerEvent) -> bool {
        self.may(server, Permission::ManageEvents)
            || (self.may(server, Permission::CreateEvents)
                && event.creator_id.is_some()
                && event.creator_id == self.state.discord_user)
    }

    pub(super) fn events_action(&mut self, action: EventsAction) -> Vec<Effect> {
        if let EventsAction::Open(server) = action {
            return self.open_events(server);
        }
        if action == EventsAction::Close {
            self.state.events_dialog = None;
            return vec![];
        }
        let Some(dialog) = self.state.events_dialog.as_ref() else {
            return vec![];
        };
        let (server, guild) = (dialog.server.clone(), dialog.guild().to_owned());
        let found = |app: &Self, id: &str| {
            app.server_events(&server)
                .iter()
                .find(|event| event.id == id)
                .cloned()
        };
        let command = match action {
            EventsAction::Open(_) | EventsAction::Close => return vec![],
            EventsAction::Back => {
                self.edit_events(|dialog| {
                    dialog.page = EventsPage::List;
                    dialog.error = None;
                });
                return vec![];
            }
            EventsAction::ToggleInterest(id) => {
                let Some(event) = found(self, &id) else {
                    return vec![];
                };
                if dialog.marking.is_some() || self.state.preview {
                    return vec![];
                }
                self.edit_events(|dialog| dialog.marking = Some(id.clone()));
                return vec![Effect::Backend(Command::Discord(
                    DiscordCommand::SetEventInterest {
                        interested: !event.mine,
                        preview: Box::new(preview_of(&guild, &event)),
                    },
                ))];
            }
            EventsAction::Users(event) => {
                self.edit_events(|dialog| {
                    dialog.page = EventsPage::Users {
                        event: event.clone(),
                        people: None,
                    };
                    dialog.error = None;
                });
                EventsCommand::Users { guild, event }
            }
            EventsAction::New => {
                if !self.may_create_events(&server) {
                    return vec![];
                }
                let form = new_form(chrono::Utc::now().timestamp());
                self.edit_events(|dialog| {
                    dialog.page = EventsPage::Form(Box::new(form));
                    dialog.error = None;
                });
                return vec![];
            }
            EventsAction::Edit(id) => {
                let Some(event) =
                    found(self, &id).filter(|event| self.may_manage_event(&server, event))
                else {
                    return vec![];
                };
                self.edit_events(|dialog| {
                    dialog.page = EventsPage::Form(Box::new(form_of(&event)));
                    dialog.error = None;
                });
                return vec![];
            }
            EventsAction::Field(field) => {
                self.edit_events(|dialog| {
                    if let EventsPage::Form(form) = &mut dialog.page {
                        form.set(field);
                        dialog.error = None;
                    }
                });
                return vec![];
            }
            EventsAction::Save => {
                let EventsPage::Form(form) = &dialog.page else {
                    return vec![];
                };
                if dialog.busy {
                    return vec![];
                }
                let draft = match form.draft(chrono::Utc::now().timestamp()) {
                    Ok(draft) => draft,
                    Err(error) => {
                        self.edit_events(|dialog| dialog.error = Some(error.to_owned()));
                        return vec![];
                    }
                };
                let event = form.editing.clone();
                if self.state.preview {
                    return vec![];
                }
                self.edit_events(|dialog| {
                    dialog.busy = true;
                    dialog.error = None;
                });
                EventsCommand::Save {
                    guild,
                    event,
                    draft: Box::new(draft),
                }
            }
            EventsAction::Start(id) | EventsAction::End(id) | EventsAction::Cancel(id)
                if found(self, &id).is_none_or(|event| !self.may_manage_event(&server, &event)) =>
            {
                return vec![];
            }
            EventsAction::Start(event) => self.event_status(guild, event, 2),
            EventsAction::End(event) => self.event_status(guild, event, 3),
            EventsAction::Cancel(event) => self.event_status(guild, event, 4),
            EventsAction::AskDelete(id) => {
                if found(self, &id).is_some_and(|event| self.may_manage_event(&server, &event)) {
                    self.edit_events(|dialog| {
                        dialog.page = EventsPage::Delete(id.clone());
                        dialog.error = None;
                    });
                }
                return vec![];
            }
            EventsAction::Delete => {
                let EventsPage::Delete(event) = dialog.page.clone() else {
                    return vec![];
                };
                if dialog.busy {
                    return vec![];
                }
                self.edit_events(|dialog| dialog.busy = true);
                EventsCommand::Delete { guild, event }
            }
        };
        if self.state.preview {
            self.edit_events(|dialog| dialog.busy = false);
            return vec![];
        }
        vec![Effect::Backend(Command::Discord(DiscordCommand::Events(
            command,
        )))]
    }

    fn event_status(&mut self, guild: String, event: String, status: u8) -> EventsCommand {
        self.edit_events(|dialog| {
            dialog.busy = true;
            dialog.error = None;
        });
        EventsCommand::Status {
            guild,
            event,
            status,
        }
    }

    fn edit_events(&mut self, change: impl FnOnce(&mut EventsDialog)) {
        if let Some(dialog) = self.state.events_dialog.as_mut() {
            change(dialog);
        }
    }

    /// Opens the dialog on a server's list, and asks Discord for the
    /// counts and this account's interest.
    fn open_events(&mut self, server: String) -> Vec<Effect> {
        if !self.state.discord_online && !self.state.preview {
            return vec![];
        }
        let dialog = EventsDialog {
            server,
            page: EventsPage::List,
            loading: !self.state.preview,
            busy: false,
            marking: None,
            error: None,
        };
        let guild = dialog.guild().to_owned();
        self.state.events_dialog = Some(dialog);
        if self.state.preview {
            return vec![];
        }
        vec![Effect::Backend(Command::Discord(DiscordCommand::Events(
            EventsCommand::Load { guild },
        )))]
    }

    pub(super) fn events_answered(&mut self, answer: EventsAnswer) -> Vec<Effect> {
        let Some(dialog) = self.state.events_dialog.as_mut() else {
            return vec![];
        };
        match answer {
            EventsAnswer::Loaded { guild, result } if guild == dialog.guild() => {
                dialog.loading = false;
                if let Err(error) = result {
                    dialog.error = Some(error);
                }
            }
            EventsAnswer::Users { event, result } => {
                if let EventsPage::Users {
                    event: shown,
                    people,
                } = &mut dialog.page
                    && *shown == event
                {
                    match result {
                        Ok(found) => *people = Some(found),
                        Err(error) => dialog.error = Some(error),
                    }
                }
            }
            EventsAnswer::Changed { guild, result, .. } if guild == dialog.guild() => {
                dialog.busy = false;
                match result {
                    Ok(()) => {
                        dialog.page = EventsPage::List;
                        dialog.error = None;
                    }
                    Err(error) => dialog.error = Some(error),
                }
            }
            EventsAnswer::Loaded { .. } | EventsAnswer::Changed { .. } => {}
        }
        vec![]
    }

    /// Discord answered an Interested button the dialog pressed; the
    /// server's events learn the change from the gateway.
    pub(super) fn events_interest_answered(&mut self, event: &str, result: &Result<(), String>) {
        self.edit_events(|dialog| {
            if dialog.marking.as_deref() == Some(event) {
                dialog.marking = None;
                if let Err(error) = result {
                    dialog.error = Some(error.clone());
                }
            }
        });
    }
}

/// The event as its link's dialog would show it, for the Interested
/// button's command.
fn preview_of(guild: &str, event: &ServerEvent) -> EventPreview {
    EventPreview {
        event: event.id.clone(),
        guild: guild.to_owned(),
        name: event.name.clone(),
        description: event.description.clone(),
        starts_at: u64::try_from(event.start).unwrap_or_default(),
        ends_at: event.end.and_then(|end| u64::try_from(end).ok()),
        room: event.room.clone(),
        location: event.location.clone(),
        creator: event.creator.clone(),
        interested_count: Some(event.interested),
        active: event.active,
        over: false,
        interested: event.mine,
    }
}

/// A new event's form: in a voice channel, starting at the next full hour.
fn new_form(now: i64) -> EventForm {
    let start = (now / 3600 + 1) * 3600;
    let (start_date, start_time) = local_date_time(start);
    EventForm {
        kind: EVENT_VOICE,
        start_date,
        start_time,
        ..EventForm::default()
    }
}

/// The form for changing an event, filled with what it is.
fn form_of(event: &ServerEvent) -> EventForm {
    let (start_date, start_time) = local_date_time(event.start);
    let (end_date, end_time) = event.end.map(local_date_time).unwrap_or_default();
    EventForm {
        editing: Some(event.id.clone()),
        kind: event.kind,
        room: event.room.clone(),
        location: event.location.clone().unwrap_or_default(),
        name: event.name.clone(),
        description: event.description.clone().unwrap_or_default(),
        start_date,
        start_time,
        end_date,
        end_time,
        cover: None,
        has_cover: event.cover.is_some(),
    }
}

impl EventForm {
    fn set(&mut self, field: EventField) {
        let cut = |text: String, limit: usize| text.chars().take(limit).collect::<String>();
        match field {
            EventField::Kind(kind) => self.kind = kind,
            EventField::Room(room) => self.room = Some(room),
            EventField::Location(location) => self.location = cut(location, EVENT_LOCATION_LIMIT),
            EventField::Name(name) => self.name = cut(name, EVENT_NAME_LIMIT),
            EventField::Description(text) => self.description = cut(text, EVENT_DESCRIPTION_LIMIT),
            EventField::StartDate(date) => self.start_date = date,
            EventField::StartTime(time) => self.start_time = time,
            EventField::EndDate(date) => self.end_date = date,
            EventField::EndTime(time) => self.end_time = time,
            EventField::Cover(cover) => self.cover = cover,
        }
    }

    /// What the form makes, or what's missing in Discord's words: a
    /// channel for an event in one, a location and an end for one
    /// elsewhere, a topic, and a start in the future (`now` in Unix
    /// seconds) for a new event, with an end after it.
    pub fn draft(&self, now: i64) -> Result<EventDraft, &'static str> {
        let external = self.kind == EVENT_EXTERNAL;
        let channel = if external {
            None
        } else {
            let room = self
                .room
                .as_deref()
                .ok_or("Choose a channel for the event.")?;
            Some(room.rsplit(':').next().unwrap_or(room).to_owned())
        };
        if external && self.location.trim().is_empty() {
            return Err("Enter a location for the event.");
        }
        if self.name.trim().is_empty() {
            return Err("Give the event a topic.");
        }
        let start = local_seconds(&self.start_date, &self.start_time)
            .ok_or("Write the start date as YYYY-MM-DD and the time as HH:MM.")?;
        if self.editing.is_none() && start <= now {
            return Err("The start time must be in the future.");
        }
        let end = if self.end_date.trim().is_empty() && self.end_time.trim().is_empty() {
            None
        } else {
            Some(
                local_seconds(&self.end_date, &self.end_time)
                    .ok_or("Write the end date as YYYY-MM-DD and the time as HH:MM.")?,
            )
        };
        if external && end.is_none() {
            return Err("An event somewhere else needs an end time.");
        }
        if end.is_some_and(|end| end <= start) {
            return Err("The end time must be after the start time.");
        }
        Ok(EventDraft {
            kind: self.kind,
            channel,
            location: external.then(|| self.location.trim().to_owned()),
            name: self.name.trim().to_owned(),
            description: self.description.trim().to_owned(),
            start,
            end,
            cover: self.cover.clone(),
        })
    }
}

/// Unix seconds as a date and a time on this computer's clock.
pub fn local_date_time(seconds: i64) -> (String, String) {
    use chrono::TimeZone;
    match chrono::Local.timestamp_opt(seconds, 0).single() {
        Some(time) => (
            time.format("%Y-%m-%d").to_string(),
            time.format("%H:%M").to_string(),
        ),
        None => (String::new(), String::new()),
    }
}

/// A date and a time on this computer's clock in Unix seconds.
fn local_seconds(date: &str, time: &str) -> Option<i64> {
    use chrono::TimeZone;
    let date = chrono::NaiveDate::parse_from_str(date.trim(), "%Y-%m-%d").ok()?;
    let time = chrono::NaiveTime::parse_from_str(time.trim(), "%H:%M").ok()?;
    chrono::Local
        .from_local_datetime(&date.and_time(time))
        .earliest()
        .map(|time| time.timestamp())
}
