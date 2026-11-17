# Panes

Everything on the canvas is a pane. The kinds today are terminal (terminals
and agents), sub (sub-agent cells), and file (text previews).

## Helper bar

Hover or select a pane to see its helper bar above the top-left corner.

- Terminal: Enter, Expand (or Collapse when expanded), Connect, Close.
- File: Open as tab, Close. Closing a file pane needs no dialog.
- Sub-agent cells have no bar.

Tooltips give the button's name and key. Hold Option in canvas mode to see
keys on the buttons. Bars hide during a drag, resize, marquee or pending layout.

The shared helper bar draws the actions for each pane kind, scaled with zoom
between 75% and 100%. It sits 8 px above the pane, clear of the wire handle.

## Saved form

`canvas.json` version 3 stores `panes: [{id, kind, rect, z, space, props}]`.
`space` is omitted for panes on Home; kind-specific fields live in `props`.

```json
{
  "version": 3,
  "panes": [
    {"id":"t3","kind":"terminal","props":{"size":[800,500]},"rect":[0,0,800,500],"z":1,"space":"s"},
    {"id":"sub:t3:a","kind":"sub","props":{"parent":"t3","sub":"a"},"rect":[880,0,220,36],"z":2,"space":"s"},
    {"id":"p1","kind":"file","props":{"path":"/tmp/plan.md"},"rect":[0,600,560,420],"z":3}
  ]
}
```

A terminal's id is its core address. A sub-agent's id is
`sub:<parent>:<id>`. File ids are `p<N>`; the saved `file_ids` counter
keeps a closed file pane's id from being reused.

Version 1 and 2 files load with cards and previews moved into panes. Card keys
are kept so saved wires stay valid. Older Canopy versions cannot open a version 3
canvas.
