# P5 - Browser and simulator in the Files panel (Claude)

Branch `feat/drawer-panes` (already pushed; open your PR into `integration/ui-round`). Owner: **Claude**. Depends on: P2 `effective_color` (soft).
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
- Panel tabs are `Vec<String>` file paths (`canvas_view.rs:52-63`; `canvas::open_panel_tab`
  canvas.rs:1887). The panel is rendered at :3558-3796.
- A browser's native view is positioned from the bounds `canopy-canvas`/`browser-canvas`
  (`expanded_browser` browser.rs:884). It is clipped to the canvas by `browser_masks`
  (`browser_mask.rs:131+`, which finds the body via `body_rect`), with `cover:*` bounds
  punched out.
- The Browser and Simulator buttons in the top strip open pinned panes (`pinned_new`, :3526-3537;
  `pane_new {"pinned":true}`, pane_commands.rs:74).

**Design.**
1. **Model.**
   - Add `enum PanelTab { File(String), Pane(String /*pane id*/) }`, serialized as today's
     string for files, plus `"pane:<id>"` for panes. File paths are absolute, so no clash.
     It replaces `Vec<String>` in state.
   - A pane is docked exactly when a `PanelTab::Pane` refers to it. This is the single source
     of truth, so no extra flag on `Pane`.
   - Docked panes keep their saved `rect` for when they move back to the canvas. They are
     skipped by canvas layout, `solve`, drawing and wire geometry.
2. **Opening.**
   - The Browser/Simulator top-strip buttons, `pane_new {kind: browser|sim}` without
     `place`, and `canopy pane new browser|sim` all dock by default.
   - Add `place: "panel" | "canvas" | "pinned"` to `pane_new`, parsed into an enum at the
     boundary. `pinned:true` is still accepted.
   - The helper bar gets "Move to canvas" when docked, and "Open in panel" on the canvas.
     Pin/Unpin stays.
3. **Rendering.**
   - The panel body for a pane tab renders the same body element `pane_card` uses for that
     kind (browser page or sim screen, including annotate), with the pane's helper actions
     as a header row of the shared helper-bar component (no hand-built buttons).
   - Record its bounds as `panel-body:<id>`.
   - `body_rect`/`browser_masks` use it for docked browser pages, clipped to the panel and
     not the canvas.
   - The sim's `show_sims` treats a docked, visible tab as shown, and stops the helper when
     the tab is hidden.
4. **Tabs.** A tab shows the kind icon, the title (the page title or device name) and, when
   wired, a 6 px dot in `tint-<effective_color(wired agent)>`. That uses P2's function;
   until P2 lands, the dot uses `foreground`.
5. **Wiring.** Connect from a docked pane uses the existing Connect flow. A docked pane has
   no canvas position, so no edge is drawn; the tab dot and a "Wired to <agent>" label in the
   panel header show the link. Annotate Send goes to the wired agent (P4).
6. **Closing.**
   - Closing a pane tab closes the pane (sim: `forget_sims`; browser: close its pages).
   - "Move to canvas" removes the tab and puts the pane back at its rect, or at the
     viewport center when it never had one.

**Tests.** `PanelTab` serde round-trip (old docs load); dock/undock keeps the rect; docked
panes are excluded from `solve` and `wire_edges`; `pane_new` place parsing.

**Docs.** `docs/wiki/Files-Panel.md`, `Browser-Pane.md`, `Simulator-Devices.md`, `Panes.md`.
`defaults/AGENTS.md`/`src/SKILL.md` for `--place`.
