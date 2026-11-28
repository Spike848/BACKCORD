//! A Discord stage the call is on: who may do what, and what each sends.

use super::*;
use crate::application::{StageInput, StageRole};
use crate::chat::StageChange;

const HALL: &str = "discord:10:14";

/// `discord_server` with its Lounge a stage, live with `topic` or not,
/// Alice owning the server (so moderating it) or not, and her in the call
/// there, in the audience or not, her hand raised or not.
fn on_stage(topic: Option<&str>, owner: bool, audience: bool, requested: bool) -> Application {
    let mut app = discord_server();
    change_server(&mut app, |server| {
        server.voice_channels[0].stage = true;
        if owner {
            server.owner_id = Some("1".into());
        }
        server.stages = topic
            .map(|topic| backcord_discord::ready::StageInstance {
                id: "90".into(),
                channel: "14".into(),
                topic: topic.into(),
            })
            .into_iter()
            .collect();
    });
    let voice = &mut app.state.voice;
    voice.phase = voice::Phase::Connected;
    voice.room = Some(HALL.into());
    voice.participants = vec![voice::Participant {
        id: "1".into(),
        user: "Alice".into(),
        person: "1".into(),
        local: true,
        muted: false,
        deafened: false,
        server_muted: false,
        server_deafened: false,
        speaking: false,
        connected: true,
        problem: None,
        streaming: false,
        video: None,
        avatar: None,
        audience,
        requested_to_speak: requested.then_some(1),
    }];
    app
}

fn stage(app: &mut Application, input: StageInput) -> Option<StageChange> {
    match <[_; 1]>::try_from(app.dispatch(Action::StageChannel(input))).ok()? {
        [Effect::Backend(Command::Discord(DiscordCommand::Stage { room, change }))] => {
            assert_eq!(room, HALL);
            Some(change)
        }
        _ => None,
    }
}

#[test]
fn a_call_in_a_voice_channel_is_on_no_stage() {
    let mut app = on_stage(None, true, false, false);
    change_server(&mut app, |server| server.voice_channels[0].stage = false);
    assert!(app.call_stage().is_none());
    assert!(stage(&mut app, StageInput::End).is_none());
}

#[test]
fn the_audience_raises_a_hand_and_says_yes_to_an_invitation() {
    // Members without Request to Speak may only lower a hand.
    let mut app = on_stage(Some("News"), false, true, false);
    let shown = app.call_stage().unwrap();
    assert_eq!(shown.topic.as_deref(), Some("News"));
    assert!(!shown.moderator);
    assert_eq!(shown.role, StageRole::Audience { hand: false });
    assert!(!shown.may_request);
    assert!(stage(&mut app, StageInput::Hand(true)).is_none());
    assert!(
        stage(&mut app, StageInput::End).is_none(),
        "only moderators end it"
    );
    assert!(
        stage(&mut app, StageInput::Speak).is_none(),
        "only when invited"
    );

    let mut app = on_stage(Some("News"), true, true, true);
    assert_eq!(
        app.call_stage().unwrap().role,
        StageRole::Audience { hand: true }
    );
    assert_eq!(
        stage(&mut app, StageInput::Hand(false)),
        Some(StageChange::Hand(false))
    );

    // Invited: not suppressed, asked; yes speaks, no goes to the audience.
    let mut app = on_stage(Some("News"), false, false, true);
    assert_eq!(app.call_stage().unwrap().role, StageRole::Invited);
    assert_eq!(stage(&mut app, StageInput::Speak), Some(StageChange::Speak));
    assert_eq!(
        stage(&mut app, StageInput::ToAudience),
        Some(StageChange::Audience)
    );
}

#[test]
fn a_moderator_starts_edits_and_ends_the_stage() {
    let mut app = on_stage(None, true, false, false);
    let shown = app.call_stage().unwrap();
    assert!(shown.moderator && shown.topic.is_none());
    app.dispatch(Action::StageChannel(StageInput::EditTopic(true)));
    assert!(app.state.stage.editor.as_ref().unwrap().starting);
    assert!(
        stage(&mut app, StageInput::SaveTopic).is_none(),
        "a topic first"
    );
    app.dispatch(Action::StageChannel(StageInput::WriteTopic(
        "  Town news ".into(),
    )));
    assert_eq!(
        stage(&mut app, StageInput::SaveTopic),
        Some(StageChange::Start("Town news".into()))
    );
    app.apply(Event::StageChanged {
        change: StageChange::Start("Town news".into()),
        result: Err("Missing Permissions".into()),
    });
    let editor = app.state.stage.editor.as_ref().unwrap();
    assert_eq!(editor.error.as_deref(), Some("Missing Permissions"));
    assert!(!editor.busy);
    app.apply(Event::StageChanged {
        change: StageChange::Start("Town news".into()),
        result: Ok(()),
    });
    assert!(app.state.stage.editor.is_none());

    let mut app = on_stage(Some("Town news"), true, false, false);
    app.dispatch(Action::StageChannel(StageInput::EditTopic(true)));
    let editor = app.state.stage.editor.as_ref().unwrap();
    assert!(!editor.starting);
    assert_eq!(editor.topic, "Town news");
    app.dispatch(Action::StageChannel(StageInput::WriteTopic(
        "x".repeat(200),
    )));
    assert_eq!(app.state.stage.editor.as_ref().unwrap().topic.len(), 120);
    assert_eq!(stage(&mut app, StageInput::End), Some(StageChange::End));
    assert_eq!(
        stage(&mut app, StageInput::Invite("2".into())),
        Some(StageChange::Invite("2".into()))
    );
    assert_eq!(
        stage(&mut app, StageInput::MoveToAudience("2".into())),
        Some(StageChange::ToAudience("2".into()))
    );
    // Another change Discord refused is said until dismissed.
    app.apply(Event::StageChanged {
        change: StageChange::End,
        result: Err("Unknown Stage Instance".into()),
    });
    assert_eq!(
        app.state.stage.error.as_deref(),
        Some("Unknown Stage Instance")
    );
    app.dispatch(Action::StageChannel(StageInput::DismissError));
    assert!(app.state.stage.error.is_none());
}

#[test]
fn a_moderator_in_the_audience_steps_up_without_asking() {
    let mut app = on_stage(Some("News"), true, true, false);
    assert_eq!(stage(&mut app, StageInput::Speak), Some(StageChange::Speak));
}
