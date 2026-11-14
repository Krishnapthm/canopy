# Settings

`settings.json` holds preferences in the Canopy app home (normally
`~/.config/canopy`). Run `canopy config` for every key, its default and a one-line
explanation. This reference and the agent guide completeness test use the same
table as the defaults.

Edit the JSON file and save. Changes apply live, except `core` (next start).
The following are hand-edited only and never appear in Settings:

| Key | Default | Meaning |
|---|---|---|
| `canvas_gap` | `24` | Cell and space gap in world pixels, 0..1024 |
| `tile_gap` | `0` | Gap between cells in [tile view](canvas.md#tile-view), 0..1024 |
| `canvas_push` | `"live"` | Push during a drag or resize; `"drop"` waits until release; `"off"` never pushes, so cells may overlap (space cards and root cells on Home still separate); `"off"` is a toggle in Settings |
| `canvas_resize_terminals` | `false` | Refit a canvas terminal's grid to its cell as it resizes. Off keeps the grid measured when the terminal was made (or last text-zoomed) and scales it to fit the cell, centered; a toggle in Settings |
| `space_fill` | `0.04` | Space background opacity, 0..1; 0 removes fill |
| `terminal_size` | `[100, 30]` | Newly expanded terminal columns (20..500) and rows (5..200) |
| `wire_limit` | `1000` | Maximum postcard length in characters |
| `wire_budget` | `8` | Agent postcards before the breaker trips; 0 disables it |

`confirm_close_terminal` defaults to true. The close dialog’s “Don't ask again”
checkbox writes false; edit it back to true to restore prompts.

A JSON error in settings.json, keys.json or programs.json raises a toast naming
file:line:column and keeps the last good values. A value out of range
rejects settings.json the same way. An unknown key is a warning toast, and the
rest of the file still applies; when one known key is close, it is suggested:
``settings.json: unknown key `canvas_gab`, did you mean `canvas_gap`?``.
The matching `.error` file records the problem until the next successful reload.
On first start, with no good values yet, Canopy uses defaults.

Reloads use the existing file watch. They add no idle timer. A bad edit is never
written over by an in-app settings change.

## Built-in themes

Canopy Dark and Canopy Light are the appearance defaults for layouts without a
specified theme. Select `canopy-dark`, `canopy-light`, `herdr-dark` or
`herdr-light` in Settings. Ink and Moss remain available. Saved `graphite`
selects Herdr Dark; saved `paper` selects Herdr Light, the blue-tinted light
counterpart of the former default. Personal theme files are preserved.

Themes use the existing color tokens. `canvas` is the canvas; `background`
is the terminal; `surface` and `chrome` are panels and bars; `popover` and
`raised` are menus and dialogs. `foreground`, `muted`/`muted-foreground`,
`border`, `selected`/`selection`, `ring` and `accent` carry the specified
text, muted text, border, selection, focus and accent colors. Canopy and
Herdr Light working, blocked and done use ANSI yellow, red and blue. Herdr
Dark retains its status colors (`#E5C07B`, `#F0705F`, `#76A8FF`).
Dim, gray, idle and unknown use ANSI bright black; hover uses selected;
input uses border; solid uses foreground and solid foreground uses terminal
background. Color aliases use their matching ANSI slots; orange mixes red
and yellow. Herdr Dark retains its original palette derivation. Theme JSON
contains the exact palette and overrides.

Herdr Light uses canvas `#F8FAFD`, panels/terminal `#EFF3F9`, raised `#FFFFFF`,
text `#111111`, muted `#111111A6`, border `#11111117`, selection `#11111112`,
focus `#11111133`, accent `#2B5F9E`. ANSI red, green, yellow, blue, magenta,
cyan are `#B4183F`, `#0B6B3F`, `#82530A`, `#2B5F9E`, `#8A1A66`, `#1A6870`.

Canopy Dark ANSI black/white are `#7A7A7A`/`#EDEDED`; bright black/white
are `#999999`/`#FFFFFF`. Canopy Light and Herdr Light use
`#111111`/`#555555`, bright `#666666`/`#111111`. Bright chromatic colors
repeat the normal colors to retain contrast. Herdr Dark preserves Graphite,
except ANSI black is `#7F8186` for readability.

`cargo test --locked builtin_contrast -- --nocapture` prints WCAG contrast
ratios for all four themes, compositing translucent text onto each background.

Terminal colors composite alpha over their background before conversion to
the renderer’s opaque RGB palette. Editors inherit the canvas color.

Herdr Dark ANSI black changes from `#45484F` (1.922:1) to `#7F8186`
(4.514:1), the first passing step when mixing toward white in 1/255 increments.
All supplied Canopy values remain exact.
