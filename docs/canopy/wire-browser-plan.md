# Wire, Linux browser and browser lag: root causes and plan

## Context

Krishna reported three problems: wiring never connects (on macOS too, in the
released v0.1.3), the Browser button is missing on Ubuntu, and a live browser
pane makes the app very laggy. This plan finds the cause of each and fixes
them in three phases on one new branch off `main`.

## Root causes

### 1. Wire: the canvas never sees the click or drop that finishes a wire (all platforms)

- Commit `c42b9e6` (2026-11-20, in v0.1.2 and v0.1.3) added `.occlude()` to
  every pane card (`src/window/canvas_view.rs:2593, 3174, 3212, 3431`). That
  let the front pane take clicks.
- In GPUI, `capture_any_mouse_down`, `on_mouse_up` and `on_mouse_move` only
  run while the element's own hitbox is hovered
  (`vendor/gpui-pre/src/elements/div.rs:152, 217, 309`). An occluding pane
  makes the canvas area behind it un-hovered (`Hitbox::is_hovered`,
  `vendor/gpui-pre/src/window.rs:782`).
- Every step that finishes a wire hangs off the canvas area:
  - `wire_pick`, which runs on the click after "+" or Connect, is called
    from the area's `capture_any_mouse_down` (`canvas_view.rs:2701`).
  - `wire_release`, which runs on a drag's drop, is called from the area's
    `on_mouse_up` (`canvas_view.rs:2936`).
  - `wire_pointer`, which makes the line follow and shows the handles, is
    called from the area's `on_mouse_move` (`canvas_view.rs:2852`).
- So a click or drop on a pane goes to the pane, and the line keeps following
  the pointer. That matches "line, no connect".
- The unit test `sticky_wire` (`wire_view.rs:381`) calls `wire_pick` and
  `wire_release` directly, so it skips hit testing and still passes.

### 2. Browser missing on Linux: it is built for macOS only

- `Cargo.toml:65-77`: `gpui-wry`, `wry` and the objc2 crates are macOS-only
  dependencies.
- `canvas_view.rs:3532`: the Browser button is drawn only when
  `cfg!(target_os = "macos")`.
- About 25 `cfg(target_os = "macos")` sites in
  `src/window/canvas_view/browser.rs` and `browser_mask.rs` cover the live
  view, the mask, snapshots, the pop-out window and annotations. Some of
  that code calls macOS APIs directly: the CALayer mask, the WKWebView
  snapshot and NSWorkspace.
- Hard limit: wry can embed a child webview only on X11, not on Wayland
  (`lb-wry-0.53.3/src/lib.rs:1412, 1447`). It also needs `gtk::init` and a
  running GTK main context. Decision: X11 child window, with Canopy running
  under XWayland on Wayland sessions.

### 3. Browser lag (macOS): suspects from reading the code, not measured yet

Ranked by likelihood:

1. gpui-wry sets the native view's bounds on every prepaint, even when they
   haven't changed (`vendor/gpui-kit/crates/webview/src/lib.rs:211`). That
   is a WKWebView `setFrame` on every canvas frame.
2. Page zoom is reapplied for every zoom step while the camera animates
   (`browser.rs:~566-578`). Each change makes WebKit lay the page out again,
   once per frame.
3. A page that rests is snapshotted through a TIFF encode and decode on the
   main thread (`browser.rs:1554-1566`). That causes a hitch on every
   click-away.
4. Saved snapshots are decoded on the main thread at start
   (`browser.rs:500`, already marked as a `shortcut:`).
5. Unknown: a render loop while idle, for example page events calling
   `cx.notify()` continuously. This must be ruled out by measuring.

## Branches

- New branch `fix/wire-and-browser` off `main`, in a worktree at
  `.claude/worktrees/wire-and-browser`. The current checkout has
  uncommitted header and icon work, so leave it alone. This plan is committed
  as `docs/canopy/wire-browser-plan.md` and pushed.
- `feat/linux-desktop-integration` (one commit, `f7dc836`, based on current
  `main`) covers native menus, title bar, window controls and the launcher.
  That is a different problem, so don't branch off it. It should merge
  before phase 3, though. Both touch Linux windowing in `window.rs` and
  `main.rs`, and its docs name "Ubuntu GNOME Wayland" as the Linux
  reference, which phase 3's XWayland choice changes. Its decoration code
  already handles both X11 and Wayland, because it checks
  `window_decorations()` at runtime.

## Phase 1: fix wiring (all platforms)

1. Reproduce it end to end on the Mac with `scripts/dev` (an isolated home
   and core): open two terminals, click "+", then click the other pane. The
   line keeps following the pointer and no wire forms. Close the window and
   run `scripts/dev-stop`.
2. Write a failing GPUI test that drives real simulated mouse events
   (move, down, up) over an occluding pane card, instead of calling
   `wire_pick` directly. Model it on the click-through test from `33ab2fd`
   ("prove a click on overlapping panes raises only the front one").
   Replace the direct-call `sticky_wire` test with it.
3. Fix it in one place. Move wire input to the window-level listeners that
   the canvas paint closure already registers (`canvas_view.rs:~2725`,
   `window.on_mouse_event`, which ignores hit testing):
   - On mouse down, in the capture phase, with the left button while
     `wire.sticky` is set: call `wire_pick`, then `stop_propagation`.
   - On mouse up with the left button while a non-sticky `wire.drag` is
     set: call `wire_release`.
   - On mouse move inside the canvas bounds `b`: call `wire_pointer`, so
     the line follows and the handles show over panes again.
   - Remove the three calls from the area listeners. Move them; don't
     duplicate them.
4. Update `docs/wiki/Wiring.md` only if the behavior described there
   changes.

## Phase 2: browser lag (macOS)

1. Measure before changing anything. Run `scripts/dev` with one live page
   (for example a localhost dev server), then profile with Instruments Time
   Profiler while idle, while panning, and while zooming. Count renders per
   second while idle; it must be 0.
2. Fix only what the profile confirms:
   - gpui-wry prepaint: skip `set_bounds` when the bounds equal the last
     ones it set (store them in the vendored element).
   - Zoom: apply page zoom once the camera settles, not on every animation
     frame.
   - Snapshot: get raw pixels instead of a TIFF round trip, or decode on
     `background_executor`.
   - Saved-snapshot decode at start: move it to the background, which
     closes that `shortcut:`.
   - Any idle render loop: remove the notify that causes it.
3. Re-profile the same three scenarios and report the before and after
   numbers.

## Phase 3: browser on Linux (X11 child, XWayland)

Do this after `feat/linux-desktop-integration` lands.

1. Dependencies: move `gpui-wry` and `wry` to `cfg(any(macos, linux))`.
   Linux needs the system libraries `libwebkit2gtk-4.1` and `libgtk-3`.
   Name their cost and license per AGENTS.md, and add them to
   `scripts/install-linux` and the README.
2. A cfg alias: `build.rs` emits `cargo::rustc-cfg=webview` on macOS and
   Linux. Browser-wide gates become `cfg(webview)`; truly macOS-only bits
   stay `cfg(target_os = "macos")`. The button gate at
   `canvas_view.rs:3532` becomes `cfg!(webview)`.
3. Backend: on Linux, unset `WAYLAND_DISPLAY` before GPUI starts so the app
   runs on X11 (XWayland). Cost: no fractional scaling on Wayland, so text
   can look soft. State this in the Linux wiki page.
4. GTK: call `gtk::init` on the main thread at start. Register the GLib main
   context's poll file descriptors with GPUI's calloop loop in the vendored
   `gpui-pre-linux`, and dispatch when they become ready. No timers, so idle
   still costs nothing.
5. Per-platform replacements for the macOS-only pieces:
   - Mask: XShape on the webview's child X window (`x11rb`, which
     `gpui-pre-linux` already uses).
   - Snapshot: webkit2gtk `WebView::snapshot` to a cairo surface, then
     RGBA.
   - Default browser name: GIO's default app for `x-scheme-handler/https`.
   - The pop-out window and annotations: the same embedding path.
6. Verify in `scripts/sandbox`, which runs on Linux: open the Browser
   button, load a page, pan and zoom, put a pane over the page, rest and
   wake it, pop it out, then `scripts/sandbox down`. CI covers the Linux
   build on the PR.

## Verification

- Phase 1: the new simulated-mouse test fails before the fix and passes
  after it. In `scripts/dev` on the Mac, a click on "+" then a pane, a drag
  from "+" onto a pane, and Connect then a pane each make a wire. Escape
  still cancels.
- Phase 2: before and after Instruments numbers for idle, pan and zoom.
- Phase 3: the `scripts/sandbox` steps above, and the Linux CI job.
- Each phase: `just test` with the shared `CARGO_TARGET_DIR`,
  ponytail-review, docs updated (`docs/wiki/Wiring.md`,
  `docs/wiki/Browser-Pane.md`, the Linux page), logical commits, pushed.

## Not verified

- I didn't reproduce wiring by running the app. The cause comes from the
  code and GPUI's source, and it matches the reported symptom.
- The lag causes are unmeasured. Phase 2 starts by measuring.
- GTK main-context integration with calloop hasn't been tried in this
  codebase.
