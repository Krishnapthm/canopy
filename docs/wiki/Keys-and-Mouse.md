# Keys and Mouse

Every default shortcut and mouse gesture, for reference. In the app, press `?` in canvas mode for the full key list, or hold `Cmd` (`Ctrl`) to see each button's key. Change any shortcut in **Settings** > **Shortcuts**.

Keys are shown for macOS first. Many canvas-mode keys are single letters, and work only when the canvas has the keyboard (see [Concepts](Concepts#typing-into-a-pane-and-canvas-mode)).

## Mouse

- Click a pane to select it. Drag it to move the selection. `Shift`+click adds or removes a pane.
- Drag empty canvas to select panes with a box. Hold `Shift` to add to the selection.
- Double-click a terminal pane to enter it. Click empty canvas or another pane to leave it.
- Pan with `Space`+drag, middle-button drag or wheel or two-finger scroll. Scroll pans over empty canvas and panes you have not entered. Inside an entered terminal, plain scroll moves through its output.
- Pinch or use `Cmd`+scroll (`Ctrl`+scroll on Linux and Windows) to zoom the canvas around the pointer.
- Drag a pane edge or corner to resize. While moving or resizing a pane, hold the pointer near a canvas edge to pan. See [Canvas](Canvas#select-and-move-panes).

## Show keys

Hold `Cmd` (`Ctrl` on Linux and Windows) alone for 0.4 seconds to show shortcut hints, including with a terminal entered. Release it or press a key to hide the hints.

On Linux, **F10** opens the [Main Menu](Linux-Desktop).

## Canvas mode keys

These keys apply when the canvas has the keyboard and no input has focus. See [Canvas](Canvas), [Terminals](Terminals) and [Images and PDFs](Images-and-PDFs) for the actions.

| Key | Action |
|---|---|
| `Enter` | Enter the selected terminal or space. Open a selected file or sub-agent transcript. |
| `Esc` | End sync input, else cancel a connection, else clear the selection, else go Home. |
| `Shift+1` / `Shift+2` / `Shift+0` | Fit everything / focus fit / 100% zoom. |
| `f` | Fit the selection, else everything. Press again to restore the previous view. |
| `Cmd+A` (`Ctrl+Shift+A` on Linux and Windows) | Select all terminal panes in the current space. |
| Arrow keys | Move the selected pane by one gap. |
| `h`, `j`, `k`, `l` | Select the nearest pane left, down, up or right. |
| `Tab` / `Shift+Tab` | Select the nearest pane right / left. |
| `x` | Collapse the selected terminal. |
| `c` | Start a wire. Select the other pane, then press `Enter`. |
| `Shift+A` / `a` | Repeat the last arrangement / choose an arrangement. |
| `=` | Equalize pane sizes. |
| `Cmd+Enter` (`Ctrl+Enter` on Linux and Windows) | Toggle sync input. |
| `n` / `Shift+N` | Open a terminal / the installed agent list. |
| `v` / `t` | Select tool / new note pane. |
| `Cmd+O` (`Ctrl+O` on Linux and Windows) | Open a file on the canvas. |
| `Cmd+W` (macOS) | Close the selected browser pane's active tab. Without a single browser pane selected, close the app tab as usual. |
| `Backspace` / `Delete` | Close the selection. |
| `?` | Show the full key list. |
| `q` | Quote the selected image. |
| `/` / `m` | Find PDF text / toggle PDF highlighting. |
| `[` / `]` / `Shift+M` | Previous PDF page / next page / clear highlights. |

## Keys anywhere

These shortcuts apply while the canvas layout is shown, including with a terminal entered. Canvas mode keys take priority.

| Action | macOS | Linux and Windows |
|---|---|---|
| Leave an entered pane | `Cmd+Esc` or `Cmd+J` | `Ctrl+Esc` or `Ctrl+Shift+J` |
| Space 1 through 9 / Home | `Cmd+1` through `Cmd+9` / `Cmd+Shift+H` | `Ctrl+1` through `Ctrl+9` / `Ctrl+Shift+H` |
| Recent space / oldest space | `Ctrl+Tab` / `Ctrl+Shift+Tab` | `Ctrl+Tab` / `Ctrl+Shift+Tab` |
| **Go to** | `Cmd+K` or `Cmd+P` | `Ctrl+Shift+K` |
| New space / toggle the Classic sidebar | `Cmd+N` / `Cmd+B` | `Ctrl+Shift+N` / `Ctrl+Shift+B` |
| New terminal / terminal like the selection | `Cmd+T` / `Cmd+Shift+T` | `Ctrl+Shift+T` / `Ctrl+Alt+T` |
| Focus fit / tile view | `Cmd+Shift+F` / `Cmd+Shift+G` | `Ctrl+Shift+F` (canvas focus) / `Ctrl+Shift+G` |
| Expand or collapse | `Cmd+Enter` | `Ctrl+Shift+Enter` |
| Open Web Inspector for the selected browser pane (macOS only) | `Cmd+Option+I` or `Ctrl+Shift+I` | `Ctrl+Shift+I` |
| Arrange / undo layout | `Cmd+Shift+A` / `Cmd+Z` | `Ctrl+Alt+Shift+A` / `Ctrl+Shift+U` |
| Zoom in / out / 100% | `Cmd+=` / `Cmd+-` / `Cmd+0` | `Ctrl+=` / `Ctrl+-` / `Ctrl+0` |
| Agents / all / terminals view | `Cmd+Shift+1` / `Cmd+Shift+2` / `Cmd+Shift+3` | `Ctrl+Shift+1` / `Ctrl+Shift+2` / `Ctrl+Shift+3` |

- Zoom keys change an entered terminal's text size. On macOS, `Ctrl+=`, `Ctrl+-` and `Ctrl+0` also work.
- An entered terminal keeps `Esc`. On Linux and Windows, `Ctrl+Shift+F` searches the entered terminal.

## Change shortcuts

1. Open **Settings** > **Shortcuts**. Search by action, description or key.
2. Click a current key or **Unbound**.
3. Press one combination to replace the row's keys. Press `Esc` to cancel or `Backspace` to clear them.

- Changes save immediately. Conflicts within the same section raise a warning and prevent the save.
- **Reset to default** restores the row's shipped keys, or clears a row without defaults.
- **Reset all shortcuts** asks for confirmation. It restores shipped keys and clears other built-in bindings, keeping custom mod actions and unrelated entries. Clear conflicting custom keys before resetting.

## keys.json

- Edit [app-home](Files-and-Folders) `keys.json`. Saved changes apply live. Sections are `global` (window), `terminal`, `layout:<name>` (shown layout), `mod:<name>` (focused mod) and `canvas-mode`.
- Actions accept command strings, `{"command":"canvas.new","args":{"like":true}}`, `send:<text>` or `terminal` to pass the key to the terminal. The recorder preserves action arguments.
- Canvas mode bindings accept modifiers but no key sequences. Unbound keys without `Ctrl`, `Alt` or `Cmd` are swallowed.
- Press `Ctrl+Alt+Shift+Esc` to recover the default layout and keys. Canopy preserves the previous files as `*.reset-<time>` backups. This key cannot be reassigned.
