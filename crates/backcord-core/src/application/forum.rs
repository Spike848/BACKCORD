//! A Discord forum, which shows its posts in place of messages, as Discord's
//! client shows one: its active posts, pinned ones first, then its older,
//! archived ones a page at a time, or the posts a search of their titles
//! finds. Each post shows as a card with its first message. The active posts
//! are the server's, as its snapshot shares them; their first messages are
//! asked for ten at a time, for the cards near the top and then for more as
//! the list scrolls, as Discord's client asks for those it shows. Archived
//! pages and a search's results bring their own. All of it is kept only
//! while the forum shows.

use std::collections::HashMap;

use super::*;
use crate::chat::{ForumTag, ForumTagEdit, ListedThread, PostStarter, PostStarters, ShownEmoji};
use threads::ThreadList;

/// The most tags a forum has, and the longest a tag's name may be, as
/// Discord takes them.
pub const FORUM_TAGS: usize = 20;
pub const FORUM_TAG_NAME: usize = 50;

/// How a forum's posts are sorted, as its Sort & View menu offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ForumSort {
    /// By their latest message, Discord's Recent Activity.
    Activity,
    /// By when they were made, Discord's Creation Date.
    Creation,
}

/// A change to how the forum on screen shows its posts, from its header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ForumViewChange {
    Sort(ForumSort),
    /// As a gallery of tiles, or with false a list.
    Gallery(bool),
    /// Shows only posts with this tag, by id, or any other chosen; chosen
    /// again, no longer.
    Tag(String),
    ClearTags,
}

/// What Edit Tags was given, for the forum on screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ForumTagsInput {
    Open,
    /// A tag's name, its emoji as Unicode text (none when empty), and
    /// whether only moderators give it, by its place in the list.
    Name(usize, String),
    Emoji(usize, String),
    Moderated(usize, bool),
    Add,
    Remove(usize),
    Save,
    Close,
}

/// The forum's tags as Edit Tags changes them, while it's open.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ForumTagsEditor {
    pub tags: Vec<ForumTagEdit>,
    /// Discord is taking them.
    pub saving: bool,
    pub error: Option<String>,
}

/// How many cards want their first messages at first, and how many more
/// each time the list scrolls near its end: a page.
const CARDS: usize = 25;
/// How many posts' first messages one request asks for, as Discord's client
/// asks for them.
const STARTERS_PER_ASK: usize = 10;

/// The forum on screen.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ForumView {
    /// Its archived posts and a search, as a channel's list of threads keeps
    /// them.
    pub list: ThreadList,
    /// Posts' first messages, by room id; none where it was deleted.
    pub starters: HashMap<String, Option<PostStarter>>,
    /// Posts whose first messages are on their way.
    asking: Vec<String>,
    /// Asking failed: it waits for the list to scroll before asking again.
    stalled: bool,
    /// How many cards, from the top, want their first messages.
    shown: usize,
    /// The last message read here when it opened, a snowflake: posts made
    /// after it, which this account never opened, are new.
    read: u64,
    /// How its header sorts and lays out its posts, where chosen, else as
    /// the forum's defaults say; and the tags it shows only posts with.
    pub sort: Option<ForumSort>,
    pub gallery: Option<bool>,
    pub tags: Vec<String>,
    /// Edit Tags, while it's open.
    pub editor: Option<ForumTagsEditor>,
}

/// A forum's posts as it shows them.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ForumPosts {
    /// Its active posts, pinned ones first, then by their latest message,
    /// or by when they were made, as the forum sorts them.
    pub active: Vec<ForumPost>,
    /// Older, archived ones, by their latest message.
    pub older: Vec<ForumPost>,
    /// What a search of their titles found, while one shows: `None` while
    /// it's on its way, or why it failed.
    pub found: Option<Option<Result<Vec<ForumPost>, String>>>,
    /// They're sorted by when they were made, and show as a gallery.
    pub by_creation: bool,
    pub gallery: bool,
    /// Only posts with the tags chosen show.
    pub filtered: bool,
}

/// A post as its card shows it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ForumPost {
    pub thread: ListedThread,
    /// Its tags, in the forum's order.
    pub tags: Vec<ForumTag>,
    /// Its first message: `None` until it comes, `Some(None)` when it was
    /// deleted.
    pub starter: Option<Option<PostStarter>>,
    /// Made since the forum was last read, and never opened.
    pub new: bool,
}

/// What a forum says of its posts, besides them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ForumInfo {
    pub tags: Vec<ForumTag>,
    pub by_creation: bool,
    /// A post must take one of its tags.
    pub require_tag: bool,
    /// The emoji a post without reactions offers.
    pub reaction: Option<chat::ShownEmoji>,
    /// Its posts show as a gallery by default.
    pub gallery: bool,
}

impl ForumView {
    /// A page of posts, or first messages, is on its way.
    pub fn loading(&self) -> bool {
        self.list.loading || !self.asking.is_empty()
    }

    /// Scrolling further wants more: of `cards`, some beyond those that
    /// have asked for their first messages, or older posts still to load.
    pub fn wants_more(&self, cards: usize) -> bool {
        self.list.search.is_none() && (self.list.more || cards > self.shown)
    }
}

impl AppState {
    /// The Discord forum on screen.
    pub fn forum(&self) -> Option<&ForumView> {
        self.forum
            .as_ref()
            .filter(|forum| self.selected.as_ref() == Some(&forum.list.room))
    }
}

impl Application {
    /// Shows a forum, by room id, in place of messages: its posts, and the
    /// first messages of those on top. Discord sends its active posts, and
    /// any made while it shows, with the server's threads.
    pub(super) fn show_forum(&mut self, room: String) -> Vec<Effect> {
        let archived = self.may(&room, chat::Permission::ReadHistory);
        let (list, mut effects) = ThreadList::open(room.clone(), archived);
        self.state.forum = Some(ForumView {
            list,
            shown: CARDS,
            read: self.forum_read(&room),
            ..ForumView::default()
        });
        effects.push(Effect::Backend(Command::Discord(
            DiscordCommand::ViewForum { room },
        )));
        effects.extend(self.ask_starters());
        effects
    }

    /// Forgets the forum, as another room shows.
    pub(super) fn close_forum(&mut self) -> Vec<Effect> {
        match self.state.forum.take() {
            Some(_) => vec![Effect::Backend(Command::Discord(
                DiscordCommand::CloseThreads,
            ))],
            None => vec![],
        }
    }

    /// Searches the posts' titles, as Enter in the forum's search does; an
    /// empty query shows the posts again.
    pub(super) fn search_posts(&mut self, query: String) -> Vec<Effect> {
        match self.state.forum.as_mut() {
            Some(forum) => forum.list.search(query),
            None => vec![],
        }
    }

    /// Wants another page of cards, as the list does when scrolled near its
    /// end: their first messages, and older posts once the active ones run
    /// out.
    pub(super) fn more_posts(&mut self) -> Vec<Effect> {
        let active = self.forum_posts().map_or(0, |posts| posts.active.len());
        let Some(forum) = self.state.forum.as_mut() else {
            return vec![];
        };
        forum.shown += CARDS;
        forum.stalled = false;
        let mut effects = vec![];
        if forum.shown > active {
            effects.extend(forum.list.more());
        }
        effects.extend(self.ask_starters());
        effects
    }

    /// Takes a page of posts for the forum, if it's the one it asked for,
    /// with their first messages.
    pub(super) fn forum_page_came(
        &mut self,
        room: String,
        query: Option<String>,
        offset: u32,
        result: Result<(Vec<ListedThread>, bool), String>,
        starters: PostStarters,
    ) -> Vec<Effect> {
        let Some(forum) = self
            .state
            .forum
            .as_mut()
            .filter(|forum| forum.list.room == room)
        else {
            return vec![];
        };
        forum.starters.extend(starters);
        forum.list.came(query, offset, result);
        self.ask_starters()
    }

    /// Takes first messages for the forum's cards, and asks for the next
    /// ones they want.
    pub(super) fn posts_came(
        &mut self,
        room: String,
        result: Result<PostStarters, String>,
    ) -> Vec<Effect> {
        let Some(forum) = self
            .state
            .forum
            .as_mut()
            .filter(|forum| forum.list.room == room)
        else {
            return vec![];
        };
        let asked = std::mem::take(&mut forum.asking);
        match result {
            Ok(starters) => {
                // One Discord didn't answer for was deleted.
                for post in asked {
                    forum.starters.entry(post).or_insert(None);
                }
                forum.starters.extend(starters);
            }
            Err(error) => {
                tracing::debug!(%error, "a forum's first messages didn't come");
                forum.stalled = true;
            }
        }
        self.ask_starters()
    }

    /// Asks for the first messages the cards on top want, unless some are
    /// on their way: the posts the server's snapshot knows, as archived
    /// pages and searches bring their own.
    pub(super) fn ask_starters(&mut self) -> Vec<Effect> {
        let wanted: Vec<String> = match self.state.forum.as_ref() {
            Some(forum) if forum.asking.is_empty() && !forum.stalled => {
                let posts = self.forum_posts().unwrap_or_default();
                posts
                    .active
                    .iter()
                    .chain(&posts.older)
                    .take(forum.shown)
                    .filter(|post| post.starter.is_none())
                    .take(STARTERS_PER_ASK)
                    .map(|post| post.thread.room.clone())
                    .collect()
            }
            _ => return vec![],
        };
        let Some(forum) = self.state.forum.as_mut().filter(|_| !wanted.is_empty()) else {
            return vec![];
        };
        forum.asking.clone_from(&wanted);
        vec![Effect::Backend(Command::Discord(DiscordCommand::Posts {
            room: forum.list.room.clone(),
            posts: wanted,
        }))]
    }

    /// The forum's posts as it shows them, while one shows.
    pub fn forum_posts(&self) -> Option<ForumPosts> {
        let forum = self.state.forum()?;
        let info = self.forum_info(&forum.list.room).unwrap_or_default();
        let sections = self.sections(&forum.list);
        let post = |thread: &ListedThread| ForumPost {
            tags: info
                .tags
                .iter()
                .filter(|tag| thread.tags.contains(&tag.id))
                .cloned()
                .collect(),
            starter: forum.starters.get(&thread.room).cloned(),
            new: !thread.archived && !thread.opened && created(thread) > forum.read,
            thread: thread.clone(),
        };
        let by_creation = forum
            .sort
            .map_or(info.by_creation, |sort| sort == ForumSort::Creation);
        // Only those with a tag chosen, as Discord's tag filter matches some.
        let tagged = |thread: &&ListedThread| {
            forum.tags.is_empty() || thread.tags.iter().any(|tag| forum.tags.contains(tag))
        };
        let mut active: Vec<_> = sections
            .joined
            .iter()
            .chain(&sections.other)
            .filter(tagged)
            .map(post)
            .collect();
        let order = |post: &ForumPost| match by_creation {
            true => created(&post.thread),
            false => post.thread.last,
        };
        active.sort_by_key(|post| (!post.thread.pinned, std::cmp::Reverse(order(post))));
        let mut older: Vec<_> = sections.older.iter().filter(tagged).map(post).collect();
        if by_creation {
            older.sort_by_key(|post| std::cmp::Reverse(created(&post.thread)));
        }
        let found = forum.list.search.as_ref().map(|search| {
            search.found.as_ref().map(|found| {
                found
                    .clone()
                    .map(|found| found.iter().filter(tagged).map(post).collect())
            })
        });
        Some(ForumPosts {
            active,
            older,
            found,
            by_creation,
            gallery: forum.gallery.unwrap_or(info.gallery),
            filtered: !forum.tags.is_empty(),
        })
    }

    /// What the forum, by room id, says of its posts.
    pub fn forum_info(&self, room: &str) -> Option<ForumInfo> {
        #[cfg(feature = "discord")]
        if let Some((server, Some(channel), _)) = self.discord_place(room)
            && let Some(forum) = server
                .server
                .channel(channel)
                .and_then(|channel| channel.forum.as_deref())
        {
            return Some(ForumInfo {
                tags: discord::forum_tags(forum),
                by_creation: forum.by_creation,
                require_tag: forum.require_tag,
                reaction: forum.reaction.as_ref().map(discord::shown_emoji),
                gallery: forum.gallery,
            });
        }
        let _ = room;
        None
    }

    /// Sorts, lays out or filters the forum's posts, as its header does;
    /// cards that come into view ask for their first messages.
    pub(super) fn change_forum_view(&mut self, change: ForumViewChange) -> Vec<Effect> {
        let room = match self.state.forum() {
            Some(forum) => forum.list.room.clone(),
            None => return vec![],
        };
        let known = self.forum_info(&room).unwrap_or_default().tags;
        let Some(forum) = self.state.forum.as_mut() else {
            return vec![];
        };
        match change {
            ForumViewChange::Sort(sort) => forum.sort = Some(sort),
            ForumViewChange::Gallery(gallery) => forum.gallery = Some(gallery),
            ForumViewChange::Tag(tag) => {
                if let Some(at) = forum.tags.iter().position(|chosen| *chosen == tag) {
                    forum.tags.remove(at);
                } else if known.iter().any(|known| known.id == tag) {
                    forum.tags.push(tag);
                }
            }
            ForumViewChange::ClearTags => forum.tags.clear(),
        }
        self.ask_starters()
    }

    /// Whether this account may change a forum's tags, by its room id: with
    /// Manage Channels there, while Discord is connected.
    pub fn may_edit_forum_tags(&self, room: &str) -> bool {
        self.state.discord_online
            && !self.state.preview
            && self.may(room, chat::Permission::ManageChannels)
    }

    /// Edit Tags for the forum on screen: opened with its tags as Discord
    /// has them, changed, then saved whole, as Discord's client saves them.
    pub(super) fn forum_tags_input(&mut self, input: ForumTagsInput) -> Vec<Effect> {
        let Some(room) = self.state.forum().map(|forum| forum.list.room.clone()) else {
            return vec![];
        };
        if input == ForumTagsInput::Open {
            if !self.may_edit_forum_tags(&room) {
                return vec![];
            }
            let tags = self.forum_tag_edits(&room);
            if let Some(forum) = self.state.forum.as_mut() {
                forum.editor = Some(ForumTagsEditor {
                    tags,
                    ..ForumTagsEditor::default()
                });
            }
            return vec![];
        }
        let Some(forum) = self.state.forum.as_mut() else {
            return vec![];
        };
        if input == ForumTagsInput::Close {
            forum.editor = None;
            return vec![];
        }
        let Some(editor) = forum.editor.as_mut().filter(|editor| !editor.saving) else {
            return vec![];
        };
        match input {
            ForumTagsInput::Name(at, name) => {
                if let Some(tag) = editor.tags.get_mut(at) {
                    tag.name = name.chars().take(FORUM_TAG_NAME).collect();
                }
            }
            ForumTagsInput::Emoji(at, emoji) => {
                if let Some(tag) = editor.tags.get_mut(at) {
                    let emoji = emoji.trim();
                    tag.emoji_id = None;
                    tag.emoji_name = (!emoji.is_empty()).then(|| emoji.to_owned());
                    tag.shown = tag.emoji_name.clone().map(|text| ShownEmoji {
                        text,
                        picture: None,
                    });
                }
            }
            ForumTagsInput::Moderated(at, moderated) => {
                if let Some(tag) = editor.tags.get_mut(at) {
                    tag.moderated = moderated;
                }
            }
            ForumTagsInput::Add => {
                if editor.tags.len() < FORUM_TAGS {
                    editor.tags.push(ForumTagEdit::default());
                }
            }
            ForumTagsInput::Remove(at) => {
                if at < editor.tags.len() {
                    editor.tags.remove(at);
                }
            }
            ForumTagsInput::Save => {
                let tags: Vec<ForumTagEdit> = editor
                    .tags
                    .iter()
                    .map(|tag| ForumTagEdit {
                        name: tag.name.trim().to_owned(),
                        ..tag.clone()
                    })
                    .collect();
                if tags.iter().any(|tag| tag.name.is_empty()) {
                    editor.error = Some("Every tag needs a name.".into());
                    return vec![];
                }
                editor.saving = true;
                editor.error = None;
                return vec![Effect::Backend(Command::Discord(
                    DiscordCommand::ForumTags { room, tags },
                ))];
            }
            ForumTagsInput::Open | ForumTagsInput::Close => {}
        }
        vec![]
    }

    /// Discord took the forum's tags, which close Edit Tags, or says why not.
    pub(super) fn forum_tags_saved(&mut self, room: &str, result: Result<(), String>) {
        let Some(forum) = self
            .state
            .forum
            .as_mut()
            .filter(|forum| forum.list.room == room)
        else {
            return;
        };
        match result {
            Ok(()) => forum.editor = None,
            Err(error) => {
                if let Some(editor) = forum.editor.as_mut() {
                    editor.saving = false;
                    editor.error = Some(error);
                }
            }
        }
    }

    /// A forum's tags, by room id, as Edit Tags starts from them.
    fn forum_tag_edits(&self, room: &str) -> Vec<ForumTagEdit> {
        #[cfg(feature = "discord")]
        if let Some((server, Some(channel), _)) = self.discord_place(room)
            && let Some(forum) = server
                .server
                .channel(channel)
                .and_then(|channel| channel.forum.as_deref())
        {
            use backcord_discord::forum::Emoji;
            return forum
                .tags
                .iter()
                .map(|tag| {
                    let (emoji_id, emoji_name) = match &tag.emoji {
                        Some(Emoji::Custom { id, name }) => (Some(id.clone()), Some(name.clone())),
                        Some(Emoji::Unicode(name)) => (None, Some(name.clone())),
                        None => (None, None),
                    };
                    ForumTagEdit {
                        id: Some(tag.id.clone()),
                        name: tag.name.clone(),
                        moderated: tag.moderated,
                        emoji_id,
                        emoji_name,
                        shown: tag.emoji.as_ref().map(discord::shown_emoji),
                    }
                })
                .collect();
        }
        let _ = room;
        Vec::new()
    }

    /// The last message read in a forum, by room id, as a snowflake.
    fn forum_read(&self, room: &str) -> u64 {
        #[cfg(feature = "discord")]
        if let Some((server, Some(channel), _)) = self.discord_place(room)
            && let Some(channel) = server.channel(channel)
        {
            return channel.marks.last_read;
        }
        let _ = room;
        0
    }
}

/// When a post was made, as a snowflake: its room id's.
fn created(thread: &ListedThread) -> u64 {
    thread
        .room
        .rsplit(':')
        .next()
        .and_then(|id| id.parse().ok())
        .unwrap_or_default()
}
