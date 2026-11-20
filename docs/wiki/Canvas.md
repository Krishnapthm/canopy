# Canvas

The canvas holds your panes. Pan, zoom and arrange them to keep your work in view.

![A canvas with agents and terminals, two of them wired](images/canvas-1.png)

## Move around

- Scroll over the canvas to pan. Drag with the middle mouse button to pan across panes.
- Hold `Cmd` (`Ctrl` on Linux and Windows) and scroll to zoom around the pointer.
- See [Keys and Mouse](Keys-and-Mouse) for the full key tables and mouse gestures.
- See [Spaces](Spaces) for Home and the top strip.

## Select and move panes

- In canvas mode, click a pane to select it. `Shift`-click adds or removes a pane.
- Drag empty canvas to draw a selection box. Hold `Shift` to add to the selection.
- Click empty canvas to clear the selection.
- Drag a selected pane to move the selection together. By default, neighbours step aside and return as you move past.
- Press `=` in canvas mode to give selected terminal panes their standard size. With none selected, this uses the focused terminal.
- Right-click a terminal pane for **Bring to front** or **Send to back**.

See [Terminals](Terminals) for expand, collapse and text zoom. See [Panes](Panes) for closing and stacking panes.

## Arrange

Right-click empty canvas or a space card. Click **Arrange** to repeat the last layout, or hover **Arrange as**:

- **Tidy** makes a grid in reading order, with wired panes together.
- **Row** makes one row.
- **Column** makes one column.
- **By wires** places parents to the left of their children and sub-agents.

Arrange uses selected panes in the last selected pane's space. With no panes selected, it uses the current space.
A space card's menu always arranges that space.

- Click **Undo layout** in the canvas or space menu to reverse the last move, file pane resize or arrange.
- Undo keeps one step until the window closes. It does not restore closed panes. A terminal corner resize is not undoable.

## Tile view

![Tile view with four panes in a grid](images/canvas-2.png)

Press `Cmd+Shift+G` (`Ctrl+Shift+G` on Linux and Windows), or click **Tile view** in the canvas menu.

- Tile view fits expanded terminal panes and other full panes into the visible canvas at 100% zoom.
- With two or more panes selected, only those participate. Otherwise, all panes in the current space participate, including panes opened later.
- Collapsed panes line the bottom. Other panes use equal sizes unless `tile_focus_grows` is on.
- Toggle tile view again, or leave the space, to restore the previous positions and view.
- Drag a pane to end tile view and keep the tiled positions.

## Focus fit

- Press `f` in canvas mode to fit the selection into the visible canvas.
- With no selection, it fits everything in the current space or Home. The files panel stays outside the fitted area.
- Press again to restore the previous view. If you pan or zoom between presses, it fits again.

## Sync input

1. Select two or more running terminal panes.
2. Press `Cmd+Enter` (`Ctrl+Enter` on Linux and Windows).
3. Type or paste into one synced pane. Every synced pane receives the input.
4. Press the shortcut again to stop sync input.

- Synced panes show an accent border and a **SYNC** badge. Clicking one to type keeps the group.
- A deselected pane leaves the group. Sync input stops when fewer than two remain, or when you cancel in canvas mode.

## Settings that change the canvas

See [Settings and Themes](Settings-and-Themes) for how to edit `settings.json`.

- `canvas_gap`: spacing between panes and space cards, in world pixels. Default: 24.
- `canvas_push`: `"live"` moves neighbours during a drag, `"drop"` waits for release, and `"off"` permits overlap. Home still separates space cards. Default: `"live"`.
- `space_fill`: space card tint opacity on Home, from 0 to 1. Default: 0.04.
- `tile_gap`: spacing between panes in tile view, in world pixels. Default: 0.
- `tile_focus_grows`: lets the selected pane take more room in tile view. Default: `false`.

## How it works

Arrange keeps pane sizes and saves your layout choice. Tile view keeps its previous positions only until the window closes.
