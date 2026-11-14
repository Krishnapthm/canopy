# Canvas layout

At rest, every cell, sub-agent cell and file preview keeps `canvas_gap` world
pixels from its neighbours (default 24). On Home, space cards keep the same gap.

The canvas owns a 36 px top strip on macOS, keeping the existing traffic-light
inset. Linux and Windows keep their system title bar or menu strip above it.
Drag empty strip space to move the window; double-click uses the platform's
existing title-bar zoom behavior.

## Top strip

Home is an icon-only, non-closable tab that opens all spaces. Space tabs follow
with a folder icon and name. Click one to enter its room. An accent dot uses the
same working/blocked rollup as the spaces list. The active tab and hovered tabs
show a close X; closing uses `space.close` and its existing confirmation. The +
after the tabs runs `space.new`.

The strip uses the theme's `chrome` (panel/sidebar) color. Active tabs use the
canvas color, including a custom `canvas_color`, with 6 px top corners. There
are no tab borders, dividers or strip bottom border. Inactive text is muted and
brightens on hover. Tabs shrink to 96 px, then scroll sideways with a wheel or
trackpad; Home, + and the control buttons stay visible.

The plain grid and file icons after + toggle tile view and the files panel.
Hover labels show the current shortcut where one exists; no keys are added.
The file button restores the panel's closed tabs through the existing panel
restore path. Grid uses `canvas.tile`; closing the panel uses `panel.close`.

When the preview panel is open, its file tabs occupy the same strip, exactly
above its left edge. Click a tab to select its preview, hover for the full path,
or use its X to close that tab. Unsaved edits replace the X with a foreground
dot until hover. The Files editor reports saved/unsaved transitions through
`panel.editor {path, dirty}` without sending contents or polling. The section's
right buttons expand to the existing maximum panel width (75% of the window),
restore the previous width, or close the panel. Dragging its resize edge leaves
expanded mode. File tab selection and close reuse the existing panel handlers.

## Spaces are rooms

Home shows each space as a card, like a Figma frame: its name and color dot
above a rounded box tinted with its color, which says how many terminals it
holds, how many are working, and shows a bell when one waits for you. Double-
click the name to rename it; right-click the card to rename, recolor or delete
it (with a confirmation). Cards sit next to the cells outside any space.
Double-click a card, or select it and press Enter, to go inside: the whole
canvas belongs to that space, with no frame, so its cells grow freely.
Use Home or `canvas.home` to leave.
`canvas.space` with `next`, `prev` or a number goes into another space, counted
in top strip tab order (⌘1..⌘9). ⌃Tab (`recent`) enters the space you were in
last, so pressing it again toggles back; ⌃⇧Tab (`oldest`) walks back through
the rest. The order is kept in memory only. A new
space opens straight away, and the app reopens where you closed it. Each place
keeps its own camera. Older `canvas.json` files move each cell into the space
whose frame held it, and the frame becomes that space's card.

## Use

- Drag a cell, or resize a preview: neighbours step aside and return when you
  move on. Drag a space card on Home: only the card moves.
- Enter [canvas mode](keys.md) to select a collapsed cell without expanding it.
  Click a cell to select it. Shift-click adds or removes cells. Click empty
  canvas to clear the selection.
- Double-click the name in an expanded cell's header (or a tile's) to rename
  it in place: Enter or clicking away saves, Esc cancels. A pill's double-click
  still enters it; rename a pill from its menu.
- Wire two cells by clicking a "+" at the side of a hovered cell and then the
  other cell, or by dragging between them. Esc or a click on empty canvas
  cancels. See [Wire](wire.md).
- Right-click a cell (its pill, header or terminal) for its own menu: Rename,
  Pop out or Bring back, Expand or Collapse, Clear finished, Close terminal
  (asks first) and Kill terminal (ends it and its sub-agents now).
- A cell is named by what you renamed it to, else "Claude Code" (the agent's
  program name), else "terminal" for a shell, else the command's name ("uv").
  An agent's own session title and a command's flags are never shown.
- Right-click a space or empty canvas: **Arrange** repeats the last
  layout, hovering **Arrange as** opens a submenu beside it to pick one
  (submenus open on hover; → and ← work too). **Tidy** is a grid in reading order with wired cells kept next to each other, **Row**
  and **Column** line cells up, **By wires** puts parents left of their
  children and sub-agents. The choice is saved in `canvas.json`.
- Edges are soft curves: they leave and enter at whichever sides give the
  shortest path, go around other cells, and every bend is rounded as far as its
  runs allow (`canvas::smooth`), so a plain hop reads as an S-curve. They draw above cells, so
  an edge stays visible even when no clear path exists.
- Arrange works on the selected cells of one space (the space of the cell you
  selected last), otherwise on the current space. A space menu always arranges
  that space.
- **Undo layout** (space menu, `canvas.undo`, Cmd+Z on macOS, Ctrl+Shift+U on
  Linux) puts geometry back as it was before the last move, resize or arrange.
  One level, kept until the window closes. The keys act only while the canvas
  has focus; a focused terminal gets them as usual.

`canvas.arrange` takes `layout` (`tidy`, `row`, `column`, `wires`), `space`
(a space id) or `picker: true` (open the layout menu).

The hand-edited [settings](settings.md) reload live: `canvas_gap` changes spacing,
`canvas_push: "drop"` leaves neighbours still until a drag or resize is released.
`space_fill` controls space opacity (default 0.04; 0 removes fill).

## How it works

`canvas::solve` starts from rest rectangles (where the person left things),
fixes the moved rectangle, then places the others nearest first, each moved
along the shorter axis until it is one gap clear. Because each pointer sample
solves from rest again, nothing drifts and pushed cells return. The same
function separates spaces after `canvas::grow` widens a space around its cells.
Shown rectangles ease toward the result over 120 ms and frames are requested
only while they move. On release the result becomes the new rest, and
`canvas.json` is saved once. Undo keeps one snapshot of rectangles by cell key,
so it never brings back a closed cell.

## Restored shells

A plain shell (zsh, bash, fish, sh) reattached after the app starts opens clean
with a fresh prompt. The app drops the replayed screen, which can hold a stray
`%` row drawn at the wrong width, and nudges the pane size so the shell redraws
its prompt. Agents and other programs are untouched, and nothing is typed.

## Expanded cells

A terminal cell opens in place into a live terminal. Click a pill to expand
it; click the header of an expanded cell to collapse it. `canvas.expand`
(optional `terminal`) toggles the named or last selected cell, and the cell
menu has Expand / Collapse. Drag the bottom-right corner to resize. Several
cells can be open at once. Pop-out is unchanged. The side panel is for files
and sub-agent transcripts only.

- Size: `terminal_size` (default [100, 30]) cells of the terminal font at zoom 1,
  header included. The size is saved per cell in `canvas.json` (`size`; older files load as pills).
- Neighbours: expanding pins the cell and runs `canvas::solve`, so the cells
  around it give way and stay one gap clear, also of each other. Each
  expansion remembers what it moved; collapsing puts back what is still where
  the expansion left it, so cells you moved yourself stay put. Collapse in
  the reverse order of expanding to get everything back.
- PTY size: columns and rows are what fits the cell as drawn once the zoom
  rests (glyphs round to whole pixels at each zoom, so measuring at zoom 1
  would leave a strip); a corner drag applies them on release. The grid is
  centred, so the padding is even on all sides. With `canvas_resize_terminals`
  off (the default) the grid is measured once, when the terminal is made or after a
  text zoom, and kept: resizing the cell scales the whole grid to fit it, centered.
  Set it to true to refit the grid to the cell as above.
- Text zoom: with a cell entered, the zoom keys and View menu resize that
  terminal's text (not the canvas) until the app restarts; the grid refits.
- Input: keys go to the terminal once it is focused, as in any terminal, so
  Backspace, Delete and Cmd+Z belong to it. The wheel scrolls a focused
  expanded terminal; over any other cell it pans the canvas. Cmd (Ctrl) held
  zooms the canvas over any cell. A pan that began on the canvas keeps panning
  across cells until the wheel is quiet for 150 ms; a scroll that began over a
  focused terminal stays in it. Middle or a mouse side button drags the canvas
  too (grabbing hand while it does).
- Zoom: `terminal::Scaled` multiplies font size, cell size and padding while a
  terminal draws, so the text is real text at every zoom. The text
  draws live at every zoom frame; the program's grid is measured only once the
  zoom has rested for 160 ms. An expanded cell keeps its size at every zoom;
  below 0.5 zoom its body is a flat fill instead of text. No timer
  runs while nothing moves.

## Tile view

Press Cmd+Shift+G (Ctrl+Shift+G), the grid icon beside the focus icon, or
**Tile view** in the canvas menu to fit the expanded cells and previews into the
visible canvas at 100% zoom, like tmux's tiled layout. With two or more cells
selected only those tile: the others stay where they are, under the tiles.
Otherwise every expanded cell and preview of the current space tiles, and cells
opened later join. Collapsed cells (and space cards on Home) stay pills and line
up along the bottom. Press again, or leave the space, to put the tiled cells
back where they were.

- The selected tile is the large one: it takes 60% of its row, and its row at
  least 60% of the height (more when needed to stay the largest); the others
  share the rest. Click or arrow to another
  cell and the layout follows. Expanding, collapsing, opening or closing a cell
  re-tiles, and so does nudging a tile with the keyboard (it snaps back).
- Dragging a tile ends tile view: the cells stay where the tiles put them, the
  camera stays, and the drag moves that cell as on the free canvas (one undo
  step). A press only becomes a drag after 3 px of movement, on the free canvas
  too, so a click never shifts a cell.
- A tile smaller than 320 x 140 shows its icon and name instead of terminal
  text, and its program keeps its last grid size. The entered cell always shows
  its terminal.
- `tile_gap` in [settings](settings.md) sets the space between tiles (default 0,
  edge to edge); editing it re-tiles live.
- Tiles size their terminals like any cell: by default (`canvas_resize_terminals`
  off in [settings](settings.md)) the program keeps its grid and the text scales
  to fit the tile; with it on, columns and rows follow the tile.
- How it works: when tile view starts it records the members in reading order
  (top to bottom, then left to right). `canvas::tile` places them in that order
  and picks `ceil(sqrt(n))` columns, with the last row short and stretched, as
  in tmux. New cells are appended, closed ones dropped; changing the selection
  only changes sizes, never the order. The geometry from before is kept in
  memory only; after a restart the tiled layout is just the layout.

## Focus fit

Press Cmd+Shift+F (Ctrl+Shift+F with canvas focus), mode `f`, or the focus icon
beside the folder icon to fit the selection, else everything in the current
space (or Home). Double-clicking a cell does the same for that cell. The canvas
bounds already exclude the file panel, so it centres in the visible area. The
second press restores the camera from before the fit; if you moved the camera
in between, it fits again instead.

In canvas mode, arrows select the cell whose centre is nearest in that
direction (ties keep document order; nothing there, nothing changes).
Shift+arrows move the selection one gap through the same solver as a drag,
saved once, as one undo step.

## Action targets

Pane actions use the selected terminal or agent cells in the current space.
With no pane selected there, they use the pane holding keyboard focus, if any.
Space cards and previews are not pane targets.

## Closing and stacking panes

Backspace or Delete closes every target pane behind one dialog that lists each
with its icon and name; "Terminate and close all" ends them, "Cancel" or Esc
closes nothing. With the terminal prompt off in settings and no agent among
them, they close at once. Cmd+A (Ctrl+Shift+A) selects every pane in the
current space, only while the canvas has the keyboard.

A click anywhere on a pane brings it to the front. The pane menu has "Bring to
front" and "Send to back" (`canvas.front`, `canvas.back`) for the targets. Draw
order is the `z` of each card in the saved document, so it persists like
positions.
