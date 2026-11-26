//! Several messages picked with Ctrl and a click, as people are in a voice
//! channel's list (`crate::picks`): their rows lit, and one menu, a right
//! click on any of them, acting on them all: their text or links copied,
//! those this account may delete deleted after one question. A click
//! without Ctrl, Escape or another channel lets go of them.

use std::sync::Arc;

use backcord_core::application::Action;
use iced::{Point, Task, widget::operation};

use crate::{
    app::{App, Message, NOBODY},
    menus::{Menu, MenuAct, MenuRow},
    pictures::Icon,
};

/// What the menu of several messages does with them.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MessagePick {
    CopyText,
    CopyLinks,
    /// This account's own of them, of those it may delete.
    DeleteOwn,
    /// All it may delete: its own, and others' where it may manage
    /// messages.
    Delete,
    Clear,
}

/// Whether `message` is one the menu deletes: `own` alone, or any this
/// account may delete.
fn deletes(message: &backcord_core::chat::ChatMessage, own: bool) -> bool {
    message.can_delete && !message.deleted && (message.own || !own)
}

/// Whether the message `item` is picked.
pub fn picked(timeline: &crate::timeline::Timeline, item: &str) -> bool {
    timeline.picked.iter().any(|pick| &**pick == item)
}

/// Picks a message with the rest, or lets it go if picked. Those no longer
/// shown (another channel's) go first.
pub fn toggle(app: &mut App, item: Arc<str>) {
    let shown: Vec<Arc<str>> = app
        .timeline
        .picked
        .iter()
        .filter(|pick| crate::timeline::row_of(app, pick).is_some())
        .cloned()
        .collect();
    app.timeline.picked = shown;
    match app.timeline.picked.iter().position(|pick| *pick == item) {
        Some(at) => {
            app.timeline.picked.remove(at);
        }
        None => app.timeline.picked.push(item),
    }
}

/// A box dragged over the channel's rows, `area` in the list's viewport:
/// the messages it touches picked, with those picked before it when Ctrl
/// was held as it started. Let go of, it's done.
pub fn boxed(app: &mut App, area: iced::Rectangle, done: bool) {
    let control = app.modifiers.control();
    let timeline = &mut app.timeline;
    let base = timeline
        .boxing
        .get_or_insert_with(|| {
            if control {
                timeline.picked.clone()
            } else {
                Vec::new()
            }
        })
        .clone();
    // The box among the rows: past where the view starts, less the room
    // a short history is held down by.
    let lead = (timeline.height - timeline.total_height()).max(0.0);
    let shift = timeline.view_top() - lead;
    let (top, bottom) = (area.y + shift, area.y + area.height + shift);
    let mut picked = base;
    for (index, message) in timeline.messages.iter().enumerate() {
        let (Some(&at), Some(&height)) = (timeline.tops.get(index), timeline.heights.get(index))
        else {
            break;
        };
        if at >= bottom {
            break;
        }
        if at + height > top && !picked.iter().any(|pick| **pick == *message.item) {
            picked.push(Arc::from(message.item.as_str()));
        }
    }
    timeline.picked = picked;
    if done {
        timeline.boxing = None;
    }
}

/// The messages picked that still show, as they're ordered in the timeline.
fn shown(app: &App) -> Vec<Arc<backcord_core::chat::ChatMessage>> {
    let lanes = crate::timeline::lanes(app);
    let aside = lanes.aside.map(|aside| aside.messages.iter());
    lanes
        .timeline
        .messages
        .iter()
        .chain(aside.into_iter().flatten())
        .filter(|message| picked(&app.timeline, &message.item))
        .cloned()
        .collect()
}

/// A right click on a message: on one of several picked, the menu of them
/// all; on any other, the picking ends and its own menu opens.
pub fn open_if_picked(app: &mut App, item: &str, at: Point) -> Option<Task<Message>> {
    if app.timeline.picked.len() >= 2 && picked(&app.timeline, item) {
        let messages = shown(app);
        if messages.len() >= 2 {
            return Some(open_menu(app, &messages, at));
        }
    }
    app.timeline.picked.clear();
    None
}

/// The menu of the messages picked: how many, then what acts on them all.
fn open_menu(
    app: &mut App,
    messages: &[Arc<backcord_core::chat::ChatMessage>],
    at: Point,
) -> Task<Message> {
    let count = messages.len();
    let deletable = messages
        .iter()
        .filter(|message| deletes(message, false))
        .count();
    let own = messages
        .iter()
        .filter(|message| deletes(message, true))
        .count();
    let act = MenuAct::PickedMessages;
    let mut rows = vec![MenuRow {
        disabled: true,
        icon: Some(Icon::Check),
        ..MenuRow::plain(
            act(MessagePick::Clear),
            backcord_iced_kit::i18n::t(format!("{count} messages picked")),
        )
    }];
    if messages.iter().any(|message| !message.body.is_empty()) {
        rows.push(MenuRow {
            separated: true,
            ..MenuRow::new(
                act(MessagePick::CopyText),
                backcord_iced_kit::i18n::tr("Copy text"),
                Icon::Copy,
            )
        });
    }
    if messages
        .iter()
        .any(|message| app.client.state().message_link(&message.item).is_some())
    {
        rows.push(MenuRow::new(
            act(MessagePick::CopyLinks),
            backcord_iced_kit::i18n::tr("Copy Message Links"),
            Icon::Chain,
        ));
    }
    // Deleting this account's own of them; and, where it may delete
    // others' too (managing messages), all of them.
    if own > 0 {
        rows.push(MenuRow {
            danger: true,
            separated: true,
            icon: Some(Icon::Trash),
            ..MenuRow::plain(
                act(MessagePick::DeleteOwn),
                backcord_iced_kit::i18n::t(format!("Delete my messages ({own})")),
            )
        });
    }
    if deletable > own {
        rows.push(MenuRow {
            danger: true,
            separated: own == 0,
            icon: Some(Icon::Trash),
            ..MenuRow::plain(
                act(MessagePick::Delete),
                backcord_iced_kit::i18n::t(format!("Delete all ({deletable})")),
            )
        });
    }
    rows.push(MenuRow {
        separated: deletable == 0,
        ..MenuRow::new(
            act(MessagePick::Clear),
            backcord_iced_kit::i18n::tr("Deselect all"),
            Icon::X,
        )
    });
    app.win.menu = Some(Menu::new(at, rows));
    operation::focus(NOBODY)
}

/// A row of the menu of the messages picked; each ends the picking. Delete
/// asks once for them all, or under Shift deletes them without asking.
pub fn choose(app: &mut App, pick: MessagePick) -> Task<Message> {
    let messages = shown(app);
    app.timeline.picked.clear();
    let focus = crate::composer::refocus(app);
    match pick {
        MessagePick::CopyText => {
            let text = messages
                .iter()
                .filter(|message| !message.body.is_empty())
                .map(|message| format!("{}: {}", message.author, message.body))
                .collect::<Vec<_>>()
                .join("\n");
            Task::batch([iced::clipboard::write(text), focus])
        }
        MessagePick::CopyLinks => {
            let state = app.client.state();
            let links = messages
                .iter()
                .filter_map(|message| state.message_link(&message.item))
                .map(|(link, _)| link)
                .collect::<Vec<_>>()
                .join("\n");
            Task::batch([iced::clipboard::write(links), focus])
        }
        MessagePick::Delete | MessagePick::DeleteOwn => {
            let own = pick == MessagePick::DeleteOwn;
            let items: Vec<String> = messages
                .iter()
                .filter(|message| deletes(message, own))
                .map(|message| message.item.clone())
                .collect();
            if app.modifiers.shift() {
                for item in items {
                    app.client.dispatch(Action::DeleteNow(item));
                }
                return Task::batch([app.sync(), focus]);
            }
            app.client.dispatch(Action::RequestDeleteMany(items));
            app.confirm_focus = true;
            Task::batch([app.sync(), operation::focus(NOBODY)])
        }
        MessagePick::Clear => focus,
    }
}
