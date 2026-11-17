# Canvas items: implementation plan

Written 2026-11-17. The plan for building
[canvas-items-proposal.md](canvas-items-proposal.md) (the design and the
verified stack). **Not started.** Nothing below runs until Krishna says go.

## 1. What we are building

Canopy becomes "Figma for your agents": one canvas where terminals and coding
agents sit next to PDFs, images and a live browser. You wire an item to an
agent so the agent can read it, and you quote a selection (a PDF line, a React
component) into any agent.

**Friday 2026-11-20 is post day.** Build days are Wednesday and Thursday.
Scope is locked: P1 to P3, macOS only.

| In for Friday | After the post |
|---|---|
| P1 item model, wiring, quoting, toolbar, note | P4 video |
| P2 image and PDF items | P5 iOS simulator (section 7), then Android (adb, scrcpy) |
| P3 browser item with React picker | Linux and Windows for browser and PDF polish |
| | P6 mod canvas kinds docs, web-view mods |

## 2. Decisions Krishna must make before go

1. **Grow the core.** AGENTS.md needs your go-ahead: the item list in
   `canvas.json`, `item.*` commands, `canopy item` CLI and the browser and
   PDF kinds are core. Toolbar and note stay mods.
2. **User-facing word for items.** Code says `item`. UI word: "item",
   "card", "frame" or other. Only UI strings change later.
3. **Toolbar keys.** Canvas mode already uses `h f a n c`. Proposal:
   no new single-letter keys for Friday; the toolbar is mouse-first and
   `n` keeps opening the new menu, which gains the item kinds.
4. **Two new dependencies** (AGENTS.md "every dependency is a cost"):
   `gpui-pdf` 0.7.0 (MIT, pulls `hayro`, `kurbo`) and `lb-wry` 0.53 via a
   vendored `gpui-webview` 0.7.1 (Apache-2.0). `react-grab` 0.2.0 (MIT) is
   bundled as a JS file, not a Rust dependency.
5. **Who builds.** Default: Codex `gpt-6.1-sol` medium in fresh worktrees,
   ponytail, one shared `CARGO_TARGET_DIR`; Claude orchestrates and reviews.

## 3. Order of work

```text
Wed AM   S1 spike gpui-pdf ─┐   S2 spike gpui-webview + react-grab ─┐   P1a item model (core)
Wed PM                      └──── P2 viewers ◀── P1a ──▶ P3 browser ┘   P1b commands, CLI, wire, quote
Thu AM   P1c toolbar mod + note mod, P2 and P3 finish
Thu PM   integration on main, review, docs, demo recording dry run
Fri      Krishna writes and posts
```

Spikes run first because they are the two things that can kill the scope.
P1a is independent of them, so it starts at the same time.

## 4. Steps

Each step is one branch and one PR that Krishna reviews, tests and merges
(section 9). "Verify" lists what the PR body must state as checkable
behaviour. "Done" means its checks pass and the docs listed travel with it.
Spikes S1 and S2 are throwaway branches with a written result, not PRs.

### S1 spike: gpui-pdf on our GPUI (2 h, throwaway branch)

- Add `gpui-pdf = { version = "0.7", features = ["search", "markup"] }`,
  open a PDF in an isolated dev window, drag-select, print the `Selection`.
- Pass: builds against `gpui-pre =0.3.8`, renders, selection returns text.
- Fail plan: `pdfium-render` 0.9.5 + bundled PDFium, P2 grows by a day;
  cut PDF highlights to plain page text read.

### S2 spike: browser view and picker (3 h, throwaway branch)

- Vendor upstream `gpui-kit` v0.7.1 `crates/webview` into
  `vendor/gpui-kit/crates/webview`. Load `http://localhost:<port>` of a tiny
  React 19 + Vite app.
- Check: placement follows the card rect; `WKWebView.takeSnapshot` gives a
  PNG; `react-grab`'s `index.global.js` as an initialization script returns
  component name and file:line on click.
- Fail plans: no snapshot → resting state shows URL and title only; no
  react-grab → DOM picker only (selector, text, box, screenshot).

### P1a item model (core)

- `src/canvas.rs`: `items: Vec<Item>` with `id, kind, rect, z, space, props`.
  Old `previews` load as `kind: "file"`; still read, never written.
- Undo, arrange, spaces and `canvas::solve` treat items like previews.
- `src/window/canvas_view.rs`: draw items by kind; `file` reuses
  `preview_card`.
- Tests: migration of a 0.1.0 `canvas.json`, round-trip, undo.
- Docs: `docs/canvas.md`.
- Verify: an existing canvas with text previews opens unchanged; previews
  move, resize, undo and arrange as before; `canvas.json` now has `items`.

### P1b agents and quoting (core)

- `src/wire.rs`: key `item:<id>`; `canopy wire` lists items with verbs.
- `src/control.rs`, `src/commands.rs`: `item.new | read | shot | open |
  close`; CLI `canopy item <verb>` as sugar over `canopy do item.<verb>`.
- `read` output wrapped in an untrusted-content envelope.
- Generalize `quote_selection` to any item's `selection()`; the `quote` mod
  lists wired agents first, then running agents, then "New agent with this".
- Mod API: `items()`, `itemKinds()`, `item.new`. Regenerate the `.d.ts`.
- Tests: wire key round-trip, `item read` envelope, quote text format.
- Docs: `docs/wire.md`, new `docs/items.md`, `docs/security.md`,
  `defaults/AGENTS.md`, `src/SKILL.md`.
- Verify: `canopy item new file ./README.md --wire` from an agent's
  terminal puts a card beside it; `canopy wire` in that agent lists `i1`;
  `canopy item read i1` prints the text inside the untrusted envelope;
  quoting a text selection lists the wired agent first and pastes without
  Enter.

### P1c toolbar and note (mods)

- `defaults/mods/toolbar`: floating bottom toolbar: select, hand, terminal,
  agent (installed CLIs via `knownAgents()`), browser, open file, note, wire.
- `mod.json` `"place": "canvas"`; `defaults/mods/note` as the first mod kind.
- Docs: `docs/mods.md`.
- Verify: the toolbar floats bottom centre at any zoom; its agent button
  lists only installed CLIs; each button spawns its kind at the viewport
  centre; a note can be typed in, wired and read by an agent.

### P2 viewers (built-in kinds)

- `image`: GPUI `img()`; drag a region, crop with `image`, quote
  `@mock.png region x,y wxh <crop path>`.
- `pdf`: wrap `gpui_pdf::PdfView`; `read` = page text; `selection()` from
  its `Selection`; highlights stored in item props as `{quote, occurrence,
  page}`, never written into the PDF.
- Open file picks the kind by extension.
- Verify: open a PNG and a PDF from the toolbar; drag a region on the image
  and quote it; select a PDF line and quote it, the agent gets page and
  text; highlights survive an app restart; the PDF file is byte-identical
  afterwards; `canopy item read` returns page text.

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
  gets file and line range; a non-React page quotes selector and text; no
  network request other than the page's own (check with the proxy log or
  Little Snitch).

### Integration (Thu PM)

- Merge order: P1a, P1b, P1c, then P2 and P3, each merged by Krishna.
- `ponytail-audit` after P1, `ponytail-debt` report at the end.
- Demo dry run in the isolated dev window (`scripts/dev`), then
  `scripts/dev-stop`.

## 5. Cut lines if behind

In this order: toolbar keys, PDF highlights (keep selection quote), react-grab
(keep DOM picker), browser live state (keep snapshot plus `item open`),
image region quote.

## 6. What is not in this plan

Video (P4), simulator (P5), Linux and Windows browser, MCP server for items,
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

1. **Resting item, public APIs only:** `xcrun simctl io <udid> screenshot`
   on demand and after each agent action. No polling.
2. **Live item, private APIs, same path as Claude:** a small Swift helper on
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

| Item | Sends | Never sends |
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
  (UNVERIFIED: S2 measures it).
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
You orchestrate the canvas items build in /Users/krishna/projects/canopy.
Read AGENTS.md, AGENTS.local.md, docs/canopy/herdr-map.md,
docs/canopy/canvas-items-proposal.md and docs/canopy/canvas-items-plan.md
first. The plan is the contract; section 2 decisions are recorded at its top.

Work in phases, one PR per step in plan section 4, in its order. Run S1 and
S2 first as throwaway spikes and report their results before any PR that
depends on them.

For each step:
1. Delegate the build to a fresh Codex gpt-6.1-sol medium session in its
   own git worktree and branch (type/short-name), with the ponytail
   SKILL.md path and the shared CARGO_TARGET_DIR. Never reuse a thread
   after the plan changes.
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

