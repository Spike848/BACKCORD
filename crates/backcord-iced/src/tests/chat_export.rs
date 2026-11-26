//! Export Chat: offered in a channel's menu, its dialog, what Export
//! starts once the file is chosen, how it counts and stops, and Escape.

use backcord_core::{
    application::ExportSpan,
    chat::{Command, DiscordCommand, Event as CoreEvent},
    chat_export::{ExportEnd, ExportFormat, ExportRange},
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

fn export(app: &mut App, event: crate::chat_export::Event) {
    let _ = app.update(Message::ChatExport(event));
}

#[test]
fn export_chat_asks_how_then_exports_to_the_file_chosen() {
    let (mut app, _folder) = discord_server(0, |_, _| {});
    let _ = crate::place_menus::open_place_menu(&mut app, SERVER, Point::new(80.0, 44.0), false);
    assert!(
        !labels(&app).contains(&"Export chat…".to_owned()),
        "a server has no chat of its own"
    );
    let _ = crate::place_menus::open_place_menu(&mut app, GENERAL, Point::new(80.0, 120.0), false);
    let at = labels(&app)
        .iter()
        .position(|label| label == "Export chat…")
        .expect("a channel's menu offers it");
    let _ = app.update(Message::Menu(crate::menus::Event::Choose(at)));
    assert_eq!(
        app.client
            .state()
            .chat_export
            .as_ref()
            .map(|dialog| dialog.room.as_str()),
        Some(GENERAL)
    );
    assert!(shows(&app, "Export chat"));
    assert!(shows(&app, "Whole chat"));
    assert!(shows(&app, "Include attachments as links"));

    // The choices, as clicked; the number field shows for the newest.
    interact(&mut app, |ui| {
        ui.click("Web page (HTML)").expect("the HTML choice");
        ui.click("Last messages").expect("the newest's choice");
    });
    let dialog = app.client.state().chat_export.clone().unwrap();
    assert_eq!(
        (dialog.format, dialog.span),
        (ExportFormat::Html, ExportSpan::Last)
    );
    export(&mut app, crate::chat_export::Event::Count("25".into()));
    export(&mut app, crate::chat_export::Event::ToggleAttachments);
    assert!(!app.client.state().chat_export.as_ref().unwrap().attachments);

    // Choosing no file starts nothing; a file starts it.
    app.client.sent.clear();
    export(&mut app, crate::chat_export::Event::Picked(None));
    assert!(app.client.sent.is_empty());
    export(
        &mut app,
        crate::chat_export::Event::Picked(Some("general.html".into())),
    );
    let started = app.client.sent.iter().find_map(|command| match command {
        Command::Discord(DiscordCommand::ExportChat(export)) => Some(export.clone()),
        _ => None,
    });
    let started = started.expect("Export Chat goes to Discord");
    assert_eq!(started.range, ExportRange::Last(25));
    assert_eq!(started.format, ExportFormat::Html);
    assert!(!started.attachments);

    // How many came, and Stop.
    app.client.apply(CoreEvent::ChatExported {
        room: GENERAL.into(),
        messages: 25,
        end: None,
    });
    let _ = app.sync();
    assert!(shows(&app, "25 messages downloaded…"));
    app.client.sent.clear();
    interact(&mut app, |ui| {
        ui.click("Stop").expect("Stop");
    });
    assert!(matches!(
        app.client.sent.as_slice(),
        [Command::Discord(DiscordCommand::StopExport { room })] if room == GENERAL
    ));
    app.client.apply(CoreEvent::ChatExported {
        room: GENERAL.into(),
        messages: 0,
        end: Some(ExportEnd::Stopped),
    });
    let _ = app.sync();
    assert!(shows(&app, "Export stopped: nothing was saved."));

    // Escape closes it.
    let _ = app.update(Message::Key(
        iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
        Default::default(),
    ));
    assert!(app.client.state().chat_export.is_none());
}
