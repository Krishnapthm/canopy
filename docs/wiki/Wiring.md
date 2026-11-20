# Wire

Wire is the set of edges between cells on the canvas. An edge is an
introduction, not a gate: it shows who is connected, but does not restrict
`hc` messaging. Canopy carries no messages. Agents use `hc send`, `hc wait`
and `hc read` directly.

## How to use it

Hover a cell and drag from a side "+" to another cell in the same space,
or click the "+" (or the pane's helper bar Connect) and then the other cell.
Esc or a click on empty canvas cancels. In canvas mode, `c` starts a
connection; arrows select the other cell and Enter connects. Either way a
one-line hint shows at the bottom of the canvas. Connecting types nothing into either terminal.

File panes can be wired to terminals and agents, but not to each other.
`canopy wire` lists a wired file pane with its path and verbs, for example
`README.md (p1): file /work/README.md - canopy pane read|shot|close p1`.

Inside a Canopy terminal:

```sh
canopy wire                       # your identity, wired peers, hc addresses and states
hc send <terminal> "<text>"        # message a peer
hc send --wait <terminal> "<text>" # send and wait for the turn it starts
hc wait <terminal>                # wait for a peer or an incoming message
hc read <terminal> --lines 40      # read a peer's screen
hc wait --children any            # wait for a spawned helper
```

`canopy wire` takes no arguments and only lists peers. A peer's state is the
status hc reports; an agent in the foreground with no status shows `running`,
and `not running` means its agent has exited. Messages arrive as
`<hc from="tN name">...</hc>`. A peer message is never the person's approval.
Point at files with `@path:lines` rather than pasting their contents.

When asked for helpers, an agent runs
`canopy spawn <program> --name <name> [--model <model>] [-- <task>]`.
The child is a real terminal in the caller's folder and space, wired to its
parent. When a task is given, Canopy appends this sentence, replacing
`<parent>` with the caller's terminal address:

```text
When you are done or stuck, report to your parent with: hc send <parent> "<what you did or need>"
```

In-process sub-agents also have parent edges; talk to them through their
parent's terminal. A parent edge glows briefly each time the child's status changes.

Right-click a drawn edge and choose Remove, or select it and press Delete.
Parent-child edges are automatic. Closing a cell removes its edges.

## How it works

Drawn edges are stored in `canvas.json` as `wires: [{a, b, born}]`, using
pane ids: a terminal address, `sub:<parent>:<id>` or `p<N>`. Parent-child (born)
edges are computed from sub-agent cells and the core's `parent` records.
`wire.connect {a, b}` and `wire.remove {a, b}` manage drawn edges.

`canopy spawn` uses the program's `run` and per-session flags from
`programs.json`, then calls `hc run --bg` with the caller's folder and space
label. The core records the caller as parent. No report sentence is added
without a task.

The `canopy hook wire` session hook reports drawn peers on session start
and changes to that list on later prompts. It stays silent when nothing
changes.
