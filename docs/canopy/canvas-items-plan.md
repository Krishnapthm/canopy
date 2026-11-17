# Canvas panes: implementation plan

Written 2026-11-17. The plan for building
[canvas-items-proposal.md](canvas-items-proposal.md) (the design and the
verified stack). **Go given 2026-11-17**; decisions 1, 2 and 4 are approved.

## 1. What we are building

Canopy becomes "Figma for your agents": one canvas where terminals and coding
agents sit next to PDFs, images and a live browser. You wire a pane to an
agent so the agent can read it, and you quote a selection (a PDF line, a React
component) into any agent.

**Friday 2026-11-20 is post day.** Build days are Wednesday and Thursday.
Scope is locked: P1 to P3, macOS only.

| In for Friday | After the post |
|---|---|
| P1 pane model, wiring, quoting, toolbar, note | P4 video |
| P2 image and PDF panes | P5 iOS simulator (section 7), then Android (adb, scrcpy) |
| P3 browser pane with React picker | Linux and Windows for browser and PDF polish |
| | P6 mod canvas kinds docs, web-view mods |

## 2. Decisions

1. **Grow the core.** Approved 2026-11-17: the pane list in
   `canvas.json`, `pane.*` commands, `canopy pane` CLI and the browser and
   PDF kinds are core. Toolbar and note stay mods.
2. ~~User-facing word~~ Decided: **pane**, everywhere (AGENTS.md).
3. **Shortcut hints everywhere.** Decided by Krishna 2026-11-17 (replaces
   "no toolbar keys"): every button on the toolbar and on every helper bar
   has a key, and its tooltip shows name and key (`Hand  H`, `Open file  ⌘O`).
   Holding Cmd (Ctrl on Linux and Windows) shows the keys on all of these
   surfaces at once, after a short hold so Cmd shortcuts do not flash them.
   The toolbar's Agent button opens its menu above the toolbar, Claude style:
   each installed agent with a muted number on the right (1 to 9), and the
   number starts that agent while the menu is open. Toolbar keys are listed
   in P1c.
4. **Two new dependencies.** Approved 2026-11-17:
   `gpui-pdf` 0.7.0 (MIT, pulls `hayro`, `kurbo`) and `lb-wry` 0.53 via a
   vendored `gpui-webview` 0.7.1 (Apache-2.0). `react-grab` 0.2.0 (MIT) is
   bundled as a JS file, not a Rust dependency.
5. **Who builds.** Default: Codex `gpt-6.1-sol` medium in fresh worktrees,
   ponytail, one shared `CARGO_TARGET_DIR`; Claude orchestrates and reviews.

## 3. Order of work

```text
Wed AM   P1a pane model (core)
Wed PM   P1b commands, CLI, wire, quote   P2 viewers ◀── P1a ──▶ P3 browser
Thu AM   P1c toolbar mod + note mod, P1d teach agents, P2 and P3 finish
Thu PM   integration on main, review, docs, demo recording dry run
Fri      Krishna writes and posts
```

No spikes: the research below settled the stack, and the first real build
of P2 and P3 surfaces anything left (fail plans in section 4).

## 4. Steps

Each step is one branch and one PR that Krishna reviews, tests and merges
(section 9). "Verify" lists what the PR body must state as checkable
behaviour. "Done" means its checks pass and the docs listed travel with it.
### Research findings (2026-11-17, from source, nothing built)

- **gpui-pdf 0.7.0** (MIT): `cargo fetch` resolves it onto our patched
  `gpui-pre 0.3.8` (it asks `^0.3`; its own lockfile used 0.3.9, so
  compiling is still unverified). It pulls `hayro` 0.8, not 0.9. Selection:
  `toggle_select_mode` plus `set_on_create_highlight(page, quote,
  occurrence, color)`; page text: `extract_page_text(doc, i).text()`;
  highlights: `set_highlights(Vec<Highlight {id, page, quote, occurrence,
  color}>)`. `PdfView` draws its own header (filename, page counter, zoom,
  find) that only `PdfStyle` colours can change; P2 hides or replaces it so
  the shared helper bar is the only bar. Fail plan: `pdfium-render` 0.9.5.
- **gpui-kit v0.7.1 `crates/webview`** is not on crates.io (only 0.10.0 is);
  vendor it from the upstream tag `v0.7.1`. Its package is named
  `gpui-wry` (one `lib.rs`, Apache-2.0, `lb-wry` 0.53.3).
- **react-grab 0.2.0** (MIT): `dist/index.global.js` is an IIFE that
  auto-inits; it installs its own React DevTools hook, so inject it at
  document start. Take picks without the clipboard with
  `registerPlugin({name, hooks: {onElementSelect(el) {...; return true}}})`
  and `__REACT_GRAB__.getSource(el)` → `{filePath, lineNumber,
  columnNumber, componentName}`. Source comes from React 19 `_debugStack`
  plus source maps it fetches from the dev server (the page's own origin).
  **Privacy:** it fetches `https://www.react-grab.com/api/version` unless
  `telemetry: false`, and its shadow-DOM CSS `@import`s Google Fonts; P3
  must set `telemetry: false` and block or override the font import. WebKit
  stack parsing exists but is unverified in WKWebView.
- Unverified until P3 builds: `WKWebView.takeSnapshot` through wry's macOS
  extension, and placement under zoom. Fail plans: no snapshot → resting
  state shows URL and title only; no react-grab → DOM picker only.

### Shortcut hints (every step)

Decision 3 applies to every surface a step adds: each helper-bar or toolbar
button has a key, its tooltip shows name and key, held Cmd (Ctrl elsewhere)
shows all keys on screen, and any menu of choices numbers them 1 to 9 in
muted text with the number as the key. The shared helper-bar component draws
tooltips and the held-Cmd hints from each action's `shortcut`, so a pane gets
them by declaring keys, never by drawing its own. Each step's Verify includes
hovering every new button and holding Cmd.

Follow-up, its own PR (Krishna, 2026-11-17): the black bar along the bottom
that lists canvas keys while Option is held (`defaults/mods/canvas-keys`)
goes. Its replacement follows the new design language: a floating,
elevated card (shadow, rounded, theme surface colours), shown by the same
held-Cmd hint, never a full-width plain bar.

### P1a pane model (core)

- `src/canvas.rs`: `panes: Vec<Pane>` with `id, kind, rect, z, space, props`.
  Terminals and sub-agent cells become panes too (Krishna, 2026-11-17):
  old `cards` load as `kind: "terminal"` or `"sub"`, old `previews` as
  `kind: "file"`; both still read, never written. Pane ids keep the old
  card keys (`t3`, `sub:<parent>:<id>`), so saved wires stay valid; file
  panes get `p1`, `p2`, ...
- Undo, arrange, spaces and `canvas::solve` treat panes like previews.
- `src/window/canvas_view.rs`: draw panes by kind; `file` reuses
  `preview_card`.
- Helper bar: one shared GPUI component that draws a pane's `actions` as a
  floating pill of icon buttons, reusing gpui-kit's `tooltip.rs` for name
  and shortcut. Shows on hover or selection, sits outside the content,
  scales with zoom. Terminal and file panes get their bars here.
- Tests: migration of a 0.1.0 `canvas.json`, round-trip, undo.
- Docs: `docs/canvas.md`, new `docs/panes.md` (helper bar section).
- Verify: an existing canvas with text previews opens unchanged; previews
  move, resize, undo and arrange as before; `canvas.json` now has `panes`;
  hovering a terminal or file pane shows its helper bar with tooltips.

### P1b agents and quoting (core)

- `src/wire.rs`: key `pane:<id>`; `canopy wire` lists panes with verbs.
- `src/control.rs`, `src/commands.rs`: `pane.new | read | shot | open |
  close`; CLI `canopy pane <verb>` as sugar over `canopy do pane.<verb>`.
- `read` output wrapped in an untrusted-content envelope.
- Generalize `quote_selection` to any pane's `selection()`; the `quote` mod
  lists wired agents first, then running agents, then "New agent with this".
- Mod API: `panes()`, `paneKinds()`, `pane.new`. Regenerate the `.d.ts`.
- Tests: wire key round-trip, `pane read` envelope, quote text format.
- Docs: `docs/wire.md`, new `docs/panes.md`, `docs/security.md`,
  `defaults/AGENTS.md`, `src/SKILL.md`.
- Verify: `canopy pane new file ./README.md --wire` from an agent's
  terminal puts a card beside it; `canopy wire` in that agent lists `p1`;
  `canopy pane read p1` prints the text inside the untrusted envelope;
  quoting a text selection lists the wired agent first and pastes without
  Enter.

### P1d teach agents Canopy (core, rules in AGENTS.md "How agents learn Canopy")

Today only Claude Code and Codex learn Canopy, through a launch note in
`defaults/programs.json` (`--append-system-prompt`, `developer_instructions`)
and Claude's `canopy hook wire`. Other installed CLIs get nothing.

- The skill is the main path. `src/SKILL.md` gains `name: canopy` and a
  `description` frontmatter (today it has none, so no harness would
  discover it) that says when to load it, plus panes: list, read, new,
  close, quote format, untrusted content.
- Skill install and update, automatic: write it into each installed
  harness's global skills folder (paths verified per CLI version, written
  to `docs/canopy/harness-notes/`). Only our own folder. Reinstall when the
  app version changes.
- Launch note and hook: name the `canopy` skill instead of "`canopy --skill`
  explains more"; the hook reports wired panes too. `canopy --skill` stays
  as the fallback for harnesses without skills.
- Project setup, with permission: a small script reads the project root.
  If `CLAUDE.md` has an `@AGENTS.md` import line, the target is
  `AGENTS.md`; else whichever of the two exists; else offer to create
  `AGENTS.md`. Canopy shows the block and the target file and appends only
  on "Yes". Marked block, idempotent, removable.
- Global files: a "Copy instructions" button with the same block. Canopy
  never writes the person's global `AGENTS.md` or `CLAUDE.md`.
- Tests: target picking on fixtures (import present, only `CLAUDE.md`,
  only `AGENTS.md`, neither, block already present), idempotent append.
- Verify: on a fresh isolated home, the skill appears in each installed
  harness's skills folder and each harness lists it (for example Claude
  Code's skill list) and loads it unprompted when asked to open a pane; opening a project shows the permission prompt
  naming the exact file; "Yes" appends once, a second run changes nothing;
  "No" leaves files byte-identical; the copy button puts the block on the
  clipboard; a non-Claude agent (for example OpenCode) can find and use
  `canopy pane read` after setup.

### P1c toolbar and note (mods)

- `defaults/mods/toolbar`: floating bottom toolbar: select, hand, terminal,
  agent (installed CLIs via `knownAgents()`), browser, open file, note, wire.
- `mod.json` `"place": "canvas"`; `defaults/mods/note` as the first mod kind.
- Community pane API: a mod pane declares `actions` (icon, label,
  shortcut, command) and the optional `read`, `shot`, `selection`, `verbs`;
  the app draws its helper bar. The bottom toolbar is the canvas's own
  helper bar, built on the same component.
- Docs: `docs/mods.md`.
- Verify: the toolbar floats bottom centre at any zoom; its agent button
  lists only installed CLIs; each button spawns its kind at the viewport
  centre; a note can be typed in, wired and read by an agent.
- Shortcut hints (decision 3), added 2026-11-17. Proposed keys, all free or
  already bound to the same action in canvas mode: Select `V`, Hand
  `Space` (hold; `H` stays `canvas.go:left`), Terminal `N`, Agent `⇧N`,
  Open file `⌘O`, Note `T`, Connect `C`. Agent opens its menu above the
  toolbar with muted numbers; `1` to `9` start an agent. Verify: hover each
  toolbar and helper-bar button and see its key; hold Cmd and see every
  key; `⇧N` then `2` starts the second installed agent.

### P2 viewers (built-in kinds)

- `image`: GPUI `img()`; drag a region, crop with `image`, quote
  `@mock.png region x,y wxh <crop path>`.
- `pdf`: wrap `gpui_pdf::PdfView`; `read` = page text; `selection()` from
  its `Selection`; highlights stored in pane props as `{quote, occurrence,
  page}`, never written into the PDF.
- Open file picks the kind by extension.
- Verify: open a PNG and a PDF from the toolbar; drag a region on the image
  and quote it; select a PDF line and quote it, the agent gets page and
  text; highlights survive an app restart; the PDF file is byte-identical
  afterwards; `canopy pane read` returns page text; both panes show their
  helper bars (image: crop, quote; PDF: search, highlight, page); every
  bar button's tooltip shows its key, and holding Cmd shows them all.

### P3 browser (built-in kind, macOS)

- `browser` kind on the vendored web view. Resting: last snapshot texture.
  Live: focused and near 100% zoom; GPUI menus and the quote chip only show
  while resting.
- `read` = URL, title, visible text; `open <url>`; `shot`.
- Picker: react-grab when React is found, DOM picker otherwise. The quote
  is a reference, not code (section 8):
  `@<url> <PricingCard> src/PricingCard.tsx:42-61 "<div class=card>"`.
- Privacy: no homepage, no app-owned requests; react-grab bundled, never
  fetched from unpkg.
- Verify: open a localhost React app; pan and zoom with it resting; click
  in to go live and use the page; pick a component and quote it, the agent
  gets file and line range; a non-React page quotes selector and text; the
  helper bar offers home, reload, screenshot, pick and open externally,
  each tooltip with its key and all keys shown while Cmd is held; no
  network request other than the page's own (check with the proxy log or
  Little Snitch).

### Integration (Thu PM)

- Merge order: P1a, P1b, P1c, P1d, then P2 and P3, each merged by Krishna.
- `ponytail-audit` after P1, `ponytail-debt` report at the end.
- Demo dry run in the isolated dev window (`scripts/dev`), then
  `scripts/dev-stop`.

## 5. Cut lines if behind

In this order: toolbar keys, PDF highlights (keep selection quote), react-grab
(keep DOM picker), browser live state (keep snapshot plus `pane open`),
image region quote.

## 6. What is not in this plan

Video (P4), simulator (P5), Linux and Windows browser, MCP server for panes,
JSON Canvas import/export, web-view mods.

## 7. iOS simulator (after the post)

Checked 2026-11-17 on this Mac (Xcode 27.2, Claude desktop app) plus web
research.

### How the Claude desktop app does it

Read from the installed bundle, not from docs (Anthropic does not document
the mechanism):

- A background helper, `Claude.app/Contents/Helpers/Claude iOS Sim.app`
  (`claude-ios-sim`, `LSBackgroundOnly`), ships `LICENSE-facebook-idb.txt`
  and contains `FBSimulatorControl` symbols: it is built on idb (MIT).
- It weak-links the private `CoreSimulator` and loads `SimulatorKit`.
  Frames come from **framebuffer IOSurface callbacks**, so no Screen
  Recording permission and no Simulator window are needed.
- Input goes through SimulatorKit's Indigo HID, with a fallback for
  "CoreSimulator-1155.4 (Xcode 27) and later" (a string in the binary):
  they already had to patch around an Xcode 27 break.
- Accessibility tree via the private `AXPTranslator`; a second binary,
  `SimulatorFrameworkBridge-iOS`, is built for the iOS Simulator platform.
- Frames are H.264/HEVC encoded with VideoToolbox, because an Electron
  renderer needs a video stream.
- Confirmed live on macOS 27.2.1: the running helper is launched through
  Claude's `disclaimer` wrapper under `sandbox-exec` with a Seatbelt
  profile whose comment says it "hosts Meta's FBSimulatorControl, which
  dlopens Apple's private CoreSimulator framework". The profile blocks file
  reads and sockets, and the wrapper keeps it out of the app's Screen
  Recording and Accessibility grants. Our helper should do the same.

### Can we pull Device Hub into the canvas?

No. Xcode 27 replaced Simulator.app with `Xcode.app/Contents/Applications/
DeviceHub.app` (no `Simulator.app` remains in Xcode 27.2). It links private
`DeviceKit` and `CoreDevice`; its only outside hook is a `devices:` URL
scheme that opens it. Apple documents no API, view, XPC service or stream to
embed it, for simulators or for real devices. `simctl io` offers
`screenshot` and `recordVideo` to a file or URL, not a live view.

### What Apple did give us: Xcode MCP

Xcode 27 ships `xcrun mcpbridge` and `mcp-server`. Per the Xcode 27 release
notes, its MCP tools boot simulators, install and launch apps, synthesize
touches and take screenshots. That is the agent side, for free, through each
agent's own MCP config. Headless mode needs `sudo` to enable. It does not
give Canopy a view.

### Plan for P5

1. **Resting pane, public APIs only:** `xcrun simctl io <udid> screenshot`
   on demand and after each agent action. No polling.
2. **Live pane, private APIs, same path as Claude:** a small Swift helper on
   idb's `FBSimulatorControl` (MIT) hands the framebuffer IOSurface to
   Canopy over a Mach port. Canopy wraps it in a `CVPixelBuffer` and draws it
   with GPUI `surface()`: zero copy, no video encoding, zooms like any card.
   This is simpler than Claude's path, which must encode for Electron.
   Input: Indigo HID through the same helper, or AXe (MIT, bundles idb).
3. **Agents:** document Xcode MCP in `defaults/AGENTS.md` for agents that
   want to drive the simulator themselves; Canopy shows the result.
4. Real iOS devices: no public path. Out of scope.
5. Android follows iOS: `adb` for screenshots and input, scrcpy
   (Apache-2.0) for a live stream, emulators and devices alike. Not
   researched in depth yet.

Risk: private frameworks break with Xcode releases (Claude's helper already
carries an Xcode 27 workaround). Prior art to read first: idb
`FBSimulatorControl`, AXe, baguette (README says it links SimulatorKit),
Radon IDE (embeds a simulator in VS Code via a `simulator-server` helper).

## 8. Context engineering: what a quote sends

Rule: **a quote is a pointer, not a copy.** The agent gets enough to find
the thing and decide whether to read it, never the content itself.

| Pane | Sends | Never sends |
|---|---|---|
| Browser, React | URL, component name, `file:start-end`, the opening tag (cut at 80 chars) | component source, full `outerHTML` |
| Browser, other | URL, CSS selector, visible text (cut at 200 chars) | page HTML |
| PDF | file, page, selected text | page text |
| Image | file, region, crop PNG path | pixels |
| Any | screenshot as a file path, on request | inline image |

### Why references beat inlined code

- **Agents re-read anyway.** Claude Code's Edit tool refuses to edit a
  file that was not Read in the session (observed in this harness), so
  inlined code is a duplicate the moment the agent edits. Codex's behaviour
  is UNVERIFIED; check against the installed CLI per AGENTS.md.
- **Prompt caching does not save context.** Caching cuts the price and
  latency of a repeated prefix, but every pasted copy still occupies the
  context window and brings compaction sooner. So "it hits the cache" does
  not make duplication free.
- **Size.** A reference is about 30 tokens; a 40-line component is about
  400 to 600 tokens, more with its `outerHTML`. react-grab's default copy
  includes the element HTML, which for a container can be thousands.
- **Freshness.** Inlined code goes stale the moment the agent edits; a
  path and range point at the live file.

### Resolving the line range

react-grab gives `file:line:col` of the element (React 19 `_debugStack`
mapped to source). Canopy turns that into `start-end` itself, in Rust, when
the user clicks:

- Read the one file (skip if over 512 KB), scan from `line:col` to the
  matching closing tag, aware of strings, comments and `{}` expressions.
- Cost: one file read and a linear scan, once per click. No cache, no
  memory kept, nothing while idle; the OS page cache already makes repeat
  reads cheap. Expected well under a millisecond for normal files
  (UNVERIFIED: P3 measures it).
- Fallback: if the scan fails or runs past 400 lines, send `file:line`
  only. Never block the quote on resolution.
- No new dependency: a hand scanner with fixture tests (self-closing tags,
  fragments, generics like `<T,>`, `>` inside expressions). If it is wrong
  too often, switch to `oxc_parser` (MIT); that is a later decision.
- The opening-tag anchor lets the agent re-find the element if lines
  shifted since the page last reloaded.

## 9. Orchestrator prompt

Paste this into the implementation session once Krishna says go.

```text
You orchestrate the canvas panes build in /Users/krishna/projects/canopy.
Read AGENTS.md, AGENTS.local.md, docs/canopy/herdr-map.md,
docs/canopy/canvas-items-proposal.md and docs/canopy/canvas-items-plan.md
first. The plan is the contract. No spikes: start with P1a. Section 4's
research findings feed P2 and P3. Before a step that needs an open section 2
decision (core growth for P1a, new dependencies for P2 and P3), ask Krishna.

Work in phases, one PR per step in plan section 4, in its order.

Disk is tight (check df first; stop and ask under 20G free):
- One shared target: CARGO_TARGET_DIR=/Users/krishna/projects/canopy/target,
  with CARGO_INCREMENTAL=0. Never a target/ in a worktree.
- Builders build and test only what their step touches. The full
  cargo test --locked runs once per step, at review.
- Prune each worktree as soon as its PR merges.

For each step:
1. Delegate the build to a fresh Codex gpt-6.1-sol medium session in its
   own git worktree under .claude/worktrees/ and branch (type/short-name).
   P1a builds on feat/pane-model, which holds this plan and
   the new AGENTS.md rules (main does not yet), so they land on main with
   P1a's PR; open no separate docs PR. Later steps branch from main. Run the companion from the main checkout so Codex's
   sandbox can write the shared target; the brief names the worktree as the
   only place to edit. Give it the ponytail SKILL.md path. Anything it must
   research it researches thoroughly from primary sources and marks
   UNVERIFIED otherwise. Never reuse a thread after the plan changes.
2. When it reports done: read the diff, run cargo test --locked (and
   just test when mods changed), run ponytail-review, fix findings, and
   check the docs listed for the step.
3. Push the branch and open the PR. The PR body must contain:
   - What changed, in plain words, and why.
   - A feature checklist: each feature, its expected behaviour, and the
     exact steps to see it (commands, clicks), copied from the step's
     "Verify" list and extended with anything new.
   - What was not verified, and ponytail-debt for the step.
   - Isolated launch commands for the app (scripts/dev, scripts/dev-stop).
4. Check the PR branch out locally for Krishna so it is ready to run: a
   review worktree (or a git switch in the main checkout when it is
   clean), built with the shared CARGO_TARGET_DIR. Do not launch the app
   and leave it running; give Krishna the commands.
5. Tell Krishna: PR link, the checklist, and that it waits for him to test,
   run a code review and merge. Do not merge PRs yourself. Do not start a
   step that depends on an unmerged PR; independent steps may proceed in
   parallel worktrees.
6. After Krishna merges: pull main, prune the merged worktree, continue.

Rules: no em dashes, commits as type(scope): description with no agent
attribution, never touch the daily herdr app or its data, never run paid
agent sessions without asking, every user-visible error is a toast, idle
costs nothing. Stop and ask Krishna when a step would change the plan.
```
