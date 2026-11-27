# Calls

The call's views and what they do are in `call/`.

- A voice channel this account isn't in shows its name, who is there and
  Join voice (Switch voice channel while in another call), or why it can't
  be joined. A click on the channel in the sidebar joins it without
  showing it, as Discord's client does; a second click, or the dock's
  channel line, shows the call.
- The stage (`call/stage/`; `layout.rs` places the tiles with no window
  in it)
  lays the tiles out in a grid, or one large with the rest in a row under
  it; a click on a tile makes it large, and a click beside it the grid
  again. The tiles scroll when there are more than fit. Each tile is a
  `tile::Tile`: a camera, a stream or the avatar (a camera shows the
  avatar until its first frame, and keeps its last if frames stop), a green ring while the
  person speaks, their name and marks, LIVE and how many watch. Someone's
  stream is always a tile of its own beside theirs, as Discord's. One not
  watched shows Discord's preview of it, stretched from a small picture and
  darkened, behind Watch Stream, which watches it alone, and an eye that
  watches it beside the others (`call::ask_previews` asks for the previews
  every four minutes). A watched one shows the quality its streamer chose
  (`720P 30FPS`, `SOURCE 60FPS`: the stream server's `max_resolution` and
  `max_framerate`, the highest said, at Discord's preset that holds them)
  and LIVE; on the large tile, or under the pointer, its sound on or off,
  its menu (Stop Watching, Mute, its volume) and the full-screen button.
  A double click goes full screen too, the stage over the whole window and the window
  full screen on the desktop until Escape, the button, a double click, or
  the tile or the call going (`call::fit_screen`, after every update). A
  window the desktop had full screen already stays so after. Closed to
  the tray meanwhile, the tile is back in the call and the next window
  opens as a window; brought forward while full screen, it stays so. A
  right click opens the person's voice menu.
- The controls under the stage, in this order: mute and
  deafen (each with its device menu), the camera (its menu, from this button
  and the one under the call alike: the camera, its quality, on or off), Go Live, the soundboard,
  Room mode's square while that's on (see [settings](settings.md)), and
  while sharing "Blur stream" (`Command::BlurStream`: the worker blurs each
  frame whole, scaled down and back up, before its preview and the
  encoder, so viewers see it hidden while the stream goes on; amber with
  "Stream blurred" while on, off each time sharing starts), and leave.
  A narrow stage takes them in (`call_controls`): the blur button its
  glyph alone and the buttons closer, then the red ones round, then the
  full-screen button at the right gone.
  Over the call full screen (the theatre and the large tile alike) the
  same blur button, leave and exit sit at the bottom right, and the
  Dynamic Island hides, so nothing covers the stream's top.
  At their right, in a voice channel or a DM, Full screen call (`Event::Theatre`)
  puts the call over the whole screen (`theatre_view`, a layer of
  `call_layers`): its tiles alone, no rail, sidebar, header or controls,
  the window full screen on the desktop as for the large tile. The same
  button at the tiles' bottom right, Escape (its layer is the last, so a
  reply or the picker in its chat goes first), leaving the call or opening
  another view go back. While its chat is closed, a button at the tiles'
  top right opens it, beside them, the tiles narrowing to make room: a
  Discord voice channel's as beside its call, a DM's its messages
  (`Event::TheatreChat`, kept only while full screen).
- The device menus (`call/popups/`: the device menus, the share menu,
  the soundboard and someone's voice menu each a file): the device to use (one chosen that
  went away shows as not connected), its volume, which plays while it's
  dragged and is kept in `layout.json` when let go, and on the input side
  the microphone's level, measured only while the menu shows and read
  as it's drawn, and the
  noise suppression kind. For a Discord account, the microphone's has
  Fake Mute and the speaker's Fake Deafen (`Event::Fake`), each ticked
  while on: see [voice](../discord/voice.md#fake-mute-and-fake-deafen).
- Go Live's menu lists the screens with a picture of each (from a stream
  worker started for them and gone once they're in), the windows, the
  system's own dialog where the portal picks, and the cameras, then the
  stream's mode and its gear, whose compact quality picker
  (`call/popups/quality.rs`, the camera menu's too) shows the resolutions,
  144p to 1080p and the source, as two rows of small buttons and the frame
  rates, 15, 30 and 60, as a third, and the encoding; it fits a short window by
  scrolling, keeping its button in view. It opens once they're listed (on
  a thread of their own); meanwhile Escape, a second click, another popup
  or leaving the call cancels it, and a listing cancelled opens nothing. While live, the same menu
  switches what's shared; where the system's dialog is its only choice
  (the portal picks and no camera is connected), Switch screen or window
  opens the dialog with no menu.
- The soundboard lists the server's sounds and the defaults in rows of
  three, each played on a click, previewed from its speaker under the
  pointer and starred as a favourite.
- Someone's voice menu (`menus.rs` with the person's choices) has their
  user and stream volumes above its rows, which keep the core's order
  (the person menu's sorts them), and checkboxes for muting them
  for this account, muting their soundboard, and on Discord the server's
  mute and deafen; one's own has mute and deafen. Move to opens the voice
  channels they may be moved to (this account moves by joining). Volumes
  play while dragged and are kept when let go, as are those muted.
- The dock (`call/dock.rs`) at the sidebar's foot: the connection's state
  and the call's time, the channel line (underlined under the pointer)
  that shows the call, noise suppression's toggle, which turns back on the
  kind last used, leave, and while sharing its state with a stop; the
  soundboard, camera and Go Live buttons, and beside them the received
  quality limit (`call/popups/limit.rs`, lit while one is set): the most
  lines (every resolution the quality pickers send at, 2160p for the
  source's 4K down to 144p) and frames a second (60, 30, 15) taken from the
  call's cameras and the streams watched, kept in `layout.json` as
  `receive_height` and `receive_fps` (`voice::set_receive_limit`).
  Cameras are asked for (op 15) at the ladder's highest step within it, so
  their senders film smaller; streams come at their streamer's quality and
  are shown no larger and no oftener (`Control::Limit`), every frame
  still decoded. Then an error, with Dismiss.
- The connection's details (`call/details.rs`): once the call measures
  its connection, its signal (the
  dock's, and the header's in the call's conversation) shows the last ping
  as its tooltip, and a click on it or the dock's status opens the details
  under it, or above without room: Connection, a graph of the last five
  minutes of pings with its top and middle marked, the average and last
  ping and the packets lost; Privacy, while the call is end-to-end
  encrypted with someone else in it, the voice privacy code in groups of
  five with Copy; Stream, while this account's screen is live, its
  encoder, codec, size, bitrate sent and aimed for, keyframes asked for and
  packets resent; and how the call is secured. What they show is copied
  out of the call only while they or the tooltip show, again each second.
  Their words and graph are the core's `call_details`.
- A DM's call (`call/dm.rs`) shows above its messages, taking more room
  while a tile is large: the stage and controls while in it, else who is
  in it with Join the call, or Accept and Decline while it rings. A call
  ringing in a DM that isn't open shows a card at the window's top, its
  ring round the caller pulsing once a second (`call::dm::Pulse`, drawn
  anew 30 times a second only while it rings), and
  switching calls asks first (Escape cancels, Enter confirms). The DM's
  header starts a call or a video call, its buttons greyed while the call
  can't be joined (Discord offline, another call joining or leaving).
- While the call isn't on screen, the stream watched, this account's
  screen or a camera floats over the window (`call/corner.rs`), at first
  in the top right, its title and stop under the pointer; a click returns
  to the call, a drag moves it anywhere right of the channel list and
  under the header, and its edges and corners resize it, keeping its
  shape. Let go over a voice channel's Join and who is in it, it moves
  the least way clear of them.
- Sounds play as the core says (joining, leaving, muting, someone
  streaming), and a DM's call rings while it's incoming.
- Push to talk and the other keybinds (the root `src/keybinds.rs`,
  started by `keybinds.rs`, which runs with Settings closed): the
  keybinds kept in `layout.json` are watched through the
  desktop portal's global shortcuts, X11 or Windows' hooks, beside the
  window. In a call, unmuted, the last key of a push to talk keybind
  (told by its scan code) types nothing into the composer it would go
  to; every other key types as ever, held or not.

Only what shows takes frames: a tile's video is mapped while its tile is
in view or in the corner, at the size it shows in the window's pixels
(the display's scale, as the window says it as it opens and moves, times
the zoom), as a message's video and the camera's preview in Settings
are; `Video` lays a frame out in those pixels, so one made at its tile's
size is copied one to one.
