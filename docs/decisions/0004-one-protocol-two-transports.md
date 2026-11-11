# 0004 - One protocol, two transports

Status: Superseded in part by [0024](0024-product-surface-foreground-web.md) (2026-11)

## Context

CLI, TUI and browser all need the same commands and events.

## Decision

One JSON message schema defined in `crates/protocol`. NDJSON over a Unix socket for the CLI and TUI; the same messages as WebSocket text frames for the browser (M5). The Unix socket is permission-guarded by the filesystem.

## Consequences

One schema, one reducer design, one set of tests. The same socket also carries the MCP shim (ADR 0010). No TCP or HTTP overhead for local clients.

## Rejected

gRPC: heavy dependency, awkward in browsers. REST plus SSE: two channels, text only, one-way.
