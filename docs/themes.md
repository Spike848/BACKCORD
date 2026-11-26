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
