//! The cards under messages for the Discord links their text has: an
//! invite's server ([`InviteCard`], in `join.rs`) and a linked message
//! ([`MessageCard`]), as Vencord's MessageLinkEmbeds shows one. Each is
//! looked up once, as a message linking to it first shows, and kept by its
//! invite's code or its message's id.

use super::*;
use crate::discord::MessageLink;

/// The card under a message for a Discord message it links to, by the
/// linked message's id in [`AppState::message_cards`].
#[derive(Clone, Debug, PartialEq)]
pub enum MessageCard {
    /// Asked of Discord, which hasn't answered yet.
    Loading,
    /// The message, as a pinned message or a search's result shows it.
    Found(Arc<ChatMessage>),
    /// Gone, or in a channel this account can't read.
    Unavailable,
}

impl Application {
    /// Looks up what the links of the messages on screen lead to, each
    /// once whatever shows it later: the invites' servers (see
    /// [`AppState::invite_cards`]) and, unless they're hidden, the
    /// messages (see [`AppState::message_cards`]). A linked message already
    /// in the timeline or the thread beside is taken from there. Like the
    /// pictures on screen, the timeline is gone through only when it, or
    /// what shows of it, changed, and the thread beside when it changed.
    pub(super) fn request_link_cards(&mut self) -> Vec<Effect> {
        if !self.state.discord_online || self.state.preview {
            return vec![];
        }
        let state = &self.state;
        let timeline = if Arc::ptr_eq(&self.carded_timeline, &state.messages) {
            &[][..]
        } else {
            visible_range(&state.messages, &self.visible_messages)
                .map_or(&[][..], |range| &state.messages[range])
        };
        let aside = state
            .aside
            .as_ref()
            .filter(|aside| !Arc::ptr_eq(&self.carded_aside, &aside.messages));
        let linking: Vec<&Arc<ChatMessage>> = timeline
            .iter()
            .chain(aside.into_iter().flat_map(|aside| aside.messages.iter()))
            .filter(|message| message.notice.is_none())
            .collect();
        let codes: Vec<String> = linking
            .iter()
            .flat_map(|message| discord::invite_codes(&message.body))
            .collect();
        let links: Vec<MessageLink> = if state.hide_message_links {
            Vec::new()
        } else {
            linking
                .iter()
                .flat_map(|message| discord::message_links(&message.body))
                .collect()
        };
        self.carded_timeline = self.state.messages.clone();
        if let Some(aside) = &self.state.aside {
            self.carded_aside = aside.messages.clone();
        }
        let mut effects = Vec::new();
        for code in codes {
            if self.state.invite_cards.contains_key(&code) {
                continue;
            }
            self.state
                .invite_cards
                .insert(code.clone(), InviteCard::Loading);
            effects.push(Effect::Backend(Command::Discord(
                DiscordCommand::ResolveInvite { code, typed: None },
            )));
        }
        for link in links {
            if self.state.message_cards.contains_key(&link.message) {
                continue;
            }
            let room = link.room();
            if let Some(loaded) = self.state.loaded_message(&room, &link.message) {
                self.state
                    .message_cards
                    .insert(link.message, MessageCard::Found(loaded));
                continue;
            }
            self.state
                .message_cards
                .insert(link.message.clone(), MessageCard::Loading);
            effects.push(Effect::Backend(Command::Discord(
                DiscordCommand::LinkedMessage {
                    room,
                    id: link.message,
                },
            )));
        }
        effects
    }

    /// What came of looking up a linked message: its card shows it, or
    /// that it isn't there.
    pub(super) fn linked_message(
        &mut self,
        id: &str,
        result: Result<Option<Box<ChatMessage>>, String>,
    ) {
        let Some(card) = self.state.message_cards.get_mut(id) else {
            return;
        };
        *card = match result {
            Ok(Some(message)) => MessageCard::Found(Arc::from(message)),
            Ok(None) | Err(_) => MessageCard::Unavailable,
        };
    }

    /// Message link cards show, or go: those on screen are looked up as
    /// they show again.
    pub(super) fn hide_message_links(&mut self, hide: bool) -> Vec<Effect> {
        self.state.hide_message_links = hide;
        if hide {
            return vec![];
        }
        self.carded_timeline = Arc::new(Vec::new());
        self.carded_aside = Arc::new(Vec::new());
        self.request_link_cards()
    }
}

impl AppState {
    /// The Discord message `id` of `room`, when the timeline or the thread
    /// beside has it loaded.
    fn loaded_message(&self, room: &str, id: &str) -> Option<Arc<ChatMessage>> {
        let item = format!("d:{id}");
        let messages = if self.selected.as_deref() == Some(room) {
            &self.messages
        } else {
            &self
                .aside
                .as_ref()
                .filter(|aside| aside.room == room)?
                .messages
        };
        messages
            .iter()
            .find(|message| message.item == item && !message.deleted)
            .cloned()
    }
}
