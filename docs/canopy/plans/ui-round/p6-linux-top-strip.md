# P6 - Linux: one-row top strip with window controls (Codex)

Branch `feat/linux-top-strip` (already pushed; open your PR into `integration/ui-round`). Owner: Codex. Depends on: **linux-desktop-integration merged (hard)**.
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
**Precondition.** `feat/linux-desktop-integration` (`f7dc836`) is merged. It replaces the 32 px
hamburger row with a 36 px header row:
- caption buttons ordered by `cx.button_layout()`, using `defaults/mods/tabs/win-*.svg`;
- a centered title;
- a Main Menu button (F10), in `src/window/titlebar.rs` `linux::frame`.

It is still a separate row above the canvas top strip (`canvas_view.rs:3453-3545`, `BAR=36` at
:3830).

**Steps.**
1. On Linux with the canvas layout, `window_frame` draws no header row. The top strip itself
   hosts:
   - the Main Menu button on the far left (or on the right, if the button layout puts the
     controls on the left);
   - the caption buttons at the outer end of the side `button_layout` names;
   - an inset like macOS `bar_inset` (:3833-3834) on whichever side holds the buttons.
2. Expose the caption buttons and menu button from titlebar.rs as functions returning
   elements, so both the canvas strip and popped-out windows use one implementation.
3. The empty strip area is a window drag region, and double-click toggles maximize, matching
   the existing `window_drag`.
4. Keep server-decoration fallbacks: when the compositor draws decorations, draw no caption
   buttons.
5. Keep the order the same as macOS: Home, spaces, +, then Browser, Simulator, Tile, Files on
   the right before the window controls. (Browser and Simulator show only where available.)

**Verify** with `scripts/sandbox` (Weston) for both left and right button layouts, maximize and
restore, dragging, and F10.

**Docs.** `docs/wiki/Linux-Desktop.md` (added by that branch).
