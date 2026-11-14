# Keyboard and mouse

The canvas works like Figma. Click a cell to select it, drag to move it (the
whole selection moves together), drag on empty canvas to draw a selection box
(Shift adds to the selection), and double-click a cell to enter it: it expands
and its terminal takes the keyboard. Click empty canvas or another cell, or
press Cmd+Esc (Ctrl+Esc on Linux and Windows), to leave; Esc itself stays with
the terminal, so agents can still be interrupted. Pan with Space+drag, middle
drag or the trackpad; zoom with pinch or Cmd+scroll. On Home, double-click a
space card (or select it and press Enter) to go inside.

Hold Option (Alt) while the canvas has the keyboard: each selected cell, or the
one under the pointer, shows its actions with their keys, and the bottom bar
lists every canvas key. Release Option to hide them.

| Canvas has the keyboard | Action |
|---|---|
| Enter | Enter the selected cell, or go into the selected space |
| Esc | Cancel a connection, else clear the selection, else go back Home |
| Shift+1 / Shift+2 / Shift+0 | Fit everything / fit the selection / 100% |
| f | Fit the selection, else everything; again restores the previous view |
| Cmd+A (Ctrl+A) | Select everything here |
| Arrows | Move the selection by one gap, with one undo step |
| h j k l, Tab / Shift+Tab | Select the nearest cell in that direction |
| x | Collapse the selected terminal |
| c | Start a wire; select its other cell, Enter connects, Esc cancels |
| t | Open the selected wire's thread, or the selected cell's first wire |
| Shift+A / a | Arrange (repeat the last layout) / choose a layout |
| n / Shift+N | New terminal here / installed agent list |
| Backspace / Delete | Close, using the usual agent and terminal confirmations |
| ? | Full key list |

Enter on a sub-agent opens its transcript; on a file card it opens the file.

| Anywhere on the canvas | macOS | Linux / Windows |
|---|---|---|
| Leave the entered cell | Cmd+Esc or Cmd+J | Ctrl+Esc or Ctrl+Shift+J |
| Go into space 1..9 | Cmd+1..9 | Ctrl+1..9 |
| Home | Cmd+Shift+H | Ctrl+Shift+H |
| Focus fit toggle | Cmd+Shift+F | Ctrl+Shift+F on canvas |
| Tile view toggle | Cmd+Shift+G | Ctrl+Shift+G |
| Expand / collapse | Cmd+Enter | Ctrl+Shift+Enter |
| Arrange | Cmd+Shift+A | Ctrl+Shift+A |
| Zoom in / out / 100% | Cmd+= / Cmd+- / Cmd+0 | Ctrl+= / Ctrl+- / Ctrl+0 |
| Agents / All / Terminals | Cmd+Shift+1/2/3 | Ctrl+Shift+1/2/3 |

On a canvas, Cmd+T / Ctrl+Shift+T opens `canvas.new {like: true}`: a terminal
beside the selected cell running what that cell runs (an agent, say), else a
plain one; Cmd+Shift+T / Ctrl+Alt+T is a plain `canvas.new`. Both shadow
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
