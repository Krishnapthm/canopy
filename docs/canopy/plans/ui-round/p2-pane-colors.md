# P2 - Pane and space colors, wire colors, edges under panes (Codex)

Branch `feat/pane-colors` (already pushed; open your PR into `integration/ui-round`). Owner: Codex (high effort). Depends on: -.
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
- Canvas space color is `canvas::Space.color: String` (`src/canvas.rs:578-587`), drawn from
  the palette `COLORS` (:1204). It is set by `canvas.space_style` (`window.rs:3063-3069`,
  `commands.rs:108`) through the menu `sub:canvas-colors` (`defaults/mods/menu/main.js:56,60`).
- There is a second, separate sidebar color in `shared("space.colors")` (`spaces/main.js:498-509`),
  used by the Classic layout. Leave it alone and keep it out of scope.
- Panes have no color. The frame is `frame_color` (canvas_view.rs:3208).
- Wires are painted in `wire_layer` (`wire_view.rs:321-370`), with the color at :342. They are
  painted above all panes (canvas_view.rs:3445-3446). The comment at wire_view.rs:319 claims
  otherwise and is stale.
- Theme tokens come from `settings::resolve` (`src/settings.rs:293+`), with hue tokens at
  :320-327 and `is_light`.

**Steps.**
1. **Hue enum.**
   - Add `pub enum Hue { Gray, Red, Orange, Yellow, Green, Teal, Blue, Purple }` to canvas.rs
     with serde lowercase. It replaces the `COLORS` string list.
   - Use a lenient deserializer that maps an unknown string to `Gray`, so old or bad docs
     still load. Saved data stays byte-compatible.
   - `Space.color: Hue`. Add `Pane.color: Option<Hue>`
     (`#[serde(default, skip_serializing_if = "Option::is_none")]`).
2. **Effective color.**
   - Add `Document::effective_color(&self, pane_id) -> Option<Hue>`: the pane's own color,
     else its space's color, else None.
   - Gray counts as a color only on the pane itself; a gray space means "no color". Write
     this in the doc comment.
   - This function is the public interface P5 uses.
3. **Tint tokens.**
   - In `resolve`, add `tint-<hue>` for each hue.
   - Dark themes: mix the hue token about 45% toward white (pale).
   - Light themes: mix it about 12% toward black (slightly darker).
   - Use the existing mix helper. Add a unit test that every tint keeps at least 3:1
     contrast against `canvas` in both canopy-dark and canopy-light.
4. **Pane color UI.**
   - Add command `canvas.pane_color {pane, color|null}` and register it in `commands.rs`.
   - Add a "Color" submenu to the cell menu (`menu/main.js`, case `canvas-card`) that reuses
     the `canvas-colors` submenu pattern plus a "Space color" item that clears the override.
   - Also add `canopy pane color <id> <hue>` only if the pane CLI already maps canvas
     commands generically. Check `docs/wiki/Agent-Pane-Commands.md` and do not add CLI
     plumbing just for this.
5. **Panes.** For a colored pane, the 2 px header accent (or frame, when not selected) uses
   the `tint-<hue>` token. Selection and sync keep their `ring`/`accent` frames. Change only
   the unselected branch of `frame_color`.
6. **Wires.**
   - The color is `effective_color(wire.a)` for born wires, where `a` is the parent
     (`wire.rs:33-44`). For drawn wires, `a` is the end where the drag started. Confirm in
     `Document::connect` (canvas.rs:1032) that `a` is the start, and fix it if not.
   - Use the tint at full strength, and at 0.6 alpha when not strong. With no color, keep
     today's neutral. A glow stays `foreground`.
   - End dots take the wire's color.
7. **Edges under panes.**
   - Give `wire_layer` a filter parameter, `above: bool`, and paint it twice:
     - once right after the background `canvas()` child (:2717-2764), with edges where not
       both ends are selected;
     - once at :3446, with edges where both ends are in `state.selected`, or the wire is the
       selected wire, plus the live drag wire.
   - Fix the stale comment at wire_view.rs:319.
   - macOS browser native views still sit above everything (unchanged).
8. **Status strip** (`defaults/mods/workspace-status/main.js`).
   - Expose `color` (the effective hue name, or null) on each pane entry in the world JSON
     the window builds (`window.rs:1773-1791`), and on spaces if the strip reads them. Find
     how canvas pane ids map to those entries first, then add the field to the generated
     `.d.ts`.
   - The branch label gets a 6 px dot in the active space's tint. Each agent chip's name
     text uses its pane's tint, and its status dot stays the status color.
   - Run `just test` (Bun mod tests).

**Docs.** `docs/wiki/Spaces.md`, `Panes.md`, `Wiring.md` (the edge color rule and edges under
panes), `Canvas.md`.
