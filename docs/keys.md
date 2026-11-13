# Keyboard

Canvas mode selects and controls cells without sending keys to a terminal.
Press Cmd+J on macOS, Ctrl+Shift+J on Linux or Windows, then release it.
The bottom hint bar reads your `canvas-mode` bindings; `?` opens the full list.
There is no timeout. Dialogs and menus keep their own keys while open.

| In canvas mode | Action |
|---|---|
| Arrows, h j k l | Select the nearest cell in that direction, without expanding |
| Tab / Shift+Tab | Next / previous space |
| Enter | Expand the selection, focus its terminal and leave mode |
| Esc | Leave mode, keeping the selection |
| x | Collapse the selected terminal |
| f | Fit the current space; again restores the previous camera |
| c | Start a wire; arrows select its other cell, Enter connects, Esc cancels |
| t | Open the selected wire's thread, or the selected cell's first wire |
| a / Shift+A | Repeat arrange / choose a layout |
| n / Shift+N | New terminal in this space / installed agent list |
| Shift+arrows | Move the selection by one gap, with one undo step |
| Backspace / Delete | Close, using the usual agent and terminal confirmations |
| ? | Full key list |

Enter on a sub-agent opens its transcript; on a file card it opens the file.
A connection stays in mode after Enter; Esc cancels a pending connection first.

| Direct action | macOS | Linux / Windows |
|---|---|---|
| Canvas mode | Cmd+J | Ctrl+Shift+J |
| Focus fit toggle | Cmd+Shift+F | Ctrl+Shift+F on canvas |
| Expand / collapse | Cmd+Enter | Ctrl+Shift+Enter |
| Arrange | Cmd+Shift+A | Ctrl+Shift+A |
| Zoom in / out / 100% | Cmd+= / Cmd+- / Cmd+0 | Ctrl+= / Ctrl+- / Ctrl+0 |
| Agents / All / Terminals | Cmd+Shift+1/2/3 | Ctrl+Shift+1/2/3 |

Cmd+T / Ctrl+Shift+T is `tab.new`, Cmd+N / Ctrl+Shift+N `space.new` and
Cmd+B / Ctrl+Shift+B `layout.toggle:sidebar`. On Linux and Windows,
Ctrl+Shift+F is terminal search while a terminal has focus. For a canvas
terminal use mode `n`; for the file panel, the folder icon.

Edit app-home `keys.json` to rebind. Mode is one window flag: while it is set,
the `canvas-mode` section is consulted before normal bindings, and an unbound
key is swallowed unless it carries Ctrl, Alt or Cmd (those reach the normal
bindings, so quit and the reset key still work; the canvas holds the focus, so
no terminal sees them). Its bindings must be single keys, never sequences. Outside
mode, plain Ctrl+letters and Esc belong to the terminal. Backspace, Delete and
undo also reach a focused terminal unchanged.
