# 0012 - SolidJS, TypeScript, Vite, plain CSS

Status: Accepted (2026-11)

## Context

Streaming text must update one text node, not a component subtree.

## Decision

The web app (M5) uses SolidJS with signals and stores, TypeScript, Vite, and plain CSS in `web/src/styles.css`. Approved runtime deps: `solid-js` and `marked` (lexer only; MIT). Approved dev deps: `typescript` and `vite`. The web app is not built yet; only the generated `web/src/protocol.gen.ts` exists. Agent output is untrusted, so `innerHTML` is never used.

## Consequences

About 7 KB runtime and no virtual DOM. No component kit, router or state library. Any new npm dependency needs an ADR.

## Rejected

React: VDOM pressure under streaming, 45 KB+. Svelte 5: same class, weaker template type checking. Leptos/WASM: bigger payload, DOM calls cross a JS bridge. Canvas/WebGL: loses selection and accessibility for a few hundred nodes.
