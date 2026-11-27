# Panes, wires, spaces and app commands

## Pane kinds

Everything on the canvas is a pane. Kinds: terminal and agent panes (ids are core
addresses like `t7`), sub-agent cells (`sub:<parent>:<id>`), and panes opened with
`canopy pane new`: `file` (text, images, PDFs), `browser`, `sim`, and mod panes
such as `mod:note` (ids `p<N>`, never reused).

```sh
canopy pane new file /abs/or/relative/path.md [--wire]
canopy pane new mod:note [--wire]
canopy pane read p1        # <pane-content pane=... kind=...> text </pane-content>, cut at 256 KiB
canopy pane shot p1 [file.png]   # needs the pane on screen (browsers excepted)
canopy pane close p1       # a note holding text refuses: only the person closes it
```

- `read` on a PDF gives its page text; on an image, its path and pixel size.
- A note's text is what `read` returns; mods save it with `canvas.pane_set`.
- `--wire` places the pane beside your terminal and wires it to you.
- Panes you open never move the person's camera. One far off screen gets a toast.

## Where a pane goes

`--place canvas` (the default for `canopy pane new`), `--place panel` (a tab in the
Files panel, browsers and sims only) or `--place pinned` (the screen's top right,
fixed while the canvas moves). Later: `canvas.pane_pin {pane, pinned?}`,
`canvas.pane_dock {pane, docked?}`. `canvas.pane_color {pane, color}` takes a
palette name (`gray`, `red`, `orange`, `yellow`, `green`, `teal`, `blue`,
`purple`) or `#RRGGBB`; `null` inherits the space's hue.

## Wires

A wire introduces two panes in the same space so their agents can find each
other. Canopy carries no messages; agents use `hc send`, and a message arrives as
`<hc from="tN name">...</hc>`. Spawned helpers and sub-agents are wired to their
parent already. The person draws a wire by dragging a pane's side "+" onto another.

```sh
canopy wire                                     # you, your peers and wired panes with their verbs
canopy do wire.connect '{"a":"t7","b":"p3"}'
canopy do wire.remove '{"a":"t7","b":"p3"}'
```

## Quotes and annotations

- The person selects lines in a file pane and sends them: they arrive pasted in
  your prompt as `@path:start-end` then the lines (a `> ` quote for Markdown, a
  fenced block otherwise), the path relative to your folder when it can be.
- Browser and simulator annotations arrive as a submitted prompt: the annotated
  screenshot first (`@<png path>`), then numbered comments. Open the image.

## Spaces, tabs and splits

New spaces and tabs show at once: create them only when asked, and always give
them a clear name.

```sh
canopy do space.new --name api --folder /path/to/api
canopy do tab.new --name "fix login" --argv '["claude"]' --task "Look at the login bug"
canopy do pane.split --direction right --pane p12
canopy do agent.focus --terminal t7        # on the canvas: also enters that pane
canopy do space.new --name "remote api" --machine vps --folder /srv/api
```

A remote space's folder is on that machine, and its tabs and splits stay there.
`hc machine ls` lists machines. Use `hc run --on vps ...`, `hc exec --on vps ...`
and `hc file ... --on vps ...` for remote work, not your local file tools.

A terminal you start with `hc` shows in the window only with the label
`herdr.space=<id>` (and optionally `herdr.tab=<id>`). Leave background workers
unlabeled.

## `canopy do`

`canopy do <command>` takes `--key value` arguments (JSON for arrays and objects)
or one JSON object, and prints the reply (`{"pane": ..., "terminal": ...}`) or
`{"error": ...}` with exit 1. Unknown argument names fail and list the valid ones.
Target ids (`id`, `pane`, `space`, `target`, `terminal`) must be strings. Commands
from an agent never open a dialog asking for missing arguments: give them all.

A successful launch reply means the terminal was created, not that its program
works. If it exits within two seconds, the window shows a toast with the exit
code and last lines.

Useful canvas commands: `canvas.fit` (fit the space in view, or back),
`canvas.arrange {layout?: "tidy" | "row" | "column" | "wires"}`,
`canvas.zoom_in`, `canvas.zoom_out`, `canvas.zoom_reset`, `canvas.expand {terminal?}`,
`canvas.close {pane}` (running agents ask the person first), `canvas.note_new {point?}`,
`panel.close`. `canopy commands` lists everything.

## Local files

`file.rename {from, to}`, `file.copy {from, to | into}`, `file.move {from, to | into}`,
`file.mkdir {path}`, `file.new {path}`, `file.trash {path}`, `file.reveal {path}`
take absolute paths; `into` picks a free name inside a folder.
`file.mode {path, mode: "source" | "preview", pane?}` switches Markdown, JSONL,
HTML and SVG between source and preview. On macOS, `preview.external {path}`
opens an image or PDF in Apple Preview.
HTML opens in a browser pane on macOS and in Source on Linux and Windows. Editing a
file keeps a draft: closing a file pane or its tab with unsaved edits asks first, and a
save conflict keeps the draft and pauses Auto save (`files.auto_save {on}`).

## Seeing the whole window

`canopy shot [file.png]` saves the window's last frame (default
`canopy-shot.png` here) and prints the path: the app's own rendering, even when
covered or minimized, no screen permission needed. Look at it before claiming a
visual change worked.
