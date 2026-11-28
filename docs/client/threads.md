# Forums and threads

## Forums

A Discord forum shows in place of a channel's messages and composer
(`forum.rs`): a search of its posts'
titles in the header's search box (`search::search_field`, its Clear
showing the posts again) and New Post; under them Sort & View, a
`Popover` menu of Sort by (Recent Activity, Creation Date) and View as
(List, Gallery), the forum's tags as pills that show only the posts with
those lit (`filter_row`), Clear, and Edit Tags where the core says this
account may (`Application::may_edit_forum_tags`); then its posts as cards (tags, title with NEW, who
wrote the first message and how it starts, the top reaction or the
forum's own, replies, how long ago, the first picture), or in a gallery
as 260 px tiles in rows that wrap (`post_tile`), the active ones
first and older ones a page at a time as the list nears its end. The
core sorts and filters them (`ForumView`'s `sort`, `gallery` and
`tags`), so the cards only follow `ForumPosts`. Edit Tags
(`tags_editor_view`) shows in the list's place, as New Post does: a row
for each tag with its emoji, a field for its emoji (only an emoji pasted
or picked takes, and taking a character away clears it), its name, the
Moderated box and Delete; Add Tag, and Save. The custom emoji pictures
of the forum's tags are kept with the cards' (`forum_pictures`). A click
opens a post beside the forum, Shift and a click in its place. New Post
shows the form in the list's place (`new_post_view`): the title, tags from
a menu of up to five, the forum's post guidelines folding away, and the
first message with files in a `thread::SideComposer`; Post makes it and
opens it beside, and closing with something written asks first. The cards
are built again only when what they show changed, and go as the forum
closes.

A post opened in place has its bar under the header (`post_bar_view`):
React to post, opening the picker under itself for the post's first
message once that has loaded, then at the right Follow or Following,
which joins or leaves it where the core says it may
(`Application::thread_followed`), and a button that copies its link.
What the bar shows is kept as the window syncs (`Post::bar`): a room
that isn't a post costs one look, and a post changes it in place.

## Threads

A thread opens beside its channel (`thread.rs`), 420 px wide until its
edge is dragged (`widgets::Splitter`)
to between 320 and 800 px, never more than leaves the chat 420 px; the
width is kept in `layout.json` as it's let go, and a double click on the
edge puts it back. It opens: from a
message's thread chip (its name, how many messages, a chevron), View
Thread in its menu, or a listed thread opened with Shift. Its head is the
thread's icon and name, More (the thread's own menu: Leave thread, Mute
thread and Notification settings, Copy link and the rest, as the
sidebar's), Open in full view and Close. A thread's notification levels
have no default row: `place_menus::Notify` checks the one that applies. Its
messages are the channel's timeline's rows numbered from
`timeline::ASIDE`, so a message's look, menu, hover bar (two quick
reactions here), spoilers and selection work the same on either side;
selecting stays on one side. It loads earlier messages as its top comes
into view, tells the core which of its messages show, and its composer
(`thread::SideComposer`) has its own draft, reply bar, files and Send.
Closing it, or the core letting go of it, drops its rows.

Keys nothing took (`composer::type_anywhere`) go on the end of the
composer written in last, the
thread's from when it opens or the channel's, which then has the keys:
text, Space, Backspace and Delete, without Ctrl; Tab only gives it the
keys, and Enter doesn't send. Not under a card, a popup, the picker or a
full view, nor while the talk key is held in a call; while stepping
through the messages only a letter or digit leaves them. Escape that
nothing took cancels, in Discord's order, the search, an edit, the
channel's reply, then the thread's reply or the New Thread form.

Create Thread, in a message's menu or the composer's + menu, opens the
New Thread form beside the channel: the message it starts from as a
found card
(`found::Found`), the thread's name (the message's text by default), the
Private Thread switch where the server allows one, and the first
message's composer, Create Thread sending both. Leaving it with something
written asks first (`thread::leave_question`).
