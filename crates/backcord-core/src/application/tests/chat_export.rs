//! Export Chat: its dialog, what it starts, how it counts and ends.

use super::*;
use crate::chat_export::{ChatExport, ExportEnd, ExportFormat, ExportRange};

const GENERAL: &str = "discord:10:11";

/// The export the effects start, if they start one.
fn started(effects: &[Effect]) -> Option<&ChatExport> {
    effects.iter().find_map(|effect| match effect {
        Effect::Backend(Command::Discord(DiscordCommand::ExportChat(export))) => Some(&**export),
        _ => None,
    })
}

#[test]
fn export_chat_asks_how_then_counts_and_stops() {
    let mut app = discord_server();
    assert!(app.state.may_export_chat(GENERAL));
    assert!(!app.state.may_export_chat("discord:10:4"), "not a category");
    assert!(!app.state.may_export_chat("discord:10"), "nor a server");
    assert!(
        app.dispatch(Action::StartExportChat("x.txt".into()))
            .is_empty(),
        "nothing asked yet"
    );

    app.dispatch(Action::AskExportChat(GENERAL.into()));
    let dialog = app.state.chat_export.clone().unwrap();
    assert_eq!(dialog.room, GENERAL);
    assert_eq!(dialog.name, "general");
    assert_eq!(
        (dialog.format, dialog.span, dialog.attachments),
        (ExportFormat::Text, ExportSpan::All, true)
    );
    let day = chrono::NaiveDate::from_ymd_opt(2026, 11, 25).unwrap();
    assert_eq!(dialog.file_name(day), "general-2026-11-25.txt");
    app.dispatch(Action::ExportFormat(ExportFormat::Html));
    assert_eq!(
        app.state.chat_export.as_ref().unwrap().file_name(day),
        "general-2026-11-25.html"
    );

    // What's typed is read as it starts: a number, or a day.
    app.dispatch(Action::ExportSpan(ExportSpan::Last));
    app.dispatch(Action::ExportCount("many".into()));
    assert!(
        app.dispatch(Action::StartExportChat("x.html".into()))
            .is_empty()
    );
    assert!(app.state.chat_export.as_ref().unwrap().error.is_some());
    app.dispatch(Action::ExportCount("250".into()));
    assert!(
        app.state.chat_export.as_ref().unwrap().error.is_none(),
        "typing clears it"
    );
    app.dispatch(Action::ExportSpan(ExportSpan::Since));
    app.dispatch(Action::ExportSince("32/13/2026".into()));
    assert!(
        app.dispatch(Action::StartExportChat("x.html".into()))
            .is_empty()
    );
    app.dispatch(Action::ExportSince("01/11/2026".into()));
    app.dispatch(Action::ToggleExportAttachments);
    let effects = app.dispatch(Action::StartExportChat("x.html".into()));
    let export = started(&effects).expect("it starts");
    assert_eq!(export.room, GENERAL);
    assert_eq!(export.title, "general");
    assert_eq!(export.format, ExportFormat::Html);
    assert!(!export.attachments);
    assert_eq!(export.path, std::path::PathBuf::from("x.html"));
    let first = chrono::NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();
    assert_eq!(
        export.range,
        ExportRange::Since(crate::chat_export::day_start_ms(first))
    );
    assert!(
        started(&app.dispatch(Action::StartExportChat("y.html".into()))).is_none(),
        "one at a time"
    );

    // How many came shows in the dialog; Stop is asked once.
    app.apply(Event::ChatExported {
        room: GENERAL.into(),
        messages: 300,
        end: None,
    });
    assert_eq!(app.state.exporting.as_ref().unwrap().messages, 300);
    assert!(matches!(
        app.dispatch(Action::StopExportChat).as_slice(),
        [Effect::Backend(Command::Discord(DiscordCommand::StopExport { room }))] if room == GENERAL
    ));
    assert!(
        app.dispatch(Action::StopExportChat).is_empty(),
        "asked once"
    );
    app.apply(Event::ChatExported {
        room: GENERAL.into(),
        messages: 0,
        end: Some(ExportEnd::Stopped),
    });
    assert!(app.state.exporting.is_none());
    let dialog = app.state.chat_export.as_ref().unwrap();
    assert_eq!(dialog.ended, Some(ExportEnd::Stopped));

    // The whole chat, saved; the dialog says where and how many.
    app.dispatch(Action::ExportSpan(ExportSpan::All));
    assert!(started(&app.dispatch(Action::StartExportChat("z.html".into()))).is_some());
    app.apply(Event::ChatExported {
        room: GENERAL.into(),
        messages: 120,
        end: Some(ExportEnd::Saved("z.html".into())),
    });
    let dialog = app.state.chat_export.as_ref().unwrap();
    assert_eq!(dialog.written, 120);
    assert_eq!(dialog.ended, Some(ExportEnd::Saved("z.html".into())));

    // Closing the dialog leaves it going; the notice tells how it ended.
    assert!(started(&app.dispatch(Action::StartExportChat("w.html".into()))).is_some());
    app.dispatch(Action::CloseExportChat);
    assert!(app.state.chat_export.is_none());
    assert!(app.state.exporting.is_some());
    app.apply(Event::ChatExported {
        room: GENERAL.into(),
        messages: 0,
        end: Some(ExportEnd::Failed("Discord said no.".into())),
    });
    assert!(app.state.exporting.is_none());
    assert_eq!(
        app.state.notice.as_deref(),
        Some("Couldn't export general: Discord said no.")
    );
}

#[test]
fn the_preview_exports_nothing() {
    let mut app = Application::default();
    app.dispatch(Action::Preview);
    let room = app.state.rooms[0].id.clone();
    assert!(!app.state.may_export_chat(&room));
    app.dispatch(Action::AskExportChat(room));
    assert!(app.state.chat_export.is_none());
}
