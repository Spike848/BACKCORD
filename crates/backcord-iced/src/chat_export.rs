//! Export Chat's dialog: the format, how much of the history (all of it,
//! the newest so many, or from a day on) and whether files go in as links;
//! Export asks where to save the file, in the Downloads folder with the
//! place's name and the day, then shows how many messages came so far,
//! with Stop. What it shows is the core's (`AppState::chat_export`,
//! `AppState::exporting`); closing it leaves the export going, and the
//! notice tells how it ended.

use std::path::PathBuf;

use backcord_core::{
    application::{Action, ExportDialog, ExportSpan},
    chat_export::{ExportEnd, ExportFormat},
};
use backcord_iced_kit::i18n::{t, tr};
use iced::{
    Element, Length, Task,
    widget::{button, container, row, text},
};

use crate::{
    app::Message,
    controls::{self, Kind, action},
    ctx::{Ctx, Cx, Focus, Request},
    dialog::buttons,
    form,
};

/// The dialog's width, and its choices' inside its padding.
const WIDTH: f32 = 460.0;
const CHOICES_WIDTH: f32 = WIDTH - 40.0;
/// The fields' ids.
const COUNT_FIELD: &str = "export-count";
const SINCE_FIELD: &str = "export-since";

#[derive(Clone, Debug)]
pub enum Event {
    Format(ExportFormat),
    Span(ExportSpan),
    Count(String),
    Since(String),
    ToggleAttachments,
    /// Export: the system's dialog asks where, then the file chosen, or
    /// none.
    Export,
    Picked(Option<PathBuf>),
    Stop,
    Close,
    /// The header's button: opens the dialog for this room.
    Open(String),
}

fn message(event: Event) -> Message {
    Message::ChatExport(event)
}

pub fn update(event: Event, cx: &mut Cx<'_>) -> Task<Message> {
    let action = match event {
        Event::Format(format) => Action::ExportFormat(format),
        Event::Span(span) => Action::ExportSpan(span),
        Event::Count(count) => Action::ExportCount(count),
        Event::Since(since) => Action::ExportSince(since),
        Event::ToggleAttachments => Action::ToggleExportAttachments,
        Event::Export => {
            let Some(dialog) = cx.state().chat_export.as_ref() else {
                return Task::none();
            };
            if cx.state().exporting.is_some() {
                return Task::none();
            }
            return save_file(dialog);
        }
        Event::Picked(None) => return Task::none(),
        Event::Picked(Some(path)) => Action::StartExportChat(path),
        Event::Stop => Action::StopExportChat,
        Event::Open(room) => Action::AskExportChat(room),
        Event::Close => {
            cx.dispatch(Action::CloseExportChat);
            cx.request(Request::Focus(Focus::Default));
            return Task::none();
        }
    };
    cx.dispatch(action);
    Task::none()
}

/// Asks where to save the file: in Downloads, named for the place and the
/// day.
fn save_file(dialog: &ExportDialog) -> Task<Message> {
    let name = dialog.file_name(chrono::Local::now().date_naive());
    let folder = backcord_core::chat_export::downloads();
    Task::perform(
        async move {
            let mut asked = rfd::AsyncFileDialog::new()
                .set_title(tr("Export chat"))
                .set_file_name(name);
            if let Some(folder) = folder {
                asked = asked.set_directory(folder);
            }
            asked.save_file().await.map(|file| file.path().to_owned())
        },
        |path| message(Event::Picked(path)),
    )
}

/// The dialog over the dimmed window, while it's open.
pub fn view(cx: Ctx<'_>) -> Option<Element<'_, Message>> {
    let dialog = cx.state.chat_export.as_ref()?;
    let theme = cx.look.theme;
    let mut body = vec![
        cx.look
            .dialog_head(tr("Export chat"), message(Event::Close)),
        form::note(theme, dialog.name.as_str()),
    ];
    let going = cx.state.exporting.as_ref();
    if let Some(going) = going.filter(|going| going.room == dialog.room) {
        body.push(form::note_in(
            t(format!("{} messages downloaded…", going.messages)),
            theme.text,
        ));
        body.push(form::note(
            theme,
            tr("You can close this window: the export goes on."),
        ));
        let stop = if going.stopping {
            tr("Stopping…")
        } else {
            tr("Stop")
        };
        body.push(buttons([
            action(theme, tr("Close"), Kind::Ghost, Some(message(Event::Close))).into(),
            action(
                theme,
                stop,
                Kind::Danger,
                (!going.stopping).then(|| message(Event::Stop)),
            )
            .into(),
        ]));
        return Some(cx.look.framed(body, WIDTH, message(Event::Close)));
    }
    let formats = [tr("Text (TXT)"), tr("Web page (HTML)"), "JSON"];
    let format = ExportFormat::ALL
        .iter()
        .position(|each| *each == dialog.format)
        .unwrap_or(0);
    let spans = [tr("Whole chat"), tr("Last messages"), tr("From a date")];
    const SPANS: [ExportSpan; 3] = [ExportSpan::All, ExportSpan::Last, ExportSpan::Since];
    let span = SPANS
        .iter()
        .position(|each| *each == dialog.span)
        .unwrap_or(0);
    body.extend([
        form::heading(theme, tr("FORMAT")),
        controls::choices_within(theme, &formats, format, CHOICES_WIDTH, |index| {
            message(Event::Format(ExportFormat::ALL[index]))
        }),
        form::heading(theme, tr("MESSAGES")),
        controls::choices_within(theme, &spans, span, CHOICES_WIDTH, |index| {
            message(Event::Span(SPANS[index]))
        }),
    ]);
    match dialog.span {
        ExportSpan::All => {}
        ExportSpan::Last => body.push(
            form::field(theme, COUNT_FIELD, tr("How many messages"), &dialog.count)
                .padding([8.0, 10.0])
                .on_input(|count| message(Event::Count(count)))
                .on_submit(message(Event::Export))
                .into(),
        ),
        ExportSpan::Since => body.push(
            form::field(theme, SINCE_FIELD, tr("dd/mm/yyyy"), &dialog.since)
                .padding([8.0, 10.0])
                .on_input(|since| message(Event::Since(since)))
                .on_submit(message(Event::Export))
                .into(),
        ),
    }
    body.push(
        button(
            row![
                container(cx.look.checkbox(dialog.attachments, false)).padding(iced::Padding {
                    top: 1.0,
                    ..iced::Padding::ZERO
                }),
                text(tr("Include attachments as links"))
                    .size(14)
                    .color(theme.secondary)
                    .width(Length::Fill)
            ]
            .spacing(10),
        )
        .padding(0)
        .width(Length::Fill)
        .on_press(message(Event::ToggleAttachments))
        .style(|_, _| button::Style::default())
        .into(),
    );
    let ended = match &dialog.ended {
        Some(ExportEnd::Saved(path)) => Some((
            t(format!(
                "Saved {} messages to {}",
                dialog.written,
                path.display()
            )),
            theme.text,
        )),
        Some(ExportEnd::Stopped) => Some((
            tr("Export stopped: nothing was saved.").to_owned(),
            theme.secondary,
        )),
        Some(ExportEnd::Failed(error)) => Some((t(error), theme.danger)),
        None => None,
    };
    if let Some((words, ink)) = ended {
        body.push(form::note_in(words, ink));
    }
    if let Some(error) = &dialog.error {
        body.push(form::note_in(t(error), theme.danger));
    }
    let busy = going.is_some();
    if busy {
        body.push(form::note(theme, tr("Another chat is being exported.")));
    }
    body.push(buttons([
        action(
            theme,
            tr("Cancel"),
            Kind::Ghost,
            Some(message(Event::Close)),
        )
        .into(),
        action(
            theme,
            tr("Export"),
            Kind::Primary,
            (!busy).then(|| message(Event::Export)),
        )
        .into(),
    ]));
    Some(cx.look.framed(body, WIDTH, message(Event::Close)))
}
