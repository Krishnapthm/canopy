# Canvas items: proposal for stack review and research

Written 2026-11-17. A proposal, not a decision. Nothing here is built.
Visuals: [mockups/canvas-items.html](mockups/canvas-items.html), also
published privately at https://claude.ai/artifact/FpJQpEjDHMAS57aBe1e4QV
(fig 1 tabletop, fig 2 quote flows, fig 3 agent view and mods menu, fig 4
resting vs live, fig 5 architecture, fig 6 merges and phases, fig 7 the
React question).

Research is done (section 6). "Tabletop" and "items" are working names; the
product name and the user-facing word for canvas items are still open. Code
and CLI use `item`, so a later rename touches only UI text.

## 1. Status this proposal assumes

- `main` at `e226dd4`: 0.1.0 plus PRs #9 (rebrand leftovers) and #10 (wire
  on hc), both merged 2026-11-17.
- Reddit post deadline: Friday 2026-11-20. Scope is locked in section 9.

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
  `Cargo.lock`. Our vendored gpui-kit 0.7.1 omits upstream's `crates/webview`
  (see section 6).
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

## 6. Stack, verified 2026-11-17

Researched by nine Codex agents with web search, then the picks below were
checked by hand against crates.io, npm, GitHub and our `vendor/`. Nothing was
installed, built or measured. Licence rule: MIT, Apache-2.0, BSD, ISC or
public domain.

| Kind | Pick | Licence, version | Why | Not chosen |
|---|---|---|---|---|
| PDF | `gpui-pdf` (built on `hayro`) | MIT, 0.7.0 (2026-10-29); hayro MIT/Apache 0.9.0 | Already a GPUI view on `gpui-pre ^0.3` (we pin 0.3.8; its lockfile used 0.3.9). Pure Rust, no native binary. Page virtualized. `markup` feature: drag-select returns `{quote, occurrence, rects}`, exactly the quote we need. `search` feature. | `pdfium-render` 0.9.5 (fallback; ships a native binary per OS), pdf.js (needs web items), PDFKit (native view, macOS only), MuPDF/Poppler (AGPL/GPL) |
| Highlights | Sidecar in `canvas.json` item props, shaped like W3C Web Annotation `TextQuoteSelector` (`quote` + `occurrence`) | spec | Never edits the user's PDF; survives re-render | in-file annotations |
| Image | GPUI `img()` + `image` crate crop | MIT/Apache, 0.25 | Already in the tree. HEIC only via macOS ImageIO later; AVIF needs `image`'s `avif-native` | libheif (LGPL) |
| Browser | `gpui-webview` from gpui-kit **v0.7.1**, the same version we vendor, on `lb-wry` 0.53 (Longbridge's wry fork) | Apache-2.0 | Our `vendor/gpui-kit/Cargo.toml` already names `crates/webview` 0.7.1; the crate is one 6.6 KB `lib.rs`. Vendor it, no GPUI version change | gpui-webview 0.10 (needs gpui-pre 0.3.12 and GPUI Fast), Servo 0.8.0 (MPL-2.0, API incomplete), CEF (very large), Blitz (no JS), Ladybird (not embeddable) |
| Component picker | `react-grab` injected as a wry initialization script, bundled (never fetched) | MIT, 0.2.0 (2026-08-16) | Ships `dist/index.global.js`; React 19 source via `_debugStack`. Runs before page scripts, so no change to the user's app (UNVERIFIED in a web view) | LocatorJS, click-to-component, code-inspector (need build plugins), bippy alone (no picker) |
| Picker fallback | Our DOM script: selector, text, box, screenshot | ours | Works for any framework | |
| Video (macOS) | `objc2-av-foundation` `AVPlayerItemVideoOutput` → `CVPixelBuffer` → GPUI `surface()` | Zlib/Apache/MIT, 0.3.2 | Zero-copy, zooms like any element, audio from AVPlayer | GStreamer (LGPL), FFmpeg (LGPL/GPL); no GPUI video crate exists |
| iOS simulator | Snapshot: `xcrun simctl io screenshot`. Input and element tree: AXe if installed | AXe MIT, v2.0.4 (2026-11-12) | No private APIs in our code; AXe covers taps, typing, accessibility tree | idb (heavier install), private SimulatorKit IOSurface (breaks per Xcode). ScreenCaptureKit streaming later (needs Screen Recording permission) |
| Android | scrcpy, later | Apache-2.0, v4.1 | Mature mirror and input | |
| Agent reads | `canopy item read` CLI (section 4.3). Output wrapped as untrusted item content | ours | Every CLI can run a command. MCP tool later for clients that prefer it; hooks only as hints | MCP resources (uneven client support), ACP (per-agent adapters) |

### Facts that changed the plan

- **gpui-kit has a web view.** The proposal said it did not. It was added
  upstream; our vendored 0.7.1 subset just omits `crates/webview`. Its README:
  experimental, renders on top of the GPUI window (covers GPUI elements under
  it), macOS and Windows only. Section 4.6 resting/live already answers the
  overlay problem: menus and the quote chip appear only while the card rests.
- **gpui-pdf already does PDF selection-to-quote.** P2 PDF shrinks from
  "pick a library and build a viewer" to "wrap `PdfView` as an item".
- **Linux web view** needs X11 or XWayland (gpui-webview 0.10 path); 0.7.1 has
  none. Linux browser items wait.
- **Prior art names**: Figma objects/frames, Obsidian and Heptabase cards,
  Kosmik items, Excalidraw elements, tldraw shapes. JSON Canvas (MIT) is a
  possible import/export format later, not our model.

### Other modules worth shipping as mods later

`gpui-whiteboard` 0.5.2 (MIT crate; its source repo `packetThrower/zorite`
is GPL-3.0, so use only the published crate and confirm with the author),
gpui-kit `TextView` for Markdown and its table for CSV, Mermaid and
Excalidraw (MIT, need web items).

## 7. The React question: decided

**D now, B narrowly later.** Friday ships native kinds plus one web-view kind
(browser). Mods stay gpui-kit JS. No per-view memory numbers exist for
WKWebView or WebView2 (only Linux WebKitGTK reports, about 56 MB per extra
window in one app), so a general "mods ship HTML" surface waits until we
measure our own. When B comes, follow the VS Code and Tauri pattern: CSP, a
`postMessage` bridge exposing only the item contract, theme tokens injected
as CSS variables.

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

### Phases

| Phase | Scope | Size | Risk |
|---|---|---|---|
| P1 Foundation | Item model and migration, contract, `item.*` commands, wire keys, `canopy wire` and hook, generalized quote, toolbar mod, note as a mod kind | L | Low. Everything else waits on it |
| P2 Viewers | Image with region quote; PDF by wrapping `gpui-pdf` `PdfView` | S-M | gpui-pdf on `gpui-pre` 0.3.8 |
| P3 Browser (macOS) | Vendor `gpui-webview` 0.7.1, resting/live, react-grab and DOM picker | L | Snapshot timing, privacy defaults |
| P4 Video | macOS `surface()` path | M | Audio sync, buffer lifetime |
| P5 Simulators | iOS snapshot via simctl, AXe input when installed; Android later | M | AXe across Xcode versions |
| P6 Extensibility | Mod canvas kinds docs and examples | M | |

P1 must land first. After it, P2 to P5 can run in parallel worktrees, each
on its own branch, sharing one `CARGO_TARGET_DIR` (disk is tight).

**Friday target (locked 2026-11-17):** P1, P2 and P3 on macOS. That is the post's
story: an agent opens a PDF and a localhost page beside itself, and the
person quotes a highlight or a React component into it. P4 and P5 ship
after the post unless P1 to P3 land early. Day one builds gpui-pdf and
gpui-webview against our GPUI to retire the two biggest risks first.

### Docs that travel with each phase

`docs/canvas.md`, `docs/wire.md`, a new `docs/items.md`,
`docs/security.md`, `defaults/AGENTS.md`, `src/SKILL.md`, the generated
`.d.ts`, and harness notes for any CLI fact.

## 10. Credits

Ship these in THIRD-PARTY-NOTICES and the post: gpui-pdf (Will Lehnertz,
MIT), hayro (Laurenz Stampfl, MIT/Apache-2.0), gpui-kit and gpui-webview
(Longbridge, Apache-2.0), wry (Tauri, MIT/Apache-2.0), react-grab (Aiden Bai,
MIT), AXe (Cameron Cooke, MIT, optional user install), objc2 (Mads
Marquart, MIT/Apache-2.0/Zlib), image (image-rs, MIT/Apache-2.0).

## 11. Not verified

- No crate was built. gpui-pdf against our `gpui-pre =0.3.8` and
  gpui-webview 0.7.1 inside our window are the first things to build.
- react-grab as an injected script in WKWebView, before React loads.
- Web view privacy: whether WKWebView Safe Browsing can be turned off for an
  embedded view.
- Memory per live web view on macOS.
