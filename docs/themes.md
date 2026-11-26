# Themes

Settings → Appearance shows the window in one of the themes BACKCORD comes with,
or one of the person's own. Each is a full set of the colours of `Theme` in
`backcord_core::theme` (`crates/backcord-core/src/theme.rs`), plus whether it's
dark, which sets the scrollbars' shade and the colours worked out from the
others. The core holds the built-in ones as `BUILT_IN` and reads custom
themes and hex colours; `crates/backcord-iced/src/themes.rs` makes the chosen
one the kit's `Theme` (`crates/backcord-iced-kit/src/theme.rs`), which every
widget paints with; see [the client's theme](client/window.md#theme). The
choice is kept in `layout.json` as `theme`, the theme's id, and applied
before the window's first frame, and again to a window made anew. With none
kept, an id BACKCORD doesn't know, or a deleted theme's, the window shows
BACKDOOR. The window-control command `theme ID` chooses and keeps
one as Appearance does.

## Custom themes

In the theme editor a left click on a colour's swatch opens a palette
under it: a saturation and brightness square over a hue strip, and the
colour's code, as the profile editor's colours have it. The pad keeps the
colour's alpha; typing a code moves the pad there, and Enter or a click
outside closes it.


New theme opens an editor on a copy of the theme shown; START FROM swaps it
for a copy of any other. The editor lists every colour by its name
(`Theme::NAMES`, with `-` for `_`), with a swatch and its hex, which takes
`#rgb`, `#rrggbb` or
`#rrggbbaa` and marks anything else. Each change shows at once, so the whole
window is the preview. Save keeps the theme and chooses it; Cancel, leaving
Appearance or closing settings shows the theme from before. A saved theme's
card has a pencil that opens it again to rename, change or delete it;
deleting the chosen one shows BACKDOOR. Closing the window to the
[tray](tray.md) closes Settings, so it drops the unsaved edit too.

`layout.json` keeps them in `custom_themes`, in the order Appearance shows
them after the built-in ones:

```json
{
  "id": "custom:1",
  "name": "Plum",
  "base": "backdoor",
  "dark": true,
  "colours": { "rail": "#111215", "chat": "#2a1a3a", "row-hover": "#1a1b20" }
}
```

`base` is the built-in theme the first copy came from (a copy of a custom
theme takes its base). BACKCORD writes every colour, but a colour that's missing,
such as one added in a later version, or that isn't a colour, takes the
base's value, and a name BACKCORD doesn't know is ignored, so themes survive new
colours. Ids are `custom:` and a number, and no built-in id has a colon, so
they can't collide; on reading, BACKCORD gives a fresh one to any custom theme
whose id isn't of that form or repeats another's. The editor's fields exist
only while it's open; otherwise a custom theme costs its parsed colours and a
card.

| Theme | Id | Dark | Accent |
| --- | --- | --- | --- |
| BACKDOOR | `backdoor` | yes | red |

A custom theme whose base this version doesn't have takes BACKDOOR's colours
for any it doesn't name.

Discord role colours stay as servers set them, and some read faintly on a
light theme; BACKCORD doesn't adjust them yet.

## Themes and Performance

Settings has two pages for how the window looks. **Themes** holds the theme cards and the theme editor,
the neon rings' intensity, **Corner roundness** (0 square, 100 as BACKCORD draws
it, 200 rounder; the renderers scale every rounded corner, pictures' too), and
**Roundness of avatars and icons** for what's round (0 square, 100 round:
avatars, status dots, pills, switches, server icons; Discord's avatar
decorations are pictures and keep their shape) and the **custom background**: a picture behind the whole window,
chosen with Choose image, placed by dragging its preview, with a blur (0 off)
and an opacity slider: it's drawn under the window, and the grounds of the
rail, the lists, the panels, the chat and the stage let it through as the
opacity says, while text, menus and controls stay solid (a pointed or
chosen row is a see-through wash); dialogs, cards and
popups over the window are drawn in the solid theme. **Direct messages
icon** replaces the picture at the top left, over the DMs, with any picture,
cut square at its middle and fitted to the tile (`home-icon.<ext>` in the
data folder); Reset puts the theme's back. The picture is copied to the data folder as
`backdrop.<ext>`. **Menus and dialogs** sets how see-through Settings, the soundboard, the
call's menus and right-click menus are, and how blurred what's behind
them shows (frosted glass: as one opens, the window under it is taken once and
blurred; it shows when they're not fully opaque, with or without a custom
background).
**Colours**, last, lists the window's parts by plain names (server list,
chat background, text, main colour…), each with its palette: a change shows
at once and is kept, a built-in theme copied first as the person's own.
Camera and stream tiles and videos in messages cut their corners as the
corner roundness says. **Performance** holds the title bar and the profile
switches.

## Export and import

Export writes the chosen theme as `layout.json` keeps a custom theme, plus
an `appearance` object with the rest of the look: `neon_glow`, `corners`,
`round`, the background's `backdrop_blur`, `backdrop_opacity`, `backdrop_x`
and `backdrop_y`, `menu_opacity`, `menu_blur`, and the background and DM
icon pictures themselves as `{ "type": "png", "data": "<base64>" }` (null
when there's none). Import adds the theme and, when the file has
`appearance`, sets all of it: the pictures are written to the data folder
as if chosen. A file up to 160 MiB is read; an older export with only the
colours imports as before.
