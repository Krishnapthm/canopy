# canophy: one set of laws for the canvas

canophy (canopy + physics) is the small engine that alone decides where panes are, how big they are, and how they look at any zoom. This document is self-contained: part 1 is the problem, part 2 is why it keeps coming back, part 3 is what others do, part 4 is the design, part 5 is the build plan, part 6 is how we prove it works.

Research inputs: `canopy-behavior-research.html` (25 Nov), a full read of the model and render code, git history since October, and primary sources from tldraw, Excalidraw, Figma, Zed/GPUI, WebRender and Bevy.

---

## Part 1. The problem (what you see)

Canvas behavior keeps regressing. Fixing zoom in one place breaks resizing in another, and a pane kind that worked last week stops working. Since October, 48 commits touch zoom, scale, resize, dock or grid behavior. Several fix the same thing twice or undo each other:

| Commits | What flip-flopped |
|---|---|
| `bfb6223` → `e904b01` (revert) → `e736be1` → `51e1c05` | Sim panes: fixed size on screen, reverted, fixed size again, then "scale like every pane". Four flips in two days. |
| `c42b9e6` → `649c727` | Added `header_world(zoom)`, then removed it. Every pane body went back to subtracting the header by hand. |
| `c6138eb` → `f0b5d97` (same day) | "Scale pane content with zoom" made terminals lose about half their rows at 10% zoom. The fix commit itself says browser CSS height "still follows the drawn body, as the header cannot scale". The same bug is still there. |
| `649c727` → `21422c0` (same day) | Camera constraints and rubber-banding were added, then deleted ("stop zoom bounce"). Three places clamped the camera and fought each other. |
| `77bf992` → `3cca91e` → `43ada16` → `276eba2` | Expanded terminals: scale, then fixed, then live text, then a reflow-vs-keep setting. |
| `fa192c3` → `6485137`, `c7852e4` → `095be36`, `4bc575e` | Edge pan never stopped, the toolbar overlapped panes, live pages measured as empty. Each one re-derives geometry locally. |

## Part 2. Why it keeps happening (root causes)

The research page called this "shotgun surgery". The code shows five concrete causes.

**1. The camera is public, so everyone does their own zoom math.** `Camera { pub x, pub y, pub zoom }` (`src/canvas.rs:46`) only converts points. It has no rect, size or pane-body method. So there are about **90 inline zoom calculations** outside it: about 33 in `canvas_view.rs`, plus `browser.rs`, `wire_view.rs`, `annotate.rs`, `sim.rs`, `helper_bar.rs`, `terminal.rs`, `chrome.rs`, `browser_mask.rs` and others. 13 of them hand-roll `rect * zoom` and 9 hand-roll `/ zoom`. `pane_screen_bounds` (`chrome.rs:275`) and `screen_rect` (`browser_mask.rs:73`) are the same function written twice.

**2. Panes are mixed-space objects. This is the root of the f0b5d97 bug class.** A pane's rect is in world px, but its header is a fixed 36/66 screen px "at every zoom" (`canvas.rs:30`). So the body's real size is `rect*zoom - header`, which changes with zoom. Every content kind has to correct for that, in different units:
- `terminal_body` (`canvas_view.rs:4241`) letterboxes text because the width and height ratios differ.
- Browser CSS height follows the drawn body.
- `image_pane.rs:6`, `default_pane_size`, `sim_pane_size` and the browser `host_frame` fallback all add a screen-px header to a world size. Each is only right at the zoom it was computed at.

**3. No declared behavior per pane kind.** "Does this kind scale?", "what is its minimum size?" and "does it have a header?" are answered inline, differently each time:
- Minimum size is 330×108 in one path, 240×120 in two others, and one of those is in screen px (`pane_commands.rs:153`).
- Default sizes are scattered: 560×420 appears twice, the placeholder pill is 240 while `PILL_WIDTH` is 220, and fit padding is 80 in one place and 48 in another.
- Sim panes are sized `÷ zoom` (`sim.rs:134, 466`); nothing else is.
- The note mod is special-cased by string, `kind == "mod:note"` (`canvas.rs:34`).
- Corner radius scales for terminals but not for other panes.
- Pinned preview/PDF/sim force scale 1, but pinned terminals don't.

**4. Writes come from everywhere, and render writes state.** About 15 sites mutate the camera and about 25 mutate rects, each with its own rules. Render and prepaint also write state:
- `pin_panes` rewrites every pinned rect each frame (`window.rs:5127`).
- `set_scale` notifies the terminal during the parent's render (`canvas_view.rs:3709`).
- The browser defers native zoom from render.
- PDF `set_display_scale` runs in render.
- `grid.fit` resizes the PTY in prepaint.

Several sites also compute the viewport differently. `open_canvas` subtracts the toolbar. `reveal_pane`, `tile_canvas`, `view_center`, `create_canvas`, `pin_pane`, `snap_usable` and cascade use the raw size, with fallbacks of either 1200×800 or the window size.

**5. Nothing checks the rules.** The tests are good examples (`camera_round_trip`, `zoom_keeps_cursor_anchor`, `terminal_grid_body_ignores_the_zoom`, ...). But no test says "this holds after any sequence of actions", so a new call site that breaks a rule compiles and passes.

## Part 3. How serious canvases solve it (short)

- **tldraw**: one camera record and two inverse functions (`screenToPage`, `pageToScreen`). `zoomSteps` are the limits. Each shape declares capabilities on `ShapeUtil` (`canResize`, `isAspectRatioLocked`, `hideResizeHandles`, `onResize`). Resize is a pure `resizeBox(shape, info, {min,max})`. Before-change handlers can fix or block a change that breaks a rule. Screen-fixed chrome (handles 8/zoom, outlines 1/zoom) is drawn in an overlay. Shapes, including iframes, scale as one picture through a single layer transform.
- **Excalidraw**: the same affine camera, `MIN_ZOOM`/`MAX_ZOOM` clamps, and one mutation entry point (`mutateElement`) that bumps a version on every change.
- **Figma** (component instance rebuild, March 2026): generic systems run in one fixed order. "Back-dirties", where a later system invalidates an earlier one, were removed. Old and new runtimes ran side by side on real files before the switch.
- **Zed/GPUI**: render must be a pure read, and effects flush after the update. GPUI has **no subtree transform** (`TransformationMatrix` only applies to sprites). It does have `window.with_rem_size` (`vendor/gpui-pre/src/window.rs:3050`), which re-renders a subtree at a different rem. Zed's `ui::WithRemSize` uses it to scale whole UIs. Laying out at screen size keeps hitboxes correct automatically.
- **WebRender/euclid**: one unit type per space (`LayoutPixel`, `WorldPixel`, `DevicePixel`), so mixing spaces is a type error.
- **Bevy / game engines**: systems run in a declared order, input becomes commands before the step, and time is an input, so runs are reproducible.
- **Canvas terminals (termcanvas)**: draw scaled while a gesture is live, and resize the PTY about 120 ms after it settles. Canopy already does this with `ZOOM_REST` (160 ms).

## Part 4. The design: canophy

### Your decisions, now laws
1. **Zoom is a picture.** Everything inside a pane scales with the canvas: header, text, page, PDF, device. Only overlay chrome is fixed on screen: helper bar, resize grips, selection outline, wires, marquee.
2. **Opening a pane** places it at its default world size. The camera then eases to 100% centered on it (or the largest zoom step that fits, if it is bigger than the view). This applies only to panes the person opens. Panes an agent or core creates (adopted terminals, `canopy spawn` helpers, CLI `pane_new` from an agent) never move the camera; the existing toast with a jump action covers them.
3. **The Files panel shrinks the canvas** (it already is a flex sibling). canophy owns the one viewport, so every rule sees the same open area. When the panel opens or resizes, the left edge stays put. If the entered or selected pane is now covered, canophy reveals it.
4. **Zoom never changes a terminal grid; resize always reflows it.** It reflows once, after the resize or settle. The hidden `canvas_resize_terminals` setting and its keep-and-scale path are deleted.

### The seven parts

**1. The camera is private.** `Camera` moves into canophy with private fields. Outside canophy, no code can read `zoom`, so the compiler lists every one of the ~90 sites, and each gets replaced, not wrapped. Rule: *if render needs a number, canophy gives it a name.*

**2. Typed spaces.**
- World geometry gets small newtypes in `canophy/units.rs`: `WorldPoint`, `WorldSize`, `WorldRect`. Their serde is `[f32;2]`/`[f32;4]`, so `canvas.json` stays byte-identical.
- Screen geometry is GPUI's own `Point/Size/Bounds<Pixels>`, which render already uses.
- The only bridge between the two is the camera.

**3. Traits per kind.** `canophy::traits(&Kind) -> Traits` is an exhaustive match with no `_ =>`, so a new kind won't compile until it declares how it behaves. Fields:
- `default: WorldSize`
- `min: WorldSize`
- `aspect: Option<f32>`
- `header: Option<f32>` (world px)
- `pushable: bool`
- `dockable: bool`
- `content: Content` (`Grid` | `Page` | `Document` | `Device` | `Picture` | `Text` | `Mod`)

Mod panes get their traits from their declaration (see phase 4). That replaces the `"mod:note"` string check.

**4. One way in: `Intent` → `step`.** Input never writes the model. It produces an `Intent`, for example:
- `Pan`, `ZoomAt`, `ZoomStep`, `Fit`, `Reveal`
- `Move`, `Resize`, `Open`, `Adopt`, `Close`
- `Dock`, `Undock`, `Pin`, `Unpin`
- `Arrange`, `Tile`, `SetSize`
- `ViewportChanged`, `GestureEnd`, `Tick(now)`

`canophy::step(doc, intent, &Viewport, now) -> Effects` applies it, then runs the laws in one fixed order:

```
apply → size laws (min, aspect) → push neighbours (canvas_push) → camera laws (zoom range, reveal) → check (debug builds)
```

No law writes something an earlier law read. That is the Figma back-dirty rule, and the order is written in code. Time is passed in, never read inside, so the same intents always give the same document.

**5. One motion state.** A single `Motion` enum replaces the separate `motion.target`, `zoomed_at` and `cell_resize` checks that each decide "rested": `Rested | Gesture { since } | Easing { from, to, start } | Resizing { pane, zone }`. `settled()` is one method.

**6. Layout is derived; render only reads.** `canophy::layout(doc, &Viewport) -> Layout` is computed once per change. For each pane it gives:
- the screen `Bounds`
- the body bounds
- `rem` (the rem size the pane is drawn at)
- the content scale
- whether its grid is frozen

It also lists the overlay placements: grips, helper-bar anchor, selection outline, wire paths in screen space. Hit tests, culling, browser masks and wires all read `Layout`. A `sync` step runs after `step`, not inside render, and pushes `Layout` into child and native views: terminal scale and grid, browser frame and page zoom, PDF scale, sim. Render then writes nothing.

Pinned panes (`Anchor::Screen`) and docked panes (`Anchor::Panel`) are anchors in layout. Their world rect is derived, not rewritten every frame. That deletes `Document::pin`'s per-frame write and the `fit_to_body` fake pane.

**7. Scale through the platform, not through math.** A small `ScaledPane` element, modeled on Zed's `WithRemSize`, wraps each pane subtree in `window.with_rem_size(base_rem × app_zoom × camera_zoom)`. Pane chrome (header, pills, space cards, compact tiles, radius) is sized in `rems()`. The whole pane then scales as one picture, with hitboxes correct for free, and most of the ~90 sites are simply deleted. It also fixes these, with no special cases:
- the 13 px text clipped in 9 px pills
- radius that scales in some panes and not others
- the app-zoom setting being ignored by headers

With the header scaling, every content kind has scale = `zoom`, exactly. `terminal_body`'s letterbox, browser CSS height drift and sim `÷ zoom` all go away.

### Enforcement (why it won't regress)
- **Compiler**: the private camera and the exhaustive `traits()` match.
- **Invariants**: `canophy::check(doc, view) -> Result<(), Violation>`, run by `debug_assert` after every step. On a failure, it panics with the last 256 intents, so the bug can be replayed. The invariants:
  - I1: rects are finite and ≥ `min`.
  - I2: aspect-locked panes keep their aspect ratio.
  - I3: pushable panes in a space don't overlap, unless push is off or a Drop-mode drag is live.
  - I4: zoom stays within `ZOOMS`, and within `zoom_range` once rested.
  - I5: body screen size = body world size × zoom.
  - I6: the terminal grid body doesn't depend on zoom.
  - I7: `to_world(to_screen(p)) ≈ p`.
  - I8: `ZoomAt` keeps its anchor fixed.
  - I9: the entered pane is visible after `Open`, `Reveal` and `ViewportChanged`.
  - I10: a pinned pane's screen rect doesn't move under pan or zoom.
  - I11: docked panes aren't in canvas layout or hit tests.
  - I12: camera-only intents never touch panes.
  - I13: determinism; replaying the same intents gives an equal document.
- **Property test**: `proptest` (dev-dependency) generates random intent sequences (0..200 steps) and checks every invariant after every step. proptest 1.12 is **already in Cargo.lock** through gpui-pre, it is MIT/Apache-2.0, and it adds zero new crates. A 20-line loop replaces `proptest-state-machine`, which is not in the lockfile.
- **Scale-equivalence render test** (Linux headless, which the suite already has): render each pane kind at zoom 1 and 0.5. Every element's bounds at 0.5 must equal its bounds at 1 × 0.5 (±1 px). Any `px()` left in pane chrome fails this test, which makes the "picture" law testable.

### What gets deleted
- ~90 inline zoom sites (goal: zero `zoom` reads outside `src/canophy/`)
- `terminal_body`
- the `keep_in`/`kept` path
- the `canvas_resize_terminals` setting
- `fit_to_body`
- the per-frame `Document::pin`
- `snap_usable`'s own fit math
- sim `÷ zoom` sizing
- `browser_mask`'s copied grip layout
- the `"mod:note"` header string
- duplicate min/default/padding constants

### Ruled out
- **An ECS or a physics crate (Rapier)**: this is rectangles and rules.
- **euclid**: it's in the lockfile, but its serde shape differs from `canvas.json`, and GPUI already types screen space. ~30 lines of world newtypes do the job.
- **A GPUI subtree transform**: it doesn't exist.
- **Per-element `* zoom` helpers**: they keep the scattered math, just renamed.
- **More AGENTS.md prose**: prose can't fail a build.

## Part 5. Build plan

The work runs on branch `integration/canophy`. Each phase is one PR into it from a `feat/canophy-*` worktree, using the shared `CARGO_TARGET_DIR`. At the end there is one PR to main. Every phase ends with ponytail-review and fixes, and docs travel with the change. Step 0 of implementation: save this plan to `docs/canopy/plans/canophy/plan.md`.

### Phase 1: Foundation (no behavior change)
- New `src/canophy/` (pure, no GPUI except geometry types):
  - `mod.rs`: laws and order doc
  - `units.rs`
  - `camera.rs`: `Camera` from `canvas.rs:46`, with `zoom_range`, `widen`, `fit`, `reveal`, `focus_fit`, `visible`
  - `traits.rs`
  - `rules.rs`: moved from canvas.rs: `solve`, `resize_from`, `cascade`, `free_spot`, `child_position`, `tile`, `arrange`
  - `check.rs`
  - `tests.rs`
- `canvas.rs` keeps the document, persistence, colors, wires and panel. `Pane.rect` becomes a `WorldRect`, serialized the same way.
- Make the camera private. Mechanically route the compiler-listed sites through named camera methods, so behavior is identical.
- Consolidate the constants into `Traits`.
- Add the `proptest` dev-dependency. Add the invariants and property test over the existing pure functions.
- Done when: `cargo test --locked` and `just test` are green, `canvas.json` round-trips byte-identical, and nothing changes visually.

### Phase 2: One way in
- Add `Intent`, `step` with the fixed law order, `Motion`, and the debug intent ring.
- Route every writer through `step`. That is every camera and rect write in `canvas_view.rs`, `browser.rs`, `sim.rs`, `dock.rs`, `pane_commands.rs`, `keyboard.rs`, `canvas.rs` (`go`, `place`, `undock`, `sync_subs`) and settings reload (`window.rs:914`).
- Add one `Viewport` value, used by every caller.
- Pinned panes are derived, not written per frame.
- `snap_usable` uses `Camera::fit`.
- Then run ponytail-audit (AGENTS.md: after phase 2).

### Phase 3: Panes are pictures (decisions 1 and 4)
- `layout()` and `Layout`, plus the `ScaledPane` element (`with_rem_size`).
- Pane chrome in `canvas_view.rs` (card, header, pills, space cards, tiles) moves to rems.
- Content scale = zoom:
  - terminal: `set_scale(z)`, grid from the world body, reflow on settle
  - browser: page zoom = z, CSS size = world body
  - PDF: display scale = z
  - sim, image: fit the body
- Delete the setting and keep-path. Old `settings.json` files that still contain the key must keep loading (verify that `parse_person` tolerates unknown keys).
- Overlay chrome is screen-fixed: grips 8 px, 1 px outline, helper bar keeps its declared 0.75–1 clamp. Grips and helper bar hide when a pane is below 48 screen px.
- Move the render-time writes into `sync`.
- Add the scale-equivalence test.

### Phase 4: The remaining laws (decisions 2 and 3, mods)
- `Open` law: camera to 100% for person-opened panes only. `Adopt` never moves the camera.
- `SetSize` "actual size" for sims: world size = device points, camera eases to 100%. This replaces `÷ zoom`.
- `ViewportChanged`: keep the left edge, reveal the focused pane if covered.
- Mod pane declaration of `header`, `size` and `min`, parsed once at the boundary (find where mod pane kinds are declared in `src/mods.rs`). Update the generated `.d.ts`, `src/SKILL.md` and `defaults/AGENTS.md`.

### Phase 5: Docs and close-out
- `docs/wiki/Canvas.md`: new "How panes behave when you zoom" section.
- `Terminals.md`: reflow rule.
- `Simulator-Devices.md`: actual size.
- `Settings-and-Themes.md`: setting removed.
- `Panes.md`.
- New `docs/canopy/canophy.md` (developer page: the laws, how to add a pane kind, how to add a law). Add its owner row in `doc-owners.md`.
- One AGENTS.md line: "Canvas geometry lives in src/canophy. Render reads Layout. New pane kinds declare Traits."
- ponytail-debt.

Phases run in order; each one needs the previous one. The mod-declaration part of phase 4 can run alongside phase 3, because it touches different files.

## Part 6. Verification

- `cargo test --locked` and `just test`, covering unit laws, invariants, the property test (`PROPTEST_CASES=5000` for a deep local run) and the determinism test. Linux CI runs the scale-equivalence render test.
- In the isolated window (`scripts/dev`, launched by the orchestrator), checking with debug invariants on:
  - each pane kind at 25/50/100/200%: the header scales and content stays proportional
  - pinch, then release: the terminal grid is unchanged
  - drag a terminal corner: it reflows once on release
  - open a terminal at 50%: the camera eases to 100% on it
  - an agent spawns a helper: the camera stays put and a toast appears
  - open or widen the Files panel with the entered pane at the right edge: it is revealed
  - pin a pane, then pan and zoom: it doesn't move
  - sim actual size
- On Linux: `scripts/sandbox` shots at the same zooms, then `scripts/sandbox down`.

## Risks and edges
- **Small text at low zoom.** Headers become unreadable below about 40%, which is the decision we accepted. The screen-fixed helper bar and tooltips carry the actions and names.
- **Pinch performance.** Chrome text changes size continuously while pinching. Terminals already do this today. Check frame time with about 20 panes during a pinch.
- **Agent-created panes** don't move the camera. That was my call, made so agents can't steal your view. Say if you want otherwise.
- **World newtypes have public fields.** A stray `px(rect.w)` still compiles. The scale-equivalence test catches it at zoom ≠ 1.
- **No data migration.** Rect values are unchanged, so at 100% the app looks the same before and after.
