# Canvas layout

Cells never overlap. Every cell, sub-agent cell and file preview keeps 24 world
pixels from its neighbours, and spaces keep the same gap from each other.

## Use

- Drag a cell, or resize a preview: neighbours step aside and return when you
  move on. Drag toward a space's edge: the space grows and pushes the spaces
  next to it, with their cells. Spaces never shrink by themselves.
- Click a cell to select it. Shift-click adds or removes cells. Click empty
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
