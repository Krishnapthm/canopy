# Canvas layout

At rest, every cell, sub-agent cell and file preview keeps `canvas_gap` world
pixels from its neighbours (default 24). On Home, space cards keep the same gap.

## Spaces are rooms

Home shows each space as a card, like a Figma frame: its name and color dot
above a rounded box tinted with its color, which says how many terminals it
holds, how many are working, and shows a bell when one waits for you. Double-
click the name to rename it; right-click the card to rename, recolor or delete
it (with a confirmation). Cards sit next to the cells outside any space.
Double-click a card, or select it and press Enter, to go inside: the whole
canvas belongs to that space, with no frame, so its cells grow freely. The
toolbar then reads **Home / space**; click Home, or run `canvas.home`, to leave.
`canvas.space` with `next`, `prev` or a number goes into another space. A new
space opens straight away, and the app reopens where you closed it. Each place
keeps its own camera. Older `canvas.json` files move each cell into the space
whose frame held it, and the frame becomes that space's card.

## Use

- Drag a cell, or resize a preview: neighbours step aside and return when you
  move on. Drag a space card on Home: only the card moves.
- Enter [canvas mode](keys.md) to select a collapsed cell without expanding it.
  Click a cell to select it. Shift-click adds or removes cells. Click empty
  canvas to clear the selection.
- Right-click a cell (its pill, header or terminal) for its own menu: Rename,
  Pop out or Bring back, Expand or Collapse, Clear finished, Close terminal
  (asks first) and Kill terminal (ends it and its sub-agents now).
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
  centred, so the padding is even on all sides.
- Input: keys go to the terminal once it is focused, as in any terminal, so
  Backspace, Delete and Cmd+Z belong to it. The wheel scrolls a focused
  expanded terminal; over any other cell, or with Cmd (Ctrl) held, it pans or
  zooms the canvas.
- Zoom: `terminal::Scaled` multiplies font size, cell size and padding while a
  terminal draws, so the text is real text at every zoom. While the zoom is
  moving (and 160 ms after) the body is a flat fill and the header stays; text
  returns once it rests. An expanded cell keeps its size at every zoom; below
0.5 zoom its body stays a flat fill instead of text. No timer
  runs while nothing moves.

## Tile view

Press Cmd+Shift+G (Ctrl+Shift+G), the grid icon beside the focus icon, or
**Tile view** in the canvas menu to fit every expanded cell and preview of the
current space into the visible canvas at 100% zoom, like tmux's tiled layout.
Collapsed cells (and space cards on Home) stay pills and line up along the
bottom. Press again, or leave the space, to put everything back where it was.

- The selected tile is the large one: it takes 60% of its row, and its row at
  least 60% of the height (more when needed to stay the largest); the others
  share the rest. Click or arrow to another
  cell and the layout follows. Expanding, collapsing, opening or closing a cell
  re-tiles; a dragged tile snaps back.
- A tile smaller than 320 x 140 shows its icon and name instead of terminal
  text, and its program keeps its last grid size. The entered cell always shows
  its terminal.
- `tile_gap` in [settings](settings.md) sets the space between tiles (default 0,
  edge to edge); editing it re-tiles live.
- Tiles resize their programs like any cell: columns and rows follow the tile.
- How it works: `canvas::tile` keeps tiles in reading order by position and
  picks `ceil(sqrt(n))` columns, with the last row short and stretched, as in
  tmux. Changing the selection only changes sizes, never the order. The geometry from before
  is kept in memory only; after a restart the tiled layout is just the layout.

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
