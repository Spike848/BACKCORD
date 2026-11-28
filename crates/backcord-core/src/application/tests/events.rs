//! A server's Events dialog, and making a server.

use super::*;
use crate::chat::{
    EventsAnswer, EventsCommand, NewServerAnswer, NewServerCommand, TemplatePreview,
};

/// `discord_server` with two events: Bob's in the Lounge, and Alice's
/// somewhere else.
fn with_events() -> Application {
    use backcord_discord::ready;
    let mut app = discord_server();
    change_server(&mut app, |server| {
        server.put_events(
            [
                serde_json::json!({"id": "50", "name": "Quiz", "status": 1,
                    "scheduled_start_time": "2030-01-01T10:00:00+00:00", "entity_type": 2,
                    "channel_id": "15", "creator_id": "2", "user_count": 4}),
                serde_json::json!({"id": "51", "name": "Picnic", "status": 2,
                    "scheduled_start_time": "2030-01-02T10:00:00+00:00",
                    "scheduled_end_time": "2030-01-02T12:00:00+00:00", "entity_type": 3,
                    "entity_metadata": {"location": "Park"}, "creator_id": "1"}),
            ]
            .iter()
            .filter_map(ready::ScheduledEvent::parse)
            .collect(),
        );
    });
    app
}

#[track_caller]
fn events_command(effects: &[Effect]) -> &EventsCommand {
    match effects {
        [Effect::Backend(Command::Discord(DiscordCommand::Events(command)))] => command,
        _ => panic!("expected an events command"),
    }
}

#[test]
fn the_events_dialog_lists_a_servers_events_and_asks_for_their_counts() {
    let mut app = with_events();
    let effects = app.dispatch(Action::Events(EventsAction::Open("discord:10".into())));
    assert_eq!(
        events_command(&effects),
        &EventsCommand::Load { guild: "10".into() }
    );
    let events = app.server_events("discord:10");
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].room.as_deref(), Some("discord:10:15"));
    assert_eq!((events[1].kind, events[1].active), (EVENT_EXTERNAL, true));
    assert!(app.state.events_dialog.as_ref().unwrap().loading);
    app.apply(Event::Events(EventsAnswer::Loaded {
        guild: "10".into(),
        result: Ok(()),
    }));
    assert!(!app.state.events_dialog.as_ref().unwrap().loading);

    // Interested goes as a link's button does.
    let effects = app.dispatch(Action::Events(EventsAction::ToggleInterest("50".into())));
    match effects.as_slice() {
        [
            Effect::Backend(Command::Discord(DiscordCommand::SetEventInterest {
                preview,
                interested,
            })),
        ] => {
            assert!(*interested);
            assert_eq!(
                (preview.event.as_str(), preview.guild.as_str()),
                ("50", "10")
            );
        }
        _ => panic!("expected the interest command"),
    }
    assert!(
        app.dispatch(Action::Events(EventsAction::ToggleInterest("50".into())))
            .is_empty(),
        "one at a time"
    );
    app.apply(Event::EventInterestSet {
        event: "50".into(),
        interested: true,
        result: Ok(()),
    });
    assert_eq!(app.state.events_dialog.as_ref().unwrap().marking, None);

    // Who's interested.
    let effects = app.dispatch(Action::Events(EventsAction::Users("50".into())));
    assert!(
        matches!(events_command(&effects), EventsCommand::Users { event, .. } if event == "50")
    );
    app.apply(Event::Events(EventsAnswer::Users {
        event: "50".into(),
        result: Ok(vec![crate::chat::EventPerson {
            id: "2".into(),
            name: "Bob".into(),
            avatar: String::new(),
        }]),
    }));
    assert!(matches!(
        &app.state.events_dialog.as_ref().unwrap().page,
        EventsPage::Users { people: Some(people), .. } if people.len() == 1
    ));
}

#[test]
fn events_are_made_and_changed_by_those_who_may() {
    use backcord_discord::ready::permission;
    let mut app = with_events();
    app.state.discord_user = Some("1".into());
    app.dispatch(Action::Events(EventsAction::Open("discord:10".into())));
    // Without Create Events, nothing can be made or changed.
    app.dispatch(Action::Events(EventsAction::New));
    assert_eq!(
        app.state.events_dialog.as_ref().unwrap().page,
        EventsPage::List
    );
    assert!(
        app.dispatch(Action::Events(EventsAction::Start("50".into())))
            .is_empty()
    );

    change_server(&mut app, |server| {
        server.put_role(
            &serde_json::json!({"id": "20", "name": "Mods", "position": 2,
            "permissions": (permission::CREATE_EVENTS).to_string()}),
        );
    });
    // Her own event, but not Bob's.
    let mine = app.server_events("discord:10")[1].clone();
    let bobs = app.server_events("discord:10")[0].clone();
    assert!(app.may_manage_event("discord:10", &mine));
    assert!(!app.may_manage_event("discord:10", &bobs));
    let effects = app.dispatch(Action::Events(EventsAction::End("51".into())));
    assert_eq!(
        events_command(&effects),
        &EventsCommand::Status {
            guild: "10".into(),
            event: "51".into(),
            status: 3
        }
    );
    app.apply(Event::Events(EventsAnswer::Changed {
        guild: "10".into(),
        event: Some("51".into()),
        result: Ok(()),
    }));

    // A new one, somewhere else: it needs a location and an end.
    app.dispatch(Action::Events(EventsAction::New));
    let field =
        |app: &mut Application, field| app.dispatch(Action::Events(EventsAction::Field(field)));
    field(&mut app, EventField::Kind(EVENT_EXTERNAL));
    field(&mut app, EventField::Name("Games night".into()));
    assert!(app.dispatch(Action::Events(EventsAction::Save)).is_empty());
    assert_eq!(
        app.state.events_dialog.as_ref().unwrap().error.as_deref(),
        Some("Enter a location for the event.")
    );
    field(&mut app, EventField::Location("Ann's place".into()));
    assert!(app.dispatch(Action::Events(EventsAction::Save)).is_empty());
    field(&mut app, EventField::StartDate("2031-05-01".into()));
    field(&mut app, EventField::StartTime("20:00".into()));
    field(&mut app, EventField::EndDate("2031-05-01".into()));
    field(&mut app, EventField::EndTime("19:00".into()));
    assert!(app.dispatch(Action::Events(EventsAction::Save)).is_empty());
    assert_eq!(
        app.state.events_dialog.as_ref().unwrap().error.as_deref(),
        Some("The end time must be after the start time.")
    );
    field(&mut app, EventField::EndTime("23:00".into()));
    let effects = app.dispatch(Action::Events(EventsAction::Save));
    let EventsCommand::Save {
        guild,
        event,
        draft,
    } = events_command(&effects)
    else {
        panic!("expected a save");
    };
    assert_eq!((guild.as_str(), event.as_deref()), ("10", None));
    assert_eq!(draft.location.as_deref(), Some("Ann's place"));
    assert_eq!(draft.end.unwrap() - draft.start, 3 * 3600);
    assert_eq!(draft.channel, None);
    app.apply(Event::Events(EventsAnswer::Changed {
        guild: "10".into(),
        event: None,
        result: Ok(()),
    }));
    assert_eq!(
        app.state.events_dialog.as_ref().unwrap().page,
        EventsPage::List
    );

    // Editing her own fills the form; deleting asks first.
    app.dispatch(Action::Events(EventsAction::Back));
    app.dispatch(Action::Events(EventsAction::Edit("51".into())));
    assert!(matches!(
        &app.state.events_dialog.as_ref().unwrap().page,
        EventsPage::Form(form) if form.location == "Park" && form.editing.as_deref() == Some("51")
    ));
    app.dispatch(Action::Events(EventsAction::Back));
    app.dispatch(Action::Events(EventsAction::AskDelete("51".into())));
    let effects = app.dispatch(Action::Events(EventsAction::Delete));
    assert_eq!(
        events_command(&effects),
        &EventsCommand::Delete {
            guild: "10".into(),
            event: "51".into()
        }
    );
}

#[test]
fn a_new_server_is_made_and_opens_once_it_lists() {
    let mut app = discord_server();
    app.dispatch(Action::NewServer(NewServerAction::Open));
    app.dispatch(Action::NewServer(NewServerAction::CreateOwn));
    app.dispatch(Action::NewServer(NewServerAction::About(false)));
    let dialog = app.state.new_server.as_ref().unwrap();
    assert_eq!(dialog.step, NewServerStep::Customize);
    app.dispatch(Action::NewServer(NewServerAction::Name("A".into())));
    assert!(
        app.dispatch(Action::NewServer(NewServerAction::Create))
            .is_empty(),
        "too short"
    );
    app.dispatch(Action::NewServer(NewServerAction::Name("Our place".into())));
    let effects = app.dispatch(Action::NewServer(NewServerAction::Create));
    assert!(matches!(
        effects.as_slice(),
        [Effect::Backend(Command::Discord(DiscordCommand::NewServer(NewServerCommand::Create {
            name, template: None, ..
        })))] if name == "Our place"
    ));
    app.apply(Event::NewServer(NewServerAnswer::Created {
        result: Ok("77".into()),
    }));
    assert!(app.state.new_server.is_some(), "waits until it lists");
    let mut snapshot = backcord_discord::service::Snapshot::clone(app.discord.as_ref().unwrap());
    let made = backcord_discord::ready::server(&serde_json::json!({
        "id": "77", "owner_id": "1", "properties": {"name": "Our place"}, "channels": [],
    }))
    .unwrap();
    snapshot.servers.push(marked(made, &[]));
    app.apply(discord_event(snapshot));
    assert!(app.state.new_server.is_none());
    assert_eq!(
        app.state.navigation,
        Navigation::Server("discord:77".into())
    );
}

#[test]
fn a_template_link_previews_the_template_before_making_a_server() {
    let mut app = discord_server();
    let effects = app.dispatch(Action::OpenTemplateLink("hgM48av5Q69A".into()));
    assert!(matches!(
        effects.as_slice(),
        [Effect::Backend(Command::Discord(DiscordCommand::NewServer(NewServerCommand::Template { code })))]
            if code == "hgM48av5Q69A"
    ));
    app.apply(Event::NewServer(NewServerAnswer::Template {
        code: "hgM48av5Q69A".into(),
        result: Ok(TemplatePreview {
            code: "hgM48av5Q69A".into(),
            name: "Friends & Family".into(),
            ..TemplatePreview::default()
        }),
    }));
    assert_eq!(
        app.state.new_server.as_ref().unwrap().step,
        NewServerStep::Template
    );
    app.dispatch(Action::NewServer(NewServerAction::Create));
    app.dispatch(Action::NewServer(NewServerAction::Name("Family".into())));
    let effects = app.dispatch(Action::NewServer(NewServerAction::Create));
    assert!(matches!(
        effects.as_slice(),
        [Effect::Backend(Command::Discord(DiscordCommand::NewServer(NewServerCommand::Create {
            template: Some(code), ..
        })))] if code == "hgM48av5Q69A"
    ));
    // A code that can't be one says so.
    app.dispatch(Action::NewServer(NewServerAction::Close));
    app.dispatch(Action::NewServer(NewServerAction::Open));
    assert!(
        app.dispatch(Action::NewServer(NewServerAction::Template(
            "not a code".into()
        )))
        .is_empty()
    );
    assert!(app.state.new_server.as_ref().unwrap().error.is_some());
}
