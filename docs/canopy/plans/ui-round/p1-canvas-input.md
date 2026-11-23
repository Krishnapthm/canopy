# P1 - Canvas input: wheel/pinch over panes, shrink resize, edge autoscroll (Codex)

Branch `feat/canvas-input` (already pushed; open your PR into `integration/ui-round`). Owner: Codex (high effort). Depends on: replaces PR #34 phase 1.
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
**Goal.** The mouse and trackpad behave the same on macOS and Linux:
- Pinch, ctrl/cmd+wheel and plain two-finger scroll over any pane that isn't entered act on
  the canvas, the same as on empty canvas.
- An entered terminal keeps its scroll.
- Panes can shrink as well as grow.
- Dragging a pane near the canvas edge pans the canvas.

**Root causes (verified by reading the code).**
- Every pane `.occlude()`s (`canvas_view.rs:3431, 3212, 2593, 3174`), which sets
  BlockMouse. gpui's `hit_test` stops there (`vendor/gpui-pre/src/window.rs:1128`), so the
  canvas div's bubble `.on_scroll_wheel` (:2782), `.on_pinch` (:2783-2805) and `.on_mouse_move`
  (:2851) never fire over a pane.
- An inactive terminal returns without `stop_propagation` (`src/terminal.rs:1051-1056`,
  `wheel_needs_focus` set at canvas_view.rs:3336), so its wheel event is free to take.
- Resize is applied only in the bubble `on_mouse_move` (:2865-2901). Shrinking moves the
  pointer over the pane being resized, which occludes it, so the events never arrive. The
  window capture listener at :2729-2744 handles moves but skips resizes (`move_pointer`
  :295-296 returns None for them).
- Linux never sends `TouchPhase::Ended`:
  - Wayland ignores `AxisStop` (`vendor/gpui-pre-linux/src/linux/wayland/client.rs:2525-2551`).
  - X11's `make_scroll_wheel_event` (x11/client.rs:2590-2609) always sends phase `Moved`.
  - So `settle_zoom` in `canvas_wheel` (:1662-1687, :1681) never runs, and a zoom past the
    limits stays rubber-banded.

**Steps.**
1. **Wheel.**
   - In the `canvas()` paint closure (:2725-2756), next to the existing Capture branch, add a
     `DispatchPhase::Bubble` branch for `ScrollWheelEvent`, gated on `b.contains(&e.position)`,
     that calls `canvas_wheel`. The first child's window listener runs last in bubble order, so
     an entered terminal (it `stop_propagation`s at terminal.rs:1057) still wins.
   - Remove `.on_scroll_wheel` at :2782 so a wheel is never handled twice.
   - Fix the stale comment at :2745-2747.
2. **Boxed panes** (preview, PDF `pdf.rs:220`, sim `sim_screen.rs:591`) consume the wheel
   themselves. In the Capture branch, also claim the wheel when the topmost pane under the
   pointer (`wire_cell_at(pos, 0.)`, wire_view.rs:111) is not the entered/selected primary
   (`selected.last()` with `!mode`). Add a helper `fn entered(&self) -> Option<&str>` and
   replace the inline `entered` checks at :3325, :3350 and :2482 with it.
3. **Pinch.** Move the :2783 body into `fn canvas_pinch`. Register
   `window.on_mouse_event::<PinchEvent>` in the same paint closure, in the Capture phase,
   gated on bounds. Nothing else handles pinch (grep to confirm). Drop `.on_pinch`.
4. **Resize.**
   - Move :2865-2901 into `fn apply_resize(&mut self, pos, cx)`.
   - In the :2729 capture listener, when `pressed_button == Some(Left)` and
     `cell_resize.is_some() || matches!(preview_drag, Some((_, Some(_), _)))`, call it and
     `stop_propagation`.
   - The floors (`.max(240./120.)`, `resize_from` mins, canvas.rs:2089-2097) stay.
5. **Boxed panes get all edges and corners.** Give boxed panes (`pane_card`, :2631-2638) the
   same 8 edge/corner zones as terminal cells (:3389-3416), reusing their code and the cursor
   styles. Keep a minimum hit size of 8 screen px whatever the zoom (today it is
   `12 * zoom`).
6. **Edge autoscroll while dragging** (moving or resizing).
   - Keep the last pointer position in `State`.
   - In `apply_canvas_move`/`layout_frame` (:410-460, :586): if the pointer is within 32 px of
     the `canopy-canvas` bounds, call `camera.pan` with a speed that ramps up with depth into
     the zone (max ~900 px/s × dt), add the same world delta to the carried rects, and
     re-arm `on_next_frame` only while the drag is live and the pointer is in the zone.
   - Copy the self-ending pattern of `animate_camera` (:1733-1772). No timers while idle.
7. **Linux wheel end.**
   - Wayland: handle `wl_pointer::Event::AxisStop` and emit a `ScrollWheelEvent` with
     `TouchPhase::Ended` and zero delta.
   - X11 has no stop event. In `canvas_wheel`, when not on macOS, arm a one-shot 150 ms timer
     (`WHEEL_GESTURE`) after each wheel that calls `settle_zoom` if no newer wheel came in.
     The timer exists only while a gesture is pending.
   - Pinch on Linux is already delivered (wayland :2677-2720; X11 XI 2.4 :1465-1520). Check
     that the canvas path handles it the same way.
8. **Wire input (absorbs phase 1 of draft PR #34, `docs/canopy/wire-browser-plan.md` on
   `fix/wire-and-browser`; same root cause).** `wire_pick` (area `capture_any_mouse_down`
   :2701), `wire_release` (area `on_mouse_up` :2936) and `wire_pointer` (area `on_mouse_move`
   :2852) never fire over an occluding pane, so a wire can't finish on a pane. Move them to the
   same window-level listeners: mouse down in the Capture phase with Left while `wire.sticky`
   is set calls `wire_pick` then `stop_propagation`; mouse up with Left while a non-sticky
   `wire.drag` is set calls `wire_release`; a move inside bounds calls `wire_pointer`. Move
   these calls, don't duplicate them. Replace the direct-call `sticky_wire` test
   (wire_view.rs:381) with a GPUI test that drives simulated move/down/up over an occluding
   pane, modeled on the click-through test from commit `33ab2fd`. Use the same
   simulated-event style for the wheel and resize tests below.
9. **AGENTS.md rule** (add under Principles): "Mouse and trackpad behave the same on every OS:
   wheel/two-finger pans, pinch and Cmd/Ctrl+wheel zoom, over empty canvas and over any pane
   that is not entered. Platform code may differ only in the modifier key (Cmd on macOS, Ctrl
   elsewhere)."

**Tests.**
- Extend the wheel test at :4214-4217 to cover a wheel over an unentered cell (it pans) and
  over an entered one (it doesn't).
- Add a resize test that shrinks a pane by moving the pointer inward. Add an autoscroll unit
  test on a pure function, `edge_pan(pointer, bounds) -> [f32; 2]`.

**Docs.** `docs/wiki/Keys-and-Mouse.md` and `Canvas.md` (pan, zoom, autoscroll).

**Verify (orchestrator).** On a macOS dev window and the Linux sandbox:
- pinch and scroll over an inactive terminal, a PDF and a sim, then over an entered terminal;
- shrink a terminal and a browser pane from each edge;
- drag a pane into each window edge.
