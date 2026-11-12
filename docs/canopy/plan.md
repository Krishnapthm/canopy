# Canopy canvas: fork plan

A self-contained plan for a fresh agent session. Read all of it before touching anything.
Written 2026-11-12. Owner: Krishna. Canopy as a whole pivots to this; the repo and name stay. Nothing below has been started.

## 1. What we are building

Canopy is a **control room for agents and terminals**: a native desktop app (Rust, GPUI) where
every terminal and coding agent lives as a card on one free, Figma-like canvas instead of in a
sidebar-and-body layout.

The full feature list, as asked for:

1. A free canvas you pan and zoom. Not panes, not a sidebar plus body layout. **Plain
   background like Figma, no dot grid**; its color is a setting.
2. **Spaces** on the canvas, each identified by a color, with an optional frame.
3. Right-click **inside a space**: open a terminal or agent in that space (its folder).
4. Right-click **outside any space**: open a plain terminal at the root (home folder).
5. Sessions can be agents (Claude Code, Codex, and any agent herdr already knows).
6. On the canvas each session is a **plain rectangle ("pill")** with its icon (LobeHub or
   equivalent) and its name: `claude` / `codex` plus the session name, or the terminal's Canopy
   name. No colored side borders or accents. Clicking a pill opens the real terminal in a **side
   panel**, which can **pop out** into its own OS window. A setting switches pills to **movable,
   resizable live terminal windows** on the canvas instead.
6a. Background sessions and sub-agents **branch out from their parent** with a connecting edge:
   no dot at the parent end, a small dot at the child end.
7. Any file opens in a **markdown / code preview** card.
8. Text in a preview is **quotable** and can be **sent to an agent** session.
8a. An agent's **plan** can be viewed, quoted and **edited** on the canvas, with the agent aware
   of the edit (Claude Code already supports editing its plan in VS Code). Claude Code and Codex
   first, other agents slowly after.
8b. Built for **reusability and interoperability**: adding an agent is mostly data (section 3.1).
9. A **view switch** in the top right (shadcn-style segmented tabs): `Agents` / `All` /
   `Terminals`. Agents shows only agent terminals across all spaces, Terminals only plain ones.
10. Name and philosophy stay **Canopy**, "a control room for agents and terminals".

## 2. Decisions already made (do not re-open)

| Decision | Choice | Why |
|---|---|---|
| Base | **Fork [herdrdev/herdr-app](https://github.com/herdrdev/herdr-app)** (Apache-2.0, Rust, GPUI) | Mature terminal rendering (libghostty), keys, IME, agent status, persistence via herdr-core. Krishna likes its design. |
| Terminal backend | **herdr-core (`hc`)**, used as an external binary, not forked | It owns PTYs, keeps terminals alive when the window closes. Already installed at `~/.local/bin/hc`. |
| Canvas | **Our own, ~400 lines, on plain GPUI primitives.** No canvas crate. | See section 4. Templum (MIT) was studied, its design rejected; we copy ideas, not its crate. |
| Not used | Templum as a base, REA, building from scratch | Templum: design. REA: herdr is open source, read the code. Scratch: terminal layer is the hard part and herdr has it. |
| License | Whole fork is **Apache-2.0** | Least paperwork. See section 9. |
| Repo and name | **This repo** (`~/projects/canopy`, GitHub `Krishnapthm/canopy`) becomes the fork. Canopy as a whole pivots; there is no `canopy-app`. | The name stays Canopy. Work happens on branch `pivot/canvas`, cut from herdr-app's history so upstream fixes can still be merged. |
| Old daemon / web / TUI | Stays in `main`'s history, not carried onto `pivot/canvas`. | When the pivot works, Krishna decides how `main` is replaced (step 12). |
| Platforms for now | macOS first, Linux must keep building | Krishna's machine is macOS. |

## 3. Hard rules for the session

- **Never touch Krishna's real herdr setup.** He uses herdr-app and herdr-core daily. The fork must
  use its own app home and its own core session/socket. A bare run of a herdr-app binary opens a
  window on the real setup (herdr's own AGENTS.md says so). Step 0.4 establishes isolation; do not
  run the app before it is done.
- Never stop, signal or restart a herdr/hc process you did not start.
- Never leave the app or a core session running when you finish a step. Stop what you started.
- No telemetry, no update checks, no feedback upload, no network requests of our own.
- Do not push, create GitHub repos, or publish anything without Krishna saying so.
- No em dashes anywhere (code, docs, commits). Use a plain dash.
- Commits: `type(scope): description`, lowercase, imperative, no agent attribution.
- Harness/agent behaviour is never coded from memory: verify against the installed CLI.
- **Ponytail at all times.** Every session and every sub-agent starts by loading the `ponytail`
  skill (level `full`) and keeps it on. Before calling any phase done, run `ponytail-review` on
  the phase's diff and fix what it finds, then `ponytail-debt` to list every `shortcut:` comment
  in the summary to Krishna. Use `ponytail-audit` once after phase 2 on the whole fork.
- Never run a real agent session (Claude Code, Codex) that spends tokens without asking Krishna
  first. Never edit his real `~/.claude`, `~/.codex` or other harness configs.

### 3.1 Reusability and interoperability

Canopy starts with **Claude Code and Codex** and adds other agents slowly. Build so that adding
an agent is mostly data, not code:

- **Agent-agnostic features first.** The canvas, cards, preview, quote, plan cards and the editor
  handoff (Phase R) work on any terminal. Nothing in them names a vendor.
- **One agent description per agent**, as data: how to detect it, how to launch it, how to send
  text, where its plan lives, how its external-editor flow works. Reuse herdr's existing program
  table / detect manifests as that description (extend its format; do not invent a parallel one).
  Adding agent number three = adding one entry plus a harness note.
- **Code per agent only where data cannot express it**, behind one small function per capability
  (for example `plan_path(agent, session) -> Option<PathBuf>`). No trait hierarchy, no plugin
  system, no capability that only one agent has today unless Krishna asked for it (ponytail:
  the second agent proves the shape, not a guess).
- **Interoperate through what the CLIs already offer**: their files, `$EDITOR`/`$VISUAL`, CLI
  flags, hooks passed per session. Never a vendor API, never their credentials.
- **Reuse inside the app**: one preview card component serves files, plans and diffs; one
  "send to agent" path serves quotes, plan edits and anything later.

## 4. What we learned from Templum (reference only)

Templum (`github.com/KingDasWinx/Templum`, MIT) has no canvas library. Its canvas is:

- A **camera**: world `origin` (point at the viewport's top-left) plus `zoom`.
  `to_screen(p) = (p - origin) * zoom`, `to_world(s) = s / zoom + origin`,
  `pan_by(d) = origin -= d / zoom`.
- **Blocks**: `Vec<{id, rect}>` in back-to-front order.
- **Drawing**: each block is an ordinary GPUI `div().absolute().left(x).top(y).w(..).h(..)` at its
  screen rect. GPUI cannot scale an element subtree, so every size and font size is multiplied by
  the zoom and laid out again.
- Non-element drawing (for us: the parent-child edges) uses GPUI's `canvas()` paint element.
  We do **not** draw a dot grid; Templum does.
- Input via GPUI `on_scroll_wheel` and `on_pinch`.

Two lessons we keep:

1. **Fixed zoom steps.** GPUI rasterizes glyphs once per font size; continuous zoom would grow the
   glyph atlas without bound. Zoom always lands on a step from this list:
   `0.25, 0.33, 0.5, 0.67, 0.75, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75, 2.0`.
2. **Level of detail** (window mode only, 7.5.3). Below zoom `0.5` a live terminal is
   unreadable; draw it as its pill instead of live text. Cheaper and easier to scan.

We skip everything else Templum has (wires, ink, tiling, camera flights, snap guides, text
"greeking"). A clone for reference may still exist at
`/private/tmp/claude-501/-Users-krishna-projects-canopy/9ca96e28-a234-4b04-b527-9d46b0e4fe9f/scratchpad/templum`
(read-only; re-clone with `git clone --depth 1` if gone). If our card drag/resize turns out buggy,
read `crates/templum-canvas/src/gesture.rs` and copy its approach with MIT credit.

## 5. Phase 0: fork, isolate, build, learn (do first, in order)

### 0.1 Branch this repo from herdr-app

Everything happens in `/Users/krishna/projects/canopy`. Start with a clean working tree except
this plan file, which is untracked on purpose and follows you across the branch switch.

```bash
cd /Users/krishna/projects/canopy
git status --short
```

```bash
git remote add upstream https://github.com/herdrdev/herdr-app
```

```bash
git fetch upstream
```

```bash
git switch -c pivot/canvas upstream/master
```

```bash
git submodule update --init --depth 1
```

(Check `git remote show upstream` for the default branch name if `master` is wrong.)

- `origin` stays `Krishnapthm/canopy`. Do not push `pivot/canvas` until Krishna says so.
- After the switch, files ignored by old `main` but not by herdr's `.gitignore` (for example
  `web/node_modules`, `web/dist`, `target/`) may show as untracked. Do not delete them; list them
  for Krishna and add nothing to git that came from old `main`.
- Move this plan into the branch as `docs/canopy/plan.md` and commit it as the branch's first
  commit (`docs(canopy): add the canvas pivot plan`).
- Add a `CLAUDE.md` containing `@AGENTS.md` (old `main` had one; herdr does not).
- The project memory folder is keyed by this path, so earlier notes stay available.

### 0.2 Toolchain

`zig` is **not installed** (the libghostty build needs it). Read `crates/ghostty-sys/build.rs` and
herdr-app's CI (`.github/workflows/ci.yml`) for the exact Zig version, then ask Krishna before
installing it (`brew install zig` or the pinned version). Also check for `just`.

### 0.3 Build

Use the repo's own recipe (`just build`, see its `justfile`/AGENTS.md). Debug build only.
Acceptance: the build finishes. Record the build time.

### 0.4 Isolation (gate: nothing runs before this passes)

Find and write down, from the source (`src/core_bin.rs`, `src/main.rs`, `src/mods.rs`,
`hc --help`):

- the env var or flag for the app home (expected: `HERDR_APP_HOME`),
- how the app picks the core session/socket, and how to point it at a separate one (an env var,
  a `--session` flag, or a socket path),
- where core keeps its state, so the scratch session cannot overwrite Krishna's.

Create `scripts/dev` in the fork that runs the debug app with a scratch home under
`$TMPDIR/canopy-dev/home` and a scratch core session, and a matching `scripts/dev-stop` that stops
only that core session. Acceptance: with Krishna's real herdr-app running, `scripts/dev` opens a
second window that shows **none** of his spaces, and `hc` for his real session still lists his
terminals unchanged afterwards. If isolation is not possible without changing the app, stop and
report to Krishna before going further.

### 0.5 Read and write a map

Read and summarize into `docs/canopy/herdr-map.md` (in the fork), one paragraph each:

- `src/layout.rs`: node kinds (`Kind` enum), how a layout JSON becomes nodes, floats.
- `src/window.rs`: where the tree is laid out and painted, where a terminal block is rendered,
  input routing (mouse, scroll, pinch, keys, focus).
- `src/terminal.rs` + `src/vt.rs`: the terminal element. **Key question: can one terminal be drawn
  with its own font size / zoom, independent of the window's zoom?** Find the "px at zoom 1" code
  path.
- `src/model.rs`: how spaces, tabs, panes and terminals are modelled, what comes from core.
- How the app tells an agent from a plain shell (`programs.rs`, programs.json, agent status).
- How a mod or the app writes text into a terminal (`terminal.paste`, `hc` commands): **can we
  write arbitrary text, or only the clipboard?**
- How `herdr-app ls/status/commands` work (useful for checking our changes without screenshots).
- `src/telemetry.rs`, `src/update.rs`, `src/feedback.rs`: every network call (grep `http`,
  `releases.herdr.dev`, `https://`).

Each answer that contradicts something in this plan: update the plan file (section 13 log) and
tell Krishna before building on it.

## 6. Phase 1: make it Canopy (rebrand, strip network)

Acceptance for the phase: `scripts/dev` opens a window called Canopy, no request leaves the
machine (check with `nettop -p <pid>` or Little Snitch if installed), tests pass.

1. **Remove telemetry entirely**: delete `src/telemetry.rs` and every call, the settings key, the
   mod API entry (`herdr.d.ts`), the Privacy settings section. Not "off by default": gone.
2. **Remove the updater and feedback**: delete `src/update.rs`, `src/update_notes.rs`,
   `src/feedback.rs` and the `whatsnew`/`feedback` frame mods, their commands and keys. Keep
   herdr-core discovery (`core_bin.rs`) but drop "install the core's release" if it downloads;
   instead show a toast: "Canopy needs herdr-core: run `<install command>`".
3. **User-visible rename only** (internals later):
   - App name, window title, macOS bundle name and id (`dev.canopy.app`), binary name `canopy`.
   - Default config home `~/.config/canopy` (must differ from herdr-app's so the two never share
     files). Env var for the home: keep herdr's name in phase 1, note it in the map.
   - Remove Herdr icons and logo, use a plain placeholder (a tree glyph from Lucide, which herdr
     already ships with its license).
   - README rewritten for Canopy, with a "Based on herdr-app" credit line.
   - Do **not** rename Rust modules, env vars or mod APIs yet. Keeps upstream merges easy.
4. License files: section 9.
5. Replace herdr's AGENTS.md with a short Canopy one (keep its build/test/sandbox facts, add the
   hard rules from section 3 of this plan, drop "ask Can").

Note: the old Canopy (`cargo install --path crates/canopy`) may already have put a binary named
`canopy` on PATH (`which canopy`). Tell Krishna; do not uninstall anything yourself.

## 7. Phase 2: the canvas (today's target)

### 7.1 Model

- **Space** = a herdr space (it already has a name and a folder). The canvas adds geometry and
  look, stored by Canopy, keyed by the herdr space id.
- **Card** = one herdr **tab with a single pane holding one terminal**. Tabs and splits are not
  shown as such; every terminal is its own card. (Verify in 0.5 that `terminal.open
  {place: "tab"}` gives exactly this. If herdr cannot hold a terminal without a tab, this mapping
  stands; if it can, still use tabs for compatibility with herdr's model.)
- **Root**: terminals opened outside any space go into one herdr space named `Canvas`, folder
  `~`, created on first use. Its frame is never drawn; its cards float freely on the canvas.
- **Membership is fixed at creation.** A card belongs to the space it was created in. Dragging a
  card is clamped to its space's rect. Dragging a space moves all its cards. A space cannot be
  resized smaller than the bounding box of its cards plus 24 px padding.
- **Parent and child.** A terminal's parent comes from herdr-core (the `parent` field in
  `hc ls --json`, verify in 0.5). Sub-agents that have no terminal of their own come from
  Phase R (question 9); until then only terminals with a core parent branch out.
- **Placing new cards nobody placed** (created by an agent via `hc`, or a sub-agent): a child
  goes 80 world px to the right of its parent's pill, below its last sibling with 16 px between
  siblings, in creation order. A card with no parent goes to the first free spot in its space,
  scanning right, then down, in steps of the pill size plus 48 px. Once placed, a card stays
  where the person drags it.

### 7.2 Storage

`~/.config/canopy/canvas.json` (inside the app home, so dev runs use the scratch home):

```json
{
  "version": 1,
  "camera": { "x": 0, "y": 0, "zoom": 1.0 },
  "view": "all",
  "spaces": [
    { "space": "<herdr space id>", "rect": [x, y, w, h], "color": "blue", "frame": true }
  ],
  "cards": [
    { "terminal": "<herdr terminal id>", "rect": [x, y, w, h], "z": 3 },
    { "subagent": "<id from Phase R>", "parent": "<herdr terminal id>", "rect": [x, y, w, h], "z": 5 },
    { "preview": "/abs/path/README.md", "space": "<herdr space id>", "rect": [x, y, w, h], "z": 4 }
  ],
  "panel": { "width": 640 }
}
```

`rect` holds the position for pills; `w`/`h` are only used in window mode (7.5.3) and keep the
last window size so switching modes back and forth loses nothing. Appearance settings live in
herdr's `settings.json`, not here: `canvas_color` (7.4) and `card_style` (7.5.3). Add both keys
to `defaults/AGENTS.md` (herdr's tests check that every settings key is listed there).

World units are logical pixels at zoom 1. Write the file on change, debounced to at most once per
500 ms while dragging, and on quit. A card whose terminal no longer exists in core is dropped on
load. Corrupt file: keep it as `canvas.json.bad`, start empty, show a toast.

### 7.3 Layout node

Add one layout node kind `canvas` to `src/layout.rs` and render it in `src/window.rs`. Ship a
Canopy default layout (`defaults/layouts/main.json`) that is a full-window `canvas` node with the
titlebar area for traffic lights, and **no sidebar or tab strip**. Keep herdr's other layouts
working (the canvas is additive).

### 7.4 Camera and input

- Camera = `origin` + `zoom`, zoom always one of the steps in section 4. Start at `{0,0}`, `1.0`.
- **Two-finger scroll / mouse wheel**: pan.
- **Pinch**, or **Cmd + scroll** (Ctrl on Linux): zoom one step per gesture tick, around the
  cursor (the world point under the cursor stays put).
- **Space + drag** on the canvas, or **middle-mouse drag**: pan.
- `Cmd+0`: zoom to 1.0 around the window center. `Cmd+=` / `Cmd+-`: one step in or out.
  `Cmd+Shift+F`: fit all spaces and cards in view (snap down to a step).
- In window mode (7.5.3), scroll over the **focused** live terminal goes to the terminal
  (scrollback), not the canvas. Everywhere else, scroll pans the canvas.
- **Background: plain, one flat color, no dot grid, no pattern** (like Figma). Default follows
  the theme: `#F5F5F5` light, `#1E1E1E` dark (Figma's own canvas colors). Setting `canvas_color`
  in Settings > Look: `Default` (follows theme) plus a color picker / hex field; `null` in
  `settings.json` means Default. Changes apply live, like herdr's other Look settings.

### 7.5 Cards

#### 7.5.1 Pill (default `card_style: "pill"`)

- A plain rectangle: height 36 world px, width fits its content (min 160, max 320, the name is
  truncated with an ellipsis past that), 8 px radius, 1 px border in the theme's `border` color,
  fill in the theme's `surface` color. **No colored left border, stripe, glow or accent of any
  kind.** Focused pill: the same 1 px border in the theme's `ring` color, nothing else.
- Content, left to right, 12 px padding, 8 px gaps:
  1. **Icon**, 16 px. Agents: their brand icon in LobeHub Mono style. herdr already ships LobeHub
     icons (`defaults/icons/LICENSE-LobeHub.txt`): find which ones and reuse that pipeline; if an
     agent has none, use the closest equivalent and note it. Plain terminals: Lucide `terminal`.
  2. **Kind label**: `claude`, `codex` (the short name from the agent description, section 3.1),
     or `terminal` for shells. Regular text color.
  3. **Name**, muted text color. Agents: the agent's session name (how to read it is Phase R
     question 10; fallback: the terminal title the program sets). Terminals: the Canopy
     terminal name, which the person can rename (double-click the name or the card's menu);
     default: the terminal title, else the folder's basename.
  4. **Status glyph** (agents only), at the right end, 12 px, theme `muted-foreground`: a small
     spinner while working, a small bell when blocked (waiting for you), nothing when idle or
     done. This is the default fallback from herdr's agent status. **For Claude Code, the
     executors should try to show its own live spinner words** ("Frolicking...",
     "Discombobulating..." and so on) in place of the session name while it works: Phase R
     question 8 decides if and how; do not hardcode a word list.
- **Click**: opens the terminal in the side panel (7.5.2) and gives it the keyboard.
  **Double-click**: centers the pill in view at zoom 1.0. **Drag**: move (clamped to its space).
  Pills do not resize. **Right-click** the pill: Rename, Open in panel, Pop out, Close terminal
  (asks to confirm if a program is running, with herdr's confirm mod).

#### 7.5.2 Side panel

- Docked to the right of the canvas, beside it (the canvas area shrinks; nothing is covered).
  Default width 45% of the window, min 360 px, max 75%; drag its left edge to resize. Width is
  saved in `canvas.json` (`panel.width`).
- Header: the same icon, kind label and name as the pill, then `Plan` (8.4, only when the agent
  has a plan), `Pop out` (8.1), and `Close` (closes the panel only; the terminal keeps running).
- Body: the live terminal at zoom 1.0, herdr's existing terminal element.
- One terminal at a time: clicking another pill swaps it. Palette commands `panel.close` and
  `panel.pop_out`. The view switch stays at the top right of the canvas area, left of the panel.

#### 7.5.3 Window mode (`card_style: "window"`, Settings > Look > Cards: Pills / Windows)

Krishna asked for this as an option even though pills are the default.

- Each card becomes a live terminal window on the canvas: a 36 px header identical to the pill's
  content (plus `Pop out` and a close button), and the live terminal body below.
- Default size 80 cols x 24 rows at the zoom-1 terminal font. **Drag the header** to move
  (clamped to its space); **drag the bottom-right 12 px corner** to resize; on release, resize
  the PTY to the new cols/rows computed at zoom 1, never from the on-screen size.
- **Click**: focus (keyboard goes to it) and raise to the top. The side panel is not used.
- **Zoom >= 0.5**: live terminal drawn at `font_size * zoom`. **Zoom < 0.5**: drawn as its pill.
  If 0.5 shows that per-card font scaling is not possible in herdr's terminal element, stop and
  report before working around it.
- Switching the setting re-renders every card in the other mode; positions are kept.

#### 7.5.4 Edges (parent to child)

- Every child (background agent, worker, sub-agent) is joined to its parent by one edge.
- Drawn with `canvas()`, below cards and above spaces. A 1 px line in the theme's
  `muted-foreground` at 50% opacity, a horizontal cubic curve (both control points at half the
  horizontal distance between the ends).
- Ends: from the middle of the parent's right side to the middle of the child's left side (if the
  child sits left of the parent, mirror: parent's left side to child's right side).
- **The parent's end has no dot. The child's end has a filled dot**, 6 px across, same color at
  100%, centered on the child's edge. That dot is the only direction marker: no arrows.
- An edge is drawn only when both ends are visible under the view switch.

#### 7.5.5 Culling

Hidden or offscreen cards and edges are not rendered (cull by the viewport rect plus a margin of
one card).

### 7.6 Spaces

- Drawn behind all cards. Fill: the space color at 8% opacity. Frame (if `frame: true`): 1 px
  border in the space color at 60%, 12 px corner radius. A label at the top-left (name, folder
  in muted text) that is also the drag handle. Resize from the bottom-right corner.
- Colors: a fixed palette of 8 named colors defined as theme tokens: `gray, red, orange, yellow,
  green, teal, blue, purple`. Pick tones that work in herdr's light and dark themes.
- Space context menu (right-click the label): Rename, Color, Show frame (toggle), Delete space
  (confirm; closes its terminals, same as herdr's space delete).

### 7.7 Right-click menu (use herdr's menu mod machinery)

On empty canvas or empty area inside a space:

- `New terminal` (cwd: the space's folder, or `~` at root)
- `New agent >` one item per **installed** agent CLI herdr knows (never list a missing one)
- `New space here` (root only): asks for a name and a folder (native folder picker via GPUI
  `prompt_for_paths`), random unused palette color, rect 960 x 640 at the click point
- `Open file...` (see phase 3)

The new card is placed with its top-left at the click point (clamped into the space).

### 7.8 View switch

- Segmented control, top right, over the canvas, not affected by zoom: `Agents` | `All` |
  `Terminals`. Style it like shadcn Tabs in herdr's neutral theme: muted track, the active
  segment raised. Lucide icons per segment (bot, layers, terminal) plus the label.
- **Agent** = a terminal whose program herdr's program table marks as an agent, or that reports
  an agent status. Everything else is a terminal. Preview cards show only in `All`.
- Filtered-out cards are not drawn and not hit-testable. Spaces always stay drawn.
- Default `All`. Saved in `canvas.json`. Commands `view.agents`, `view.all`, `view.terminals` in
  the palette, keys `Cmd+Shift+1/2/3` (Ctrl+Shift on Linux) in both keys files.
- If the focused card gets hidden by the filter, focus clears.

### 7.9 Phase 2 acceptance (Krishna can check each by hand)

Run `scripts/dev`, then:

1. Pan with two fingers, pinch to zoom; the point under the cursor stays put.
2. Right-click empty canvas, `New space here`, pick a folder: a colored space appears.
3. Right-click inside it, `New terminal`: `pwd` prints the space's folder.
4. Right-click outside: `New terminal`: `pwd` prints `~`.
5. `New agent > Claude Code` inside the space: a plain pill with the Claude icon, `claude` and a
   name; the status glyph spins while it works. No colored border anywhere.
6. Click the pill: the terminal opens in the side panel; type into it; resize the panel; close
   the panel and the agent keeps running.
7. Switch to `Agents`: only the agent pill shows. `Terminals`: only shells. `All`: everything.
8. From a terminal, start a child with `hc` (the exact command from 0.5): its pill appears to
   the right of the parent, joined by an edge with a dot at the child end only.
9. Settings > Look: change the canvas color; it changes live. Back to Default follows the theme.
10. Settings > Look > Cards: Windows. Pills become live terminal windows; move and resize one;
    zoom to 0.33 and they show as pills; zoom back: live text. Switch back to Pills.
11. Drag pills and spaces; quit and reopen: everything is where it was, terminals still running
    (core kept them).

Unit tests (pure Rust, no window): camera `to_screen`/`to_world` round-trip, zoom-around-cursor
keeps the anchor, zoom step snapping, card clamping into a space, space min-size, child placement
beside the parent, edge end selection (mirrored when the child is to the left), free-spot search,
view filter including hidden edge ends, `canvas.json` load of a corrupt file.

## 7A. Phase R: agent research (plans, quote, edit)

A separate, read-mostly research phase. It changes no app code. It can start as soon as phase 0
is done and run beside phases 1 and 2. **Phase 3 (8.3 and 8.4) does not start until it is done
and Krishna has read its summary.**

### Goal

Let a person see an agent's **plan** on the canvas, quote parts of it back, and **edit** it, with
the agent aware of the edit. Claude Code already supports opening its plan in an external editor
(VS Code) and picks up the changes; Canopy should be that editor. Claude Code and Codex first.

### Installed versions at plan time (re-check, they change)

`claude --version` gave `2.1.322 (Claude Code)`; `codex --version` gave `codex-cli 0.167.1`.
Old `main` has earlier notes: read them with `git show main:docs/harness-notes/claude.md` and
`git show main:docs/harness-notes/codex.md` (also `acp.md`). Re-verify every fact you reuse.

### Questions to answer, per CLI (Claude Code, then Codex)

1. **Plan existence**: does it have a plan mode or plan step? How is it entered and left?
2. **Plan storage**: is the plan written to a file? Exact path pattern, when it is written, when
   it is updated, when it is deleted. Is it markdown?
3. **External editor flow**: which key or command opens the plan (or the prompt) in an editor?
   Which env var or setting chooses the editor (`$VISUAL`, `$EDITOR`, a config key, VS Code
   detection)? Does it wait for the editor to exit (`code --wait` style) or watch the file?
4. **Change awareness**: after an edit, what exactly does the CLI do (re-read the file, show a
   diff, tell the model "the user edited the plan")? Does an edit by **another process** while
   no editor is open get noticed, or only edits through its editor flow?
5. **Other hooks into the plan**: hooks or events (for example a hook on leaving plan mode),
   flags that pass settings or hooks **per session** without touching the user's config files,
   session/transcript files that contain the plan.
6. **Sending text**: is a bracketed paste into the PTY handled as one paste (multi-line, no
   accidental submit)? Any length limits?
8. **Live status words** (Claude Code first): can Canopy read the spinner word the CLI shows
   while working ("Frolicking...", "Discombobulating..." and so on) reliably? From the terminal
   screen (herdr already reads screens for status), a status line hook, or a file. Recommend the
   least fragile source, or recommend falling back to the plain spinner glyph.
9. **Sub-agents and background agents**: how does each CLI run them (in-process, separate
   processes, separate PTYs), and what can Canopy see of each (process tree, transcript files,
   hooks, herdr-core's `parent`)? Do not assume they have no terminal of their own; find out.
   Recommend the best way to show them as child nodes and what clicking one opens. Fallback if
   nothing better exists: a child node that opens a read-only transcript in the side panel.
10. **Session name**: where each CLI keeps a human name or title for the session (and whether it
   changes over time), for the pill's name.
11. **What herdr already does** for these agents: read herdr-core's agent integration
   (`src/integration/assets`, `src/detect/manifests` in `herdrdev/herdr-core`) and note whether it
   installs hooks into the user's config. Canopy must not, unless Krishna approves.

### Method

- First without spending tokens: `--help` output, `/help`, settings schema, docs shipped with
  the CLI, the CLI's own files on disk, herdr-core source.
- Then, **only with Krishna's OK**, a short live session per CLI in a scratch folder, recorded
  with the plan editor flow exercised once. Use the `$EDITOR` shim experiment below.
- Every fact gets the CLI version and the evidence (command and output, or file and line).

### The `$EDITOR` shim experiment (the bold idea to prove or kill)

Hypothesis: if Canopy launches each agent with `EDITOR`/`VISUAL` set to a small `canopy edit`
command, then whenever the CLI opens a plan or prompt "in your editor", the file opens as a
Canopy preview card instead. `canopy edit <file>` blocks until the person clicks Done on the card
(or closes it), exactly like `code --wait`, so the CLI picks up the change through its own normal
path. This would work for **any** agent that honours `$EDITOR`, with zero vendor-specific code.

Prove it with a throwaway script (no app code): set `EDITOR` to a script that logs its argv and
the file contents, edits a line, and exits. Run the CLI's editor flow once per CLI. Record: was
the shim called, with which file, did the CLI see the edit, what did the model receive.

### Deliverables (in the fork)

- `docs/canopy/harness-notes/claude-code.md` and `docs/canopy/harness-notes/codex.md`: the
  answers above, each with version and evidence.
- Fixtures for anything Canopy will parse (a sample plan file, a sample transcript line) under
  `tests/fixtures/agents/`.
- `docs/canopy/agent-integration.md`, one page: the agent description fields both CLIs need
  (section 3.1), filled in for both, the chosen mechanism for plan view and plan edit, and what
  a third agent would need. Recommend one design; list rejected alternatives in one line each.
- A summary to Krishna: what works, what does not, the recommendation, open questions.

## 8. Phase 3: pop-out, previews, quote to agent

### 8.1 Pop-out

- `Pop out` (side panel header, pill right-click menu, or window-mode header) opens a new GPUI window (`cx.open_window`) titled with the terminal's title,
  showing that terminal full-window at zoom 1.0, using herdr's existing terminal element.
- The pill stays on the canvas with a small Lucide `external-link` glyph after its name; clicking
  it focuses the popped-out window. Its right-click menu (and the window-mode card body) offers
  `Bring back`. Closing the window
  or `Bring back` returns it. The terminal keeps running throughout (core owns it).
- Popped-out state is not persisted: on restart everything is back on the canvas.

### 8.2 Preview cards

- Open a file by: Cmd-click a file path in a terminal (herdr's `src/links.rs` already finds
  paths: hook into it), or `Open file...` in the right-click menu (native picker), or dragging a
  file from Finder onto the canvas.
- `.md` renders as markdown (first check whether herdr's vendored `gpui-kit` has a markdown
  component; use it if so, otherwise ask Krishna before adding a dependency). Other text files
  render as code with line numbers (syntax highlighting only if gpui-kit has it). Binary files:
  a toast "Can't preview <name>".
- The preview re-reads the file when it changes (herdr's `src/files.rs` already watches files
  with `notify`; reuse it, no timers).
- Untrusted content: never execute or load anything from the file (no remote images).

### 8.3 Quote to agent

- Select text in a preview. Right-click: `Send to agent >` lists agent cards (focused space first,
  then others, each with space name). Also `Copy as quote`.
- Sent text, written into the agent's terminal as one bracketed paste, **without pressing Enter**
  (the person reviews and sends):

  For markdown:
  ```
  @docs/plan.md:12-18
  > line 12
  > line 13
  ```
  For code, a fenced block tagged with the file extension:
  ````
  @src/main.rs:40-52
  ```rs
  ...selected lines...
  ```
  ````
  The path is relative to the agent's cwd when inside it, otherwise absolute. Line numbers are
  1-based and inclusive.
- After sending, focus moves to the agent's card and the camera centers it if offscreen.
- Mechanism: whatever 0.5 found for writing arbitrary text into a terminal. If only clipboard
  paste exists, set the clipboard, paste, then restore the old clipboard, and note it as a
  `shortcut:` comment.
- Phase R item 6 decides whether a bracketed paste is safe for each CLI; follow its note.

### 8.4 Plan cards (needs Phase R)

The exact mechanism comes from `docs/canopy/agent-integration.md`. The behaviour is fixed:

- When an agent has a current plan, the side panel header (and the window-mode header) shows a
  `Plan` button. It opens the plan in a preview card (the same component as 8.2) placed beside
  the agent's pill and joined to it by an edge (7.5.4). The card follows the plan file live.
- **Quote**: selecting text in the plan and `Send to agent` works as in 8.3, sending to the plan's
  own agent by default.
- **Edit**: an `Edit` toggle turns the preview into a plain text editor (gpui-kit's input /
  editor component if it has one; otherwise ask Krishna). `Save` writes the file.
- **Agent awareness**: if Phase R proves the `$EDITOR` shim, Canopy launches agents with
  `EDITOR`/`VISUAL` set to `canopy edit`, and any "open in editor" from the CLI lands in this card
  with a `Done` button that releases the waiting CLI. If the shim fails for a CLI, use the
  fallback Phase R recommends for it, and say plainly in the card when an edit will not be seen
  by the agent until a given step.
- Nothing here names a vendor in code outside the per-agent description (section 3.1).

Acceptance: for Claude Code and Codex each, open a plan, quote two lines back, edit one line,
and observe the agent acknowledge the edited plan (live run, with Krishna's OK).

## 9. License and credit (do in phase 1)

- Keep herdr-app's `LICENSE` (Apache-2.0) as the repo license and `THIRD-PARTY-NOTICES` as is.
- Add a `NOTICE` file: "Canopy is based on herdr-app (https://github.com/herdrdev/herdr-app),
  licensed under the Apache License 2.0. Modified by the Canopy authors." Keep existing copyright
  lines in source files.
- Files we change substantially get a one-line header: `// Modified for Canopy.`
- The names "Herdr" / "herdr-app" and their logos may only appear to credit the origin.
- herdr-core is not redistributed (users install it), so it adds no obligations. If we ever bundle
  it, the same rules apply.
- If any Templum code is copied (only `gesture.rs` ideas are expected), keep its MIT copyright and
  license text in a header of the copied file.

## 10. Later (not today, do not build without asking)

Minimap; marquee select and multi-drag; snapping guides; search across terminals; a floating
agents drawer; git worktree per space; SSH machines on the canvas (herdr supports them, so check
it still works); click-to-mark web previews sent to an agent; renaming internals from herdr to
canopy; replacing `main` with `pivot/canvas`; upstreaming the canvas node to herdr.

## 11. Verification checklist before telling Krishna a phase is done

- `just test` (or the fork's equivalent) and clippy pass.
- `ponytail-review` run on the phase's diff and its findings fixed; `ponytail-debt` output
  included in the summary.
- No vendor name in code outside the per-agent descriptions (section 3.1); check with
  `grep -rniE 'claude|codex' src/` and justify each hit.
- The phase's acceptance list was run by hand through `scripts/dev` and each item observed.
- `scripts/dev-stop` run; `pgrep -fl canopy` and the scratch core show nothing of ours left.
- Krishna's real herdr session untouched (his `hc` lists the same terminals as before).
- Say plainly what was not verified (for example Linux build, light theme, pop-out on Linux).

## 12. Order of work and parallelism

Phase 0 and phase 1 are sequential and done by one session, in this repo on `pivot/canvas`.
Parallel sessions later each work in their own git worktree branched from `pivot/canvas`.

Phase R (7A) runs as its own session right after phase 0, beside phases 1 and 2. It only writes
under `docs/canopy/`, `tests/fixtures/agents/` and its own scratch folder, so it never conflicts
with app work. 8.3 and 8.4 wait for it.

Replacing `main` happens only when Krishna asks. Two options to put to him then: merge
`pivot/canvas` into `main` with `--allow-unrelated-histories` after deleting the old tree (keeps
both histories), or rename the old `main` to `legacy` and make `pivot/canvas` the new `main`
(cleanest, keeps upstream merges simple). Recommend the second. In phase 2, 7.1 to 7.5 (model,
storage, layout node, camera, cards) are one piece of work that everything else needs. After it
compiles, these can run as separate sessions on separate files: spaces + right-click menu (7.6,
7.7), view switch (7.8), pop-out (8.1), previews + quote (8.2, 8.3).

## 13. Plan corrections log

Write here when the code disproves something above (date, what, new decision).

- 2026-11-12: no `upstream` remote (Krishna). `pivot/canvas` was branched from a one-off clone of herdr-app `master` at `4c8c622`; pull later fixes with `git pull https://github.com/herdrdev/herdr-app master` when wanted.
