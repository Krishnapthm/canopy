@~/.agents/AGENTS.md

# Canopy

Canopy is a native Rust/GPUI control room for agents and terminals, based on
herdr-app. Read docs/canopy/herdr-map.md before work.
Read AGENTS.local.md too when present.

## Rules

- Never touch the daily herdr app, core session, config or user data.
- Never stop, signal or restart processes you did not start.
- Launch only with an isolated app home and core session/socket. The
  orchestrator owns app launches after review. Never run a bare app binary.
- Never leave an app or core you started running after a step.
- No telemetry, update checks, feedback uploads or app-owned network requests.
- No push, repo creation, publishing or release without Krishna's instruction.
- No em dashes. Commit as type(scope): description, lowercase, imperative,
  without agent attribution.
- Never run paid agent sessions without asking. Never edit real harness config.
- Verify harness behavior against the installed CLI, not memory.
- Use Ponytail full. Review each phase with ponytail-review, fix findings, and
  report ponytail-debt. Use ponytail-audit after phase 2.
- Agent features stay agent agnostic. Reuse programs.json/detect manifests.
  Vendor-specific code belongs in a small capability function only when data
  cannot express it. Use CLI files, editor integration, flags and hooks, never
  vendor APIs or credentials.
- Preserve released data and APIs unless the task explicitly changes them.
- Keep defaults/AGENTS.md and src/SKILL.md current with agent-facing behavior.
  The generated .d.ts files describe the full mod API.
- Do not edit CHANGELOG.md. Preserve LICENSE and THIRD-PARTY-NOTICES.
- Docs travel with the change. Whoever touches a feature updates its page in
  docs/ (or adds one) and the doc comments in the same change: what it is,
  how to use it, how it works, in that order. Short and exact, no filler, no
  restating code. The orchestrator reviews docs for accuracy and brevity
  before a phase is done and fixes what falls short. Document what you
  touch, not the whole app.

## Principles

- Priorities: correctness of agent control (a message, approval or stop does
  exactly what it says), then smoothness, then the smallest design that works.
- Idle costs nothing: no polling, periodic timers or token spend while nothing
  happens. A timer may exist only while work is pending.
- Agent output is untrusted. Render it as text, never as markup or commands.
- Every error a user causes or can see reaches them as a toast
  (defaults/mods/toast), never only in a log.
- UIs list only installed agent CLIs and never name a missing one.
- Write each verified harness fact, with the CLI version, into
  docs/canopy/harness-notes/.
- Every dependency is a cost: say what it replaces, its size and its license
  (MIT, Apache-2.0, BSD, ISC or public domain) before adding it.

## Core vs mods

The core (model, layout documents, commands, shared mod APIs) stays small;
everything else is a mod. Before growing the core, check: does the app itself
act on it, can only the app see it, and can mods not do it correctly? If all
three are no, it belongs in a mod. Otherwise ask Krishna first.

## Shipped defaults

defaults/ is built into the app. Shipped layouts and mods are replaced on
start; edited ones move to personal copies. keys.json (keys.macos.json on Mac)
is installed when missing and upgraded only if it matches an earlier shipped
original byte for byte. Keep those originals in src/keys/shipped/ when
changing default keys.

## Build and checks

Disk is tight. Never create a per-worktree `target/`: builds in a worktree
use the one shared `CARGO_TARGET_DIR` the orchestrator gives you. No extra
profiles or flags unless needed. Prune a worktree once its branch merges.

Zig 0.16 is required. On macOS:

```sh
export PATH="$(brew --prefix zig@0.16)/bin:$PATH"
cargo build --locked
cargo test --locked
```

just build builds; just test also runs Bun mod tests; just check aliases test.
CI tests Linux, macOS and Windows. Test OS behavior on its actual OS.
just install installs the app and affects the user's installation, so leave
that to the orchestrator.

scripts/dev opens the isolated development window; close it before
scripts/dev-stop stops only its reserved core. Do not run either during
source-only work.

On Linux use scripts/sandbox up, do <command> [json], status, shot,
move|click|drag, type, key, hc, restart and down for isolated UI checks.
It uses a private Weston/Xwayland display, home and core under
/var/tmp/herdr-app-sandbox. Never fake input on a real display.
Run scripts/sandbox down when finished.
