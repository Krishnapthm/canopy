# canophy

canophy (canopy + physics, always lowercase) is the one place that decides where panes are,
how big they are, and how the canvas is looked at. It lives in `src/canophy/`. Read this page
before you touch pane geometry, zoom, resize or docking, or before you add a pane kind.

## Why it exists

Before canophy, about 90 places in the window code did their own zoom math. A pane's rect was
in canvas pixels but its header was a fixed number of screen pixels, so every content kind
corrected for the header differently, and each fix broke another kind at another zoom. The
history is in [plans/canophy/plan.md](plans/canophy/plan.md). canophy replaces those sites
with three things: each kind declares how it behaves once, every change goes through one
function, and everything draws from one layout.

## The four laws

These are product decisions. Code that breaks one is a bug, whatever the test suite says.

1. **Zoom is a picture.** Everything inside a pane, header included, scales with the canvas.
   Only overlay chrome keeps its screen size: the helper bar, resize grips, the selection
   outline, wires and the marquee.
2. **A pane the person opens eases the camera to 100%.** Panes an agent creates never move
   the camera; when one lands out of view, a toast with a jump action says so.
3. **The Files panel shrinks the canvas.** The canvas keeps its left edge, and the entered
   pane is revealed when the panel covers it.
4. **Zoom never changes a terminal's grid; resizing reflows it,** once, on release.

## The pieces

| Piece | File | What it is |
|---|---|---|
| World and screen px | `units.rs`, `camera.rs` | World px are canvas px at 100% zoom. Screen px are GPUI's. Only `Camera` converts between them, and its fields are private. |
| `Traits` | `traits.rs` | What a pane kind is: default size, min, aspect, header height, whether it pushes neighbours or docks, and what its body draws. |
| `step(doc, Intent, &Viewport)` | `step.rs` | The only way to change pane geometry or the camera. Applies the intent, then the size law, the camera law and (debug builds) the invariants, in that order. |
| `Layout` | `layout.rs` | `layout(camera, pane)` gives the pane's screen rect, header height, body rect, scale, and body size in world px. Render reads this and nothing else. |
| `ScaledPane`, `wpx` | `src/window/canvas_view/scaled.rs` | Draws one pane subtree at its layout scale by changing the rem size, so chrome written in `wpx(n)` is `n` px at 100%. |
| `Motion` | `motion.rs` | Camera eases and momentum. Time is passed in; each frame is an `Intent::Camera`. |
| `check` | `check.rs` | The invariants (no bad rects, nothing below its min, zoom in range, the camera round trip, camera actions leaving panes where they are). |

A frame goes like this. Input becomes an `Intent`, `step` applies it, the window runs push
(`solve_layout`, which needs each pill's measured size), and render calls `layout` for each
pane and wraps its subtree in `ScaledPane::new(layout.scale, ...)`. `sync_panes` then hands
each live view what it needs from the layout: a terminal its scale and `body_world` (its
grid comes from `body_world`, so zoom cannot change it), a PDF its display scale.

Who asked matters for law 2. Commands carry `person`: true for clicks, keys and mods, false
for the control socket (agents and scripts). Paths that open panes take it and call
`show_full_size` for the person or `toast_out_of_view` for an agent.

## Add a pane kind

1. Add the variant to `canvas::Kind`. The match in `traits()` has no catch-all, so the build
   fails until you declare the kind's `Traits`.
2. Draw its body inside the pane frame the canvas already draws, from `Layout`. Size chrome
   with `wpx(n)`, never `px(n * zoom)`. Size content from `layout.body_world` when it must not
   change with zoom, and from `layout.body` when it fills the screen rect.
3. Open it with an `Intent` (`Place` for a new rect), never by writing `pane.rect`. If the
   person can open it, call `show_full_size` after it lands.
4. Give it helper bar actions through the shared helper bar.
5. Add its parts to the scale-equivalence test if it draws its own chrome.

A mod pane kind needs no Rust: its `mod.json` `"pane"` block declares `header`, `size` and
`min`, which `mods::pane_shape` feeds into `traits()` (see the Layouts and Mods wiki page).

## Add or change a law

1. Write the law as one sentence a person would notice, and agree it with Krishna first.
2. Put it in `step` at its place in the order (apply, size, camera, check), or, if it is an
   ease over time, in the window with `Motion`, landing each frame as an `Intent`.
3. Add the invariant to `check::document` (or a `Violation` for the step that breaks it) so
   debug builds catch any path that skips it.
4. Add a unit test for the law and an `Op` to the property test in `tests.rs` that exercises
   it in random sequences.
5. Update the laws above, the module docs in `mod.rs`, and the user-facing page (Canvas in
   the wiki).

## Testing

`cargo test --locked` runs the unit laws, the property test (set `PROPTEST_CASES=5000` for a
deeper local run), a `canvas.json` round trip, and the scale-equivalence test, which renders
real pane chrome at 100% and 50% and checks every part is exactly half. A debug build panics
on a broken invariant with the violation, the intent that broke it and the last 256 intents,
so feeding those intents to `step` in a test reproduces the bug. Delete any
`proptest-regressions/` folder a failing run leaves behind once the bug has its own test.
