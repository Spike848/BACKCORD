//! Several people picked with Ctrl and a click, in a voice channel's list
//! or the member list: their rows lit, and one menu, a right click on any
//! of them, acting on them all, as the core's `picks.rs` offers.

use backcord_core::application::{Action, MenuChoice, PersonAsk};
use iced::{Point, Task, widget::operation};

use crate::{
    app::{App, Message, NOBODY},
    controls::Check,
    menus::{Menu, MenuAct, MenuRow},
};

/// Whether `user` is picked.
pub fn picked(state: &backcord_core::application::AppState, user: &str) -> bool {
    state.picked.iter().any(|pick| pick.user == user)
}

/// A click on someone's row in `place`: with Ctrl, picks them or lets them
/// go; without, ends any picking, and the click does what it does.
pub fn click(app: &mut App, place: Option<String>, user: &str) -> Option<Task<Message>> {
    if app.modifiers.control()
        && let Some(place) = place
    {
        app.client.dispatch(Action::Pick {
            place,
            user: user.to_owned(),
        });
        return Some(app.sync());
    }
    if !app.client.state().picked.is_empty() {
        app.client.dispatch(Action::ClearPicks);
    }
    None
}

/// A right click on someone: on one of several picked, the menu of them
/// all; on anyone else, the picking ends and their own menu opens.
pub fn open_if_picked(app: &mut App, user: &str, at: Point) -> Option<Task<Message>> {
    let state = app.client.state();
    if state.picked.len() >= 2 && picked(state, user) {
        return Some(open_menu(app, at));
    }
    if !state.picked.is_empty() {
        app.client.dispatch(Action::ClearPicks);
    }
    None
}

/// The menu of everyone picked: how many, then what acts on them all, a
/// checkbox ticked when it's on for every one.
fn open_menu(app: &mut App, at: Point) -> Task<Message> {
    let core = &app.client.app;
    let state = app.client.state();
    let count = state.picked.len();
    let all = |on: &dyn Fn(&str, &str) -> bool| {
        state.picked.iter().all(|pick| on(&pick.place, &pick.user))
    };
    let mut rows = vec![MenuRow {
        disabled: true,
        ..MenuRow::plain(
            MenuAct::Picked(MenuChoice::CopyId),
            backcord_iced_kit::i18n::t(format!("{count} people picked")),
        )
    }];
    let mut group = None;
    for choice in core.picked_choices() {
        let ticked = match choice {
            MenuChoice::LocalMute => Some(all(&|_, user| backcord_core::volume::muted(user))),
            MenuChoice::SoundboardMute => Some(all(&|_, user| {
                backcord_core::volume::soundboard_muted(user)
            })),
            MenuChoice::ServerMute => Some(all(&|place, user| {
                core.voice_person(place, user)
                    .is_some_and(|person| person.server_muted)
            })),
            MenuChoice::ServerDeafen => Some(all(&|place, user| {
                core.voice_person(place, user)
                    .is_some_and(|person| person.server_deafened)
            })),
            _ => None,
        };
        let starts = crate::place_menus::group_of(choice);
        let separated = group.replace(starts).is_some_and(|last| last != starts) || rows.len() == 1;
        let mut row = MenuRow {
            icon: Some(crate::place_menus::icon(choice)),
            danger: crate::person_menu::danger(choice),
            separated,
            check: ticked.map_or(Check::None, Check::Box),
            ..MenuRow::plain(MenuAct::Picked(choice), crate::place_menus::label(choice))
        };
        if choice == MenuChoice::MoveTo {
            row.submenu = core
                .picked_move_targets()
                .into_iter()
                .map(|(room, name)| MenuRow::plain(MenuAct::MovePicked, name).key(room))
                .collect();
        }
        rows.push(row);
    }
    app.win.menu = Some(Menu {
        location: crate::person_menu::CONTEXT_MENU,
        ..Menu::new(at, rows).width(crate::menus::FIELD_WIDTH)
    });
    operation::focus(NOBODY)
}

/// A row of the menu of everyone picked: a checkbox's new state goes to
/// each, a timeout, a kick, a ban, an unfriending or a block asks once for
/// all, and the rest act on each as their own menus would. Anything but a
/// checkbox ends the picking.
pub fn choose(app: &mut App, choice: MenuChoice, ticked: Option<bool>) -> Task<Message> {
    let picked = app.client.state().picked.clone();
    let ask = match choice {
        MenuChoice::Timeout => Some(PersonAsk::Timeout),
        MenuChoice::Kick => Some(PersonAsk::Kick),
        MenuChoice::Ban => Some(PersonAsk::Ban),
        MenuChoice::Unfriend => Some(PersonAsk::Unfriend),
        MenuChoice::Block => Some(PersonAsk::Block),
        _ => None,
    };
    if let Some(ask) = ask {
        app.client.dispatch(Action::AskPicked(ask));
        let focus = app.person.focus(app.client.state());
        return Task::batch([app.sync(), focus]);
    }
    let checkbox = matches!(
        choice,
        MenuChoice::LocalMute
            | MenuChoice::SoundboardMute
            | MenuChoice::ServerMute
            | MenuChoice::ServerDeafen
    );
    let mut tasks = Vec::new();
    for pick in picked {
        tasks.push(match choice {
            MenuChoice::LocalMute
            | MenuChoice::SoundboardMute
            | MenuChoice::ServerMute
            | MenuChoice::ServerDeafen
            | MenuChoice::Disconnect => crate::call::popups::person::choose_voice(
                app, pick.place, pick.user, choice, ticked,
            ),
            MenuChoice::RemoveTimeout => {
                app.client.dispatch(Action::ChangeMember {
                    place: pick.place,
                    user: pick.user,
                    change: backcord_core::chat::MemberChange::Timeout {
                        until_ms: None,
                        reason: String::new(),
                    },
                });
                Task::none()
            }
            _ => crate::person_menu::choose_person(
                app,
                pick.user,
                crate::person_menu::CONTEXT_MENU,
                choice,
                None,
            ),
        });
    }
    if !checkbox {
        app.client.dispatch(Action::ClearPicks);
    }
    tasks.push(app.sync());
    Task::batch(tasks)
}

/// Everyone picked moved to the voice channel `to`, then the picking ends.
pub fn move_picked(app: &mut App, to: &str) -> Task<Message> {
    for action in app.client.app.move_picked(to) {
        app.client.dispatch(action);
    }
    app.client.dispatch(Action::ClearPicks);
    app.sync()
}
