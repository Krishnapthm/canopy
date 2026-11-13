# Canopy: Wire, canvas feel and keyboard

Plan written 2026-11-13 on `pivot/canvas`. Owner: Krishna. Phases 1 to 6 are built and
merged; `docs/wire.md` and `docs/sub-agents.md` describe what shipped, and where Wire
now differs from this plan (plain message headers, idle lines, `canopy spawn`), they win.
Revised the same day with Krishna's decisions and the phase 0 results (Claude done,
Codex and two app items still open).
It follows `docs/canopy/plan.md` (rules in its section 3 and AGENTS.md still hold) and
answers `docs/canopy/fable-prompt.md`. Each phase is one or two sittings for a fresh
session and ends with something Krishna can check by hand.

Checked against: Claude Code 2.1.322, codex-cli 0.167.2, hc 0.1.20. Marks used below:
**[V]** verified on this machine (help text, installed binary, source, files on disk),
**[D]** from vendor docs only, **[?]** not verified, needs the phase 0 run.

## The one-page version

1. **The sub-agent bug** is three small faults in the current path, not a missing layer.
   Fix them in place (phase 1), then let Wire replace the "Message" action (phase 2).
2. **Wire** is our own protocol: *postcards over a doorbell*. A drawn edge is the address
   book and the permission. A message is a short postcard that points at files instead of
   carrying them. Canopy rings the receiver's terminal only at a turn boundary. Every
   postcard is kept in a plain file the person can read and write in. No server, no
   polling, no vendor API, no tokens until somebody actually writes.
3. **Canvas feel**: one small solver, used at two levels (cells in a space, spaces on the
   canvas), always solved from where the person left things, so neighbours give way and
   come back.
4. **Keyboard**: one prefix opens "canvas mode", where single letters are safe because
   nothing reaches the terminal. A hint bar teaches it.
5. **Order**: 0 verify, 1 sub-agents and small UI, 2 Wire, 3 spacing and arrange,
   4 in-canvas terminals, 5 keyboard, 6 settings.

## Decisions by Krishna, 2026-11-13 (do not re-open)

- The protocol is called **Wire**.
- Breaker: 8 agent postcards per wire to start.
- A plain terminal can be a peer, **read-only and only on demand**: an agent reads it
  when it needs to (`canopy wire read <peer>`). Nothing watches or streams a terminal.
- Postcard length is a `wire_limit` key in `settings.json` (default 1000). It is
  hand-edited only and never appears in the Settings screen.
- One level of undo for moves and arrange.
- Canvas mode prefix is **Cmd+J** (Ctrl+Shift+J on Linux). Ctrl+B is out: Claude Code
  and tmux use it inside cells.
- **Delete a cell from the canvas**: with a pill selected, Backspace or Delete closes
  it. A running agent always gets a confirm alert ("Terminate and close"). A plain
  terminal gets a confirm dialog with a "Don't ask again" checkbox.
- **Docs travel with the change** (AGENTS.md rule): implementers update the page in
  `docs/` and the doc comments for what they touch; the orchestrator reviews them for
  accuracy and brevity and fixes what falls short. No one-shot documentation pass.

## Research summary (what the decision rests on)

How agents talk today, and why none of it is our transport:

| Approach | Transport | Survives restart | Idle cost | Why not ours |
|---|---|---|---|---|
| Claude sub-agents, `SendMessage` [V strings, D] | in-process | transcript yes, live state no | none | one vendor, parent only |
| Claude agent teams [D] | mailbox JSON under `~/.claude/teams` | in-process teammates are not restored | none | experimental flag, one vendor |
| Claude cross-session messaging [D] | per-session Unix socket | [?] | none | one vendor |
| Claude MCP channels [V strings, D] | MCP push | no (server is a child of the session) | none | research preview, needs a "dangerously" flag |
| Codex `queue --thread` [V help] | daemon | yes | none | one vendor; needs the daemon we launch without |
| Codex `agent_message_board`, `wake_on_agent_messages` [V features list] | n/a | n/a | n/a | "under development", off |
| MCP server we serve | tool calls | config yes | tool schemas in every request | receiver only sees mail when it calls a tool; per-vendor config; a server process per agent |
| A2A, ACP [D] | JSON-RPC servers | depends | none | each agent must be a server; ACP is editor to agent, not peer to peer |
| tmux `send-keys` orchestrators, claude-squad [D] | keystrokes | pane yes | none | collides with typing and dialogs; no record |
| MCP Agent Mail [D] | MCP + git/SQLite mailbox | yes | none unless polled | agents must poll or be told to check |
| `hc send` / `hc wait` (what we use now) [V] | core types into the PTY, waits for the person's draft, tags `<hc from=...>` | terminal yes | none | right transport, but no address book, no record, no canvas presence, no limit on chatter |

Three things worth noticing:

- Both vendors are building private message boards. They will not interoperate. A thin
  layer on what every CLI must have (a terminal, a shell, files) will outlive them.
- Every durable design is a mailbox, and every mailbox has the same hole: who wakes the
  reader. Canopy is the only party that already owns each agent's terminal and already
  gets a core event when a turn ends. That is the missing doorbell, for free.
- `hc send` already solved the hard part of typing into a live agent safely (it waits
  for an unfinished draft, refuses a blocked dialog). We build on it, we do not redo it.

Facts that shape the design:

- Claude records its system prompt once per conversation and reuses it on resume, even
  when a later launch passes different text [V help, V live: a note added on resume
  was invisible to the model]. So the launch note must be static. Anything that
  changes (who you are wired to) cannot live there.
- Claude hooks `SessionStart` and `UserPromptSubmit` `additionalContext` reach the
  model on a fresh start and, with fresh content, on resume (`source: "resume"`)
  [V live].
- A line typed into Claude mid-turn is **queued by Claude itself** and handled after
  the running tool; it does not interrupt [V live]. Idle, it is taken as a prompt.
- The core reports Claude's `working` / `done` status with no integration installed
  [V live], so the doorbell has its event.
- `SubagentStart` carries only `agent_id` and `agent_type`. `SubagentStop` adds the
  transcript path, `last_assistant_message` and `background_tasks` with the
  description. Resuming a sub-agent fires `SubagentStart` again with the same id.
  The sidecar `subagents/agent-<id>.meta.json` also holds the description [V live,
  V on disk].
- A Claude parent passes a message on to a finished sub-agent with `SendMessage`, and
  the sub-agent answers with its earlier context [V live].
- Codex accepts hooks per session (`-c 'hooks.SessionStart=[{hooks=[{type="command",
  command="..."}]}]'`) but shows a "Hooks need review" trust dialog for new hooks
  [V live]. Whether their context reaches the model is still [?].

Evidence: `harness-notes/claude-code.md` and `harness-notes/codex.md`, sections
"Live checks for Wire".

## Phase 0: verify before building (partly done)

**Done 2026-11-13 (Claude 2.1.322, Haiku, scratch core):** hook context on start and
resume, the resume prompt snapshot, a `<wire>` line while idle and mid-turn, core
status, sub-agent payloads, parent relay to a sub-agent. All passed; details above.

**Done 2026-11-13, second pass (Codex 0.167.2 gpt-6-luna, Claude Haiku, scratch core):**
1. Codex hooks run fresh and on resume and their text is stored, but the model never
   uses it: Codex ships with the static note and pull only, no hooks. A typed line
   while idle is a prompt; mid-turn it steers the running turn (not a queue); during
   a dialog `hc send` refuses. Hook trust persists only by writing the person's config.
2. Claude: a typed line while a permission dialog is up is refused by `hc send`.
3. The live pill repro moves into the phase 1 hand check.
4. The renderer spike moves into phase 4, as its first step.
5. Codex allow rule: `prefix_rule(pattern=["canopy", "wire"], decision="allow")` in the
   person's `~/.codex/rules/`; no per-launch flag.

How to run a check without the app: start a separate core (`HC_SESSION`, `HC_SOCKET`,
`XDG_STATE_HOME`, `XDG_CONFIG_HOME` under a scratch folder, as `scripts/dev` does),
launch with `hc run --bg --name x -- env -i HOME=$HOME PATH=$PATH TERM=xterm-256color
<agent> ...`, drive it with `hc send`, `hc keys`, `hc read --lines 0`, then
`hc server stop`. Use `env -i`: a launch that inherits another Claude session's
variables turns transcript saving off. Claude's Bash tool refuses a bare `sleep`, so
use `python3 -c "import time; time.sleep(25)"` when a test needs a long turn.

**Consequence (item 1 failed):** Codex ships with the static note and the pull path
only (`canopy wire`), no hooks. Still correct, just later to notice a new wire.

**Check by hand.** Read the "Live checks for Wire" sections in
`docs/canopy/harness-notes/`; each line has a version and what proved it.

## Phase 1: sub-agents become cells, and four small UI changes

### 1.1 The bug, explained

Root cause, all in the current path (`src/subs.rs`, `src/window/canvas_view.rs:1693-1769`):

1. **Wrong name.** The hook stores `agent_type` as the pill name (`subs.rs:41`).
   `SubagentStart` has nothing better [V], so every pill reads "general-purpose". The
   real name ("Audit sidebar name resolution") sits in the `.meta.json` sidecar, which
   the app reads only for the side panel title.
2. **Never leaves.** `SubagentStop` rewrites the label with `idle: true` and nothing
   ever removes `canopy.sub.<id>`. Labels live on the core terminal, so finished
   sub-agents pile up for the life of the terminal, across resumed sessions too.
3. **Not a cell.** Sub pills are drawn from a computed position inside `render`. They
   are not `Card`s: no drag, no focus, no saved position, no edge you can use. Left
   click opens a preview only if the transcript file already exists. Right click offers
   one item, "Message", which pastes `Message your sub-agent X (id): ` into the parent
   without Enter, which looks like nothing happened.

The payload facts are verified live; the pills themselves have not been seen in the
app yet (phase 0 item 3).

### 1.2 Decision

Keep the two hooks. They are the only source of a sub-agent's identity and they cost
nothing when idle. Replace everything after them:

- A sub-agent is a real card in `canvas.json` (`{"sub": "<id>", "parent": "<terminal>"}`),
  placed beside its parent once, then it stays where the person drags it.
- Name: the sidecar's `description`, falling back to the agent type. Read when the label
  event arrives and again when the transcript file appears (existing folder watch).
  `SubagentStop` also carries the description, so the stop hook can store it.
- A second `SubagentStart` with the same id (the parent resumed it) puts the cell back
  to `working`.
- State: `working`, then `done` (not "idle"). Done cells dim. They leave when the person
  picks "Clear finished" on the parent, and automatically when the parent's session
  ends (one more hook: `SessionEnd` clears the labels).
- Open: the card expands to its transcript, followed live (the existing
  `transcript_markdown`, re-read on file change). Read-only text, never markup.
- Message: through Wire in phase 2. Until then the menu item says what it does:
  "Message through parent".
- It is wired to its parent automatically (phase 2 makes the existing parent edge a
  wire).

**Ruled out.**
- *Turn off Claude's Agent tool and force helpers through `hc run`*: every helper would
  become a full session (slower, dearer, loses the vendor's own orchestration). The
  note already offers `hc run` for cross-vendor helpers; both can exist.
- *Scrape the parent's screen for sub-agent state*: unstable.
- *A separate sub-agent store*: labels already persist and already arrive as events.

**Open question.** Should done sub-agents fold into one "3 finished" chip on the parent
instead of staying as dim cells? I would start with dim cells and add the chip only if
it feels cluttered.

### 1.3 Small UI changes (same sitting)

- **Quote glyph.** No quote icon ships today [V `defaults/icons`]. Add Lucide `quote`
  (same icon family and license file we already ship) and use it in the quote popover.
- **View switch.** Remove the Agents / All / Terminals control from the top right; keep
  the file icon. macOS: the View menu already has the items (`src/macos.rs:61`), add a
  check mark on the active one. Linux: we draw our own chrome, so add one "primary
  menu" button (three lines) at the left of the titlebar, as GNOME apps do, built from
  the same list as the macOS menu bar so the two cannot drift. Keys and palette
  commands stay.
- **Quieter spaces.** 1 px border at 40% of the space color, fill at 4% (today 8% fill,
  60% border). One setting, `space_fill` (phase 6).
- **Delete a cell.** With a pill selected (not expanded, so no terminal has the
  keyboard), Backspace or Delete closes it. Reuse the confirm mod:
  - running agent: always an alert, "Terminate <name> and close?", buttons Terminate
    and Cancel. No way to silence it.
  - plain terminal: "Close this terminal?" with a "Don't ask again" checkbox, stored
    as `confirm_close_terminal: false` in `settings.json` so it can be turned back on
    by hand.
  - a sub-agent cell is not a process: Delete just clears it when done, and is
    refused with a toast while it works.
  Closing a cell removes its wires.

**Check by hand.** `scripts/dev`, Claude cell, ask for two sub-agents: two cells appear
named by their task, joined to the parent; drag one; click it and watch its transcript
grow; they turn "done"; "Clear finished" removes them. The top right shows only the
file icon; View menu switches views; spaces look like outlines.

## Phase 2: Wire (the main event)

### 2.1 What it is

**Wire: postcards over a doorbell.** Four ideas, each small:

1. **The edge is the address book.** There is no registry and no discovery. If a line
   is drawn between two cells, each can reach the other; if not, they cannot. Drawing
   is granting. Deleting the line revokes.
2. **Postcards, not transcripts.** A message is at most 1000 characters of text plus
   references: `@src/vt.rs:120-150`, the same form quote-to-agent already uses. A
   longer thought must be written to a file and pointed at. The size limit is the
   protocol's way of enforcing "no context dumping": nobody can paste their context
   into someone else's.
3. **Doorbell delivery.** Canopy types one tagged line into the receiver's terminal,
   and only at a turn boundary (the agent is idle and the person has no unfinished
   draft). Never mid-turn, never over a dialog. If the door is closed the postcard
   waits in the file and the bell rings when the core reports the turn ended. That
   report is an event Canopy already receives, so nothing polls.
4. **The person is on every wire.** All postcards of a space are one append-only file.
   Click an edge and the thread opens in the file panel as a conversation. Type there
   and you are a peer. Quote-to-agent becomes "a postcard from you".

And one guard that I think is new: **the breaker.** Each edge allows a small number of
agent-to-agent postcards (default 8) since a person last wrote on it or reset it. Then
the wire trips: further sends are refused with a clear reason, the edge turns dashed
with a reset dot, and a toast says so. Two agents can never burn tokens talking in
circles while you are at lunch. One click resets it.

### 2.2 The decisions asked for

**What an edge means.** "These two cells may send each other postcards." It is
undirected: a reply needs the way back, and a direction would imply a boss, which makes
Canopy an agent framework. Two kinds, one mechanism: *drawn* edges (by the person) and
*born* edges (a sub-agent or an `hc run` child to its parent, created automatically and
drawn with today's child dot). One-to-many and many-to-one are just more edges. An
edge belongs to the cell (the core terminal), not to the agent session, so it survives
the agent being closed and restarted in that cell. Closing the cell removes its edges.

**How an agent learns about a connection.** Three layers, cheapest first:
- *Static note at launch* (replaces today's `note` in `programs.json`): "You are in
  Canopy. `canopy wire` lists the cells you are wired to and any mail. Send with
  `canopy wire tell|ask <peer> "<text>"`. Point at files, do not paste them. Use it
  when the person asks, or when you are blocked on something a peer owns. Being wired
  is not a reason to write." Static, so Claude's prompt snapshot is fine.
- *Pull*: `canopy wire` prints the current peers. Always right, costs one tool call
  only when the agent needs it.
- *Nudge, where the CLI has hooks* (data in `programs.json` flags): a `SessionStart`
  and `UserPromptSubmit` hook runs `canopy hook wire`, which prints one line only when
  the peer list differs from what this session was last told
  ("Wired: +reviewer (codex), -web"). Unchanged means no output, so no tokens. On a
  restart or resume it prints the full list once: that is "told again automatically".
Connecting therefore types nothing into anyone. The agent finds out at its next turn,
which is exactly when it could act on it.

**How a message arrives, and whether you can see it.** The receiver sees:

```text
<wire from="reviewer" n="17" kind="ask">Does parse() own the buffer? @src/vt.rs:120-150</wire>
```

`kind` is `tell` (no reply expected), `ask` (reply expected) or `reply` (`re="17"`).
An agent answers with `canopy wire reply 17 "..."`. A sender that needs the answer
before going on runs `canopy wire wait 17`, a shell command that blocks and costs
nothing while it waits (same idea as `hc wait`). On the canvas: the edge pulses once
on delivery, shows a small count while postcards are undelivered, and a hollow dot
while an ask is unanswered. Clicking the edge opens the thread.

**Busy or not running.**
- Working: by default the postcard waits and the bell rings when the turn ends; the
  sender is told `queued`. A CLI verified to queue typed input itself is marked
  `"wire": "queue"` in `programs.json` and gets the line at once (Claude: verified,
  it shows as a queued message and never interrupts). This is data, not code.
- Blocked on a dialog: waits; the receiver's cell already shows the bell glyph, so the
  person knows why.
- Agent exited (shell left by core keep): waits; the sender is told `not running`.
  Canopy never starts an agent to deliver mail (no token spend without the person).
  When an agent starts in that cell, one line rings: "3 postcards waiting: canopy wire".
- Cell closed: the edge is gone, the send is refused, the sender is told.
- Breaker tripped: refused, with the reason and "ask the person".

**Quote-to-agent.** Same pipe. A quote is a postcard from `you` with one reference and
your optional message. One difference kept from today: a quote is pasted into the
prompt and *not* submitted, because you asked to review before sending. It is logged
in the thread like any other postcard. `quote.send` keeps working for scripts.

**Sub-agents on the wire.** A sub-agent has no terminal, so its postcards are delivered
to its parent's terminal addressed to it:
`<wire for="sub a1b2 'Audit names'" from="you" n="21">...</wire>`, and the note tells
the parent to pass these on. Verified live: Claude relays with `SendMessage` and the
sub-agent answers with its earlier context. A CLI without such a tool leaves its
sub-agent cells read-only, and the cell says so.

### 2.3 How it is built (small on purpose)

- **Edges** live in `canvas.json`: `"wires": [{"a": "<cell>", "b": "<cell>", "born": false}]`,
  serde-defaulted so old files load. Existing parent edges are computed as born wires,
  not stored.
- **Thread file**: `<app home>/wires/<space id>.jsonl`, one JSON object per line
  (`n, at, from, to, kind, re, text, refs, delivered`). Append-only, one `write` per
  line. The app watches the folder (existing watcher). The file panel renders it with
  the same reader used for sub-agent transcripts. Postcard text is always shown as
  text.
- **`canopy wire`** (new subcommand next to `canopy hook`): `(none)` peers and mail,
  `tell`, `ask`, `reply`, `wait`, `read <n>`. It reads `$HC_TERMINAL_ID`, checks the
  edge in `canvas.json`, checks the breaker, appends the line, then rings if the door
  is open by calling the core the way `hc send --if idle` does [V `hc send --help`].
  So an idle peer gets mail even with the Canopy window closed.
- **The postman** is the app: on a core status event that says a cell became idle, it
  rings for that cell's undelivered postcards. No timer.
- **Per-agent support stays data.** `programs.json` gets the new static note and, for
  CLIs with hooks, the two hook entries in `flags` (Claude: the existing `--settings`
  JSON, verified). Codex takes `-c hooks.<Event>=[...]` but asks the person to trust
  new hooks, so it starts with note and pull only until phase 0 item 1 says otherwise.
  An agent with neither still works through the note and pull. No vendor name in Rust.
- **Permissions.** `canopy wire` is one command prefix, so each CLI's own allow rule
  can cover it (Claude: add `Bash(canopy wire:*)` to the session `--settings`; Codex:
  phase 0 confirms the equivalent). Without it the first send asks the person once,
  which is acceptable.
- **Drawing.** Hover a cell: a 10 px round "+" at each corner. Drag from one to another
  cell: a live curve follows the cursor, the target cell highlights, release connects.
  Release on empty canvas cancels. Select an edge and press Delete, or right-click it:
  "Open thread", "Reset", "Remove". Edges only within one space in this plan; dragging
  to a cell in another space shows "Same space only for now" as a toast.
- **Docs.** `defaults/AGENTS.md` and `src/SKILL.md` gain a "Wire" section; the `hc
  send` guidance for helpers stays for parent and child work started with `hc run`.

**Not painting into a corner (cross-space, later).** Addresses in the file are cell
ids, not names; the thread file is per space, so a cross-space wire is one more file
at the root and one removed check. Core addresses already carry a machine
(`machine/t7`), so remote cells need no new idea either.

### 2.4 Ruled out

- **An MCP server served by Canopy.** Tool schemas ride in every request of every wired
  agent, the receiver still only sees mail when it calls a tool, each vendor needs its
  own config, and we would run a process per agent. More parts, worse idle cost.
- **Vendor channels (Claude channels, cross-session messaging, Codex queue/board).**
  Each works for one vendor and several are previews.
- **A message in an escape sequence (the agent prints an OSC, the terminal routes
  it).** Lovely idea, works over ssh for free, but an agent's shell tool does not
  write to the terminal, so it would not get out [?]. Parked; worth one experiment
  later because it would make remote wires trivial.
- **Telling agents the peer list in the system prompt.** Wrong on resume because of
  the prompt snapshot [V].
- **Typing a "you are now connected" line on connect.** That is a turn, so tokens, so
  it breaks "connecting never triggers chatter".
- **Shared instruction file edited per connection (AGENTS.md / CLAUDE.md).** Touches
  the person's repo and is only re-read at session start.
- **Hard enforcement below `canopy wire`.** An agent with a shell can still call
  `hc send` directly. Blocking that means patching the core. The wire is the taught
  and logged path; raw `hc` stays the escape hatch for parent and child work.

### 2.5 Risks and questions for Krishna

- **Risk: a model treats a postcard as an instruction from the person.** The note says
  postcards are from peers, not from you, like `hc`'s wording today. Watch for it in
  the first live runs.
- **Risk: hooks again.** The nudge hook is optional by design; if it proves fragile,
  delete the two flags and the pull path still gives correct behaviour.
- **Plain terminals as peers (decided: yes, read-only, on demand).** `canopy wire
  read <peer>` returns the last lines of that terminal's screen once, when the agent
  asks. `tell` and `ask` to a plain terminal are refused. No watcher, no stream, no
  cost while nobody reads.
- Breaker 8, limit in `wire_limit`, name Wire: decided, see the top of this plan.

### 2.6 Check by hand

1. Two agent cells in one space. Hover: "+" at the corners. Drag: an edge appears.
   Neither terminal shows any new text.
2. In A: "ask the other agent which test runner this repo uses". A runs
   `canopy wire ask ...`; the edge pulses; B shows one `<wire ...>` line and answers;
   A gets a `reply`. Click the edge: the thread reads as a conversation.
3. Make B busy, send again: the edge shows a count, B gets the line when its turn ends.
4. Quit B's agent, send: A is told "not running". Start an agent in B's cell: one line
   says mail is waiting.
5. Restart Canopy: edges are there. Resume A: its first turn knows its peers.
6. Tell both to "keep discussing": the wire trips at 8, a toast explains, one click
   resets.
7. `cat ~/.config/canopy/wires/*.jsonl` (scratch home under `scripts/dev`): every
   postcard is there in plain JSON.
8. Idle check: with edges drawn and nobody talking, `top` shows no Canopy CPU and no
   agent spends a token.

## Phase 3: spacing, growing spaces, arrange

**Goal.** Cells never overlap, keep one gap, and the motion feels calm.

**What good tools do.** Almost none push live: Figma, tldraw, Miro, Obsidian and React
Flow let things overlap and offer "tidy up" afterwards. Dashboard grids
(react-grid-layout) push live but only on a grid. The known bad feel comes from
*incremental* pushing: chains that cascade and cards that stay displaced after you
move away. The fix is the same everywhere it is done well: keep a rest position and
solve from it.

**Decision: a rest-position solver, about 80 lines, no dependency.**
- Every cell has a *rest* rect (where the person left it) and a *shown* rect.
- While one cell is dragged, resized or expanded it is pinned. Each frame:
  `shown = solve(rest, pinned)`. Solve walks outward from the pinned cell: any cell
  closer than the gap is moved along the axis of least overlap until it is one gap
  away, and is then fixed for the rest of the pass. One pass, deterministic, nothing
  oscillates, and a cell springs back the moment the pinned one leaves, because the
  input is always the rest layout, never last frame's output.
- Shown rects ease toward their target (about 120 ms, the existing `ease_out`), which
  is what makes it feel magnetic instead of mechanical. Frames are requested only
  while something moves.
- On release, shown becomes rest and `canvas.json` is saved once.
- **Spaces use the same function one level up.** A space's rect is at least the union
  of its cells plus padding, so it grows live; the grown rect is the pinned input to a
  solve over spaces, which pushes other spaces (and carries their cells). A space
  never shrinks by itself.
- 50 cells is about 1,200 pair checks per frame: far under a millisecond.

**Arrange.** One command, `canvas.arrange`, on the selection if there is one, else the
current space. Four layouts, picked from a small popover that remembers the last one
(so the one-click action is "do the last one again"):
- **Tidy** (default): a grid that keeps rough reading order, so nothing jumps far.
- **Row** and **Column**.
- **By wires**: parents left, children right, layered by edge distance. Twenty lines
  of breadth-first layering; no layout library.
Arrange animates with the same easing and is one undo step.

**Ruled out.** Physics / force layout (drifts, never quite settles, and "idle costs
nothing" forbids a simulation ticking). A constraint solver crate such as Cassowary
or VPSC (a dependency and far more power than rectangles with one gap need). Snapping
grid (Krishna asked for a free canvas). Masonry, radial, pack: nobody has asked.

**Risks and questions.**
- Pushing a neighbour against its space's edge grows the space. That is the decided
  rule, but it means a drag near an edge moves other spaces. If it feels busy, the
  `canvas_push: "drop"` setting (phase 6) is the pressure valve.
- **Undo (decided: yes).** One level: Cmd+Z on the canvas (Ctrl+Shift+Z on Linux is
  taken by nothing in `layout:main`; check before binding) restores the layout as it
  was before the last move, resize or arrange. Keep one snapshot of rest rects, not a
  history.

**Check by hand.** Drag a cell through a cluster: neighbours part with an even gap and
close behind it. Drag toward the space's border: the space grows and the next space
slides away. Release, restart: same layout. Arrange > Tidy on a messy space, then By
wires on a parent with three sub-agents. Unit tests: solve is deterministic, restores
on leave, keeps the gap, never moves the pinned cell.

## Phase 4: terminals expand in the canvas

**Goal.** A cell opens in place into a live terminal. The side panel is for files only.

**Decisions.**
- Expand is the solver's third pinned case: the cell grows from pill to terminal size
  (default 100 x 30 cells of the terminal font at zoom 1) and neighbours give way.
  Collapse restores them exactly, because rest positions were never changed.
- Resize from the corner while expanded. The PTY is resized on release, to columns and
  rows computed at zoom 1, never from on-screen size (plan 7.5.3's rule).
- Several cells can be expanded at once. Nothing is covered: that is the solver's job.
- Pop-out stays as it is.
- The side panel loses terminal tabs. Sub-agent transcripts and wire threads are
  files, so they still open there, or expanded in place for a sub-agent cell.

**The hard part, said plainly.** Today one terminal cannot be drawn at its own scale:
font size, cell metrics and padding come from global settings
(`docs/canopy/herdr-map.md`, "Terminal drawing and zoom"). In-canvas terminals at
any zoom other than 1 need `TerminalView` to take a scale. Plan:
1. Add a `scale` to the terminal element that multiplies font size, cell size and
   padding, and divides pointer coordinates. Phase 0 item 4 is the spike.
2. Live text only when the camera is settled. While a zoom gesture is moving, an
   expanded cell shows its header and a flat body, then re-renders. This avoids
   filling the glyph cache with a font size per frame of a pinch.
3. Below 0.5 zoom an expanded cell draws as its pill (plan section 4's lesson).
**Fallback if the spike says no:** expanded terminals are live only at 100% zoom, and
expanding a cell flies the camera there. Less magical, zero renderer risk. Krishna
decides after the spike.

**Ruled out.** Rendering the terminal to a texture and scaling it (blurry text, and
GPUI has no subtree scaling). A second terminal renderer.

**Risks.** This is the highest technical risk in the plan. Scroll routing must be
exact: wheel over a *focused* expanded terminal scrolls its history, anywhere else
pans the canvas.

**Check by hand.** Click a cell: it opens where it is, neighbours move aside, you can
type. Resize it; `stty size` matches. Open a second one beside it. Zoom out and in:
text is sharp once the motion stops. Collapse: everything returns. The side panel
opens only for files.

## Phase 5: keyboard first

**Principle (already in `src/keys.rs:27`): plain Ctrl+letter always belongs to the
terminal.** Agents need Esc, Ctrl+B (Claude backgrounds a task, tmux prefix), Ctrl+G,
Ctrl+O, Ctrl+R, Ctrl+T, Ctrl+J, Shift+Tab. So Canopy never binds a bare Ctrl+letter
and never takes Esc outside its own mode. macOS uses Cmd (terminals never see it).
Linux uses Ctrl+Shift, which terminal programs cannot tell apart from Ctrl without
the kitty protocol and so do not rely on.

**Canvas mode.** Press the prefix, release: a hint bar appears along the bottom
listing the keys, and keys stop reaching the terminal until Esc or Enter. Inside,
single letters are safe and easy to remember:

| Key in canvas mode | Does |
|---|---|
| arrows (or h j k l) | move focus to the nearest cell in that direction |
| Tab / Shift+Tab | next / previous space |
| Enter | expand the focused cell and type in it (leaves the mode) |
| Esc | leave the mode, focus stays |
| x | collapse the focused cell |
| f | focus: fit the current space; again: back to the previous view |
| c | connect: arrows pick the other cell, Enter draws the wire, Esc cancels |
| t | open the focused wire's thread |
| a | arrange (last layout); A opens the layout picker |
| n | new terminal here; N opens the agent list |
| Shift+arrows | nudge the focused cell by one gap |
| Backspace / Delete | close the cell (confirm rules from phase 1.3) |
| ? | full key list |

**Direct keys** (no prefix), for the things done all day:

| Action | macOS | Linux |
|---|---|---|
| canvas mode (prefix) | Cmd+J | Ctrl+Shift+J |
| focus / zoom toggle | Cmd+Shift+F | Ctrl+Shift+F |
| expand / collapse | Cmd+Enter | Ctrl+Shift+Enter |
| new terminal | Cmd+T | Ctrl+Shift+T |
| new space | Cmd+N | Ctrl+Shift+N |
| arrange | Cmd+Shift+A | Ctrl+Shift+A |
| file panel | Cmd+B | Ctrl+Shift+B |
| zoom in / out / 100% | Cmd+= / Cmd+- / Cmd+0 | Ctrl+= / Ctrl+- / Ctrl+0 |
| views | Cmd+Shift+1/2/3 | Ctrl+Shift+1/2/3 |

**Focus fit** accounts for the file panel: the fit viewport is the canvas area that is
actually visible, so the space is centred in what you can see. The previous camera is
remembered once, so the second press returns. The same toggle is the focus control at
the top right, beside the file icon.

**Implementation notes.** `keys.json` has no sequences today; do not add a general
chord system. Canvas mode is one flag plus a `canvas-mode` section in `keys.json`
(`"layout:main"` exists already as a section [V]); while the flag is set, that section
is consulted first and unbound keys are swallowed. Cmd+J and friends go in
`layout:main`. Changing shipped keys means adding the current originals to
`src/keys/shipped/` first (AGENTS.md rule), or nobody's file upgrades.

**Ruled out.** Ctrl+B as the prefix (clashes with Claude Code and tmux inside cells).
A timeout on the prefix (modes that vanish under you are the worst part of tmux).
Vim-style always-on normal mode (the terminal must own the keyboard by default).

**Notes.**
- Prefix decided: Cmd+J, Ctrl+Shift+J on Linux.
- Outside canvas mode, Backspace and Delete close a *selected, collapsed* pill (phase
  1.3). They reach the terminal as usual when a cell is expanded and focused.
- Linux Ctrl+= for zoom is taken from the terminal only in `layout:main`, as today.

**Check by hand.** Without touching the mouse: prefix, move to a cell, Enter, type
`ls`, prefix, `c`, pick another cell, Enter (wire drawn), `f` (space fits, with the
file panel open it is centred in the visible part), `f` again (back), `a`. Inside a
Claude cell, Esc, Ctrl+B, Ctrl+O and Shift+Tab still do what Claude says they do.

## Phase 6: settings that earn their place, and kinder config files

Seven new keys in `settings.json`, flat like the existing `canvas_color`:

| Key | Default | Who changes it, and why |
|---|---|---|
| `canvas_gap` | `24` | Small screens want 12 to fit more; people who present want 40. |
| `canvas_push` | `"live"` | `"drop"`: neighbours move only on release. For people who find live motion distracting, and for slow or remote displays. |
| `space_fill` | `0.04` | `0` gives outline-only spaces; a few people want 0.08 back for contrast on bright screens. |
| `terminal_size` | `[100, 30]` | Columns and rows of a newly expanded cell. Big monitors want 140 x 45; laptop users 80 x 24. |
| `wire_budget` | `8` | Agent postcards per wire before it trips. People running a long pair (writer and reviewer) raise it; `0` turns the breaker off. |
| `wire_limit` | `1000` | Postcard length in characters. File only, never in the Settings screen. Someone with terse models may lower it; raise it only knowingly, it is the guard against context dumping. |
| `confirm_close_terminal` | `true` | Written as `false` by the "Don't ask again" checkbox. Edit back to `true` to get the dialog again. |

Deliberately not settings: edge color, easing time, corner "+" size,
the arrange layouts. Nobody would choose differently for a reason.

**What to copy from herdr about hand-editing.** What I could verify: herdr-core reads
`config.toml` at each use, and on an invalid file falls back to defaults and logs the
error once to `core.log` [V `hc --skill`]. That is the wrong half to copy: a log is
where AGENTS.md says user-caused errors must not end up. The herdr TUI has a
`herdr config` subcommand and `server reload-config` [V help]; I did not run them
(daily app). Canopy already reloads live and already turns a bad key binding into a
toast [V tests in `src/keys.rs`]. Proposal, three small things:

1. **Errors name the place.** A parse error in `settings.json`, `keys.json` or
   `programs.json` raises a toast with file, line and column, and the app keeps the
   last good values instead of defaults, so a typo never resets your font mid-edit.
   [?] check what `settings.rs` does today before changing it.
2. **Unknown keys are reported, not ignored**: "settings.json: unknown key
   `canvas_gab`, did you mean `canvas_gap`?"
3. **Defaults you can read.** `canopy config` prints every setting with its default
   and one line of explanation, generated from the same table that the
   `defaults/AGENTS.md` test already checks, so it cannot go stale. JSON has no
   comments, so this replaces a commented sample file.

**Ruled out.** A settings UI for these (the Settings mod can grow later). TOML for our
files (a second format). A schema file to maintain by hand.

**Check by hand.** Edit `settings.json` while Canopy runs: set `canvas_gap` to 60 and
the cells spread live; write `canvas_gab` and a toast names it; break the JSON and a
toast gives the line while the app keeps working. `canopy config` lists the new keys.

## Order, size and parallel work

| Phase | Depends on | Rough size | Can run beside |
|---|---|---|---|
| 0 verify (rest) | nothing | half a sitting | everything |
| 1 sub-agent cells, small UI | 0 (item 3) | one sitting | 3 |
| 2 Wire | 1, 0 (item 1, Codex only) | two sittings | 3 |
| 3 spacing, arrange | nothing | one to two sittings | 1, 2 |
| 4 in-canvas terminals | 3, 0 (item 4) | two sittings | 2 |
| 5 keyboard | 2, 3, 4 | one sitting | 6 |
| 6 settings, config | 3 (gap, push), 2 (budget) | one sitting | 5 |

Phases 2 and 3 touch different files (`canopy wire` CLI, `programs.json`, edge
drawing versus the solver in `src/canvas.rs`), so they can be two worktrees. Each
phase ends with `just test`, `ponytail-review`, `ponytail-debt` and the hand check,
per `plan.md` section 11.

**Docs, every phase.** Canopy is open source, so each phase leaves docs behind, but
only for what it touched: one short page per feature in `docs/` (for example
`docs/wire.md`, `docs/canvas.md`, `docs/keys.md`), in the order what it is, how to
use it, how it works; plus a doc comment on each new public function or type. Short
and exact: no filler, no restating the code, no changelog prose. The implementer
writes them in the same change. The orchestrator reads them against the code, and
rewrites anything wrong, padded or vague before the phase is called done.

## What changes for released data and APIs

- `canvas.json`: new `wires` list and `sub` cards, both defaulted, so old files load.
- `programs.json`: the top-level `note` text changes; entries gain hook flags. A
  person's own `programs.json` overrides still win.
- Commands: `canvas.sub_message` is replaced by Wire; keep the name as an alias for
  one release. `view.*` commands stay. New: `canvas.arrange`, `canvas.expand`,
  `canvas.connect`, `canvas.mode`, `wire.*`.
- Shipped keys change, so `src/keys/shipped/` gains today's originals.
- `defaults/AGENTS.md` and `src/SKILL.md` are updated in phases 1, 2, 5 and 6.

## Not verified in this plan

- The app was not launched: the sub-agent pills have not been seen live. The cause is
  established from source plus live hook payloads.
- Codex: whether hook context reaches the model, how it treats a typed line mid-turn,
  whether hook trust persists, its allow rule for one command.
- Claude: a typed line while a permission dialog is up.
- The terminal scale change (phase 4) is a design, not a spike result.
- The canvas-tool survey came from web search; several official pages could not be
  fetched, so "nobody pushes live" is likely but not proven.
- The herdr TUI's own config handling was not inspected beyond its help text.
