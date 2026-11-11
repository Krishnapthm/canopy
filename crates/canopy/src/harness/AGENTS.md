# Adapter rules

Applies to every file in this directory. Wire facts live in `docs/harness-notes/`, each with
the CLI version it was verified on. Code only what a note or a fixture shows.

## Every adapter

- Report through `Out` in `mod.rs`. Text goes through `Stream` or `Out::delta`, never one
  event per token (ADR 0006). `Out::due` is the only timer and exists only while text is buffered.
- Emit `Ev::Started` once, `Ev::Exited` exactly once, and an `Ev::Status` for every turn
  start and end. The hub derives notices, `wait_agent`, and rollups from these (ADR 0019).
- Answer every `Command::Send` with the `Delivery` that actually happened.
- Unknown lines, types, and fields are counted or ignored, never an error.
- Policy is enforced here: `Yolo` allows all, `AutoEdit` allows edits, `Ask` surfaces every
  request as a `Permission` item. A decision applies once.

## Claude (`claude.rs`, notes: `docs/harness-notes/claude.md`)

- Launch needs `--verbose` and `--permission-prompt-tool stdio`, or no `can_use_tool`
  requests arrive and the CLI decides permissions itself.
- Every user line carries `client_composed: true`; without it a message starting with a slash word runs as a
  slash command instead of reaching the model.
- A mid-turn message becomes its own turn with its own `result`: count pending turns.
- `terminal_reason: "aborted_streaming"` is an interrupt, not an error.
- `Agent` tool sub-agents are native children keyed by `parent_tool_use_id`; their stream
  events are not forwarded, only complete `assistant` lines.
- New behaviour starts with a fixture `fixtures/claude-<version>-<scenario>.jsonl` and a
  replay test in `claude.rs`. Pipe recordings through `scripts/record-fixture.py FILE`
  from the repository root; it scrubs paths, identity and installed extension lists.

## Codex (`codex.rs`, notes: `docs/harness-notes/codex.md`)

- One `codex app-server` for all sessions; messages route by `threadId`. Never kill its
  process group for one session.
- Messages omit `jsonrpc`. `initialize` is required before any thread call.
- Turn, steer, and approval shapes come from the 0.165.0 schema only. Record a live fixture
  before changing them.

## ACP (`acp.rs`, notes: `docs/harness-notes/acp.md`)

- Verified only against `canopy fake-agent`. A real ACP agent is added with
  `canopy daemon --acp name=command`, and its notes and fixture come first.
