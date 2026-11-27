# Rail and sidebar

The rail (`rail.rs`) holds, from the top: the DM home (the logo; its
badge counts DM mentions and friend
requests), the DMs with unread messages, a line, then the servers and
folders, then the "Add a server" tile after the last of them, which opens Join a Server (a plus
cut from a disc on a raised ground, a step toward the rail's at rest; it has no menu and is
there while Discord is connected or in the preview). Each tile has its pill at the
rail's edge (open, pointed at, unread), its mentions in a red badge, a
speaker at its top right, and its name in a tooltip;
a server out of reach in a Discord outage shows faded, its tooltip "Name
(unavailable in a Discord outage)".
The speaker is green while this account is in a call there, and grey
while others are in a Discord server's voice channels. It follows
Discord's server list (GuildMediaStateStore): a screen when someone
shares theirs, else a camera when someone has theirs on, else a speaker;
in the call this account is in, a screen only for its own share. Those
counted are in voice channels this account may see, other than Stages,
the inactive (AFK) channel and muted channels (Discord leaves out only
a channel muted itself, not one in a muted category), and aren't
blocked or ignored. A muted server shows none unless this account is in
a call there.
Under the pointer, a server with people in voice has a card for its
tooltip, made then and only for it: its name, then each busy voice
channel's name, by a screen when someone in it shares theirs, else a
speaker, over its people's overlapping faces, the first four channels
and six faces each, with how many more. Discord's card puts everyone in
one row; by channel, it says who is together. A muted server still lists
them, and so does the card list the people of muted channels and Stages
(only the inactive channel and hidden ones are left out), though they
don't count on the speaker; the faces it shows are fetched as the server
is pointed at. Someone seen joining voice in one server leaves whatever
other server's channel they were listed in, which a large server that
isn't subscribed may never have said; a bot stays in each. A server
pointed at is subscribed to as one opened is (`Action::PeekServer`, op 37
once a session), so its calls' comings and goings arrive live rather than
as Discord's occasional sums (PASSIVE_UPDATE), and a server back from an
outage shows its calls as its GUILD_CREATE says, not as they were.
Ctrl and a click picks a server, ringed in the accent, or lets it go, and
a box dragged from the rail's margins (or anywhere with Ctrl) picks every
server it touches. A right click on one of several picked opens their
menu: how many, then Mark as read, Mute server and Unmute server for all
of them.
Whatever is picked (people, DMs, messages, servers) goes with a left
click let go anywhere that doesn't take it, without Ctrl; a box or a drag
stops when another press comes, the window loses its focus or the pointer
leaves it, so nothing stays held in mid-air.
A folder opens and closes on a click, its servers on a strip of its
colour; closed it shows its first four servers' icons, and the speaker
only for the call this account is in, as Discord's does.
A right click on a server, a folder or an unread DM opens its menu, the
one its row in the sidebar opens; a folder's marks its servers read while
they aren't, and always offers Folder colour (`folder_colour.rs`, its own
layer): a dialog with the folder's name, Discord's preset colours as
swatches, the kit's colour pad with the colour's code under it
(`form::colour_card`), a tile of how the folder will look, Reset (Discord's
default colour), Cancel and Save, greyed until something changed. Enter in
either field saves; Escape or a click outside closes it.

The sidebar's right edge and the member list's left are each a kit
`Splitter` laid over the panel's edge (`app/view.rs`, `members.rs`): a drag
sets `sidebar_width` or `members_width` in the layout, within the panel's
limits and what leaves the conversation 420 px, and letting go writes it;
a double click unsets it. Everything that places itself by either panel
reads `sidebar::width`, `members::width` or `conversation::navigation`,
never the panels' own widths.

The sidebar (`sidebar/`: its rows' model in `rows.rs`, the top, channels,
voice and DM rows each a file) shows, on the DM
home, the search field (which keeps the DMs whose names hold its text,
and empties as the rail goes elsewhere),
Friends and Requests (rows 38 px tall, in the navigation greys), "Direct
Messages" and each DM with its person's
presence, the voice channel they're in (a click goes to it, in its
server) and what they're up to; a Discord DM's row, under the pointer or
while selected, has a close button (an x), which closes the DM as its menu's
Close DM does, or for a group asks to leave it as Leave Group does; in a
server its name, which opens the server's menu, then its pinned channels,
then each category's heading and channels. A category folds on a click,
keeping the open channel, unread ones and busy voice channels, as
Discord's does; under the pointer its heading offers a "+" that makes a
channel in it, where this account may. A channel's row shows its kind's
icon, its unread mark at the sidebar's edge, its mentions, and muted or
locked faint; a forum's row counts its unread posts in a bold number
where a text channel's mentions go, and its badge the mentions in its posts
(the row alone: the server's badge counts each post already); a channel's
row under the pointer or while selected offers Create invite, to that
channel, for those who may invite; a busy voice channel lists who is in it (the call this
account is in, in the call's order: this account first; this account in
the call from another client dimmed until pointed at, without its marks,
its tooltip "Connected on another client") and how long the
call has gone on, which a timer set for the next whole minute brings up
to date only while one shows (the same timer as open polls', the
Activity panel's cards' and Active Now's); under the pointer, a card beside its row names who is there
(how many, who is live or filming, the first eight and how many more), put
away by a click or its menu until the pointer leaves. A voice channel with a chat offers Open chat on its row under
the pointer or while selected, which opens the chat beside the call
(`call/stage`) without joining, as Discord's: a panel at the window's
right, 400 px (less where the call would keep under 320), with the
channel's name, a close button, its messages and its composer, the stage
narrower beside it. The header's chat button opens and closes it, lit
while it shows. A right click on
a row opens its menu (`place_menus.rs`), built from the core's
`menu_choices`, at the pointer, each row with its icon. A server's
menu mutes it (for a time, in a submenu, or until it's turned back on
with a click on the row itself) or unmutes it, saying until when; its
Notification settings' submenu picks the level and ticks Suppress
@everyone and @here and Suppress all role @mentions; Hide muted channels
and Show all channels are boxes ticked in place. A channel's or category's
mutes it the same way and picks its level (or its category's or server's
default, named under it); the header's bell opens the open channel's
mute and levels under it, 6 px down from its left edge. Pin and Unpin, Add to and Remove from the channel list,
Unfollow category, Join and Leave thread, Invite, Create channel and
category, Duplicate and Delete, and Leave server go to the core, those
that ask first or need more opening their dialog (see
[dialogs](dialogs.md)). A menu
taller than the window scrolls, kept inside it. A
DM's menu mutes it for a time and, for its person, offers what their own
menu does, with Invite to server and Apps in submenus.

In a Discord server, channels and categories drag to move them
(`arrange.rs`): each category and
each channel showing outside a folded one has a place, as Discord's client
lists them (pinned channels and threads stay put). A row pressed and moved
4 px (`widgets::Drags`) starts a drag where the core lets this account
move it; a press let go sooner is a click. While it's dragged, a faint copy
follows the pointer, the rows it can't land among dim (all under a
category, voice and text channels under each other), the green line
(`drop_line_view`) marks where it would land, across the top of the row it's
over when dragged up and the foot when dragged down, and the list scrolls
8 px each 16 ms while the pointer is within 40 px of its ends. Let go over
a place it may land on, the core moves it there; a channel moved into a
category whose permissions it could take asks first, "Sync permissions?",
Sync Permissions or Keep Current Permissions, closing it leaving the
channel where it was.

Someone listed under a voice channel drags the same way onto another
voice channel of the server, which moves them as their menu's Move to
does (`Application::move_to`: this account joins it, someone else is
moved there), where Move to offers one. While they're dragged the channels
it offers stay bright and the rest of the list dims, the one under the
pointer is outlined in the same green, they follow the pointer just below
it, and the list scrolls near its ends; let go anywhere else, beside the
list included, or Escape, and nothing moves. The drag stops once no
channel listed takes them, as when another list shows. One of several
picked with Ctrl and a click takes them all along, as their menu's Move to.
The list stays where it's scrolled as any drag starts and ends: what's
dragged is a layer over it, in a stack that's always there. The servers on
the rail don't drag.

On the DM home (`sidebar/folders.rs`), DMs pinned from their menu (Pin,
Unpin) come first, then the DM folders, each a heading (a chevron, a folder,
its name or "New folder", how many DMs it lists, their mentions) over its
DMs, set in 12 px, then the rest. A click on a folder's heading folds it,
keeping the open DM; its menu renames it, in a field in its place (Enter
keeps the name), or deletes it, its DMs staying. A DM's menu offers New
folder after Pin while it's in none, and Remove from folder while it's in
one. A DM is in one place at a time: pinned, in one folder, or loose.
A box dragged with Ctrl held, from anywhere on the sidebar list
(`widgets::marquee`), picks every DM it touches, as it grows, adding to
those picked already; without Ctrl a press drags, as below. Ctrl and a click picks a DM, outlined in the accent, or lets it go; a click
without Ctrl, or a right click on one not picked, ends the picking. A
right click on one of several picked opens their menu: how many, then Mark
as read, Pin, Unpin, New folder and Remove from folder, as they fit. A DM
pressed and moved 4 px drags, the others picked with it if it's one of
them, its face and how many more following the pointer; what it would land
on is outlined in green. Let go on a folder's heading, they go in it; on a
DM in a folder, in that one; on a pinned DM, pinned; on another DM, in a
new folder with it, as servers dropped on each other; on the "Direct
Messages" heading, out of their folders and unpinned. A new folder opens
its name's field. Pins and folders are the core's (`dm_folders.rs`), kept
in `layout.json` as `pinned` and `dm_folders`.

A server's top: with a banner, the picture with a
shade over its top and the name and chevron in white on it, the banner
asked for only while the server is open; Invite people beside the name for
those who may invite; under it, while the server shows its Boost progress
bar, the Boost Goal pill (its boosts of 33, filled in pink). The banner
stays behind the header and the list's top, and the list scrolls over it
as it fades out, as Discord's; once it's gone the header has its line.
The list's first rows, which scroll with the channels: room for the
banner, then, on a server with events or one where this account may
create them, the "N Events" row, Channels & Roles where the server has
onboarding questions to answer again, and Browse Channels in a Community
server. The events row's menu lists
each event with "Happening now" or when it starts and where, or "No
upcoming events" greyed; an event's row does nothing. A verified
server's name has Discord's green badge before it
(`Look::drawn`, an icon in its own colours), a Community server's the
Community badge, and another at Boost level 1 to 3 has the level, its
number in a scallop, before the name.

The account's card (`account.rs`) spans the foot of both: the avatar with
its decoration and presence, in a green ring while the microphone carries
voice in a call (without a Discord presence, in the preview or a Matrix
build, a dot instead: the accent in the preview, green online, grey
offline), the name and status (the username under the
pointer), with the account's nameplate filling the card (`PlateRow::account`:
.4 strong, .6 and playing under the pointer, its palette from .1 to .4), and
the mic, headphones and Settings. The avatar and the name open the
status's popover once Discord is linked. The mic and headphones mute and
deafen in a call, greyed while it joins or leaves; out of
one they set whether the next call starts unmuted or deafened. The carets
beside the mic and headphones open the input and output menus, and a muted
or deafened one sits on a dark ground with a faint seam before its caret. Settings
open on the section they last showed (`Window::settings_section`), My
Account at first. While
BACKCORD is offline (not in the preview), Reconnect stands before Settings
and connects again at once (`Action::RetrySync`), greyed while a sync
runs. The call's
dock sits above the card while this account is in a call (see
[calls](calls.md)).

In a voice channel someone is in, each person's row shows them speaking
(a green ring), muted or deafened, and LIVE while they stream, which
watches the stream on a click, joining the call first. A click on the
row opens their card at the pointer, and a right click their voice menu.

Open folders, folded categories and Developer Mode (which shows Copy ID)
are kept in the data folder's `layout.json` (`layout.rs`), and so is what
the core counts of this device's use, handed back to it as the window
opens (`Layout::remember`, `Layout::keep_uses`): each server's channel
last open, which a click on the server opens again, how often each
reaction was added, and the stickers sent and kept. The last channels
are handed back again whenever a backend starts (`Client::started`), as
signing out forgets them. Each change
sets its one key in the file as
it's kept in memory, and a thread of its own writes it a second after the
changes stop (reading it, and the window ending, write what waits first),
through a file beside it renamed over it (`backcord_core::layout_file`).
A key that doesn't read loses
only itself, and a file that isn't a layout is kept aside as
`layout.json.unread` rather than overwritten.

Browse Channels (`browse_channels.rs`),
from the top of a Community server's list, takes the conversation's place:
a search of the channels' names, then each category's heading with its
Follow category switch and its channels, each with its icon, name, topic,
View, Open or Join, which does what a click on the channel in the list
does (Join joins a voice channel's call), and a box ticked while the
channel list follows it; a
click on the row ticks it or clears it, unless its category follows it.

## Typing in the list

A channel's row, and a DM's or a group's, shows small moving dots while
someone else types in it, from the same `AppState::typing` the typing line
above the composer reads: the core keeps every conversation's typers, each
for 10 seconds unless they type again, and no longer forgets the others on
switching channels. Typing comes for DMs and for the servers opened this
session. Silent Typing's Hide others typing hides the dots too.
