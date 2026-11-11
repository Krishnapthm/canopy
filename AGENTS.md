# AGENTS.md

Canopy runs coding agents from different vendors as one live tree: a Rust daemon, a TUI, and
a web app that share one protocol. It is for developers with two or more agent subscriptions. Read [PLAN.md](PLAN.md) for the design until it is deleted
by the user; after that the code and `docs/decisions/` are the truth.

This file is loaded into every agent session. It is capped at 150 lines and checked by
`make check-docs`. Keep it that way.

## Priorities, in order

1. Correctness of agent control (a message, approval, or stop must do exactly what it says).
2. Performance budgets in `crates/canopy/tests/bench.rs`. A change that breaks one is a bug.
3. Simplicity. The smallest design that fully works. No speculative options or layers.

## Commands

- `make check` - format, clippy with warnings denied, `tsc --noEmit`
- `make test` - unit and fixture tests
- `make e2e` - real daemon in a temp home against fake agents
- `make bench` - load test and budget assertions
- `make gen` - regenerate `web/src/protocol.gen.ts` from the Rust types
- `make web` - build and check the web app bundle
- `make build` - release binary with the web app embedded
- `make check-docs` - lint every AGENTS.md and the ADR index

Run `make check test` before saying a change is done. Run `make e2e` when touching the
daemon, an adapter, or the protocol. Full `make build` only when the task needs it.

## Hard rules

- Never leave a daemon, dev server, or agent process running. Tests and manual checks use
  `canopy daemon --foreground` with a temp `CANOPY_HOME` and stop it before continuing.
- Never run a live harness (`CANOPY_LIVE=1`) without being asked: it spends real tokens.
- Never touch the user's real `~/.canopy` or their harness configs (`~/.claude`, `~/.codex`,
  `~/.gemini`) from tests or scripts.
- Canopy is local only. No telemetry, no network requests of our own, no server mode.
- Never read, store, or pass through harness credentials, and never call a vendor API. We
  only drive the official CLIs the user is already logged into.
- Never edit generated files: `web/src/protocol.gen.ts`, `docs/decisions/README.md`.
- No em dashes anywhere. Use a plain dash.
- Commits: `type(scope): description`, lowercase, imperative, no agent attribution.

## Architecture invariants

- All daemon state is owned by the hub task in `crates/canopy/src/daemon/hub.rs`. Nothing
  else mutates it. No `Mutex` around session state (ADR 0003).
- The daemon does no work when idle: no polling, no periodic timers, no file watchers. A
  timer may exist only while output is pending (ADR 0003).
- Types flow one way: `crates/protocol` is the source of truth, TypeScript is generated
  (ADR 0015). A protocol change means: edit Rust, `make gen`, fix what `tsc` reports.
- Token deltas are coalesced in the adapter before they reach the hub (ADR 0023). Never
  forward per-token events.
- Clients receive transcript events only for sessions they subscribed to (ADR 0007).
- Only completed items are written to SQLite (ADR 0008).
- Adapters deserialize tolerantly: unknown fields and variants are counted, never fatal.
- Harness behaviour is never coded from memory. Verify against the installed CLI, write the
  fact and the CLI version into `docs/harness-notes/`, and record a fixture.
- Agent output is untrusted. The web app never uses `innerHTML`.
- Every error a user causes or can see reaches them as a toast, never only in daemon.log.
  Web: sonner via `web/src/toast.ts` (ADR 0025).
- A sub-agent inherits its parent's permission policy and cannot escalate it.
- Plugin and skill changes go through the vendor's own command or app-server method, on an
  explicit user action, and nothing is ever auto-accepted (ADR 0020).

## Dependencies

Every dependency is a cost. Adding one, Rust or npm, requires an ADR that says what it
replaces, its size, its license (MIT, Apache-2.0, BSD, ISC, or public domain only; the
project is MIT), and why the standard library or twenty lines of our own code will not do.
The approved list is in ADR 0001 (Rust) and ADR 0022 plus 0025 (web).

## Conventions

- Rust: errors with `anyhow` in the binary, typed errors in `crates/protocol`. No `unwrap`
  outside tests. Comments explain why, above the item, briefly.
- Web (M5, ADR 0022): React with useSyncExternalStore and shadcn/ui; no router or state
  library; use the generated neutral theme in web/src/index.css.
- Every user action is a command in a registry with the same key in the TUI and the web
  (PLAN.md section 6.1). The registries land with M4 and M5.
- UI: icons are required, never an emoji, no eyebrow labels; global scope is an earth globe
  (PLAN.md section 6.1).
- UIs show the last 8 characters of a session id; the daemon resolves a unique suffix (ADR 0018).
- Tests are focused: one behaviour each, through the real daemon where possible. No snapshot
  dumps, no tests for deleted features.
- Bug fixes start with an `e2e` reproduction.
- `README.md` is for users who know Claude Code or Codex but not Rust: plain words, short
  sentences, copy-paste commands that work from a fresh clone, no internals. Contributor
  detail goes here, in `docs/`, or in a nested AGENTS.md. Re-run its steps when you change them.

## Where things live

- Wire types: `crates/protocol/src/`
- Harness adapters and their local rules: `crates/canopy/src/harness/`
- Cross-vendor spawn tools: `crates/canopy/src/daemon/mcp.rs`; semantics in ADR 0019
- Scripted test agent: `crates/canopy/src/fake_agent.rs` (ADR 0016)
- Web app and its local rules: `web/`
- Decisions and their reasons: `docs/decisions/`

## Keeping this file true

This file is maintained by the agents who work here. At the end of every task, before the
final commit, ask:

1. Did the user correct me on something a future agent would also get wrong?
2. Did a non-obvious constraint cost me real time?
3. Did I make a decision that constrains future work?
4. Did I find a line here that is false, stale, or unneeded?

If yes, change the docs in the same commit as the code, following these rules:

- **Executable beats prose.** If the lesson can be a type, test, lint, or Makefile target,
  make it one and write nothing here. When a rule here becomes enforced, delete it here.
- **Edit beats append.** Fix or delete the wrong line. Never add a newer line that
  contradicts an older one.
- **Stay under the cap.** At 150 lines, adding means removing or demoting to a nested
  AGENTS.md or an ADR.
- **Every line is checkable.** Name a path, a command, or an ADR. No vague advice.
- **Put it at the right tier.** Applies everywhere: here. Applies to one directory: that
  directory's AGENTS.md (cap 80 lines). Explains why: a new ADR in `docs/decisions/`.
- **ADRs are immutable.** Reversing a decision is a new ADR that marks the old one
  superseded, plus an edit to every line here that cited it.
- **Do not record** task status, changelogs, or anything `grep` or `git log` answers.

Lessons go below, one line each, ending with a `(YYYY-MM)` stamp. `make check-docs` warns
when a stamp is older than 90 days: re-verify it, then refresh the stamp, promote it to a
check, or delete it.

## Lessons

- Probe Codex live from the main session; sandboxed sub-agents get 401 (`docs/harness-notes/codex.md`). (2026-11)
- `make build` does not put `canopy` on PATH; build assets with `make web`, then `cargo install --path crates/canopy`. (2026-11)
- `make bench` reads process stats with `ps`; sandboxed agents get "Operation not permitted", so run it from the main session. (2026-11)
