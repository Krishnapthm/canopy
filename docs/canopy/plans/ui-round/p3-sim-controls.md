# P3 - Simulator: shut down, close, sizes (Codex)

Branch `feat/sim-controls` (already pushed; open your PR into `integration/ui-round`). Owner: Codex. Depends on: P1 for edge resize (soft).
For the whole round (other plans, merge order, names) read `00-orchestrator.md` in this folder.

## Shared rules
- Read `AGENTS.md`, `AGENTS.local.md` and `docs/canopy/herdr-map.md` first. Load the
  ponytail skill (full) and finish with ponytail-review, then fix its findings.
- Your branch already exists and carries these plans. Create your own worktree on it, using the shared
  `CARGO_TARGET_DIR` the orchestrator gives you. Never create a per-worktree `target/`.
  Run `export PATH="$(brew --prefix zig@0.16)/bin:$PATH"` on macOS.
- Checks: `cargo test --locked` (scoped with `-- <module>` while iterating), plus `just test`
  when mods change. Do not run `scripts/dev`, the app, `just install` or a bare binary: the
  orchestrator owns launches. On Linux, UI checks go only through `scripts/sandbox`.
- Types: use one enum instead of booleans that must agree. Parse JSON args once, at the
  boundary. Every user-visible error is a toast. Match exhaustively, with no `_ =>`.
- No em dashes. Commits are `type(scope): description`, in a few logical commits. Push to your
  branch (standing authorization for the canopy repo), open a PR into `integration/ui-round`
  (never main), and merge nothing yourself.
- Docs travel with the change. Update the `docs/wiki/` page named in your plan (what it is,
  how to use it, how it works; follow `docs/canopy/doc-style.md` and `doc-owners.md`). Update
  `defaults/AGENTS.md` and `src/SKILL.md` when agent-facing behavior changes. Never edit
  CHANGELOG.md.
- End with a short report: what changed, how to see it, what you could not verify.

---

## Decisions (from Krishna)
- **Notifications**: a macOS system notification when Canopy isn't the front app. It uses the
  app icon. The body is the agent's OSC 9/777 text, else the terminal title, else a generic
  line. Settings points to macOS's own banner (temporary) vs alert (persistent) style.
  Linux comes later.
- **Color**: each space has a color, and a pane inherits it unless it has its own. A wire takes
  the color of the pane it starts from (born wires: the parent; drawn wires: end `a`). The
  color also shows in the status strip and on the edges. Tints are pale on dark themes and a
  touch darker on light themes.
- **Edges**: drawn under panes. An edge is drawn above panes only while both of its endpoints
  are selected, or the wire itself is selected.
- **Browser wiring**: per pane, not per tab. A docked page's tab shows the wire color dot.
- **Drawer**: browser and simulator open in the Files panel by default. Pinned stays as a
  third place.
- **Annotate**: one feature, combining Claude's and ChatGPT's annotate flows, in Canopy's
  design language:
  - tools are icon-only (select, pen, box, text, X); actions are text-only pills;
  - color and stroke show only once a tool needs them;
  - the bar can't be dragged;
  - an inline single-line comment box, Shift+Enter for a new line, with an Add ↵ /
    Send ⌘↵ menu on the send button;
  - no mic, no sliders;
  - the target agent is chosen in the browser chrome next to Annotate, never inside the
    page; with exactly one wired agent, no picker at all;
  - react-grab's own toolbar is removed.
- **Linux**: built on `feat/linux-desktop-integration`, which must merge first. Window
  controls and the menu move into the top strip, one row.
- **SVG**: a preview picture for the Canopy layout card in Settings > Layouts.

## The task
**Today.**
- `src/sim.rs` has `available`, `devices`, `boot`, `screenshot` and `open_device_hub`. It has
  no shutdown.
- Closing calls `forget_sims` (`canvas_view/sim.rs:127`), which leaves the device booted.
- Close fails in three ways:
  1. A pinned pane sits at y = 16 px, so its helper bar is drawn at about −22 px
     (`helper_bar.rs:151,167`) and clipped by the canvas's `overflow_hidden` (:2692).
  2. Esc in the sim screen calls `window.blur` (`sim_screen.rs:333-335`), so focus never
     comes back to the canvas, and `canvas.close` returns early when the canvas lacks focus
     (canvas_view.rs:973).
  3. Clicking the body only raises the pane, it doesn't select it (`pane_card` :2595-2602).

**Steps.**
1. **Shut down.**
   - Add `sim::shutdown(udid)` running `xcrun simctl shutdown <udid>`.
   - Verify the exit code and message against the installed Xcode for an already-shut-down
     device, and write the facts with the version into
     `docs/canopy/harness-notes/xcode-simctl.md`.
   - Add a `canvas.sim_shutdown {pane}` arm in `sim_command` (:281-306), register it in
     `commands.rs`, and give it a helper-bar `BarAction` "Shut down" (`helper_bar.rs:86-95`).
   - It stops the live helper (`sim_screen` sync), shuts the device down, then shows the
     device picker in the pane (empty udid).
   - Errors go to a toast. Toast the result too, because it happens out of sight.
   - Add "Shut down and close" in the pane's right-click menu.
2. **Close.**
   - Helper bar: when `y - 8 - bar_height` falls above the canvas top, draw the bar inside
     the pane's top edge instead (`helper_bar.rs:151-167`), for every pane kind.
   - Esc in the sim screen focuses the canvas (`self.focus.focus`) rather than blurring the
     window.
   - A mouse-down on any boxed pane body selects it the way a header mouse-down does (:2619).
3. **Sizes.**
   - The helper bar gains "Actual size": the pane body becomes the device's point size ×
     camera zoom, plus margins (device points come from `sim_live` frame size / scale, or from
     the simctl device type; verify). It also gains "Default size" (340×700 / zoom, sim.rs:116).
     The existing zoom ×1.25/×0.8 buttons stay.
   - Edge resizing comes from P1. Do not touch the resize code here.
   - After a resize, snap the pane to the device's aspect ratio on release, so there is no
     letterbox. The device stays fitted by `sim_live::fit` (:400).
4. Update the wiki text "corner handle or the zoom buttons below", which is wrong.

**Docs.** `docs/wiki/Simulator-Devices.md` (Shut down, sizes, closing).
`defaults/AGENTS.md`/`src/SKILL.md` if `canopy pane` gains `shutdown`.
