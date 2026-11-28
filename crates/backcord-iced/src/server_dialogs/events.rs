//! A server's Events dialog (`state.events_dialog`), as Discord's has it:
//! each event that hasn't ended as a card, its cover over when it is, its
//! name, what it says, where it is, who made it and how many are
//! interested, with Interested, Share and, for those who may, Edit, Start or
//! End, Cancel and Delete; who's interested; and the form that makes or
//! changes one. Everything it shows and holds is the core's.

use backcord_core::{
    application::{
        AppState, EVENT_EXTERNAL, EVENT_STAGE, EVENT_VOICE, EventField, EventForm, EventsAction,
        EventsDialog, EventsPage,
    },
    chat::ServerEvent,
};
use backcord_iced_kit::{
    i18n::{t, tr},
    widgets::scroll_view,
};
use iced::{
    Alignment, Element, Length,
    widget::{column, container, row, text},
};

use super::{Event, View, message};
use crate::{
    app::Message,
    controls::{self, Kind, action},
    dialog::buttons,
    fonts::BOLD,
    form,
    style::rounded,
};

/// The dialog's width, and how tall its list gets before it scrolls.
const WIDTH: f32 = 520.0;
const LIST_HEIGHT: f32 = 460.0;
/// A cover's size on a card: Discord's 2.5 by 1, the card's width.
const COVER_WIDTH: f32 = WIDTH - 72.0;
const COVER_HEIGHT: f32 = COVER_WIDTH / 2.5;
/// Someone's avatar in Who's interested.
const AVATAR: f32 = 32.0;
/// The form's fields' ids.
const NAME_FIELD: &str = "event-name";

/// The pictures the dialog shows: the covers on its list, and the avatars
/// of those interested.
pub(super) fn pictures(state: &AppState) -> Vec<String> {
    let Some(dialog) = &state.events_dialog else {
        return Vec::new();
    };
    match &dialog.page {
        EventsPage::List => events_of(state, dialog)
            .iter()
            .filter_map(|event| event.cover.clone())
            .collect(),
        EventsPage::Users {
            people: Some(people),
            ..
        } => people.iter().map(|person| person.avatar.clone()).collect(),
        _ => Vec::new(),
    }
}

fn events_of<'a>(state: &'a AppState, dialog: &EventsDialog) -> &'a [ServerEvent] {
    state
        .servers
        .iter()
        .find(|server| server.id == dialog.server)
        .map_or(&[], |server| server.events.as_slice())
}

fn act(action: EventsAction) -> Message {
    message(Event::Events(action))
}

impl<'a> View<'a> {
    pub(super) fn events_view(self) -> Option<Element<'a, Message>> {
        let dialog = self.cx.state.events_dialog.as_ref()?;
        let theme = self.cx.look.theme;
        let close = act(EventsAction::Close);
        let mut body = match &dialog.page {
            EventsPage::List => self.events_list(dialog),
            EventsPage::Users { event, people } => {
                self.event_users(dialog, event, people.as_deref())
            }
            EventsPage::Form(form) => self.event_form(dialog, form),
            EventsPage::Delete(event) => {
                let name = events_of(self.cx.state, dialog)
                    .iter()
                    .find(|known| &known.id == event)
                    .map_or_else(String::new, |known| known.name.clone());
                vec![
                    self.cx.look.dialog_head(tr("Delete Event"), close.clone()),
                    form::note(
                        theme,
                        t(format!(
                            "Are you sure you want to delete {name}? This can't be undone."
                        )),
                    ),
                ]
            }
        };
        if let Some(error) = &dialog.error {
            body.push(form::note_in(t(error), theme.danger));
        }
        let foot: Vec<Element<'a, Message>> = match &dialog.page {
            EventsPage::List => vec![],
            EventsPage::Users { .. } => {
                vec![
                    action(
                        theme,
                        tr("Back"),
                        Kind::Ghost,
                        Some(act(EventsAction::Back)),
                    )
                    .into(),
                ]
            }
            EventsPage::Form(form) => vec![
                action(
                    theme,
                    tr("Back"),
                    Kind::Ghost,
                    Some(act(EventsAction::Back)),
                )
                .into(),
                action(
                    theme,
                    if form.editing.is_some() {
                        tr("Save Event")
                    } else {
                        tr("Create Event")
                    },
                    Kind::Primary,
                    (!dialog.busy).then(|| act(EventsAction::Save)),
                )
                .into(),
            ],
            EventsPage::Delete(_) => vec![
                action(
                    theme,
                    tr("Cancel"),
                    Kind::Ghost,
                    Some(act(EventsAction::Back)),
                )
                .into(),
                action(
                    theme,
                    tr("Delete Event"),
                    Kind::Danger,
                    (!dialog.busy).then(|| act(EventsAction::Delete)),
                )
                .into(),
            ],
        };
        if !foot.is_empty() {
            body.push(buttons(foot));
        }
        Some(self.cx.look.framed(body, WIDTH, close))
    }

    /// The events, soonest first, with Create Event for those who may.
    fn events_list(self, dialog: &'a EventsDialog) -> Vec<Element<'a, Message>> {
        let theme = self.cx.look.theme;
        let events = events_of(self.cx.state, dialog);
        let title = match events.len() {
            0 => tr("Events").to_owned(),
            1 => tr("1 Event").to_owned(),
            count => t(format!("{count} Events")),
        };
        let mut body = vec![self.cx.look.dialog_head(title, act(EventsAction::Close))];
        if self.cx.app.may_create_events(&dialog.server) {
            body.push(
                row![
                    container(form::note::<Message>(
                        theme,
                        tr("Plan something for the server: a game night, a stage talk, a meetup.")
                    ))
                    .width(Length::Fill),
                    action(
                        theme,
                        tr("Create Event"),
                        Kind::Primary,
                        Some(act(EventsAction::New))
                    ),
                ]
                .spacing(12)
                .align_y(Alignment::Center)
                .into(),
            );
        }
        if events.is_empty() {
            body.push(form::note(
                theme,
                if dialog.loading {
                    tr("Loading...")
                } else {
                    tr("No upcoming events")
                },
            ));
            return body;
        }
        let mut cards = column![].spacing(12);
        for event in events {
            cards = cards.push(self.event_card(dialog, event));
        }
        body.push(
            container(
                scroll_view(cards.padding(iced::Padding {
                    right: 10.0,
                    ..iced::Padding::ZERO
                }))
                .width(Length::Fill)
                .style(backcord_iced_kit::style::scrollbar(theme)),
            )
            .max_height(LIST_HEIGHT)
            .into(),
        );
        body
    }

    /// One event's card.
    fn event_card(self, dialog: &'a EventsDialog, event: &'a ServerEvent) -> Element<'a, Message> {
        let theme = self.cx.look.theme;
        let mut card = column![].spacing(6);
        if let Some(handle) = event.cover.as_deref().and_then(|cover| {
            self.cx
                .picture(cover, COVER_WIDTH as u32, COVER_HEIGHT as u32, true)
        }) {
            card = card.push(
                iced::widget::image(handle)
                    .width(COVER_WIDTH)
                    .height(COVER_HEIGHT),
            );
        }
        let twelve = self.cx.layout.prefs.twelve_hour_clock;
        let when = if event.active {
            tr("Happening now").to_owned()
        } else {
            t(backcord_core::clock::event(
                u64::try_from(event.start).unwrap_or_default(),
                event.end.and_then(|end| u64::try_from(end).ok()),
                backcord_core::clock::now_ms() / 1000,
                twelve,
            ))
        };
        card = card.push(text(when).size(13).font(BOLD).color(if event.active {
            theme.live
        } else {
            theme.muted
        }));
        card = card.push(
            text(event.name.as_str())
                .size(17)
                .font(BOLD)
                .color(theme.text)
                .width(Length::Fill),
        );
        if let Some(description) = &event.description {
            card = card.push(
                text(description.as_str())
                    .size(14)
                    .color(theme.secondary)
                    .width(Length::Fill),
            );
        }
        let place = match (&event.location, &event.room) {
            (Some(location), _) => location.clone(),
            (None, Some(room)) => self
                .cx
                .state
                .rooms
                .iter()
                .find(|known| &known.id == room)
                .map_or_else(String::new, |known| known.name.clone()),
            (None, None) => String::new(),
        };
        let mut details = Vec::new();
        if !place.is_empty() {
            details.push(place);
        }
        if let Some(creator) = &event.creator {
            details.push(t(format!("by {creator}")));
        }
        if !details.is_empty() {
            card = card.push(form::note(theme, details.join(" · ")));
        }
        let marking = dialog.marking.as_deref() == Some(event.id.as_str());
        let guild = dialog.guild();
        let link = format!("https://discord.com/events/{guild}/{}", event.id);
        let mut acts = row![
            action(
                theme,
                if event.mine {
                    tr("Interested ✓")
                } else {
                    tr("Interested")
                },
                if event.mine {
                    Kind::Primary
                } else {
                    Kind::Plain
                },
                (!marking).then(|| act(EventsAction::ToggleInterest(event.id.clone()))),
            ),
            action(
                theme,
                t(format!("{} interested", event.interested)),
                Kind::Ghost,
                Some(act(EventsAction::Users(event.id.clone()))),
            ),
            action(
                theme,
                tr("Share"),
                Kind::Ghost,
                Some(message(Event::CopyEventLink(link)))
            ),
        ]
        .spacing(6)
        .align_y(Alignment::Center);
        if self.cx.app.may_manage_event(&dialog.server, event) {
            let busy = dialog.busy;
            let id = || event.id.clone();
            acts = acts.push(action(
                theme,
                tr("Edit"),
                Kind::Ghost,
                (!busy).then(|| act(EventsAction::Edit(id()))),
            ));
            if event.active {
                acts = acts.push(action(
                    theme,
                    tr("End Event"),
                    Kind::Danger,
                    (!busy).then(|| act(EventsAction::End(id()))),
                ));
            } else {
                acts = acts.push(action(
                    theme,
                    tr("Start Event"),
                    Kind::Plain,
                    (!busy).then(|| act(EventsAction::Start(id()))),
                ));
                acts = acts.push(action(
                    theme,
                    tr("Cancel Event"),
                    Kind::Danger,
                    (!busy).then(|| act(EventsAction::Cancel(id()))),
                ));
            }
            acts = acts.push(action(
                theme,
                tr("Delete"),
                Kind::Danger,
                (!busy).then(|| act(EventsAction::AskDelete(id()))),
            ));
        }
        card = card.push(acts.wrap());
        let sunken = theme.sunken;
        container(card)
            .padding(14)
            .width(Length::Fill)
            .style(move |_| rounded(sunken, 8))
            .into()
    }

    /// Who's interested in an event.
    fn event_users(
        self,
        dialog: &'a EventsDialog,
        event: &str,
        people: Option<&'a [backcord_core::chat::EventPerson]>,
    ) -> Vec<Element<'a, Message>> {
        let theme = self.cx.look.theme;
        let name = events_of(self.cx.state, dialog)
            .iter()
            .find(|known| known.id == event)
            .map_or("", |known| known.name.as_str());
        let mut body = vec![
            self.cx
                .look
                .dialog_head(t(format!("Interested in {name}")), act(EventsAction::Close)),
        ];
        let Some(people) = people else {
            body.push(form::note(theme, tr("Loading...")));
            return body;
        };
        if people.is_empty() {
            body.push(form::note(theme, tr("No one is interested yet.")));
            return body;
        }
        let mut list = column![].spacing(4);
        for person in people {
            list = list.push(
                row![
                    self.cx.avatar(
                        Some(person.avatar.as_str()),
                        &person.name,
                        AVATAR,
                        AVATAR / 2.0,
                        1.0
                    ),
                    text(person.name.as_str()).size(15).color(theme.text),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            );
        }
        body.push(
            container(scroll_view(list).width(Length::Fill))
                .max_height(LIST_HEIGHT)
                .into(),
        );
        body
    }

    /// Making or changing an event: where, which channel or location, its
    /// topic, when, what it says, and its cover.
    fn event_form(
        self,
        dialog: &'a EventsDialog,
        form_: &'a EventForm,
    ) -> Vec<Element<'a, Message>> {
        let theme = self.cx.look.theme;
        let field = |change: fn(String) -> EventField| {
            move |typed: String| act(EventsAction::Field(change(typed)))
        };
        let title = if form_.editing.is_some() {
            tr("Edit Event")
        } else {
            tr("Create Event")
        };
        let kinds = [
            tr("Stage Channel"),
            tr("Voice Channel"),
            tr("Somewhere Else"),
        ];
        let chosen = match form_.kind {
            EVENT_STAGE => 0,
            EVENT_EXTERNAL => 2,
            _ => 1,
        };
        let mut body = vec![
            self.cx.look.dialog_head(title, act(EventsAction::Close)),
            form::heading(theme, tr("WHERE IS YOUR EVENT?")),
            controls::choices_within(theme, &kinds, chosen, WIDTH - 40.0, |index| {
                act(EventsAction::Field(EventField::Kind(
                    [EVENT_STAGE, EVENT_VOICE, EVENT_EXTERNAL][index],
                )))
            }),
        ];
        if form_.kind == EVENT_EXTERNAL {
            body.extend([
                form::heading(theme, tr("LOCATION")),
                form::field(
                    theme,
                    "event-location",
                    tr("Add a location, link, or something."),
                    &form_.location,
                )
                .padding([8.0, 10.0])
                .on_input(field(EventField::Location))
                .into(),
            ]);
        } else {
            body.push(form::heading(theme, tr("CHANNEL")));
            let channels: Vec<_> = self
                .cx
                .state
                .servers
                .iter()
                .find(|server| server.id == dialog.server)
                .into_iter()
                .flat_map(|server| &server.channels)
                .filter_map(|room| self.cx.state.rooms.iter().find(|known| &known.id == room))
                .filter(|room| room.voice)
                .collect();
            if channels.is_empty() {
                body.push(form::note(theme, tr("This server has no voice channels.")));
            }
            let mut picks = row![].spacing(6);
            for room in channels {
                let picked = form_.room.as_ref() == Some(&room.id);
                picks = picks.push(action(
                    theme,
                    room.name.as_str(),
                    if picked { Kind::Primary } else { Kind::Plain },
                    Some(act(EventsAction::Field(EventField::Room(room.id.clone())))),
                ));
            }
            body.push(picks.wrap().into());
        }
        body.extend([
            form::heading(theme, tr("EVENT TOPIC")),
            form::field(theme, NAME_FIELD, tr("What's your event?"), &form_.name)
                .padding([8.0, 10.0])
                .on_input(field(EventField::Name))
                .into(),
            form::heading(theme, tr("START DATE AND TIME")),
            row![
                form::field(theme, "event-start-date", "YYYY-MM-DD", &form_.start_date)
                    .padding([8.0, 10.0])
                    .on_input(field(EventField::StartDate)),
                form::field(theme, "event-start-time", "HH:MM", &form_.start_time)
                    .padding([8.0, 10.0])
                    .on_input(field(EventField::StartTime)),
            ]
            .spacing(8)
            .into(),
            form::heading(
                theme,
                if form_.kind == EVENT_EXTERNAL {
                    tr("END DATE AND TIME")
                } else {
                    tr("END DATE AND TIME (OPTIONAL)")
                },
            ),
            row![
                form::field(theme, "event-end-date", "YYYY-MM-DD", &form_.end_date)
                    .padding([8.0, 10.0])
                    .on_input(field(EventField::EndDate)),
                form::field(theme, "event-end-time", "HH:MM", &form_.end_time)
                    .padding([8.0, 10.0])
                    .on_input(field(EventField::EndTime)),
            ]
            .spacing(8)
            .into(),
            form::heading(theme, tr("DESCRIPTION")),
            form::field(
                theme,
                "event-description",
                tr("Tell people a little more about your event."),
                &form_.description,
            )
            .padding([8.0, 10.0])
            .on_input(field(EventField::Description))
            .into(),
            form::heading(theme, tr("COVER IMAGE")),
        ]);
        let cover = match &form_.cover {
            Some(path) => std::path::Path::new(path)
                .file_name()
                .map_or_else(String::new, |name| name.to_string_lossy().into_owned()),
            None if form_.has_cover => tr("The event keeps its cover.").to_owned(),
            None => tr("No cover image").to_owned(),
        };
        body.push(
            row![
                container(form::note::<Message>(theme, cover)).width(Length::Fill),
                action(
                    theme,
                    tr("Upload Image"),
                    Kind::Plain,
                    Some(message(Event::PickCover))
                ),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into(),
        );
        body
    }
}
