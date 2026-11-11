# 0022 - React and shadcn/ui web app

Status: Accepted (2026-11)

## Context

The user chose React and the installed shadcn/ui controls. This supersedes
[0012](0012-solidjs-web-app.md) and [0021](0021-solid-jsx-build-plugin.md).
Custom controls would duplicate accessible focus, keyboard and dialog behavior.
The standard library cannot provide React JSX or these composable primitives in
twenty lines.

## Decision

Replace Solid, its JSX plugin, the SVG sprite and handwritten CSS controls with
React 19, React DOM, shadcn/ui (Base UI base-nova), Tailwind v4 and lucide.
Use an immutable external store with useSyncExternalStore, no state library or
router. Keep marked as a lazy lexer, rendering tokens as elements only.
Use next-themes for system dark mode and react-resizable-panels for the graph split.

Approved packages below are already installed. Installed sizes are approximate KiB
on disk, not browser payloads; tree shaking and lazy loading determine transfer.

| Package | Installed KiB | License | Replaces or provides |
|---|---:|---|---|
| react / react-dom | 260 / 7988 | MIT | Solid rendering |
| @base-ui/react | 20216 | MIT | Handwritten control behavior |
| @shadcn/react | 76 | MIT | Chat scrolling primitives |
| shadcn | 680 | MIT | Generated component styles |
| lucide-react | 50588 | ISC | Custom icon sprite |
| next-themes | 44 | MIT | System theme synchronization |
| react-resizable-panels | 2264 | MIT | Pane resizing |
| cmdk | 116 | MIT | Command palette keyboard and search |
| cn | 540 | MIT | Class merging |
| class-variance-authority | 44 | Apache-2.0 | Component variants |
| marked | 576 | MIT | Markdown lexer, unchanged |
| tailwindcss / @tailwindcss/vite | 860 / 24 | MIT | Handwritten stylesheet/build |
| @vitejs/plugin-react | 72 | MIT | Solid JSX plugin |
| tw-animate-css | 56 | MIT | Component transitions |

TypeScript (Apache-2.0), Vite (MIT), and React/Node type declarations (MIT)
remain build dependencies. No fonts are bundled. Geist was removed because its
OFL license is outside the project's allowed license list. Unused installed UI
components and unused direct runtime dependencies are removed.

## Consequences

The JS budget becomes 120 KB gzip for the initial entry and its static import
closure. The workspace shell, views and Markdown load as separate lazy chunks.
The build manifest drives measurement, so shared static chunks are counted once.
Initial JS measures about 69 KiB gzip; this excludes the lazy workspace and must
not be described as the total first workspace transfer. All assets receive gzip
sidecars for embedding.

Streaming batches publish one immutable snapshot per frame. React rerenders
subscribers, so the 50-agent frame-time and heap budgets still require a visible
browser performance check. This decision does not claim those budgets passed.
