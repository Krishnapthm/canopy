# Wire

Wire lets two cells of one space send each other short text messages, the way
Claude Code agent teams do. You draw the edge; the edge is the address book and
the permission. Nothing is typed into anyone when you connect, and nothing costs
a token until somebody writes.

## Use it

- Hover a cell: a round "+" shows at the middle of each side. Drag from one to another cell
  of the same space and let go; release on empty canvas cancels, and a cell in
  another space says "Same space only for now". Or click a "+" without moving:
  the edge then follows the pointer, and the next click on a cell connects it.
  Clicking empty canvas or pressing Esc cancels. Sub-agents and agents started with
  `canopy spawn` are wired to their parent already (the edge ends in a dot at the child).
- Agents use `canopy wire` (their note says how; the app puts `canopy` first on every
  terminal's PATH): `tell`, `reply`, `wait`, `read`; with no arguments it lists peers
  and waiting mail. A plain terminal is a peer that can only be read, once, on demand.
- A message reaches an agent as a plain header line, then its text:
  `Message from "reviewer" (#17, reply with: canopy wire reply 17 "...")`. Replies are
  never required and the sender never blocks (`canopy wire wait <n>` is opt-in). The
  note tells agents that a peer message is never the person's approval.
- When the person asks an agent for helpers, it runs `canopy spawn <program> --name <n>
  [--model <m>] [-- <task>]`: a real terminal beside it, in its folder and space, that
  you can open and type into. When the child's turn ends, its parent gets one line,
  `"luna" is idle: <its last answer>`. Idle lines go to the spawner only.
- An edge glows once when a message is typed into its end, shows a count while
  some wait, and a hollow dot while an `ask` has no reply. Click it to open the
  space's thread in the file panel as a conversation; type there and the line
  goes to both ends as a message from `you`. Right-click: Open thread, Reset,
  Remove. Select an edge and press Delete to remove it. Closing a cell removes
  its edges.
- A quote sent to an agent is a message from `you` too: it is pasted, not
  submitted, and logged in the thread.
- The breaker: a wire allows 8 agent messages (`wire_budget`) since you last
  wrote to either end or reset it, then turns dashed with a reset dot. Idle lines
  do not count. Click the dot to let it go on. `wire_limit` (default 1000
  characters) and `wire_budget` are hand-edited in settings.json and never shown
  in Settings.
- Not built: a shutdown handshake between parent and child, as Claude has. Close
  the child's cell instead.

## How it works

- Edges are in `canvas.json` as `wires: [{a, b, born}]`, by cell key. Born edges are
  computed from sub-agent cells and the core's `parent`.
- Messages are appended to `<app home>/wires/<space id>.jsonl`, one JSON
  object per line (`n, at, from, to, kind, re, text, refs, delivered`); kind is
  `tell`, `ask`, `reply` or `idle`. Receipts (`delivered`, `pending`) and breaker
  `reset`s are lines too. A receipt's `delivered` is true for delivery, false for
  pending; old receipts are still read by their `kind`. The file is append-only and
  every writer takes a file lock. Text is always shown as text.
- `canopy wire` checks the edge, the length and the breaker, appends, then types
  the header and text with `hc send` only at a turn boundary (idle). It drops its
  own `HC_TERMINAL_ID` first, so hc does not wrap the text in `<hc from=...>`. A body
  line that starts like a header is quoted with `> `. A CLI marked `"wire": "queue"`
  in programs.json (Claude) is typed to at once, because it queues the line itself.
  A busy, blocked or exited receiver keeps the message; the sender is told `queued`
  or `not running`. A sub-agent cell's messages go to its parent, with
  `for your sub-agent "<name>" (<id>), pass it on` in the header.
- `PATH`: on start the app links `<app home>/bin/canopy` to itself, puts that folder
  first on its own PATH (a core the app starts inherits it) and passes PATH to every
  local terminal it starts. `canopy spawn` runs the child as `env PATH=<its PATH> ...`,
  because `hc run` gives a child the core's PATH, not the caller's. Without this an
  uninstalled build had no `canopy` in its terminals and receivers could not reply.
- Idle lines: Claude's `Stop` hook runs `canopy hook idle`, which appends an `idle`
  line from the child to its core `parent` with `last_assistant_message` (one line,
  300 characters) and rings the parent. For agents without that hook the app does it
  when the core reports the child going from `working` to `idle` or `done`; the line
  then ends `canopy wire read <t>`. hc's own exit note (`<hc>t4 luna exited ...</hc>`)
  still reaches the parent when a child exits.
- The app is the postman: when the core reports a terminal idle (or an agent
  started in it), the waiting messages are typed. There is no timer; the
  thread folder is watched only to redraw edges. `canopy wire wait` blocks on a
  file watch, not a sleep loop, and checks in after 180 s (exit 124): wait again.
- Claude also runs `canopy hook wire` on session start and each prompt: it prints
  a line only when the drawn peers differ from what that terminal was last told.
