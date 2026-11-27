//! Settings: a theme chosen in Appearance shows at once and is kept, a
//! custom theme made, edited and deleted there, preferences kept across a
//! restart, a device chosen in Voice & video, and push to talk's key
//! recorded and kept.

use backcord_core::voice;
use iced::keyboard::{Key, Location, key};

use super::{change, interact, local, preview, shows, start};
use crate::{
    app::{
        App, Message,
        layers::{Layer, Popup},
    },
    settings::{Event, Section, voice::Event as VoiceEvent},
};

/// Settings on a section, in a window tall enough to show its whole page.
fn open(app: &mut App, section: Section) {
    app.shell.frame.height = 2400.0;
    crate::window_bar::fit_window(app);
    let _ = app.update(Message::Settings(Event::Open(section)));
}

fn stored(folder: &tempfile::TempDir) -> serde_json::Value {
    serde_json::from_slice(&{
        crate::layout::flush(folder.path());
        std::fs::read(folder.path().join("layout.json")).expect("the layout")
    })
    .expect("JSON")
}

/// Settings open moving, once: their motion starts as they open, stays
/// through what's done in them, and turns to closing as they close, the
/// Settings closed kept, taking nothing, until they've been drawn going.
#[test]
fn settings_open_and_close_as_a_popup() {
    let (mut app, _folder) = preview();
    assert!(app.popups_moving().is_empty());
    open(&mut app, Section::Account);
    let opening = app.popups_moving();
    assert_eq!(opening.len(), 1, "Settings' motion");
    assert!(!opening[0].1, "opening");
    open(&mut app, Section::Appearance);
    assert_eq!(
        app.popups_moving(),
        opening,
        "the same motion, not a new one"
    );
    let _ = app.update(Message::Settings(Event::Close));
    assert!(app.win.settings.get().is_none(), "closed");
    assert!(app.win.settings.leaving().is_some(), "drawn going");
    assert_eq!(app.popups_moving(), [(opening[0].0, true)], "closing");
    // Escape is the window's again while they go.
    assert!(app.layers().all(|layer| layer != Layer::Settings));
    let _ = app.update(Message::PopupGone(Popup::Layer(Layer::Settings)));
    assert!(app.win.settings.leaving().is_none(), "gone");
    assert!(app.popups_moving().is_empty());
}

/// Settings opened again as they close are the new ones, opening: the
/// ones going go at once.
#[test]
fn settings_opened_again_as_they_close_open_anew() {
    let (mut app, _folder) = preview();
    open(&mut app, Section::Account);
    let _ = app.update(Message::Settings(Event::Close));
    open(&mut app, Section::Appearance);
    assert!(app.win.settings.leaving().is_none());
    let moving = app.popups_moving();
    assert_eq!(moving.len(), 1);
    assert!(!moving[0].1, "opening");
}

#[test]
fn a_theme_chosen_in_appearance_shows_at_once_and_is_kept() {
    let (mut app, folder) = preview();
    open(&mut app, Section::Account);
    assert!(shows(&app, "My account"));
    interact(&mut app, |ui| {
        ui.click("Themes").expect("the section");
    });
    assert!(shows(&app, "BACKDOOR"), "the theme's tile");
    app.themes.custom.push(light());
    assert!(app.choose_theme("custom:1"));
    interact(&mut app, |ui| {
        ui.click("BACKDOOR").expect("its tile");
    });
    assert_eq!(app.theme.id, "backdoor", "at once");
    assert_eq!(stored(&folder)["theme"], "backdoor");
    // Escape closes Settings and lets go of what it held.
    let _ = app.update(Message::Key(
        Key::Named(key::Named::Escape),
        Default::default(),
    ));
    assert!(!app.win.settings.is_open());
    assert_eq!(
        start(&folder, true).theme.id,
        "backdoor",
        "the next start too"
    );
}

/// A light theme of the person's own, as the tests' only light one.
fn light() -> backcord_core::theme::Custom {
    use backcord_core::theme::{BACKDOOR, Colour, Custom, Theme};
    Custom {
        id: "custom:1".into(),
        name: "Paper".into(),
        base: "backdoor",
        theme: Theme {
            dark: false,
            chat: Colour::rgb(0xffffff),
            text: Colour::rgb(0x202020),
            ..BACKDOOR
        },
    }
}

#[test]
fn a_custom_theme_is_made_edited_and_deleted_in_appearance() {
    use crate::settings::appearance_event as theme;
    let (mut app, folder) = preview();
    open(&mut app, Section::Appearance);
    interact(&mut app, |ui| {
        ui.click("New theme").expect("New theme");
    });
    assert!(shows(&app, "NEW THEME") && shows(&app, "START FROM"));
    // A colour typed shows at once; one that isn't a colour says so.
    let chat = backcord_core::theme::Theme::NAMES
        .iter()
        .position(|name| *name == "chat")
        .expect("chat");
    let _ = app.update(theme(crate::settings::ThemeEvent::Colour(
        chat,
        "#102030".into(),
    )));
    assert_eq!(app.theme.chat, crate::style::hex(0x102030));
    let _ = app.update(theme(crate::settings::ThemeEvent::Colour(
        chat,
        "#1020".into(),
    )));
    assert!(shows(&app, "Not a colour"));
    assert_eq!(
        app.theme.chat,
        crate::style::hex(0x102030),
        "the last good one"
    );
    let _ = app.update(theme(crate::settings::ThemeEvent::Colour(
        chat,
        "#203040".into(),
    )));
    let _ = app.update(theme(crate::settings::ThemeEvent::Name("Harbour".into())));
    interact(&mut app, |ui| {
        ui.click("Save").expect("Save");
    });
    assert_eq!(app.theme.id, "custom:1", "chosen once saved");
    let layout = stored(&folder);
    assert_eq!(layout["theme"], "custom:1");
    assert_eq!(layout["custom_themes"][0]["name"], "Harbour");
    assert_eq!(layout["custom_themes"][0]["colours"]["chat"], "#203040");
    assert!(shows(&app, "Harbour"), "among the tiles");
    // Edited, then cancelled: the theme from before comes back.
    let _ = app.update(theme(crate::settings::ThemeEvent::Edit("custom:1".into())));
    assert!(shows(&app, "EDIT THEME"));
    let _ = app.update(theme(crate::settings::ThemeEvent::Colour(
        chat,
        "#ffffff".into(),
    )));
    assert_eq!(app.theme.chat, crate::style::hex(0xffffff));
    interact(&mut app, |ui| {
        ui.click("Cancel").expect("Cancel");
    });
    assert_eq!(app.theme.chat, crate::style::hex(0x203040));
    // Deleted, asked once more: the chosen theme goes, and BACKDOOR shows.
    let _ = app.update(theme(crate::settings::ThemeEvent::Edit("custom:1".into())));
    interact(&mut app, |ui| {
        ui.click("Delete theme").expect("Delete theme");
    });
    assert!(shows(&app, "Delete it for good"));
    interact(&mut app, |ui| {
        ui.click("Delete it for good").expect("Delete it for good");
    });
    assert_eq!(app.theme.id, "backdoor");
    assert_eq!(stored(&folder)["custom_themes"], serde_json::json!([]));
}

#[test]
fn preferences_are_kept_across_a_restart() {
    let (mut app, folder) = preview();
    // The newest message sent at a time of its own, as Discord's are.
    change(&mut app, |state| {
        let mut messages = (*state.messages).clone();
        let last = messages.last_mut().expect("a message");
        let mut message = (**last).clone();
        message.timestamp = 1_790_000_000;
        *last = std::sync::Arc::new(message);
        state.messages = std::sync::Arc::new(messages);
    });
    open(&mut app, Section::Messages);
    interact(&mut app, |ui| {
        ui.click("12-hour").expect("the clock");
    });
    interact(&mut app, |ui| {
        ui.click("Show seconds").expect("the switch");
    });
    interact(&mut app, |ui| {
        ui.click("Collapse long messages").expect("the switch");
    });
    interact(&mut app, |ui| {
        ui.click("Large").expect("the text size");
    });
    assert!(app.layout.prefs.twelve_hour_clock && app.layout.prefs.clock_seconds);
    // The timeline's times follow at once: 9:41:05 PM, say.
    let time = &app.timeline.looks.last().expect("a message").time;
    assert!(
        time.matches(':').count() == 2 && (time.ends_with("AM") || time.ends_with("PM")),
        "{time}"
    );
    open(&mut app, Section::Notifications);
    interact(&mut app, |ui| {
        ui.click("Show desktop notifications").expect("the switch");
    });
    open(&mut app, Section::Advanced);
    interact(&mut app, |ui| {
        ui.click("Developer Mode").expect("the switch");
    });
    let again = start(&folder, true);
    let prefs = &again.layout.prefs;
    assert!(prefs.twelve_hour_clock && prefs.clock_seconds);
    assert!(again.layout.collapse_long && again.layout.developer_mode);
    assert_eq!(prefs.text_size, Some(2));
    assert_eq!(prefs.notifications, Some(false));
    // `layout.json` keeps them under the keys older versions wrote.
    let layout = stored(&folder);
    assert_eq!(layout["twelve_hour_clock"], true);
    assert_eq!(layout["notifications"], false);
}

/// A file chosen for a sound under Discord's makes a preset that plays it
/// for that sound alone, kept across a restart; deleting the preset plays
/// Discord's again and lets go of its copy.
#[test]
fn a_file_chosen_for_a_sound_makes_a_preset_that_deleting_lets_go() {
    use backcord_core::sounds::Sound;
    let (mut app, folder) = preview();
    open(&mut app, Section::Notifications);
    let root = folder.path();
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testing/fixtures");
    assert!(crate::settings::copy_sound(root, &fixtures.join("../fake_discord.ts")).is_err());
    let chosen = crate::settings::copy_sound(root, &fixtures.join("soundboard-tone.mp3"))
        .expect("an MP3 BACKCORD plays");
    assert_eq!(chosen.name, "soundboard-tone.mp3");
    let copy = root.join("sounds").join(&chosen.file);
    assert!(copy.exists());
    let _ = app.update(Message::Settings(Event::SoundPicked(
        6,
        Some(Ok(chosen.clone())),
    )));
    let voice = &app.layout.voice;
    assert_eq!(voice.sound_preset, Some(0));
    assert_eq!(voice.sound_presets[0].name, "Preset 1");
    assert_eq!(voice.sound_presets[0].files[&Sound::Message], chosen);
    assert_eq!(voice.sound_file(root, Sound::Message), Some(copy.clone()));
    assert_eq!(voice.sound_file(root, Sound::Join), None);
    assert!(shows(&app, "soundboard-tone.mp3"));
    let again = start(&folder, true);
    assert_eq!(
        again.layout.voice.sound_presets,
        app.layout.voice.sound_presets
    );
    assert_eq!(again.layout.voice.sound_preset, Some(0));
    interact(&mut app, |ui| {
        ui.click("Delete preset").expect("the button");
    });
    assert!(app.layout.voice.sound_presets.is_empty());
    assert_eq!(app.layout.voice.sound_preset, None);
    assert!(!copy.exists(), "no preset chooses it");
}

#[test]
fn a_device_chosen_in_voice_settings_is_the_one_used() {
    let (mut app, _folder) = preview();
    change(&mut app, |state| {
        let device = |id: &str, name: &str| voice::Device {
            id: id.into(),
            name: name.into(),
        };
        state.voice.microphones = vec![device("mic-a", "Desk"), device("mic-b", "Headset")];
        state.voice.microphone = Some("mic-a".into());
    });
    open(&mut app, Section::Voice);
    assert!(shows(&app, "Desk"), "the one chosen");
    assert!(!shows(&app, "Headset"), "the rest only in the list");
    interact(&mut app, |ui| {
        ui.click("Desk").expect("the dropdown");
    });
    interact(&mut app, |ui| {
        ui.click("Headset").expect("its option");
    });
    assert_eq!(
        app.client.state().voice.microphone.as_deref(),
        Some("mic-b")
    );
    assert!(
        app.win.settings.get().is_some_and(|s| s.open.is_none()),
        "closed"
    );
}

#[test]
fn the_soundboard_volume_moves_with_its_drag() {
    let (mut app, _folder) = preview();
    change(&mut app, |state| state.discord_linked = true);
    open(&mut app, Section::Voice);
    assert!(shows(&app, "Soundboard volume"));
    assert!(!shows(&app, "37%"));
    let voice = |event| Message::Settings(Event::Voice(event));
    let _ = app.update(voice(VoiceEvent::SoundboardVolume(37, false)));
    assert!(shows(&app, "37%"), "the slider follows the drag");
    // Let go, what the drag left is sent, not what the slider was built
    // with, and the slider shows the core's volume again.
    let _ = app.update(voice(VoiceEvent::SoundboardVolume(80, true)));
    assert!(
        app.win
            .settings
            .get()
            .is_some_and(|settings| settings.voice.soundboard.is_none())
    );
    assert!(!shows(&app, "37%"));
}

#[test]
fn a_recorded_push_to_talk_key_is_named_and_kept() {
    use crate::settings::keybinds::recorded;
    use iced::keyboard::key::{Code, Physical};
    let v = recorded(
        &Key::Character("v".into()),
        Physical::Code(Code::KeyV),
        Location::Standard,
    )
    .expect("a key");
    assert_eq!(v.name, "V");
    if cfg!(target_os = "linux") {
        assert_eq!(v.scancode, 47, "evdev's, as winit gives it");
        assert_eq!(v.trigger.as_deref(), Some("v"));
    }
    let ctrl = recorded(
        &Key::Named(key::Named::Control),
        Physical::Code(Code::ControlRight),
        Location::Right,
    )
    .expect("a key");
    assert_eq!(ctrl.name, "Right Ctrl");
    if cfg!(target_os = "linux") {
        assert_eq!(ctrl.trigger.as_deref(), Some("Control_R"));
    }

    let (mut app, folder) = preview();
    open(&mut app, Section::Voice);
    interact(&mut app, |ui| {
        ui.click("Push to talk").expect("the input mode");
    });
    interact(&mut app, |ui| {
        ui.click("Record keybind").expect("the button");
    });
    assert!(app.recording_keybind());
    assert!(shows(&app, "Press a key..."));
    // The keys pressed together are the keybind once they're all up, and
    // show as they're pressed.
    let heard = |key: &backcord_core::keybinds::Key, pressed| {
        Message::Settings(Event::Heard(crate::settings::keybinds::Heard::Key {
            key: key.clone(),
            pressed,
            escape: false,
        }))
    };
    let _ = app.update(heard(&ctrl, true));
    let _ = app.update(heard(&v, true));
    assert!(shows(&app, "Right Ctrl + V") || shows(&app, "Ctrl + V"));
    let _ = app.update(heard(&v, false));
    assert!(app.recording_keybind(), "a key is still down");
    let _ = app.update(heard(&ctrl, false));
    assert!(!app.recording_keybind());
    assert!(shows(&app, "Edit keybind"));
    let layout = stored(&folder);
    assert_eq!(layout["input_mode"], "push_to_talk");
    assert_eq!(layout["push_to_talk_key"]["name"], "V");
    assert_eq!(layout["push_to_talk_key"]["with"][0]["name"], "Right Ctrl");
    // Escape, pressed first, stops without changing it.
    interact(&mut app, |ui| {
        ui.click("Edit keybind").expect("the button");
    });
    let escape = backcord_core::keybinds::Key {
        scancode: 1,
        name: "Escape".into(),
        trigger: None,
    };
    let _ = app.update(Message::Settings(Event::Heard(
        crate::settings::keybinds::Heard::Key {
            key: escape,
            pressed: true,
            escape: true,
        },
    )));
    assert!(!app.recording_keybind());
    assert_eq!(stored(&folder)["push_to_talk_key"]["name"], "V");
    // Where the key works, as its watcher tells from its own thread,
    // shows without anything else happening; a word from a watcher gone
    // since doesn't.
    let told = app.keybinds.count;
    let status = |reach| {
        crate::keybinds::Told::Status(backcord_core::keybinds::Status {
            reach,
            given: Default::default(),
        })
    };
    let everywhere = "Hold the key in any app to talk.";
    let _ = app.update(Message::Keybinds(
        told,
        status(backcord_core::keybinds::Reach::Everywhere),
    ));
    assert!(shows(&app, everywhere));
    let focused = backcord_core::keybinds::Reach::Focused("Your desktop has no global keys");
    let _ = app.update(Message::Keybinds(told - 1, status(focused.clone())));
    assert!(shows(&app, everywhere));
    // Kept while Settings are closed, for when they open.
    let _ = app.close_settings();
    let _ = app.update(Message::Keybinds(told, status(focused)));
    open(&mut app, Section::Voice);
    assert!(shows(
        &app,
        "Your desktop has no global keys, so push to talk works only while BACKCORD's window is in front."
    ));
    let again = start(&folder, true);
    assert_eq!(
        again
            .layout
            .voice
            .push_to_talk_key
            .as_ref()
            .map(backcord_core::keybinds::Shortcut::name),
        Some("Ctrl + V".to_owned())
    );
}

#[test]
fn keybinds_are_added_chosen_recorded_and_removed() {
    let (mut app, folder) = preview();
    open(&mut app, Section::Keybinds);
    assert!(shows(&app, "Add a Keybind"));
    interact(&mut app, |ui| {
        ui.click("Add a Keybind").expect("the button");
    });
    assert!(shows(
        &app,
        "This action doesn't do anything! Please choose one from the drop down."
    ));
    let keybind = |event| Message::Settings(Event::Keybind(event));
    use crate::settings::keybinds::{Event as KeybindEvent, Heard, Target};
    let _ = app.update(keybind(KeybindEvent::Action(0, 4)));
    assert!(shows(&app, "Toggle Deafen"));
    let _ = app.update(Message::Settings(Event::Record(Target::Row(0), true)));
    assert!(app.recording_keybind());
    let d = backcord_core::keybinds::Key {
        scancode: 32,
        name: "D".into(),
        trigger: Some("d".into()),
    };
    for pressed in [true, false] {
        let _ = app.update(Message::Settings(Event::Heard(Heard::Key {
            key: d.clone(),
            pressed,
            escape: false,
        })));
    }
    assert!(!app.recording_keybind());
    let layout = stored(&folder);
    assert_eq!(layout["keybinds"][0]["action"], "toggle_deafen");
    assert_eq!(layout["keybinds"][0]["keys"]["name"], "D");
    assert_eq!(layout["keybinds"][0]["id"], 1);
    // The watcher watches it: a toggle from it deafens.
    assert!(
        app.keybinds
            .watched
            .iter()
            .any(|watched| watched.id == "keybind-1")
    );
    // As the account card's button: out of a call, the next one starts
    // deafened.
    let deafened =
        |app: &crate::app::App| (app.client.state().voice.deafened, app.layout.deafen_on_join);
    let before = deafened(&app);
    let _ = app.update(Message::Keybinds(
        app.keybinds.count,
        crate::keybinds::Told::Toggled(backcord_core::keybinds::Action::ToggleDeafen),
    ));
    assert_ne!(deafened(&app), before);
    let _ = app.update(keybind(KeybindEvent::Remove(0)));
    assert!(!shows(&app, "Toggle Deafen"));
    assert_eq!(stored(&folder)["keybinds"], serde_json::json!([]));
    assert!(app.keybinds.watched.is_empty());
}

/// Appearance's neon glow: at its default as it starts, as dragged at once,
/// and kept once let go.
#[test]
fn the_neon_glow_follows_its_slider_and_is_kept() {
    let (mut app, folder) = preview();
    open(&mut app, Section::Appearance);
    assert!(shows(&app, "Neon ring intensity"));
    assert_eq!(app.layout.prefs.neon_glow, None);
    let glow =
        |value, done| Message::Settings(Event::Pref(crate::settings::Pref::NeonGlow(value, done)));
    let _ = app.update(glow(80, false));
    assert_eq!(
        backcord_iced_kit::style::glow(),
        80,
        "glows as it's dragged"
    );
    assert_eq!(app.layout.prefs.neon_glow, Some(80));
    let _ = app.update(glow(80, true));
    assert_eq!(stored(&folder)["neon_glow"], 80);
    let _ = app.update(glow(0, true));
    assert_eq!(backcord_iced_kit::style::glow(), 0);
    assert_eq!(
        backcord_iced_kit::style::glow_shadow(iced::Color::WHITE, 100.0),
        iced::Shadow::default(),
        "none at 0"
    );
    let _ = app.update(glow(
        u32::from(backcord_iced_kit::style::DEFAULT_GLOW),
        true,
    ));
}

#[test]
fn closing_settings_lets_go_of_an_edited_theme() {
    let (mut app, _folder) = preview();
    open(&mut app, Section::Appearance);
    interact(&mut app, |ui| {
        ui.click("New theme").expect("New theme");
    });
    let _ = app.update(crate::settings::appearance_event(
        crate::settings::ThemeEvent::Dark(false),
    ));
    assert!(!app.theme.dark, "shown while edited");
    let _ = app.update(Message::Settings(Event::Close));
    assert!(!app.win.settings.is_open());
    assert_eq!(app.theme.id, "backdoor", "the theme from before");
}

/// Every key the core tags has its tag in Settings' pages.
#[test]
fn every_tagged_setting_has_its_tag() {
    let pages = [
        include_str!("../settings/pages/mod.rs"),
        include_str!("../settings/pages/about.rs"),
        include_str!("../settings/pages/account.rs"),
        include_str!("../settings/pages/advanced.rs"),
        include_str!("../settings/pages/discord.rs"),
        include_str!("../settings/pages/games.rs"),
        include_str!("../settings/pages/messages.rs"),
        include_str!("../settings/pages/notifications.rs"),
    ]
    .concat();
    for tag in &backcord_core::new_tags::NEW {
        assert!(
            pages.contains(&format!("self.fresh(\"{}\")", tag.key)),
            "{}",
            tag.key
        );
    }
}

#[test]
fn a_theme_switch_lays_the_timelines_text_out_in_the_new_inks() {
    use backcord_core::chat::{ChatMessage, Notice, NoticeKind};
    let (mut app, _folder) = preview();
    super::change(&mut app, |state| {
        state.messages = std::sync::Arc::new(vec![std::sync::Arc::new(ChatMessage {
            item: "n0".into(),
            author: "Ann".into(),
            time: "12:01".into(),
            notice: Some(Notice {
                kind: NoticeKind::Pinned(Some("m1".into())),
                text: String::new(),
            }),
            ..Default::default()
        })]);
    });
    let ink = |app: &crate::app::App| {
        app.timeline.looks[0].body.parts[0]
            .runs
            .iter()
            .find(|run| run.text.contains("Ann"))
            .and_then(|run| run.color)
    };
    assert_eq!(ink(&app), Some(crate::theme::rgb(app.theme.text)));
    app.themes.custom.push(light());
    assert!(app.choose_theme("custom:1"), "a light theme");
    let _ = app.sync();
    assert_eq!(
        ink(&app),
        Some(crate::theme::rgb(app.theme.text)),
        "the author's name in the light theme's ink"
    );
}

#[test]
fn the_theme_command_switches_the_colours_and_keeps_the_choice() {
    let (mut app, folder) = preview();
    assert_eq!(app.theme.id, "backdoor", "the default");
    app.themes.custom.push(light());
    let _ = app.update(Message::Control("theme custom:1".into()));
    assert_eq!(app.theme.id, "custom:1");
    assert!(!app.theme.dark);
    assert_eq!(app.theme().palette().background, app.theme.chat);
    let layout: serde_json::Value = serde_json::from_slice(&{
        crate::layout::flush(folder.path());
        std::fs::read(folder.path().join("layout.json")).expect("the layout")
    })
    .expect("JSON");
    assert_eq!(layout["theme"], "custom:1");
    // A theme BACKCORD doesn't have changes nothing.
    let _ = app.update(Message::Control("theme gone".into()));
    assert_eq!(app.theme.id, "custom:1");
}

#[test]
fn a_custom_theme_in_the_layout_shows_from_the_start() {
    let folder = tempfile::tempdir_in(local()).expect("test folder");
    std::fs::write(
        folder.path().join("layout.json"),
        r##"{"theme": "custom:1", "custom_themes": [{"id": "custom:1", "name": "Plum",
            "base": "backdoor", "colours": {"chat": "#2a1a3a", "accent": "#c04080"}}]}"##,
    )
    .expect("the layout");
    let app = start(&folder, true);
    assert_eq!(app.theme.id, "custom:1");
    assert_eq!(app.theme.chat, crate::style::hex(0x2a1a3a));
    assert_eq!(app.theme.accent, crate::style::hex(0xc04080));
    assert_eq!(app.theme.selection.a, 0.35, "worked out from its accent");
}

#[test]
fn a_layout_key_of_the_wrong_kind_loses_only_itself() {
    let folder = tempfile::tempdir_in(local()).expect("test folder");
    std::fs::write(
        folder.path().join("layout.json"),
        r#"{"theme": "backdoor", "collapse_long": "yes", "members_hidden": true}"#,
    )
    .expect("the layout");
    let app = start(&folder, true);
    assert_eq!(app.theme.id, "backdoor");
    assert!(app.layout.members_hidden);
    assert!(!app.layout.collapse_long);
}

#[test]
fn edit_profile_from_settings_closes_them_under_the_editor() {
    let (mut app, _folder) = preview();
    change(&mut app, |state| state.discord_user = Some("42".into()));
    open(&mut app, Section::Account);
    let _ = app.update(Message::Settings(Event::EditProfile));
    assert!(
        app.client.state().profile_editor.is_some(),
        "the editor opens"
    );
    assert!(app.win.settings.get().is_none(), "Settings close under it");
}

/// A call going on in Lounge, joined as the preview joins, unmuted
/// unless `muted`: after the join, which turns the microphone on.
fn in_a_call(app: &mut App, muted: bool) {
    for command in [
        voice::Command::Join("lounge".into()),
        voice::Command::Mute(muted),
    ] {
        app.client
            .dispatch(backcord_core::application::Action::Voice(command));
        let _ = app.sync();
    }
    let voice = &app.client.state().voice;
    assert_eq!(
        (voice.active(), voice.muted, voice.deafened),
        (true, muted, false)
    );
}

fn testing(app: &App) -> bool {
    app.win
        .settings
        .get()
        .is_some_and(|settings| settings.voice.testing && settings.voice.gate.is_some())
}

#[test]
fn the_mic_test_deafens_a_call_while_it_plays_back_as_discords_does() {
    let (mut app, _folder) = preview();
    in_a_call(&mut app, false);
    open(&mut app, Section::Voice);
    assert!(shows(
        &app,
        "You'll be muted and deafened in your call while you test."
    ));
    interact(&mut app, |ui| {
        ui.click("Let's check").expect("the mic test's button");
    });
    assert!(testing(&app), "the meter plays the microphone back");
    let call = |app: &App| {
        let voice = &app.client.state().voice;
        (voice.muted, voice.deafened)
    };
    assert_eq!(call(&app), (true, true), "deafened, and so muted");
    assert!(shows(&app, "Stop testing"));
    interact(&mut app, |ui| {
        ui.click("Stop testing").expect("the button");
    });
    assert!(!testing(&app));
    assert_eq!(call(&app), (false, false), "as it was");
    // Undeafened while it plays, the test ends, as Discord's does.
    let _ = app.update(Message::Settings(Event::Voice(VoiceEvent::MicTest)));
    assert!(testing(&app));
    app.client
        .dispatch(backcord_core::application::Action::Voice(
            voice::Command::Deafen(false),
        ));
    let _ = app.sync();
    assert!(!testing(&app));
    assert!(!app.client.state().voice.deafened);
}

#[test]
fn leaving_voice_or_settings_ends_the_mic_test_and_the_call_hears_again() {
    let (mut app, _folder) = preview();
    in_a_call(&mut app, true);
    open(&mut app, Section::Voice);
    let mic_test = Message::Settings(Event::Voice(VoiceEvent::MicTest));
    let _ = app.update(mic_test.clone());
    assert!(app.client.state().voice.deafened);
    let _ = app.update(Message::Settings(Event::Open(Section::Advanced)));
    let voice = &app.client.state().voice;
    assert_eq!(
        (voice.muted, voice.deafened),
        (true, false),
        "muted as before"
    );
    open(&mut app, Section::Voice);
    assert!(!testing(&app), "it doesn't start again");
    let _ = app.update(mic_test);
    assert!(app.client.state().voice.deafened);
    let _ = app.update(Message::Settings(Event::Close));
    assert!(!app.client.state().voice.deafened, "closing ends it too");
}

#[test]
fn the_mic_test_without_a_call_deafens_nothing() {
    let (mut app, _folder) = preview();
    open(&mut app, Section::Voice);
    assert!(!shows(
        &app,
        "You'll be muted and deafened in your call while you test."
    ));
    let call = |app: &App| {
        let voice = &app.client.state().voice;
        (voice.muted, voice.deafened)
    };
    let before = call(&app);
    let _ = app.update(Message::Settings(Event::Voice(VoiceEvent::MicTest)));
    assert!(testing(&app));
    assert_eq!(call(&app), before);
}

/// Relationship Notifier's card in Extra Features, and its dialog's switch
/// for each removal it tells of, kept across a restart.
#[test]
fn relationship_notifier_chooses_what_it_tells_of_and_keeps_it() {
    let (mut app, folder) = preview();
    open(&mut app, Section::Extras);
    assert!(shows(&app, "Relationship Notifier"));
    let _ = app.update(Message::Settings(Event::Extra(Some(
        crate::settings::Extra::RelationshipNotifier,
    ))));
    for name in [
        "Friends",
        "Friend requests",
        "Group DMs",
        "Servers",
        "While offline",
    ] {
        assert!(shows(&app, name), "{name}");
    }
    interact(&mut app, |ui| {
        ui.click("Group DMs").expect("the switch");
    });
    assert_eq!(app.layout.prefs.relationship_groups, Some(false));
    let again = start(&folder, true);
    assert_eq!(again.layout.prefs.relationship_groups, Some(false));
    assert_eq!(stored(&folder)["relationship_groups"], false);
}

/// Fake Nitro's card in Extra Features, off as it starts, and its dialog:
/// the emoji size, masked links and received links as emoji, kept across a
/// restart.
#[test]
fn fake_nitro_starts_off_and_keeps_its_settings() {
    let (mut app, folder) = preview();
    open(&mut app, Section::Extras);
    assert!(shows(&app, "Fake Nitro"));
    assert_eq!(app.layout.prefs.fake_nitro, None, "off unless turned on");
    let _ = app.update(Message::Settings(Event::Extra(Some(
        crate::settings::Extra::FakeNitro,
    ))));
    for name in [
        "EMOJI SIZE",
        "Use masked links",
        "Show received emoji links as emoji",
    ] {
        assert!(shows(&app, name), "{name}");
    }
    interact(&mut app, |ui| {
        ui.click("96").expect("the size");
    });
    interact(&mut app, |ui| {
        ui.click("Use masked links").expect("the switch");
    });
    interact(&mut app, |ui| {
        ui.click("Show received emoji links as emoji")
            .expect("the switch");
    });
    let prefs = &app.layout.prefs;
    assert_eq!(
        (
            prefs.fake_nitro_size,
            prefs.fake_nitro_masked,
            prefs.fake_nitro_received
        ),
        (Some(3), Some(false), Some(false))
    );
    let again = start(&folder, true);
    assert_eq!(again.layout.prefs.fake_nitro_size, Some(3));
    assert_eq!(stored(&folder)["fake_nitro_masked"], false);
    assert_eq!(stored(&folder)["fake_nitro_received"], false);
}

/// Badge Spoofer's card in Extra Features, off as it starts, and its
/// dialog's badges, house, Nitro and boost, kept across a restart. On, one's
/// own profile shows the badges chosen, with Discord's pictures asked for,
/// and anyone else's shows what Discord said.
#[test]
fn badge_spoofer_starts_off_and_dresses_only_ones_own_profile() {
    use backcord_core::{
        application::{Action, ProfileView},
        badge_spoofer::{BADGES, NITRO},
        chat::Profile,
    };
    let (mut app, folder) = preview();
    open(&mut app, Section::Extras);
    assert!(shows(&app, "Badge Spoofer"));
    assert_eq!(app.layout.prefs.badge_spoofer, None, "off unless turned on");
    assert!(!app.client.state().badge_spoofer.on);
    let _ = app.update(Message::Settings(Event::Extra(Some(
        crate::settings::Extra::BadgeSpoofer,
    ))));
    for name in [
        "BADGES",
        "Discord Staff",
        "Completed a Quest",
        "HYPESQUAD HOUSE",
        "NITRO",
        "SERVER BOOSTER",
        "Show my real badges too",
    ] {
        assert!(shows(&app, name), "{name}");
    }
    interact(&mut app, |ui| {
        ui.click("Discord Staff").expect("its box");
    });
    interact(&mut app, |ui| {
        ui.click("Brilliance").expect("the house");
    });
    interact(&mut app, |ui| {
        ui.click("Gold").expect("the Nitro level");
    });
    interact(&mut app, |ui| {
        ui.click("1 year").expect("the boost");
    });
    let prefs = &app.layout.prefs;
    assert_eq!(
        (
            prefs.badge_spoofer_badges,
            prefs.badge_spoofer_house,
            prefs.badge_spoofer_nitro,
            prefs.badge_spoofer_boost
        ),
        (Some(1), Some(2), Some(4), Some(6))
    );
    // The core has the choice, still off.
    let spoofer = app.client.state().badge_spoofer;
    assert!(!spoofer.on && spoofer.has(0) && spoofer.nitro == 4);
    let again = start(&folder, true);
    assert_eq!(
        again.client.state().badge_spoofer,
        spoofer,
        "told as it starts"
    );
    assert_eq!(stored(&folder)["badge_spoofer_house"], 2);

    // Turned on, with one's own profile and someone else's on screen.
    app.layout.prefs.badge_spoofer = Some(true);
    app.client
        .dispatch(Action::SpoofBadges(app.layout.prefs.badge_spoofer()));
    let view = |id: &str| {
        std::sync::Arc::new(ProfileView {
            profile: Profile {
                id: id.into(),
                name: "Me".into(),
                handle: "me".into(),
                ..Default::default()
            },
            ..Default::default()
        })
    };
    change(&mut app, |state| {
        state.discord_user = Some("42".into());
        state.profile = Some(view("42"));
    });
    let badges = |app: &App| -> Vec<String> {
        app.profiles
            .shown
            .as_ref()
            .expect("the profile")
            .look
            .badges
            .iter()
            .map(|badge| badge.description.clone())
            .collect()
    };
    assert_eq!(
        badges(&app),
        [
            "Discord Staff",
            "HypeSquad Brilliance",
            "Nitro Gold",
            "Server Booster: 1 year"
        ]
    );
    let pictures = crate::profile::profile_pictures(app.ctx());
    for hash in [BADGES[0].hash, NITRO[3].hash] {
        assert!(
            pictures.iter().any(|url| url.contains(hash)),
            "{hash}'s picture is asked for"
        );
    }
    change(&mut app, |state| state.profile = Some(view("7")));
    assert!(
        badges(&app).is_empty(),
        "another's profile is as Discord said"
    );
    // Off again, one's own shows its own.
    change(&mut app, |state| state.profile = Some(view("42")));
    assert_eq!(badges(&app).len(), 4);
    app.layout.prefs.badge_spoofer = Some(false);
    app.client
        .dispatch(Action::SpoofBadges(app.layout.prefs.badge_spoofer()));
    change(&mut app, |_| {});
    assert!(badges(&app).is_empty());
}

/// Auto Delete's card in Extra Features, off as it starts, and its dialog's
/// times, kept across a restart.
#[test]
fn auto_delete_starts_off_and_keeps_its_time() {
    let (mut app, folder) = preview();
    open(&mut app, Section::Extras);
    assert!(shows(&app, "Auto Delete"));
    assert_eq!(app.layout.prefs.auto_delete, None, "off unless turned on");
    let _ = app.update(Message::Settings(Event::Extra(Some(
        crate::settings::Extra::AutoDelete,
    ))));
    for name in ["DELETE AFTER", "30 seconds", "5 minutes", "7 days"] {
        assert!(shows(&app, name), "{name}");
    }
    interact(&mut app, |ui| {
        ui.click("1 hour").expect("the time");
    });
    assert_eq!(app.layout.prefs.auto_delete_after, Some(4));
    let again = start(&folder, true);
    assert_eq!(again.layout.prefs.auto_delete_after, Some(4));
}

/// The voice limiter under the speakers' volume: off as it starts, in dB as
/// it's dragged, and kept once let go.
#[test]
fn the_voice_limiter_starts_off_and_keeps_its_level() {
    let (mut app, folder) = preview();
    // Other tests' windows play their volumes as they open.
    let _volumes = crate::layout::tests::hold_volumes();
    open(&mut app, Section::Voice);
    assert!(shows(&app, "Voice limiter"));
    assert_eq!(app.layout.voice.voice_limit, None, "off unless set");
    let voice = |event| Message::Settings(Event::Voice(event));
    let _ = app.update(voice(VoiceEvent::VoiceLimit(28, false)));
    assert!(shows(&app, "-12 dB"), "the slider follows the drag");
    let ceiling = backcord_core::volume::voice_limit().expect("on");
    assert!((ceiling - 10f32.powf(-12.0 / 20.0)).abs() < 1e-4);
    let _ = app.update(voice(VoiceEvent::VoiceLimit(28, true)));
    let stored = || crate::layout::Layout::read(folder.path()).voice.voice_limit;
    assert_eq!(stored(), Some(-12));
    let _ = app.update(voice(VoiceEvent::VoiceLimit(40, true)));
    assert_eq!(stored(), None, "all the way up is off");
    assert_eq!(backcord_core::volume::voice_limit(), None);
}

/// The voice compressor under the limiter: off as it starts, kept once
/// set.
#[test]
fn the_voice_compressor_starts_off_and_is_kept() {
    let (mut app, folder) = preview();
    let _volumes = crate::layout::tests::hold_volumes();
    open(&mut app, Section::Voice);
    assert!(shows(&app, "Voice compressor"));
    assert!(!app.layout.voice.voice_compressor, "off unless set");
    let voice = |event| Message::Settings(Event::Voice(event));
    let _ = app.update(voice(VoiceEvent::VoiceCompressor(true)));
    assert!(backcord_core::volume::voice_compressor());
    assert!(
        crate::layout::Layout::read(folder.path())
            .voice
            .voice_compressor
    );
    let _ = app.update(voice(VoiceEvent::VoiceCompressor(false)));
    assert!(!backcord_core::volume::voice_compressor());
    assert!(
        !crate::layout::Layout::read(folder.path())
            .voice
            .voice_compressor
    );
}

/// The stereo widener and the panning under echo reduction: off and in
/// the middle as they start, kept once set.
#[test]
fn the_stereo_widener_and_panning_start_off_and_centred_and_are_kept() {
    let (mut app, folder) = preview();
    let _volumes = crate::layout::tests::hold_volumes();
    open(&mut app, Section::Voice);
    assert!(!app.layout.voice.microphone_wide, "off unless set");
    assert_eq!(app.layout.voice.microphone_pan, None, "in the middle");
    let voice = |event| Message::Settings(Event::Voice(event));
    let _ = app.update(voice(VoiceEvent::MicWide(true)));
    assert!(
        crate::layout::Layout::read(folder.path())
            .voice
            .microphone_wide
    );
    // Dragged three steps left, then let go.
    let _ = app.update(voice(VoiceEvent::MicPan(7, false)));
    assert_eq!(app.layout.voice.microphone_pan, Some(-30));
    let _ = app.update(voice(VoiceEvent::MicPan(7, true)));
    assert_eq!(
        crate::layout::Layout::read(folder.path())
            .voice
            .microphone_pan,
        Some(-30)
    );
    let _ = app.update(voice(VoiceEvent::MicPan(10, true)));
    assert_eq!(
        crate::layout::Layout::read(folder.path())
            .voice
            .microphone_pan,
        None
    );
    // The intensity: three quarters unless set, in 5% steps, kept.
    assert_eq!(app.layout.voice.microphone_width, None);
    let _ = app.update(voice(VoiceEvent::MicWidth(18, true)));
    assert_eq!(
        crate::layout::Layout::read(folder.path())
            .voice
            .microphone_width,
        Some(90)
    );
    // The bit depth: 32 bit float unless set; the slider's 0 is 1 bit.
    assert_eq!(app.layout.voice.microphone_bits, None);
    let _ = app.update(voice(VoiceEvent::MicBits(7, true)));
    assert_eq!(
        crate::layout::Layout::read(folder.path())
            .voice
            .microphone_bits,
        Some(8)
    );
    let _ = app.update(voice(VoiceEvent::MicBits(31, true)));
    assert_eq!(app.layout.voice.microphone_bits, None, "32 bit float");
    // The sample rate: 48 kHz unless set; 8 kHz kept.
    assert_eq!(app.layout.voice.microphone_rate, None);
    let _ = app.update(voice(VoiceEvent::MicRate(3, true)));
    assert_eq!(
        crate::layout::Layout::read(folder.path())
            .voice
            .microphone_rate,
        Some(8_000)
    );
}

/// The high-pass and the gate on others' voices: off as they start, the
/// gate in dB as it's dragged, both kept.
#[test]
fn the_high_pass_and_the_voice_gate_start_off_and_are_kept() {
    let (mut app, folder) = preview();
    let _volumes = crate::layout::tests::hold_volumes();
    open(&mut app, Section::Voice);
    assert!(shows(&app, "High-pass filter (100 Hz)"));
    assert!(shows(&app, "Voice gate"));
    assert!(!app.layout.voice.voice_high_pass, "off unless set");
    assert_eq!(app.layout.voice.voice_gate, None, "off unless set");
    let voice = |event| Message::Settings(Event::Voice(event));
    let _ = app.update(voice(VoiceEvent::VoiceHighPass(true)));
    assert!(backcord_core::volume::voice_high_pass());
    let _ = app.update(voice(VoiceEvent::VoiceGate(21, false)));
    assert!(shows(&app, "-50 dB"), "the slider follows the drag");
    assert_eq!(backcord_core::volume::voice_gate(), Some(-50.0));
    let _ = app.update(voice(VoiceEvent::VoiceGate(21, true)));
    let stored = crate::layout::Layout::read(folder.path()).voice;
    assert!(stored.voice_high_pass);
    assert_eq!(stored.voice_gate, Some(-50));
    let _ = app.update(voice(VoiceEvent::VoiceGate(0, true)));
    let _ = app.update(voice(VoiceEvent::VoiceHighPass(false)));
    assert_eq!(backcord_core::volume::voice_gate(), None);
    assert!(!backcord_core::volume::voice_high_pass());
    let stored = crate::layout::Layout::read(folder.path()).voice;
    assert!(!stored.voice_high_pass);
    assert_eq!(stored.voice_gate, None);
}

/// The Dynamic Island: off unless turned on, and then only with a Spotify
/// linked and something to show; Spotify's song in its player.
#[test]
fn the_dynamic_island_shows_spotify_only_when_on_and_linked() {
    let (mut app, _folder) = preview();
    let song = backcord_core::chat::NowPlaying {
        title: "Europe (Hardtekk)".into(),
        artists: "Gxnrc, DJ Yamazora".into(),
        cover: None,
        playing: true,
        likeable: true,
        liked: Some(false),
    };
    change(&mut app, |state| {
        state.spotify_linked = true;
        state.spotify = Some(song.clone());
    });
    assert!(!crate::island::shown(&app), "off unless turned on");
    assert!(!shows(&app, "Europe (Hardtekk)"));
    open(&mut app, Section::Appearance);
    let _ = app.update(Message::Settings(Event::Pref(
        crate::settings::Pref::Island(true),
    )));
    assert!(
        shows(&app, "Glow intensity"),
        "its look's settings show once on"
    );
    let _ = app.update(Message::Settings(Event::Close));
    assert!(crate::island::shown(&app));
    assert!(shows(&app, "Europe (Hardtekk)"));
    assert!(shows(&app, "Gxnrc, DJ Yamazora"));
    assert_eq!(crate::island::tabs(&app), [crate::island::Tab::Player]);
    change(&mut app, |state| state.spotify_linked = false);
    assert!(!crate::island::shown(&app), "only with a Spotify linked");
    change(&mut app, |state| {
        state.spotify_linked = true;
        state.spotify = None;
    });
    assert!(!crate::island::shown(&app), "nothing playing and no call");
}
