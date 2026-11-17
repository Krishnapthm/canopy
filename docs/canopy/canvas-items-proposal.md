# Canvas items: proposal for stack review and research

Written 2026-11-17. A proposal, not a decision. Nothing here is built.
Visuals: [mockups/canvas-items.html](mockups/canvas-items.html), also
published privately at https://claude.ai/artifact/FpJQpEjDHMAS57aBe1e4QV
(fig 1 tabletop, fig 2 quote flows, fig 3 agent view and mods menu, fig 4
resting vs live, fig 5 architecture, fig 6 merges and phases, fig 7 the
React question).

The next session's job is web research: confirm or replace each candidate in
section 6, answer the questions in section 10, and decide the React question
in section 7.

## 1. Status this proposal assumes

- `main` at `bcb3def` (0.1.0 released).
- PR #9 `chore/rebrand-leftovers`: CI green, mergeable. Pending: Krishna's
  README rewording, and the CC-BY-SA win98 font decision.
- PR #10 `refactor/wire-lean-hc-adapter`: CI green, mergeable. Against #9 it
  conflicts in `README.md` only (checked with `git merge-tree`).
- Reddit post deadline: Friday 2026-11-20. What ships by then is decided
  after research (section 9).

## 2. Goal

Canopy is a tabletop: one infinitely pannable canvas where you put anything
next to your agents and terminals. Like Figma, but the frames are agents,
browsers, PDFs, images, videos, simulators and anything a mod adds.

1. **Spawn anything** from a Figma-style floating bottom toolbar.
2. **Wire anything** to anything. An agent wired to an item knows it is
   there and can read it.
3. **Quote anything** into any agent: a PDF highlight, a browser element or
   React component, an image region, a video frame, a simulator screen.
4. **Extend anything**: users and agents make new canvas tools as mods.
5. **Stay light**: idle costs nothing, the core stays small.

## 3. What exists today (facts)

- **Cards and previews.** `canvas.json` (`src/canvas.rs` `Document`) holds
  `cards` (terminals and sub-agents) and `previews` (text files, drawn by `preview_card` in
  `src/window/canvas_view.rs`).
  Previews are the seed of a generic item: rect, space, path.
- **Wire is key-based.** `Wire {a, b, born}` joins two strings
  (`Card::key()`: a terminal address `tN` or `sub:<parent>:<id>`). Nothing in
  `src/wire.rs` cares what a key points to.
- **Agents learn peers** through `canopy wire` (`src/wire.rs` `run`) and the
  `canopy hook wire` session hook (`told_line`), which says
  `Wired: ... canopy wire lists your peers.` only when the list changes.
- **Quoting exists for text previews.** `quote_selection` builds a quote,
  shows a chip, the `quote` mod lists running agents, `quote.send` pastes
  into the terminal without Enter.
- **Control socket.** `canopy ls | status | shot [file.png] | do <command>`
  (`src/control.rs`). `canopy do` already runs any app command with JSON args.
- **Rendering.** GPUI `img()` decodes png, jpeg, webp, gif, svg, bmp, tiff,
  ico, pnm. GPUI `surface()` draws a macOS `CVPixelBuffer` inside the scene
  (`vendor/gpui-pre/src/elements/surface.rs`), so video frames can zoom and
  clip like any element. `image` and `resvg` crates are already in
  `Cargo.lock`. gpui-kit has no web view ("It is not an Electron or a
  Tauri"); its `a11y.rs` only names `Video` and `WebView` as roles.
- **Mods.** gpui-kit JS, placed by `mod.json` `place` (`right`, `left`, `top`,
  `bottom`, `none`), floats via layouts. No canvas placement yet.
  `defaults/AGENTS.md` says mods cannot use HTML or a web view.
- **New menu.** `defaults/mods/new-menu` already lists installed agents via
  `knownAgents()`; the toolbar's agent picker reuses it.
- **Canvas-mode keys** already take `h j k l f a x c n` and more
  (`defaults/keys.macos.json`, section `canvas-mode`). Figma-style tool keys
  (V, H, T, B, F, N) collide with some of these.

## 4. Proposed design

### 4.1 Item model

One new list in `canvas.json`. Terminals stay `cards` (the core owns them);
everything else is an item.

```json
"items": [
  { "id": "i4", "kind": "browser", "rect": [640, 80, 900, 600], "z": 12,
    "space": "s1", "props": { "url": "http://localhost:3000/pricing" } },
  { "id": "i2", "kind": "pdf", "rect": [80, 700, 520, 680], "z": 9,
    "props": { "path": "/repo/spec.pdf", "page": 4 } },
  { "id": "i7", "kind": "mod:ci-runs", "rect": [...], "z": 3, "props": {} }
]
```

- `kind` is a built-in name (`file`, `image`, `pdf`, `video`, `browser`,
  `sim`, `note`) or `mod:<name>`.
- Wire key for an item: `item:<id>`, so it can never collide with `tN` or
  `sub:` keys. Agents see the short id (`i4`).
- `previews` migrate to `kind: "file"` on load; the old field is still read.
- Layout (`canvas::solve`, arrange, undo, spaces) treats items like previews
  do today: rectangles with keys.

### 4.2 Item contract

Every kind, built-in (Rust) or mod (JS), implements the same small set.
Only `render` is required.

| Function | Who uses it | Returns |
|---|---|---|
| `render(rect, zoom, live)` | canvas | the element |
| `read(args)` | agents, `canopy item read` | text: page text, URL and DOM outline, image size and path |
| `shot()` | agents, quotes | a PNG path |
| `selection()` | quote chip | `{ref, text, files?}` |
| `verbs` | `canopy wire`, `canopy item <verb>` | kind commands: `open`, `page`, `tap`, `seek` |
| `wake()` / `rest()` | canvas | go live; return a snapshot and stop |

This is the whole "protocol". It is an in-process interface, not a wire
format. Wire edges and `hc` stay as they are.

### 4.3 Agents and items

- `canopy wire` lists wired items next to agents, with their verbs
  (mockup 3).
- The session hook already reports peer changes; items appear in it.
- Agent commands, one family to keep the top level clean:
  `canopy item read i2 [--page 4]`, `canopy item shot i4`,
  `canopy item open i4 <url>`, `canopy item tap i6 <x> <y>`.
  Each is sugar over `canopy do item.<verb> {...}`.
- Agents can spawn items too: `canopy item new pdf ./spec.pdf --wire` puts
  a viewer on the canvas wired to the caller. This is the agent story for
  the post: an agent finishes a change and opens the result beside itself.

### 4.4 Quoting

Generalize today's flow. The chip appears for any item whose `selection()`
returns something. The popover lists agents wired to that item first, then
other running agents, then "New agent with this". The agent gets a
reference it can open, never a dump of the content:

```text
@spec.pdf p.4 "The recommended tier is centered, raised by 8px..."
@http://localhost:3000/pricing <PricingCard tier="pro"> src/components/PricingCard.tsx:42  main > section .card:nth-child(2)
@mock.png region 120,40 300x200 /tmp/canopy/i5-crop.png
@demo.mp4 00:42 /tmp/canopy/i9-0042.png
@iPhone 17 "Buy" button at 180,620 /tmp/canopy/i6-screen.png
```

### 4.5 Bottom toolbar

Floating, centered, Figma-like (mockup 1): select, hand, terminal, agent
(installed CLIs only), browser, open file (picks the viewer by type), simulator
(macOS, only when Xcode is present), note, wire, mods menu.

- It is a shipped mod (`defaults/mods/toolbar`), not core. The app provides
  `items()`, `itemKinds()` and `item.new {kind, props}`.
- Keys: decide in review. Canvas mode already uses `h`, `f`, `a`, `n`, `c`.
  Options: reuse `n` for a tool picker, add tool keys only outside canvas
  mode, or rebind canvas mode. Any default key change needs the old
  `keys.json` kept in `src/keys/shipped/`.

### 4.6 Resting and live

A native view (web view, PDFKit, AVPlayerLayer) cannot zoom, clip or sit
under GPUI overlays. So heavy items have two states (mockup 5):

- **Resting**: a snapshot texture in the GPUI scene. Zooms and layers like
  any card. No process ticking, no frames requested.
- **Live**: focused and near 100% zoom. The native view sits exactly over
  the card. Blur or zoom out: snapshot and step aside.

Terminal cells already work this way (pill vs expanded). Kinds that render
into GPUI directly (image, PDF via a rasterizer, video via `surface()`)
need no live state at all.

### 4.7 Mods as canvas kinds

`mod.json` gains `"place": "canvas"` and an optional
`"tool": {"icon", "label"}` for the mods menu. The mod's class may define
`read`, `shot`, `selection` and `verbs` as in 4.2. The app hands it props and
rect, and keeps it alive only while shown (idle rule).

Mods need two new host APIs to draw arbitrary content: an image from a file
path or bytes (today `image()` reads only under the app home), and possibly
a frame surface for streams.

### 4.8 Core vs mods test (from AGENTS.md)

| Piece | App acts on it | Only app sees it | Mods cannot | Where |
|---|---|---|---|---|
| Item model, wire keys, layout | yes | yes | yes | core |
| Item commands, control socket verbs | yes | yes | yes | core |
| Toolbar | no | no | no | mod |
| Note | no | no | no | mod (proves the mod-kind path) |
| Image | no | no | needs a file-image API | core API, viewer as mod |
| PDF, video, browser, simulator | no | no | yes (native decode, native views) | built-in kinds |

Growing the core for the item model needs Krishna's go-ahead per AGENTS.md.

## 5. Do we need our own core or protocol?

No. herdr-core runs terminals and agents and carries their messages. Items
are app-side: no PTY, no core state. The core is untouched by this
proposal. Our own core stays a separate, later decision (wire-lean plan:
ask upstream for a message event first). No new protocol is needed: wire
keys already accept any string, and agents reach items through the existing
control socket.

## 6. Candidates per kind (to verify in research)

Licence rule: MIT, Apache-2.0, BSD, ISC or public domain. Anything else
needs Krishna's explicit exception. "Verify" means do not trust this table.

### PDF (with highlighting)

| Option | Licence | Notes to verify |
|---|---|---|
| `pdfium-render` + PDFium binary | MIT/Apache crate; PDFium BSD-3/Apache | Character boxes for highlight, search, page bitmaps. Binary size per OS, how to ship it |
| `hayro` (pure Rust) | verify | Render quality, text extraction with positions, maturity |
| pdf.js in a web item | Apache-2.0 | Text layer and selection for free. Depends on section 7 |
| PDFKit (macOS) | system | Native highlight UI, but a native view and macOS only |
| MuPDF, Poppler | AGPL, GPL | Excluded |

Open: where highlights live (`canvas.json`, a sidecar, never inside the PDF?).

### Image

GPUI `img()` covers common formats. Region quote: drag a rectangle, crop
with the `image` crate (already present). Verify HEIC and AVIF needs.

### Video

| Option | Licence | Notes |
|---|---|---|
| AVFoundation → `CVPixelBuffer` → GPUI `surface()` (macOS) | system; `objc2-av-foundation` MIT/Apache/Zlib (verify) | Zero-copy, zoomable. Audio output, scrubbing |
| Media Foundation (Windows), GStreamer (Linux) | system; GStreamer LGPL | LGPL is outside the list: exception or skip Linux video |
| `<video>` in a web item | system codecs | Depends on section 7 |
| FFmpeg, libmpv | LGPL/GPL | Excluded |

### Browser

| Option | Licence | Shape | Notes |
|---|---|---|---|
| `wry` | MIT/Apache | System web view (WKWebView, WebView2, WebKitGTK) as a native child view | Light. Needs resting/live. Linux embedding into a non-GTK window is a known pain: verify |
| Servo | MPL-2.0 | Rust engine, embeddable | The "lightweight Rust browser" ask. Licence exception needed. Size, embedding API status, can it render to a texture |
| CEF offscreen | BSD | Chromium renders into a buffer | Solves zoom and z-order (texture). Very large (verify size) |
| Blitz (Dioxus) | MIT/Apache | HTML/CSS renderer, no JS | Not a browser. Maybe for static HTML items |
| Ladybird | BSD-2 | Engine | Not embeddable yet (verify) |

Privacy to verify per engine: no app-owned requests. No homepage, search
suggestions or favicon services. Check WebView2 and WKWebView defaults
(SmartScreen, Safe Browsing) and turn off what can be turned off.

### Element and component picking

An injected script reports the hovered element: selector, text, box, and for
React the component name and source location. Candidates (verify licences
and React 19 support): `bippy`, `react-grab`, LocatorJS,
`click-to-component`, `code-inspector-plugin` (also Vue, Svelte), the React
DevTools global hook. React 19 removed `_debugSource`; file:line needs
`_debugStack` parsing or a dev bundler plugin. Research how the Codex app and
Claude Code desktop do it.

### iOS simulator (macOS)

| Need | Candidates | Notes |
|---|---|---|
| Frames | ScreenCaptureKit on the Simulator window; idb `video-stream`; private SimulatorKit IOSurface | Screen recording permission; private APIs break between Xcode versions |
| Input | idb (MIT), AXe (verify), private HID | User installs idb? |
| Element info for quotes | idb `ui describe-point`, accessibility tree | |

Research how Claude Code desktop's iOS simulator tool works.

### Android (optional, same kind)

scrcpy (Apache-2.0) mirrors and controls emulators and real devices.
Emulator gRPC (Apache-2.0). Verify.

### Other kinds worth researching (mods first)

Notes and stickies (native), Markdown (existing TextView), diff (existing
`changes` mod), CSV and tables (gpui-component table), Mermaid (MIT, web or
native port?), Excalidraw (MIT, web), log tail, CI runs, database table.
tldraw's licence needs a key: excluded.

## 7. The React question

| Option | What it means | For | Against |
|---|---|---|---|
| A. Native only (today) | Built-ins in Rust GPUI, mods in gpui-kit JS | Light, one look, everything zooms | Every viewer hand-built, no web ecosystem |
| B. Hybrid web items | A `web` kind hosts any HTML/React page. Mods may ship `index.html`. A small bridge maps the item contract (`read`, `selection`, `verbs`) to `postMessage` | pdf.js, video, Excalidraw, Mermaid, shadcn UIs, any React the user knows | Native view zoom and z-order, memory per live item, bridge security, two toolkits to theme, changes the "no web view" mod rule |
| C. Whole app in React (Tauri, Electron) | Rewrite the shell | One web stack | Contradicts the native pivot, terminals in GPUI, weight |
| D. Defer | Ship A now; keep the contract toolkit-neutral so B slots in later | No rework either way | Some kinds wait |

Current lean: D now, research B. B wins only if research shows: an
acceptable memory cost per live web view, a snapshot API on each OS, a
working answer for overlays above a live view, and Linux support. C is out.

## 8. Security, privacy, idle cost

- Item content is untrusted. Quotes are text, delimited, never run.
  `canopy item read` on a web page is a prompt-injection path: label the
  output as page content, and never treat it as the person's approval.
- The web bridge (if B) exposes only the item contract, never app commands.
- No app-owned network requests. A browser item loads only what the person
  or an agent asks for.
- Idle: resting items cost nothing. Live items stop on blur. No polling.
  Video decodes only while playing and shown.
- Every user-visible error is a toast.

## 9. Merge and build order

### Merges (now)

1. Krishna finishes the README rewording on #9; decide the win98 fonts.
2. Merge #9.
3. Rebase #10 on main, resolve `README.md`, merge #10.

### Phases

| Phase | Scope | Size | Risk |
|---|---|---|---|
| P1 Foundation | Item model and migration, contract, `item.*` commands, wire keys, `canopy wire` and hook, generalized quote, toolbar mod, note as a mod kind | L | Low. Everything else waits on it |
| P2 Viewers | Image with region quote, PDF with highlight | M | PDF library choice |
| P3 Browser | Web item, resting/live, element and component picker | XL | Native view, Linux, privacy defaults |
| P4 Video | macOS `surface()` path first | M | Audio, other OSes |
| P5 Simulators | iOS (macOS), Android optional | L | Private APIs, permissions |
| P6 Extensibility | Mod canvas kinds docs and examples; web items if B | M | Section 7 |

P1 must land first. After it, P2 to P5 can run in parallel worktrees, each
on its own branch, sharing one `CARGO_TARGET_DIR` (disk is tight). Friday
scope is chosen after research, by risk: P1 plus whatever is proven.

### Docs that travel with each phase

`docs/canvas.md`, `docs/wire.md`, a new `docs/items.md`,
`docs/security.md`, `defaults/AGENTS.md`, `src/SKILL.md`, the generated
`.d.ts`, and harness notes for any CLI fact.

## 10. Research brief for the next session

Answer each with sources, versions and licences. Write verified facts into
this file or a sibling notes file.

1. PDF: pdfium-render vs hayro vs pdf.js. Text positions, highlight, size,
   licence, macOS/Linux/Windows.
2. Browser engine: wry vs Servo vs CEF offscreen. Size, memory per view,
   snapshot API per OS, rendering into a texture, Linux embedding, privacy
   defaults, licence.
3. Overlays over a live native view: how do Zed, Tauri apps, VS Code
   webviews, Figma desktop and Arc handle menus above web content?
4. Component picking: which library gives component name and file:line on
   React 19, Next.js, Vite; framework-neutral options; how Codex and Claude
   Code desktop do it.
5. iOS simulator: how Claude Code desktop streams and taps; ScreenCaptureKit
   vs idb vs private frameworks; permissions; Xcode version breakage.
6. Video: AVFoundation into `CVPixelBuffer` with GPUI `surface()`; Linux and
   Windows paths inside the licence list.
7. React question: evidence for options B and D (memory, startup, theming
   web items with Canopy theme tokens, bridge security).
8. Other OSS canvas modules worth shipping as mods (section 6 last list),
   with licences.
9. Prior art for canvas-of-everything apps (tldraw computer, Muse, Heptabase,
   Figma, Kosmik, Obsidian Canvas, Excalidraw): toolbar patterns and how
   they embed web and media.
10. Toolbar keys vs existing canvas-mode keys: propose a binding set.

## 11. Not verified

No library was installed, built or measured. Licences, sizes and APIs in
section 6 come from memory and must be checked. The mockups are
illustrations, not final pixels.
