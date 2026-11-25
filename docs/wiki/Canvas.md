# Canvas

The canvas is the open surface that holds a space's panes. Unlike a terminal multiplexer, there is no fixed grid: you place panes wherever they make sense, zoom out to see everything, and zoom in on what you are doing. When you want structure, Canopy can arrange or tile your panes for you.

![A canvas with agents and terminals, two of them wired](images/canvas-1.png)

This page covers moving around, selecting and arranging panes, and the views that help you focus. If "canvas mode" is new to you, read [Concepts](Concepts#typing-into-a-pane-and-canvas-mode) first.

An empty space (or Home with no spaces) shows shortcut hints for new terminal, new agent, go to pane, command palette and fit; a hint shows only when its action has a key.

## Move around

- **Pan:** scroll over the canvas (two fingers on a trackpad), drag with the middle mouse button, or hold `Space` and drag. You can also turn on the **Hand** tool on the toolbar and drag with the left button.
- **Zoom:** hold `Cmd` (`Ctrl` on Linux and Windows) and scroll, or pinch. Zoom follows the pointer.
- **Zoom limits:** zooming in stops while the biggest pane still fits the window with a margin, and zooming out stops once everything fits (or at 100% if everything already fits). Zoom stops right at the limit, with no bounce. Panning has no limit.
- **Fit:** press `Shift+1` in canvas mode to fit everything in the space on screen, and `Shift+0` to return to 100% zoom (or the nearest zoom inside the limits).

Canvas zoom changes pane frames. Terminal text and PDF pages keep their own zoom, unless the viewer is set to fit its pane.

Pan and zoom work over empty canvas and over any pane you have not entered. An entered terminal keeps plain scroll for its output. Pinch and `Cmd`+scroll (`Ctrl`+scroll on Linux and Windows) still zoom the canvas over a terminal.

Each space, and Home, remembers its own pan and zoom.

## Select and move panes

In canvas mode, a click on a pane selects it. Selection is how you tell Canopy which panes an action applies to.

- `Shift`-click adds a pane to the selection or removes it.
- Drag on empty canvas to draw a selection box. Hold `Shift` to add to the selection. Click empty canvas to clear it.
- Drag a selected pane to move the whole selection.
- Drag a pane until it touches the edge of the canvas and the canvas pans that way to make room. Push further to pan faster, and move back to stop.
- Arrow keys nudge the selected pane by one gap. `h`, `j`, `k` and `l` (or `Tab` and `Shift+Tab`) move the selection to the nearest pane in that direction.

Drag any edge or corner to resize an expanded terminal or a pane such as a PDF, browser or simulator. While you resize, hold the pointer near the edge of the canvas to pan and reach more room. The canvas pans faster as you approach the edge, and stops when you move back toward the middle or release.

When you drag a pane into others, the neighbors step aside and slide back once you have passed, so panes never end up stacked by accident. The `canvas_push` setting changes this (see [Settings](#settings-that-change-the-canvas)).
A drag follows the pointer even across other panes. The canvas uses the latest pointer position for each frame and applies any remaining movement when you release. Dragging a selected group moves it together while neighbors slide aside.

A few more things you can do with selected panes:

- Press `=` to give the selected terminals the standard size from the `terminal_size` setting. With none selected, it resizes the focused terminal.
- Right-click a terminal pane for **Bring to front** or **Send to back** when panes overlap.

[Terminals](Terminals) covers expanding, collapsing and resizing terminals. [Panes](Panes) covers closing them.

## Arrange panes

When a canvas gets messy, let Canopy lay it out. Right-click empty canvas (or a space card on Home), hover **Arrange as** and pick a layout:

| Layout | Result |
|---|---|
| **Tidy** | A grid in reading order, with wired panes kept together. |
| **Row** | One row, left to right. |
| **Column** | One column, top to bottom. |
| **By wires** | Each parent to the left of its helpers and sub-agents, like a tree. |

After the first time, **Arrange** repeats your last choice, and `Shift+A` in canvas mode does the same. Press `a` to pick a layout from a menu.

Arrange works on the selected panes if you have selected some, otherwise on the whole space. A space card's menu always arranges that whole space. Pane sizes are kept.

Changed your mind? Click **Undo layout** in the canvas or space menu, or press `Cmd+Z` (`Ctrl+Shift+U` on Linux and Windows). Undo reverses the last move, arrange or file pane resize. It keeps one step, until the window closes. Closed panes and terminal sizes stay as they are.

## Tile view

![Tile view with four panes in a grid](images/canvas-2.png)

Tile view temporarily fits your panes into an even grid that fills the screen at 100% zoom, so every terminal is readable at once. It is the view for "show me all four agents while they work".

Press `Cmd+Shift+G` (`Ctrl+Shift+G` on Linux and Windows), or click **Tile view** in the canvas menu or the grid button in the top strip.

- With two or more panes selected, only those are tiled. Otherwise every expanded pane in the space is, including panes you open while tile view is on.
- Collapsed terminals line up along the bottom.
- Panes get equal space. Turn on `tile_focus_grows` to let the selected pane take more room.

Press the shortcut again, or leave the space, to put every pane back where it was. If you drag a pane while tiled, tile view ends and the panes keep their tiled positions.

## Focus fit

Focus fit zooms to what you are working on and back again. Press `f` in canvas mode, or `Cmd+Shift+F` anywhere on macOS. On Linux and Windows, `Ctrl+Shift+F` does the same unless a terminal is entered, where it searches the terminal instead.

- With panes selected, it fits them on screen.
- With nothing selected, it fits the whole space (or all of Home).
- Press it again to return to the view you had before. If you panned or zoomed in between, it fits again instead.

The Files panel is left out of the fitted area, so fitted panes are never hidden behind it.

## Sync input

Sync input sends your typing to several terminals at once. Use it when you need the same command in many places, such as pulling and rebuilding in three checkouts.

> **Warning:** Sync input sends keys to agents too. Whatever you type into one synced agent, including `Enter`, goes to every synced agent.

1. Select two or more running terminal panes.
2. Press `Cmd+Enter` (`Ctrl+Enter` on Linux and Windows) in canvas mode.
3. Type or paste in any one of them. Every synced terminal receives the same input.
4. Press the shortcut again to stop.

Synced panes show an accent border and a **SYNC** badge. Clicking into one to type keeps the group together. A pane you deselect leaves the group, and sync stops when fewer than two remain or when you press `Esc` in canvas mode.

## Settings that change the canvas

Edit these in `settings.json` (see [Settings and Themes](Settings-and-Themes)).

| Setting | Default | What it does |
|---|---|---|
| `canvas_gap` | `24` | Space between panes and between space cards, in canvas pixels. |
| `canvas_push` | `"live"` | `"live"` moves neighbors while you drag, `"drop"` waits until you let go, `"off"` lets panes overlap. Space cards on Home are always kept apart. |
| `space_fill` | `0.04` | How strongly space cards on Home are tinted with their color, from 0 to 1. |
| `tile_gap` | `0` | Space between panes in tile view, in canvas pixels. |
| `tile_focus_grows` | `false` | Lets the selected pane take more room in tile view. |

## The status strip

A 24px strip under the canvas shows the branch of the open space (when it has one) and one chip per agent: a colored dot and the agent's name, agents that need you first. Click a chip to go to that agent's space and type into it right away. With more than six agents, a final **+N** chip opens the picker. The palette key sits at the right.

The dot beside the branch uses the [space's color](Spaces#rename-recolor-or-delete-a-space). An agent's name and dot use its [pane color](Panes#color-a-pane), including inherited colors. Without a pane color, the dot shows its current state.

## How it works

Arrange moves panes for good and remembers the layout you chose. Tile view and focus fit are temporary views: they remember the previous positions or view only until you leave them or close the window.

## See also

- [Keys and Mouse](Keys-and-Mouse) for every canvas key.
- [Spaces](Spaces) for Home, the top strip and switching between spaces.
- [Panes](Panes) for pane kinds and closing panes.
