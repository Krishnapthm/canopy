# Canopy UI round - orchestrator

## Context
Krishna wants one round of UI/UX fixes, run with maximum parallelism: Claude takes the
complex pieces and Codex takes the rest. The plans live in `docs/canopy/plans/ui-round/`, one file per task, each starting with the shared rules.

Names, for Krishna:
- **Status strip**: the 24 px bar at the bottom, the `workspace-status` mod
  (`defaults/layouts/main.json:19-24`). The Classic layout calls it the "status bar".
- **Toolbar**: the floating pill at the bottom center of the canvas.
- **Top strip**: the 36 px row at the top.
- **Files panel**: the right drawer.
- **Helper bar**: the hover pill above each pane.

Already built, so not planned: agent sounds (`src/sound.rs`, Settings > Alerts, played
from `fn alert` at `src/window.rs:2186`) and per-space canvas colors (`canvas::Space.color`).
"Cancelled" is dropped, per Krishna.

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

## Ownership and parallelism
| Plan | Owner | Branch | Main files | Depends on |
|---|---|---|---|---|
| P1 canvas input + wire input | Codex (high effort) | feat/canvas-input | canvas_view.rs input region, canvas.rs, gpui-pre-linux | replaces PR #34 phase 1 |
| P2 colors + edges | Codex (high effort) | feat/pane-colors | canvas.rs Hue, settings.rs tints, wire_view.rs, menu, workspace-status | - |
| P3 simulator | Codex | feat/sim-controls | sim.rs, helper_bar.rs, commands.rs | P1 for edge resize (soft) |
| P4 annotate | **Claude** | feat/annotate-redesign | annotate.rs, browser.js, browser.rs pick | - |
| P5 drawer | **Claude** | feat/drawer-panes | canvas_view.rs panel, browser_mask.rs, pane_commands.rs | P2 `effective_color` (soft) |
| P6 Linux top strip | Codex | feat/linux-top-strip | titlebar.rs, canvas_view.rs top strip | **linux-desktop-integration merged (hard)** |
| P7 notifications | Codex | feat/desktop-notifications | new src/notify_mac.rs, vt.rs, window.rs alert, settings | - |
| P8 layout art | Codex (small) | feat/layout-preview | defaults/layouts/main.svg | - |

**Sequential steps for Krishna:**
1. Merge `feat/linux-desktop-integration` before P6 starts.
2. Suggested merge order: P8, P7, P1, P2, P3, then rebase P5 and P4, then P6.
3. P5 and P6 both touch the top strip's right-hand buttons (`canvas_view.rs:3526-3537`), so
   expect one small conflict.
4. Draft PR #34 (`fix/wire-and-browser`) has only a plan doc. Its phase 1 (wire input) is folded
   into P1, so drop it there. Its phases 2-3 (browser lag, Linux browser) touch `browser.rs`;
   run them after P4/P5 merge, or expect conflicts.

## Orchestrator prompt
> You orchestrate the Canopy UI round. The plans are in `docs/canopy/plans/ui-round/` on
> every task branch and on `integration/ui-round`. Every task PR targets
> `integration/ui-round`; Krishna reviews one final PR from it into main.
> 1. `feat/linux-desktop-integration` gets a PR into `integration/ui-round`. Krishna reviews and
>    merges it first; P6 waits for it and then rebases onto the integration branch.
> 2. Each session creates its own worktree on the branch named in the table (the branches already exist and are pushed).
>    Export one shared `CARGO_TARGET_DIR` for all of them. Give P4 and P5 to Claude and the
>    rest to Codex: high effort for P1 and P2, the default for P3, P6, P7 and P8.
>    Paste the plan file as the brief, word for word. The briefs make every design call, so
>    agents should not ask design questions back; if one hits an ambiguity, it stops and
>    reports.
> 3. Agents never launch the app. When a branch reports done:
>    - run ponytail-review on its diff and send the findings back;
>    - check its docs against doc-style.md;
>    - copy and strings-check the binary, then launch the isolated dev window (`scripts/dev`)
>      and check the plan's Verify list by hand on macOS. For P1 and P6, also use
>      `scripts/sandbox` on Linux.
>    - Close the window, then run `scripts/dev-stop`.
> 4. Merge every task PR into `integration/ui-round`, never main. Order: P8, P7, P1, P2, P3, then P5 and P4 (rebased; resolve the top-strip and
>    browser.rs conflicts), then P6. Run `cargo test --locked` and `just test` on the integration branch after
>    each merge.
> 5. After all of them, run ponytail-audit and ponytail-debt, and report to Krishna:
>    - what shipped, per plan;
>    - what was not verified, especially real macOS notifications (needs the installed app)
>      and Linux trackpad on real hardware;
>    - any leftover `shortcut:` debt.
> Never touch the daily app or core, never push to main without Krishna, and never run paid
> agent sessions without asking.

## Verification summary
- Unit tests per plan (listed above), plus `cargo test --locked` and `just test`.
- Hands-on in the isolated dev window (orchestrator): P1 gestures and resize, P2 colors in
  both themes, P3 shut down, close and sizes, P4 annotate flow end to end with a wired agent,
  P5 dock and undock with wiring, P8 the preview card.
- Linux sandbox: P1 and P6.
- Installed app only: P7.
