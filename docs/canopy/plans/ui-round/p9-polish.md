# P9 polish: root causes and fix plan

Branch `fix/ui-round-polish` from `origin/integration/ui-round` at `5086e2f`. The PR targets
`integration/ui-round`. One build at the end, then one check pass in an isolated dev window.

Items 3 and 9 were reproduced in an isolated dev window (scratch home and core) on the
`5086e2f` build. The rest are traced in source.

## Root causes

| # | Symptom | Root cause | Where |
|---|---|---|---|
| 1 | Files panel tabs do not look like Chrome tabs | The top strip already uses `strip_tab` with fill = canvas color over the `chrome` strip. The Files strip passes fill `hover` over strip `canvas`, then draws `border_b_1` under the strip, so the active tab never joins the body below. No tab draws Chrome's separators. | `canvas_view.rs` panel strip, `browser.rs` docked tab strip, `strip_tab` |
| 2a | Canvas zoom changes what panes show | `649c727` draws terminals at scale 1 and sizes their body by `rect * zoom`, so zoom reflows them. Images fit their frame, the PDF stays at its own zoom inside a smaller frame, and live pages reflow. Content needs to scale like a picture instead. | `render_canvas` terminal loop, `pdf_card`, `browser.rs` `page_zoom` / `host_frame` |
| 2b | Bounce past max zoom | `zoom_gesture` uses `rubber_band`, then `settle_zoom` eases back. Zoom out never bounces only because `constrain_camera` hard-clamps the minimum. | `zoom_gesture`, `canvas::rubber_band` |
| 4 | Panning does nothing | `649c727` added `canvas_constrain` (default on). `constrain_camera` clamps the camera to the panes' bounds plus a small pad, and centers it when content fits the view, so a wheel, trackpad or hand pan is undone on the same event. | `canvas::constrain_camera`, every call site |
| 5 | Padding behind the toolbar | `27eaf10` moved the toolbar into a flex "lane" beside the canvas, so the canvas ends above the toolbar plus its margin. | `render_canvas_toolbar`, `surface` in `render_canvas` |
| 6 | Auto-pan starts late | Edge pan follows the pointer (within 32 px of the edge), not the pane. The constrained camera also refuses to move until the dragged pane itself grows the content bounds, so about half the pane must leave the view first. | `edge_pan`, `layout_frame` |
| 7 | Closing the last terminal closes the space | `Model::remove_tab` removes a space with no tabs, and `tab.close` of a last tab dispatches `space.close`. On the canvas a browser, PDF or simulator is not a model tab, so the space dies with its last terminal. | `model.rs` `remove_pane` / `remove_tab`, `window.rs` `tab.close` |
| 8 | Home shows only shortcuts | `empty_hints` lists keys only. The model already keeps `recent` (name and folder of closed spaces) but nothing on Home shows it. | `empty_hints` |
| 9 | Files panel shows nothing | The tree root comes from `doc.panel.space`, which only a click on a space tab or card sets. After a restart inside a space, and always on Home, it is empty, so the Files mod shows "Click a space to see its files". | `panel_root` |
| 3 | Browser goes blank after a click | Only local `file://` pages. `439c455` loads them with an incognito store and a deferred `loadFileURL`. WebKit renders them (snapshots are correct), but the live view never appears: a click wakes the page and the pane turns white. An `http://` page with the same HTML renders live. | `ensure_view`, `load_local_file` |
| icon | Old tree glyph | `defaults/icons/canopy.svg` is the old Lucide tree. Welcome and first-run setup draw it. The new mark is `assets/icons/canopy-status.svg`. | `defaults/icons/canopy.svg` |

## Fixes and commits

1. **fix(canvas): free the camera and stop zoom bounce** (items 4, 2b). Delete `canvas_constrain` (unreleased, added today), `constrain_camera` and its calls. `zoom_gesture` clamps to `gesture_range` instead of rubber-banding. Delete `rubber_band`. Zoom limits stay in `zoom_range`.
2. **fix(canvas): float the toolbar over the canvas** (item 5). The toolbar lane becomes an absolute overlay on its edge of the work area. It registers its `cover:` bounds like other canvas floats, so clicks, wheel and live-page masks treat it as covered.
3. **fix(canvas): auto-pan when a dragged pane touches the edge** (item 6). During a pane move, edge speed comes from the pane's screen rect against the canvas bounds: any side at or past an edge pans toward it. Resizes keep the pointer rule.
4. **feat(canvas): scale pane content with canvas zoom** (item 2a, picture-style). Terminals go back to `set_scale(zoom)` with the PTY grid fixed by the world-size body. PDFs get a display scale in `gpui-pdf` that multiplies layout without touching the PDF's own zoom or its label. Live pages use WebKit zoom = own zoom x canvas zoom at the world-size CSS viewport, so they never reflow. Images and simulators already scale with their frame. File previews scale their text. Mod panes (notes, community panes) keep screen-size text, because GPUI has no element transform. Headers keep their screen size.
5. **fix(spaces): keep spaces open without terminals** (item 7). In canvas layouts, removing the last tab leaves the space open with no tabs, and `tab.close` of the last tab closes only the tab. Only an explicit `space.close` removes a space. Classic layouts keep today's behavior.
6. **fix(panel): root the Files tree in the shown space** (item 9). `panel_root` uses the room being shown, then the clicked Home card, then the most recently entered space.
7. **fix(browser): show live local pages** (item 3). In the build, check two variants against the repro: the persistent store with the per-view rule list (rules are added to the view's own content controller, so they still apply), and loading through wry after the rules are ready. Keep the smallest variant that renders and still blocks network loads from local pages.
8. **style(tabs): match Chrome tabs in the Files panel** (item 1). One `strip_tab` for every strip: the strip uses `chrome`, the active tab uses the body color, and the Files panel body uses the canvas color, so the active tab flows into it. No border under the strip. Add Chrome's 1 px separators between inactive tabs, hidden next to the active or hovered tab.
9. **feat(home): list recent spaces under the shortcuts** (item 8). A wide "Recent" list under the hints, VS Code style: name, then folder, from `model.recent`, without spaces that are already open. A click runs `space.new` with that name and folder.
10. **style(app): use the new canopy mark** (icon). Replace `defaults/icons/canopy.svg` with the new mark as a single-color glyph, so every `svg("icons/canopy.svg")` updates.

Each commit updates its wiki page, `defaults/AGENTS.md` and `src/SKILL.md` where behavior changes.

## Check pass

One `cargo build --locked` with the shared target, copied into a scratch bundle, then `scripts/dev`
with its own `TMPDIR`. Walk the ten items above in the dev window, then `cargo test --locked`
and `just test`, ponytail-review, and the PR into `integration/ui-round`.

## Review fixes

A review of the PR found eight issues, fixed on the same branch:

1. Terminal grids measured the body under a screen-size header, so zooming out lost rows. The grid now comes from the body at zoom 1 (`terminal_body`), drawn at the scale that fits it under the header. Live pages keep a fixed CSS width the same way; their CSS height still follows the drawn body.
2. A dragged pane bigger than the canvas pushed forever. `edge_push` pushes on an axis only when one edge is passed; across an axis it covers, the pointer rule decides.
3. `recent` deduped by name, so unnamed spaces overwrote each other. It is keyed by folder, and Home filters by open folders.
4. The floating toolbar lost its overflow scroll. Longer than the canvas, it scrolls in a lane across its edge.
5. Fit, zoom limits and the empty hints ignored the toolbar over the canvas. They use `open_canvas`, the canvas less the toolbar's strip.
6. Local pages fell back to an incognito store before macOS 14. They use WebKit's default store there.
7. PDF gaps and padding stayed in screen px under the display scale, so long documents drifted. They scale with it.
8. `tab.close` on a canvas space's last terminals ended them without `confirmed`. It asks a person and needs `confirmed: true` from commands.

## Not in scope

- The Files mod polls every 10 s (`cx.timer.every(10000, ...)`), which breaks "idle costs nothing". Worth a follow-up.
