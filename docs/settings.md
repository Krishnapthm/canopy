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

Settings > Shortcuts lists actions, their command descriptions, current keys
and shipped defaults for this OS. Rows are grouped by keys.json section;
argument-bearing actions keep separate rows. A warning identifies equivalent
keys assigned to different actions in the same section.

Search by action, description or key. Click a key to record a replacement;
Esc cancels and Backspace clears. Changes save immediately to app-home
`keys.json` on every OS and use the existing live config reload. Invalid JSON,
conflicts and failed saves raise a toast without overwriting the file.

Reset to default restores a row's shipped keys (or clears an unshipped action).
Reset all shortcuts uses the confirmation dialog, restoring built-in actions
while keeping custom mod actions and unrelated entries. Clear conflicting
custom keys before resetting. The recorder retains existing command arguments;
configure new argument-bearing actions in keys.json.
