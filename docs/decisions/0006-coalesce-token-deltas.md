# 0006 - Coalesce token deltas at ingestion

Status: Superseded by [0023](0023-25ms-coalesce-window.md) (2026-11)

## Context

Nobody reads faster than about 20 updates per second, yet harnesses emit a delta per token.

## Decision

Adapters coalesce deltas in `harness::Out` with a 40 ms window. The buffer is flushed before any non-delta event so event order is preserved. Per-token events never reach the hub.

## Consequences

Event volume drops about 20x for the hub, sockets and both UIs. Any new adapter must emit through `harness::Out`, not straight to the hub.

## Rejected

Forwarding every token: the daemon, socket and renderer all do 20x the work for no visible gain.
