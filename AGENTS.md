@~/.agents/AGENTS.md

# Canopy

Canopy is a native Rust/GPUI control room for agents and terminals, based on
herdr-app. Read docs/canopy/herdr-map.md before work.
Read AGENTS.local.md too when present.

## Rules

- Never touch the daily herdr app, core session, config or user data.
- Never stop, signal or restart processes you did not start.
- Launch only with an isolated app home and core session/socket. The
  orchestrator owns app launches after review, except the dev window (see
  Build and checks). Never run a bare app binary.
- Never leave an app or core you started running after a step, except the dev
  window.
- No telemetry, update checks, feedback uploads or app-owned network requests.
- No push, repo creation, publishing or release without Krishna's instruction.
- No em dashes. Commit as type(scope): description, lowercase, imperative,
  without agent attribution.
- Never run paid agent sessions without asking. While developing, never edit
  real harness config (the app's own setup, below, is tested only in an
  isolated home).
- Verify harness behavior against the installed CLI, not memory.
- Use Ponytail full. Review each phase with ponytail-review, fix findings, and
  report ponytail-debt. Use ponytail-audit after phase 2.
- Agent features stay agent agnostic. Reuse programs.json/detect manifests.
  Vendor-specific code belongs in a small capability function only when data
  cannot express it. Use CLI files, editor integration, flags and hooks, never
  vendor APIs or credentials.
- Preserve released data and APIs unless the task explicitly changes them.
- Keep defaults/AGENTS.md and src/skill/ (SKILL.md and its references) current with agent-facing behavior.
  The generated .d.ts files describe the full mod API.
- Do not edit CHANGELOG.md. Preserve LICENSE and THIRD-PARTY-NOTICES.
- Docs travel with the change. Whoever touches a feature updates its page in
  docs/wiki/ (or adds one, and lists it in docs/wiki/_Sidebar.md) and the doc
  comments in the same change: what it is, how to use it, how it works, in
  that order. Follow docs/canopy/doc-style.md and put each fact on the page
  docs/canopy/doc-owners.md names. Write for a reader new to Canopy: why
  before how, real examples, exact facts. docs/wiki/ is published to the
  GitHub Wiki on merge. The orchestrator reviews docs for accuracy and
  clarity before a phase is done and fixes what falls short. Document what you
  touch, not the whole app.

## Principles

- Priorities: correctness of agent control (a message, approval or stop does
  exactly what it says), then smoothness, then the smallest design that works.
- Idle costs nothing: no polling, periodic timers or token spend while nothing
  happens. A timer may exist only while work is pending.
- Mouse and trackpad behave the same on every OS: wheel/two-finger pans,
  pinch and Cmd/Ctrl+wheel zoom, over empty canvas and over any pane that is
  not entered. Platform code may differ only in the modifier key (Cmd on macOS,
  Ctrl elsewhere).
- Agent output is untrusted. Render it as text, never as markup or commands.
- Every error a user causes or can see reaches them as a toast
  (defaults/mods/toast), never only in a log. See Toasts below.
- UIs list only installed agent CLIs and never name a missing one.
- Write each verified harness fact, with the CLI version, into
  docs/canopy/harness-notes/.
- Every dependency is a cost: say what it replaces, its size and its license
  (MIT, Apache-2.0, BSD, ISC or public domain) before adding it.

## Desktop integration

- Share commands, state, labels, settings, toasts and GPUI components across
  platforms. Keep native menus, window decorations, shortcuts and launcher
  identity at the platform boundary. Use GPUI and system capabilities first.
- macOS and Linux are the primary desktops. Verify native UI changes on both.
  Use Ubuntu GNOME Wayland as the Linux reference and check X11 when changing
  window or launcher behavior. Keep shared code usable by Windows so it can
  add its own desktop integration later.

## How agents learn Canopy

Skills are not fetched by running a command: a harness discovers an
installed skill by its name and description and loads it into context when
the task fits. So the installed skill is how agents learn Canopy.
`canopy --skill` only prints the same text, as a fallback for harnesses
without skill support.

- **Our skill, automatic.** Installing or updating Canopy's skill (with
  `name` and `description` frontmatter that says when to use it) in each
  installed harness's global skills folder happens without asking. Canopy
  writes only its own skill folder, never other entries, and removes it on
  uninstall. Verify each harness's skills path against the installed CLI.
- **Project instruction files, only with explicit permission.** Canopy asks
  before appending a short marked block to the project's `AGENTS.md` or
  `CLAUDE.md`. The block says the project runs in Canopy and names the
  `canopy` skill and when to use it (panes, wiring, quotes, peers); it does
  not tell the agent to run a command to read instructions. A small
  script picks the file: if `CLAUDE.md` imports `AGENTS.md` (an `@AGENTS.md`
  line), append to `AGENTS.md` only; if only one exists, use it; if neither,
  offer to create `AGENTS.md`. The block has begin and end markers so
  updates replace it and removal is clean. Never append twice.
- **Global instruction files, never.** Canopy does not write the person's
  global `AGENTS.md` or `CLAUDE.md`. It offers the same block to copy and
  paste; using it is entirely the person's choice.

## Panes and floating helpers

- Canvas geometry lives in src/canophy. Render reads Layout. New pane kinds
  declare Traits. Read docs/canopy/canophy.md before touching zoom, resize or
  dock.

- Everything on the canvas is a **pane**: terminal, agent, browser, PDF,
  image, note, or a community pane from a mod. Say "pane" in UI, CLI, APIs
  and docs.
- Every pane and every canvas surface gets a **floating helper bar**: a
  small pill of icon buttons for its common actions (a browser pane: home,
  reload, screenshot, record, annotate). Each button has a tooltip with its
  name and shortcut. The bar shows on hover or selection, never covers the
  content being worked on, and is drawn by the one shared helper-bar
  component, never hand-built per feature.
- Mods and community panes declare their actions through the pane API; the
  app draws the bar. A feature without its helpers is not done.

## Types

Lauren Tan's type-system discipline, applied only to code a change touches.
No sweeping rewrites.

- Make illegal states unrepresentable: one enum, not two booleans that must
  stay in sync.
- Ids and other semantic strings that cross files get their own newtype.
- Parse untyped input (JSON, args, socket, hook payloads) into domain types
  once, at the boundary. A bad input becomes a toast.
- Match domain enums exhaustively, with no `_ =>` catch-all.
- Derive types from the authoritative schema; never hand-copy one.
- Prefer a typed, shared solution over a one-off fix when behavior repeats
  across panes, kinds or places: declare it once (a Traits field, an enum,
  one function) and let every caller read it.
- Don't force it. Keep boring code when the shape is local and won't grow.

## Toasts

Toast:

- every error the user caused or can see;
- something that keeps happening out of sight (a page playing in the
  background, an agent needing you or finishing while you are elsewhere);
- the result of an action that lands outside where the user is looking (a
  screenshot saved, a link copied, a file written elsewhere).

Don't toast what is already visible in place, the routine success of a
visible action, progress, or anything that repeats. Every toast that refers
to something gets a jump action. One toast per event.

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
CI tests Linux on PRs; pushes to main add macOS and Windows. Test OS behavior
on its actual OS.
just install installs the app and affects the user's installation, so leave
that to the orchestrator.

scripts/dev opens the isolated development window; close it before
scripts/dev-stop stops only its reserved core. Do not run either during
source-only work (docs, tests, refactors with nothing visible to check).

When a change is ready for Krishna to see working (anything visible in the
app), start the dev window yourself; do not ask them to. Run
`cargo build --locked` first (scripts/dev needs the debug app), then
`scripts/dev` in the background, and say what to look at. The window is theirs
to close: never stop or restart it, and if a scratch core is still reserved,
ask before running scripts/dev-stop.

Once the change works and the checks pass, ask Krishna what to do next: build
locally (`just build`), push the branch, open a PR, or keep iterating. Do none
of them before they answer. `just install` stays with the orchestrator.

On Linux use scripts/sandbox up, do <command> [json], status, shot,
move|click|drag, type, key, hc, restart and down for isolated UI checks.
It uses a private Weston/Xwayland display, home and core under
/var/tmp/herdr-app-sandbox. Never fake input on a real display.
Run scripts/sandbox down when finished.

<!-- canopy:begin -->
## Canopy

This project is open in Canopy, a canvas of terminals, agents and panes (files, notes). When the person mentions panes, the canvas, wiring, quotes or peers, use the `canopy` skill.
<!-- canopy:end -->
