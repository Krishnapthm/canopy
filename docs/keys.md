# Keyboard and mouse

The canvas works like Figma. Click a cell to select it, drag to move it (the
whole selection moves together), drag on empty canvas to draw a selection box
(Shift adds to the selection), and double-click a cell to enter it: it expands
and its terminal takes the keyboard. Click empty canvas or another cell, or
press Cmd+Esc (Ctrl+Esc on Linux and Windows), to leave; Esc itself stays with
the terminal, so agents can still be interrupted. Pan with Space+drag, middle
drag or the trackpad; zoom with pinch or Cmd+scroll. On Home, double-click a
space card (or select it and press Enter) to go inside.

Hold Cmd (Ctrl on Linux and Windows) alone for 0.4 s to show keys on every
surface, including the toolbar and pane helper bars, even with a terminal
focused. Release it or press any key to hide the hints. Option no longer shows them.

Anywhere: Cmd+K or Cmd+P (Ctrl+Shift+K) opens Go to, a fuzzy search over
spaces, terminal panes and open file tabs (then the old tab list). Cmd+1..9
(Ctrl+1..9) goes into the Nth space tab, and Ctrl+Tab goes back to the space you
were in last (again toggles; Ctrl+Shift+Tab walks further back).

| Canvas has the keyboard | Action |
|---|---|
| Enter | Enter the selected cell, or go into the selected space |
| Esc | Cancel a connection, else clear the selection, else go back Home |
| Shift+1 / Shift+2 / Shift+0 | Fit everything / fit the selection / 100% |
| f | Fit the selection, else everything; again restores the previous view |
| Cmd+A (Ctrl+Shift+A) | Select every pane in the current space |
| Arrows | Move the selection by one gap, with one undo step |
| h j k l, Tab / Shift+Tab | Select the nearest cell in that direction |
| x | Collapse the selected terminal |
| c | Start a wire; select its other cell, Enter connects, Esc cancels |
| Shift+A / a | Arrange (repeat the last layout) / choose a layout |
| n / Shift+N | New terminal here / installed agent list |
| V | Select tool |
| T | New note pane |
| ⌘O (Ctrl+O elsewhere) | Open file on the canvas |
| Backspace / Delete | Close, using the usual agent and terminal confirmations |
| ? | Full key list |

Enter on a sub-agent opens its transcript; on a file card it opens the file.
In the agent menu, 1-9 start the agent with that number.

| Anywhere on the canvas | macOS | Linux / Windows |
|---|---|---|
| Leave the entered cell | Cmd+Esc or Cmd+J | Ctrl+Esc or Ctrl+Shift+J |
| Go into space 1..9 | Cmd+1..9 | Ctrl+1..9 |
| Home | Cmd+Shift+H | Ctrl+Shift+H |
| Focus fit toggle | Cmd+Shift+F | Ctrl+Shift+F on canvas |
| Tile view toggle | Cmd+Shift+G | Ctrl+Shift+G |
| Expand / collapse | Cmd+Enter | Ctrl+Shift+Enter |
| Select every pane (canvas focus) | Cmd+A | Ctrl+Shift+A |
| Arrange | Cmd+Shift+A | Ctrl+Alt+Shift+A |
| Zoom in / out / 100% | Cmd+= / Cmd+- / Cmd+0 | Ctrl+= / Ctrl+- / Ctrl+0 |
| Agents / All / Terminals | Cmd+Shift+1/2/3 | Ctrl+Shift+1/2/3 |

On a canvas, Cmd+T / Ctrl+Shift+T opens a plain terminal pane (`canvas.new`),
full size and interactive. Each one cascades 32 px down and right of the last,
back to the view's top left when there is no room, never exactly on another
pane. Cmd+Shift+T / Ctrl+Alt+T is `canvas.new {like: true}`: the same, running
what the selected cell runs (an agent, say). Both shadow
`tab.new` while a canvas layout is shown (Ctrl+T stays the terminal's own).
With a cell entered, zoom in / out / 100% (Cmd or Ctrl, plus Ctrl+= / Ctrl+- /
Ctrl+0 on macOS too) resize that terminal's text, in memory, instead of the
canvas. Cmd+N / Ctrl+Shift+N `space.new` and
Cmd+B / Ctrl+Shift+B `layout.toggle:sidebar`. On Linux and Windows,
Ctrl+Shift+F is terminal search while a terminal has focus.

Edit app-home `keys.json` to rebind; changes apply as you save. The
`canvas-mode` section holds the canvas keys above: it is consulted first
whenever the canvas itself has the keyboard (no cell entered, no input focused),
and an unbound key is swallowed unless it carries Ctrl, Alt or Cmd. Its
bindings may carry modifiers but are never sequences. An entered terminal gets
every key except the bindings in `layout:main` and `global`.

Settings > Shortcuts shows current and shipped keys alongside descriptions from
the command catalog. It includes unbound commands and custom key actions, with
separate rows for sections and command arguments. Equivalent keys in one section
warn when they name different actions.

Use the search field to filter action names, descriptions and keys. Click a
current key (or Unbound) and press one combination to replace that action's
bindings in its section. Esc cancels; Backspace clears. A conflicting key is
refused with a warning so another action is never silently replaced. The
reserved Ctrl+Alt+Shift+Esc recovery key cannot be assigned.

Reset to default restores every shipped key for that row, or clears an action
with no default. Reset all shortcuts asks for confirmation, restores this OS's
shipped actions and clears other bindings to built-in commands. Custom mod
actions and unrelated sections remain; clear a conflicting custom key first.

Edits use app-home `keys.json`, including on macOS. Writes re-read the file,
preserve other entries and use the file API's atomic expected-text check.
Concurrent edits are retried at most twice; errors raise a toast. The existing
config reload applies saved keys and refreshes the page, with no new watcher.
Commands needing arguments can be configured as object actions in keys.json;
the recorder keeps existing arguments and replaces only the keys.

Select-all uses `canvas.select_all` only with canvas focus. An entered terminal
keeps Cmd+A for `terminal.select_all`; plain Ctrl+A still reaches shells.
