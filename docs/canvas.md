# Canvas layout

Cells keep 24 world pixels of spacing. Dragging a cell or resizing a preview
pushes neighbours aside. Moving away lets them return. Spaces grow to contain
their cells with 24 pixels of padding, pushing other spaces and their cells.
Spaces do not shrink automatically. Cells outside spaces also push spaces aside.

Right-click a space, a terminal cell or empty canvas and choose **Arrange** to
repeat the last layout, or **Arrange >** to pick Tidy, Row, Column or By wires. Tidy keeps rough reading
order. By wires puts parents left of children using terminal parent edges.
Click a cell to select it; Shift-click terminal cells to add or remove them.
Click empty canvas to clear the selection. A space menu always arranges that
space. `canvas.arrange` uses selected cells, otherwise the current space;
`{ "space": "id" }` explicitly arranges a space. Add `layout` to choose a
layout, or `picker: true` to open the picker. The last layout survives restart.

**Undo layout** in the space menu or `canvas.undo` restores geometry before the
last move, resize or arrange. With the canvas focused, Cmd+Z on macOS or
Ctrl+Shift+U on Linux does the same. Focused terminals keep their keys.
Undo has one level and lasts until the window closes.

The solver starts from rest rectangles on each pointer sample, fixes the
moved rectangle, then places neighbours nearest first. The same function
separates spaces. Display rectangles ease toward targets over 120 ms and
request frames only until settled. Releasing commits the resolved layout;
`canvas.json` saves once after motion settles. Undo stores geometry only,
so it never restores a closed terminal.
