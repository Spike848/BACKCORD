# Settings

Settings (`settings/`) is a card over the window dimmed by 70%, 72 px in
from its
edges and at most 1400 px wide, centred; on its left, on the chat's ground, the account with Edit Profile,
a search that keeps the sections whose names hold it, the sections under
Account, Experience and App Settings (Discord and Game Activity only
outside the preview), and which build this is, which a click copies; on
the right the section's title and the close button over its page, a
column at most 696 px wide that starts at its top for each section. The
account card's gear opens it on the section it last showed (My Account at
first), a device menu's Voice Settings on Voice & Video. Edit Profile
closes it under the profile editor. Escape closes an open dropdown, then stops recording a
key, then Settings; so does a click outside or the close button.

It's a screen on the contract in [Screens](structure.md#screens): its
state (`Settings`)
lives in the app's slot only while it's open, and closing it lets go of
the theme being edited, the camera being tried, the gate's meter and the
recording. Each change is kept at once in
`layout.json` under its own key (`layout::Prefs` and
`layout::Voice` read them) and handed to the core.

- My account (`pages/`, a file a page, with the preferences' changes in
  its `mod.rs`): the account's card, where its login is kept,
  Reconnect while offline, Edit profile, the custom status, and Sign out
  (Back to sign in in the preview).
- Voice & video (`voice.rs`): the microphone, speakers and camera, each a
  dropdown of the devices the core lists (listed again as the page shows),
  the microphone's and speakers' volumes, which play while dragged and are
  kept when let go; Discord's mic test (Let's check), which runs the
  gate's meter with playback and lights its notches to the level, with
  Discord's captions (read as they're drawn), and the call's deafening
  while it runs (see [Voice](../voice.md#microphone-test)); the camera's
  preview, filmed only while Test video
  is on (`camera:settings` among the window's video sources, at 400 by
  225); under the mic test the input mode, and either the gate's sensitivity
  over the microphone's level as the gate hears it (read as it's drawn,
  twenty times a second, only while it shows) or push to talk's keybind and release delay;
  then in Discord calls the soundboard's volume (Discord
  keeps it; the slider follows a drag and what it's left at is sent),
  studio mode, noise suppression and its strength, and echo reduction,
  the last two applying in studio mode too; after the
  camera, joining unmuted, the entrance sound for every server or one (Choose, with
  Nitro, opens the sounds Discord has as the call's soundboard does,
  `soundboard::sound_board`, a click choosing one) and, on Windows,
  Windows' own effects. Where no global key works
  (the root crate's `keybinds` says why), the window's own key events
  hold it while the window has the focus: an event
  filter hands each key's scan code to the root crate with no message.
  What the watcher tells of where keybinds work comes from its thread as
  a message (`Message::Keybinds`), kept by the root (`keybinds.rs`) for
  the note under the key whether Settings show or not.
  Record keybind records it as the Keybinds page records one (below).
- Keybinds (`keybinds.rs`), as Discord's page: Add a Keybind, and each
  keybind on the sunken ground with its action (Discord's six), its keys
  with Record keybind, a remove button, Discord's words for what the
  action does, and what the desktop gave it while the desktop runs them;
  over them, where keybinds work. Recording takes the keys pressed
  together, wherever the keys are, up to four, showing them as they're
  pressed, and ends once they're all up again (or the focus goes);
  Escape, pressed first, stops without changing anything; on Windows
  Mouse 4 and 5 record too. Each key is its scan code as winit gives it
  (iced's key codes are winit's by name), its name and its keysym, as the
  core's `keybinds::Key` keeps it, and the keybinds are kept in
  `layout.json` as `keybinds`. While one records, keybinds rest (the root
  crate's `pause`). The root crate's watcher watches push to talk's and
  the page's as one list, started again as they change; a toggle's press
  comes as a message and mutes, deafens or switches the input mode as
  its button does. [Keybinds](../keybinds.md) has the rest.
- Messages: a message as the clock and text size show it; the text size
  (which the timeline's text takes), the clock and its seconds, the date's
  order (message times are worked out again for each), when GIFs and emoji
  play, link previews (hidden ones leave "A card, hidden while link
  previews are off." where a card was the whole message), long messages
  folding, boost gems in the member list, sharing that this account types
  (off, the core's typing commands go nowhere), mentioning those replied
  to and tracking taken out of links.
- Notifications (desktop notifications on or off, then the sounds: the
  volume, the preset playing, which plays a sound when chosen, New preset,
  and for a preset its name and Delete preset; each sound's own volume with
  its play button, a button choosing a file for it, one going back to
  Discord's, and the file it plays) and Advanced.
  Advanced holds Start when you sign in, Open Discord
  links in BACKCORD (outside the preview), Close to tray, saying where
  there's no tray, and the renderer's switch: while it differs from the
  renderer drawing, when it takes effect and Restart BACKCORD.
- Appearance (`appearance.rs`): each theme as a small window in its own
  colours, as many 150 px or wider across as fit; a click shows one and
  keeps it. New theme opens the editor on a copy of the theme shown (Start
  from takes another, and its name unless it was renamed); each custom
  theme's pencil edits it. A colour's field shows its change at once, or
  says "Not a colour"; Save keeps the theme among the person's own and
  chooses it, Cancel (or leaving Appearance or Settings) shows the theme
  from before, and a saved one's Delete asks once more. Then the system
  title bar's switch, where BACKCORD can draw its own, and the profile
  switches, which the core is told of.
- Blocked accounts (`Section::Discord` in the code): linking, as the
  sign-in screen's panel, while none is linked; then Unlink in a build
  with Matrix, blocked people's messages hidden, and those blocked, with
  their pictures, five more at a time, each with Unblock. Reads marked on
  Discord are among Messages' switches, and Show Hidden Channels among
  Extra Features. Whether someone is
  using BACKCORD (`away.rs`): a key, a click or the
  wheel counts; once nobody has for 15 s, Discord hears since when, and
  goes AFK, so the phone gets notifications, never showing idle, and moving
  pictures hold still until someone is back. Input while someone is using
  it costs no message, only a time noted; one timer checks it.
- Game Activity: sharing the game played (Discord keeps it), and the games
  named by hand, added, edited and removed, which detection takes at once.
- Updates (Aggiornamento in Italian; the About page in the code): the build with Copy version, the update switches, the releases
  (Updates in [the window](window.md#the-window-and-the-desktop)), and
  the links.

A setting that came with an update has a NEW tag after its name, and its
page's row in the list one too, until Settings leaves that page; what was
seen is kept in `layout.json` (`seen_new`), with the version a fresh
install has met (`first_version`). Which settings are tagged and when one
is new is the core's (`backcord_core::new_tags`); `form::new_tag` draws
it.

## Extra Features

Extra Features (`pages/extras.rs`), under its own group at the foot of the
column, has a card for each feature, its switch with what it does and its
own choices under it while it's on, each kept in `layout.json`. All of
them but Anti-Disconnect, Fake Nitro and Badge Spoofer start on, Message Logger with Log my own
messages, Silent Typing (not sharing this account's typing) and No Push To
Talk included, and Stream Proof's button sits in the header bar, with
Local Blur and Auto Stream Proof on:

- Emulate Platforms (`platform_indicators`, `emulated_platform`,
  `emulation_depth`): where someone is online shows beside their name in
  the member list, a message's author line and a profile's username line
  (`Ctx::platforms` in `looks.rs`), a monitor, phone, globe, gamepad or VR
  headset for each platform, in its status's colour, its name on a
  tooltip. Its dialog chooses the platform this account says it's on, none
  by default, in two rows, and how much of it it says, low, medium or high;
  [people](../discord/people.md#where-someone-is-online) has what each says.
- Anti-Disconnect (`anti_disconnect`, off by default) and Follow
  (`follow_voice`): see [voice](../discord/voice.md#anti-disconnect-and-follow).
  Follow adds Follow, ticked while following them, to the menus of another
  Discord user, after Call: theirs in the member list, by a message, as a
  friend, and their voice menu in a channel's list or on a call's tile;
  turned off, it stops following.

- Clickable Roles (`clickable_roles`): a role's chip on a profile opens the
  card of who holds it (`info_cards.rs`), as a role mention does.
- Permissions Viewer (`permissions_viewer`): a member's menu in a server
  offers Permissions, a server's, channel's and category's menus My
  Permissions, and a role's card a Permissions button, each opening its
  window (`permissions.rs`, the layer `Permissions`); see
  [servers](../discord/servers.md#permissions-viewer).
- View Icons (`view_icons`, `icon_format`, `icon_size`): a large profile's
  avatar and any profile's banner open large (`icon_viewer.rs`), and the
  menus of a person and a server offer View Avatar and View Icon. A
  picture on Discord's CDN is asked for at the format and size chosen; a
  moving one as a GIF.
- Stream Proof (`stream_proof*`, `stream_proof.rs`): a button on the chat
  bar or the header turns it on and off. While on, Local Blur shows
  messages as "Hidden by Stream Proof." until shown one by one, Blackout
  Stream leaves the window out of captures on Windows
  (`SetWindowDisplayAffinity`), and Auto Stream Proof follows this
  account's Go Live.
- Message Logger (`message_logger`, the core's `extras.rs`): a deleted
  message stays in a loaded channel, red, with "(deleted)", and an edited
  one shows what it said before each edit above its text.
- Silent Typing: this account's typing is Messages' `share_typing`; Hide
  others typing (`hide_others_typing`) drops the typing line.
- No Push To Talk (`no_push_to_talk`): on, a channel that denies Use Voice
  Activity is spoken in as any other; off, push to talk is required there,
  as Discord's client requires it (`push_to_talk::set_required`).
- Message Link Previews (`message_link_previews`): a card under a message
  for each Discord message it links to (`message/linked.rs`); see
  [message link cards](../discord/messages.md#message-link-cards). Off, the
  core is told (`Action::HideMessageLinks`) and looks none up.
- Relationship Notifier (`relationship_notifier`, and `relationship_friends`,
  `_requests`, `_groups`, `_servers` and `_offline` for its dialog's
  switches, each on by default): the removals it tells of reach the core as
  `extras::set_relationship_alerts`, none while it's off. Its alerts list
  in the inbox's Friend Alerts tab (`window_bar.rs`), with Clear alerts; see
  [relationship alerts](../discord/people.md#relationship-alerts).
- Fake Nitro (`fake_nitro`, off by default, and `fake_nitro_size`,
  `fake_nitro_masked` and `fake_nitro_received` for its dialog): its
  emoji size (32 to 160 px, 48 by default), Use masked links and Show
  received emoji links as emoji, both on, reach the core as
  `extras::set_fake_nitro`. On, the emoji and sticker pickers unlock what
  this account may not use, which goes as a link to its picture; see
  [Fake Nitro](../discord/messages.md#fake-nitro).
- Badge Spoofer (`badge_spoofer`, off by default, and for its dialog
  `badge_spoofer_badges`, a bit per badge ticked, `badge_spoofer_house`,
  `_nitro` and `_boost`, none by default, and `badge_spoofer_real`, on):
  its dialog ticks badges two to a row and chooses the house, Nitro level
  and boost level in rows of choices, and scrolls in a short window. They
  reach the core as `Action::SpoofBadges` (`Prefs::badge_spoofer`), at
  start too; see [Badge Spoofer](../discord/people.md#badge-spoofer).
