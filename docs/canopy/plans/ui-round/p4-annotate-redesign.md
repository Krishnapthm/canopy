# P4 - Annotate redesign (Claude)

Branch `feat/annotate-redesign` (already pushed; open your PR into `integration/ui-round`). Owner: **Claude**. Depends on: -.
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
**References (in `img/`, look at all three before designing).**
- `img/annotate-dom-highlight.png`: Claude's browser annotate. Select is on, with the smooth
  DOM box and its selector label, and the compact icon-only bar (select, pen, box, text, X).
- `img/annotate-tools-expanded.png`: ChatGPT's bar with the pen picked, which expands color and
  stroke, plus the text-only action pill ("Add to chat").
- `img/annotate-composer.png`: the inline comment composer anchored to the picked element.
  It has a blue pin, a dark single-line pill, and a round send button whose menu shows
  "Add ↵" and "Send ⌘↵". Drop its sliders and mic buttons.
Match the behavior and layout, but draw them in Canopy's own design language (theme tokens,
radii and type), not a copy of either app's look.

**Today.**
- `annotate.rs` (1758 lines) works on a frozen still.
- The bar is drawn by `annotation_bar` (:1349-1450): icon+text buttons, a "more" menu, color
  swatches, undo/redo, crop, hide, compare, trash, Send (icon+text), close, and a horizontal
  drag grip (`bar_drag`/`bar_offset`, `annotation_bar_limit` :1300).
- The comment editor is `annotation_editor` (:1243), a `gpui_base::Input` in a 238×78 box.
- Separately, react-grab (`vendor/react-grab`, injected by `browser.rs:31-35`, initialized at
  `browser.js:34` without `toolbar:{enabled:false}`) leaves its own floating cursor toolbar in
  the page. `canvas.browser_pick` → `browser_picked` (browser.rs:999, :1027) is a second,
  separate pick-to-quote feature. Its smooth hover box is react-grab's `selectionBox`.

**Target.** One Annotate feature on browser and simulator panes.
- The bar is a fixed bottom-center pill (not draggable) with icon-only tools: Select, Pen, Box,
  Text, and X to stop.
- Color and stroke sub-controls expand only while Pen, Box or Text is active (the ChatGPT
  screenshot).
- Actions are text-only pills: "Send" when something is queued. No icon+text buttons anywhere.
- **Select on the browser:** the hover shows react-grab's smooth DOM box, with its toolbar
  disabled.
  - Recommended path: keep the page live while Select is active and let react-grab's
    `onElementSelect` give the element (already wired, browser.js:36).
  - Freeze to a still only when a drawing tool is picked, or at send time for the screenshot.
  - Throttled `__canopy.at` hover on the still is the fallback, if live picking fights with
    the freeze.
- On select, a blue pin drops on the element and an inline composer opens anchored to it (the
  composer screenshot):
  - a dark single-line pill input that grows with Shift+Enter, and a round send button;
  - hovering or long-pressing send shows a small menu: "Add ↵" queues the comment as a
    numbered pin, "Send ⌘↵" sends everything queued;
  - Enter = Add, Cmd+Enter (Ctrl+Enter on Linux) = Send;
  - no sliders, no mic. The composer uses theme tokens: `popover`, `foreground`,
    `muted-foreground`, `ring`.
- On the simulator, Select drops the pin at a point, with the same composer.
- **Target agent.**
  - Exactly one wired agent: Send goes there, with no menu.
  - Otherwise, an agent picker sits in the browser chrome next to the Annotate button (and in
    the sim's helper bar). It shows when annotating starts and lists wired agents first, then
    other installed and running agents (`quote_menu` ordering, canvas_view.rs:2390-2397).
  - Never inside the page.
- Remove react-grab's toolbar (`toolbar:{enabled:false}`, and clear
  `react-grab-toolbar-state`).
- Remove `canvas.browser_pick` as a separate user feature. Grep `docs/`, `defaults/`,
  `src/SKILL.md` and the CLI for users. If agents use pick through the CLI, keep the
  command and route it into the Annotate Select tool.
- Cut controls that aren't in the target, and list each one in the report: crop, hide,
  compare/adjust form, the "more" menu, the grip. Undo/redo stay as keys (Cmd+Z /
  Shift+Cmd+Z) without buttons. Krishna asked for exactly the screenshot set, so ask him before
  keeping any other.
- **Icons "failing".** Reproduce first:
  - Icons are served from `~/.config/canopy/icons`, written by `programs::install_icons`
    (programs.rs:357).
  - Check whether install skips existing or stale files, so a new icon is missing in an
    existing home.
  - Check whether a GPUI element over a live page lacks a `cover:` bound (browser_mask.rs:176).
  - Fix the root cause.
- Keep `message()`, `burn()` and the delivery path (`send_annotation` :1493,
  `route_annotation` :1584) as they are, and keep their tests passing.

**Docs.** `docs/wiki/Browser-Pane.md` (Annotate, remove the picker section) and
`Simulator-Devices.md`.
