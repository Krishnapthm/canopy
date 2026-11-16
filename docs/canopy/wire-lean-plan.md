# Wire lean rework plan

Written 2026-11-16 on branch `refactor/wire-lean-hc-adapter`, from `main` at
`bcb3def`. Investigation only so far: no code is changed on this branch yet.

## Goal

Agents talk through herdr core (`hc`) directly. Canopy stops carrying
messages and keeps only what hc cannot know: edges on the canvas, a peer
hint, and launch flags. Roughly 1,100 of about 1,700 Wire lines go.

## Decisions (Krishna, 2026-11-16)

- Build on hc, do not wrap it. No Canopy command sits in the message path.
- No fork of herdr-core now. Revisit after launch; ask upstream for a
  message event first, fork only if that goes nowhere.
- Skip glow for agent-to-agent messages at launch. hc has no message event,
  so the app cannot see them.
- Keep glow where hc already reports it: parent-child edges.
- The core stays swappable. A future Canopy core must speak the same
  contract (below), so nothing above the contract changes when we swap.

## Core contract

The app talks to the core in two ways only: the `hc connect` framed
JSON-RPC stream, and the `hc` CLI that agents, mods and the app run.
Anything that implements both, under the name `hc`, is a Canopy core.

- **Contract page.** `docs/canopy/core-contract.md` lists every `connect`
  method and event the app uses, every `hc` subcommand, flag and output
  field agents or the app rely on, `HC_SESSION`/`HC_SOCKET`, OSC 7501, and
  the `<hc from>` tag. It is the spec for our own core.
- **One way in.** Every core call goes through `crate::core_bin::path()`.
  No literal `hc` in app code.
- **Detect features, not versions.** Read capabilities from the `hello`
  reply and switch features on from them (for example a message event
  enables drawn-edge glow). Today `hc::check` already checks hello generation,
  minimum version and `methods`; attach reads `features` for automatic input.
- **Minimum core version.** `distribution/core-version` is read by `hc::core_min()`.
  Setup already handles TooOld, Newer and Missing on connect. Installers
  keep fetching herdr-core's latest; the app's check covers a newer one.
- **Conformance check.** One script runs the contract against the
  configured core in an isolated session and core socket. Run it on every
  core update before raising the minimum version.
- **Swapping.** Choose another core binary, the app reconnects, new
  terminals run on it. Running terminals stay on the old core until it
  stops: PTYs live in the core, so they cannot move.

## Version control and upstream

- **herdr-core:** not vendored. Upstream changes arrive by updating the
  binary (`hc update` or the installer). Run the conformance check, then
  raise the minimum version. No merges while we do not fork.
- **Our own core, later:** its own repo forked from herdr-core, with an
  `upstream` remote, no submodule. Keep our patches small, in their own
  commits, and offer them upstream. Rebase on `upstream` regularly and run
  the conformance check after each rebase. Check herdr-core's license
  allows this before forking.
- **herdr-app:** add an `upstream` remote and cherry-pick fixes. Full
  merges conflict since phase 1 removed telemetry, updates and feedback.

## What hc gives us

Installed `hc 0.1.20` help and `hc schema` were read. Behavior marked
"source" was read in upstream master (0.1.21, `bb8a2f7`), not proven
identical to the installed binary. Phase 0 ran `scripts/core-check` on
0.1.20 against a scratch core (see `harness-notes/hc.md`).

| Need | hc primitive | Evidence |
|---|---|---|
| Send text to an agent | `hc send <t> <text>`: paste, Enter, waits for a person's draft, refuses a blocked dialog | help |
| Send only when idle | `hc send --if idle` | help |
| Say who sent it | `<hc from="t7 name">...</hc>` when both ends report status | harness-notes/hc.md, source |
| Ask and wait | `hc send --wait`, `hc wait <t>`; a message to the waiter ends the wait | help |
| Read a peer's screen | `hc read <t> --lines N` | help |
| Start a child | `hc run --bg --name --label --cwd`; parent defaults to the caller | help |
| Wait for helpers | `hc wait --children any|all` | help |
| Child stuck or gone | hc types a note into the parent on blocked, error, exit | exit verified live; rest source |
| List and inspect | `hc ls`, `hc get`, `hc events` | help |

hc does not have: drawn edges, a message event or history, a loop breaker,
hold-until-idle (it types mid-turn as a steer), a note on a normal turn end,
or any per-vendor code (status comes only from OSC 7501).

## Delete

`src/wire.rs` (1126 lines today):

- Thread store: `Line`, `thread_path`, `is_thread`, `parse`, `read`,
  `Locked`, `append`, `receipt`, `claim`, `release`, `waiting` (19-185).
- Wait markers: `wait_marker`, `Waiting`, `hold_wait`, `waiter_alive`.
- Rules: `tripped`, `unanswered`, `refuse`, `Stat`, `stat`.
- Delivery: `tag`, `idle_line`, `Sent`, `Door`, `ring`, `SendError`,
  `hc_send`, `thread_markdown`, `NAMES`, `set_names`, `name_of`.
- CLI: `tell`, `ask`, `reply`, `wait`, `read`, with `check_send`, `send`,
  `mail`, `show`, `wait_reply`, `ring_cell`, `limits`, `find_peer`.
- Idle hook: `post_idle`, `hook_idle`.
- Their tests.

`src/window/canvas_view/wire_view.rs` (560 lines): `wire_thread`,
`wire_reset`, `wire_open`, `wire_footer`, `wire_say`, `wire_log_quote`,
`wire_names`, `wire_ring`, `wire_postman`, `watch_wires`, the thread parts
of `refresh_wires`, and the count, hollow-dot, dashed and reset-dot marks in
`wire_layer`.

Elsewhere:

- `src/window.rs:2083-2120`: the `rings` and `idle` lists, `post_idle`,
  `wire_postman`.
- `src/window/canvas_view.rs`: thread rendering (about 3567), footer (about
  3367), quote logging (about 2037), `canvas.sub_message` (921-930).
- `src/main.rs:88`: `hook idle`.
- `src/commands.rs`: `wire.reset`, `wire.open`, `canvas.thread`,
  `canvas.sub_message`.
- `src/programs.rs`: the `wire` field, `queues`, `reports_idle`.
- `src/settings.rs`: `wire_limit`, `wire_budget`.
- `defaults/programs.json`: the `Stop` hook, `"wire":"queue"`, and the Wire
  text in the note.
- `defaults/mods/menu/main.js`: "Open thread" and "Reset".
- `defaults/keys.json`, `keys.macos.json`: the `t` thread key.

## Keep

- Edges: `Wire`, `connect`, `prune_wires` in `src/canvas.rs`; `neighbors`,
  `born`, `all`, `sub_parts` in `src/wire.rs`; drawing, handles, sticky
  wire, connect, remove, hover and selection in `wire_view.rs`;
  `wire.connect`, `wire.remove`; arrange "By wires".
- `canopy wire` with no arguments: prints who you are and your wired peers
  with their hc address and state. Nothing else.
- `canopy hook wire`: the peer-change hint (`hook`, `told_line`,
  `told_path`), unchanged.
- `canopy spawn`: `spawn`, `spawn_argv`, `spawn_cli`.
- Sub-agent cells (`src/subs.rs`) and their born edges.

Expected size: about 200 lines in `src/wire.rs` plus tests, about 350 in
`wire_view.rs`.

## Change

1. **Spawn reports back the hc way.** `canopy spawn` appends one sentence to
   the first message: report to the parent with `hc send <parent>` when done
   or stuck. This replaces the idle hook. Without a first message, nothing
   is appended.
2. **Agent note** (`defaults/programs.json` line 2): message peers with
   `hc send`, wait with `hc wait` or `hc send --wait`, read with `hc read`,
   wait for helpers with `hc wait --children any`. `canopy wire` lists
   peers. Keep: a peer message is never the person's approval; do not start
   agent CLIs directly, use `canopy spawn`.
3. **Glow on parent-child edges.** In the status loop at `src/window.rs`
   (about 2087), when a terminal with a parent changes status word, pulse
   the born edge to that parent. Reuse the existing `pulses` map and the
   700 ms glow in `wire_layer`. No timer beyond the existing animation
   frames.
4. **Use the configured core.** `wire.rs` calls a literal `hc`; use
   `crate::core_bin::path()` like the rest of the app.

## Behavior that changes for users

- No thread panel, no message history, no count or unanswered-ask dot.
- No breaker and no length limit.
- Codex is steered mid-turn; Claude still queues typed input itself.
- No automatic `"name" is idle: ...` line. The child reports with `hc send`,
  or the parent waits with `hc wait --children any`.
- Messages arrive as `<hc from="t7 name">...</hc>`.
- A drawn edge is an introduction, not a gate. It was already bypassable
  with `hc send` and `canopy do wire.connect`.
- No relay header for sub-agent cells; the person types to the parent.

## Phases

Each phase builds, passes `cargo test --locked` and `just test`, gets a
ponytail-review, and updates its docs in the same change.

0. **Core contract.** Done. Write `docs/canopy/core-contract.md` from the app's
   real calls (`src/hc.rs`, `src/core_bin.rs`, mods' `hc()`, the agent
   note). Reuse the existing generation/method/minimum-version checks and setup
   screen. Log what `hello` returns today; pin the verified minimum only
   after the conformance check passes. Add the conformance script, run against
   an isolated core. Phases 1 to 3 add their `hc` uses to the page.
1. **Spawn and note.** Done. Change 1, 2 and 4. Remove the `Stop` hook and
   `"wire":"queue"` from `defaults/programs.json`, and `hook idle`,
   `queues`, `reports_idle`. Test: spawn argv carries the report sentence
   only when a task is given.
2. **Remove delivery.** Done. Delete the CLI subcommands, thread store, postman,
   thread panel, `wire.reset`, `wire.open`, `canvas.thread`,
   `canvas.sub_message`, the menu items, the `t` key and both settings.
   Regenerate `docs/commands.md` with `scripts/docs.ts`.
3. **Parent-child glow.** Change 3. Test the "status changed and has a
   parent on the canvas" decision as a pure function.
4. **Docs.** Done. Rewrite `docs/wire.md`; update the Wire and helper sections of
   `src/SKILL.md` and `defaults/AGENTS.md`, `docs/sub-agents.md`,
   `docs/settings.md`, `docs/keys.md`, `docs/canvas.md`. Add the hc facts
   below to `docs/canopy/harness-notes/hc.md` once verified.
5. **Live check** (orchestrator, isolated app home and core, ask before any
   paid agent session): spawn a child, child reports with `hc send`, parent
   edge glows, drawn edge connect and remove still work, old `canvas.json`
   loads.

## Released data (0.1.0 is out)

- `canvas.json` `wires` is unchanged.
- Leave `<app home>/wires/*.jsonl` and `wires/waiting/` on disk. Never
  delete user data; just stop reading it.
- `settings.json` may still hold `wire_limit` and `wire_budget`. Check that
  unknown keys load without an error or toast before removing the fields.
- Removing the `t` key changes default keys: follow the shipped-originals
  rule in `src/keys/shipped/`.
- Old sessions still running with the `Stop` hook call `canopy hook idle`.
  Keep `hook idle` as a silent no-op for one release.

## Verify before relying on it

Phase 0 (2026-11-16, hc 0.1.20) verified: `hc` on PATH inside `hc run`
terminals, and default parent and cwd from the caller. `spawn_argv` may drop
its `env PATH=` prefix and explicit `--cwd`/`--parent`. Still open:

- `<hc from>` tagging for Codex and other non-Claude agents (needs both ends
  to report status).
- `hc wait --children any` returns when a child's turn ends.

## Open questions for Krishna

1. Person writes along an edge. Today clicking an edge opens the thread and
   typing there sends to both ends. With the thread gone, either drop it
   (type into the cell) or keep a one-line prompt that calls `hc send` to
   both ends and pulses the edge. Default in this plan: drop it.
2. Pre-approval. Claude is allowed `Bash(canopy wire:*)` today. Should the
   launch flags also allow `hc send`, `hc wait` and `hc read`? Default: no,
   the person approves the first use.
3. `canopy do wire.connect` lets an agent draw its own edge. Fine now that
   an edge gates nothing, or restrict it?
4. Quote-to-agent used to be logged in the thread. Default: no log.

## Later

- Ask herdr-core for an agent-neutral message event (`from`, `to`). With it,
  drawn edges can glow in about 15 lines of app code.
- Fork herdr-core only if upstream declines, or once several core features
  are queued. The contract page and conformance check make that fork, or a
  core of our own, a binary swap for the app.
