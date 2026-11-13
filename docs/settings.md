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
| `canvas_push` | `"live"` | Push during a drag or resize; `"drop"` waits until release |
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
