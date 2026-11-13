# Canvas layout

Cells never overlap. Every cell, sub-agent cell and file preview keeps 24 world
pixels from its neighbours, and spaces keep the same gap from each other.

## Use

- Drag a cell, or resize a preview: neighbours step aside and return when you
  move on. Drag toward a space's edge: the space grows and pushes the spaces
  next to it, with their cells. Spaces never shrink by themselves.
- Enter [canvas mode](keys.md) to select a collapsed cell without expanding it.
  Click a cell to select it. Shift-click adds or removes cells. Click empty
  canvas to clear the selection.
- Right-click a space, a cell or empty canvas: **Arrange** repeats the last
  layout, **Arrange >** picks one. **Tidy** is a grid in reading order, **Row**
  and **Column** line cells up, **By wires** puts parents left of their
  children and sub-agents. The choice is saved in `canvas.json`.
- Arrange works on the selected cells of one space (the space of the cell you
  selected last), otherwise on the current space. A space menu always arranges
  that space.
- **Undo layout** (space menu, `canvas.undo`, Cmd+Z on macOS, Ctrl+Shift+U on
  Linux) puts geometry back as it was before the last move, resize or arrange.
  One level, kept until the window closes. The keys act only while the canvas
  has focus; a focused terminal gets them as usual.

`canvas.arrange` takes `layout` (`tidy`, `row`, `column`, `wires`), `space`
(a space id) or `picker: true` (open the layout menu).

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

- Size: 100 x 30 cells of the terminal font at zoom 1, header included. The
  size is saved per cell in `canvas.json` (`size`; older files load as pills).
- Neighbours: expanding pins the cell and runs `canvas::solve`, so the cells
  around it give way and stay one gap clear, also of each other. Each
  expansion remembers what it moved; collapsing puts back what is still where
  the expansion left it, so cells you moved yourself stay put. Collapse in
  the reverse order of expanding to get everything back.
- PTY size: columns and rows come from the cell's size at zoom 1 and are
  applied on release of the corner, never from the on-screen size.
- Input: keys go to the terminal once it is focused, as in any terminal, so
  Backspace, Delete and Cmd+Z belong to it. The wheel scrolls a focused
  expanded terminal; over any other cell, or with Cmd (Ctrl) held, it pans or
  zooms the canvas.
- Zoom: `terminal::Scaled` multiplies font size, cell size and padding while a
  terminal draws, so the text is real text at every zoom. While the zoom is
  moving (and 160 ms after) the body is a flat fill and the header stays; text
  returns once it rests. Below 0.5 zoom the cell draws as its pill. No timer
  runs while nothing moves.

## Focus fit

Press Cmd+Shift+F (Ctrl+Shift+F with canvas focus), mode `f`, or the focus icon
beside the folder icon to fit the current space. The canvas bounds already
exclude the file panel, so the space centres in the visible area. The second
press restores the camera from before the fit; if you moved the camera in
between, it fits again instead. With no current space, it fits the cells
outside spaces.

In canvas mode, arrows select the cell whose centre is nearest in that
direction (ties keep document order; nothing there, nothing changes).
Shift+arrows move the selection one gap through the same solver as a drag,
saved once, as one undo step.
