# 0014 - hyper, fastwebsockets, rust-embed, no web framework

Status: Accepted (2026-11)

## Context

The web server has two routes: static assets and a WebSocket upgrade.

## Decision

Use `hyper`, `hyper-util`, `http-body-util`, `fastwebsockets` and `rust-embed`, with no router. Assets are embedded and pre-compressed so the release is one file. Deferred to M5: nothing is built yet and none of these crates are in `Cargo.toml`.

## Consequences

One file to ship, no second process. The crates are approved in ADR 0001 but must still be added deliberately.

## Rejected

`axum`: fine but unneeded. A separate static server: a second process.
