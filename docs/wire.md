# Wire

Wire lets two cells of one space send each other short text postcards. You draw
the edge; the edge is the address book and the permission. Nothing is typed into
anyone when you connect, and nothing costs a token until somebody writes.

## Use it

- Hover a cell: a round "+" shows at the middle of each side. Drag from one to another cell
  of the same space and let go; release on empty canvas cancels, and a cell in
  another space says "Same space only for now". Sub-agents and `hc run` helpers
  are wired to their parent already (the edge ends in a dot at the child).
- Agents use `canopy wire` (their note says how; `canopy` on their PATH must be
  this app, which the installed app's `~/.local/bin/canopy` link provides): `tell`, `ask`, `reply`,
  `wait`, `read`; with no arguments it lists peers and waiting mail. A plain
  terminal is a peer that can only be read, once, on demand.
- An edge glows once when a postcard is typed into its end, shows a count while
  some wait, and a hollow dot while an `ask` has no reply. Click it to open the
  space's thread in the file panel as a conversation; type there and the line
  goes to both ends as a postcard from `you`. Right-click: Open thread, Reset,
  Remove. Select an edge and press Delete to remove it. Closing a cell removes
  its edges.
- A quote sent to an agent is a postcard from `you` too: it is pasted, not
  submitted, and logged in the thread.
- The breaker: a wire allows 8 agent postcards (`wire_budget`) since you last
  wrote to either end or reset it, then turns dashed with a reset dot. Click the
  dot to let it go on. `wire_limit` (default 1000 characters) and `wire_budget`
  are hand-edited in settings.json and never shown in Settings.

## How it works

- Edges are in `canvas.json` as `wires: [{a, b, born}]`, by cell key. Born edges
  are computed from sub-agent cells and the core's `parent`.
- Postcards are appended to `<app home>/wires/<space id>.jsonl`, one JSON
  object per line (`n, at, from, to, kind, re, text, refs, delivered`);
  receipts (`delivered`, `pending`) and breaker `reset`s are lines too. A receipt’s
  `delivered` is true for delivery, false for pending; old receipts are still read
  by their `kind`. The file is append-only and every writer takes a file lock.
  Text is always shown as text.
- `canopy wire` checks the edge, the length and the breaker, appends, then
  types `<wire from="name (address)" n="17" kind="ask">text</wire>`
  using `hc send` only at a turn boundary (idle). The address distinguishes peers
  running the same program. A CLI marked `"wire": "queue"` in programs.json
  (Claude) is typed to at once, because it queues the line itself. A busy,
  blocked or exited receiver keeps the postcard; the sender is told `queued` or
  `not running`. A sub-agent cell's postcards go to its parent, addressed
  `<wire for="sub <id> '<name>'" ...>`.
- The app is the postman: when the core reports a terminal idle (or an agent
  started in it), the waiting postcards are typed. There is no timer; the
  thread folder is watched only to redraw edges. `canopy wire wait` blocks on a
  file watch, not a sleep loop, and checks in after 180 s (exit 124): wait again.
- Claude also runs `canopy hook wire` on session start and each prompt: it prints
  a line only when the drawn peers differ from what that terminal was last told.
