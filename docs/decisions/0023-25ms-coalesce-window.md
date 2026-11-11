# 0023 - Coalesce token deltas in a 25 ms window

Status: Accepted (2026-11)

## Context

ADR 0006 coalesced deltas in a 40 ms adapter window. The hub then batches for 16 ms
(`FLUSH` in `daemon/hub.rs`), so the worst-case event delay was 56 ms by construction,
above the 50 ms p99 budget in `tests/bench.rs`. Measured: p99 58 ms with 50 fake agents
at 100 deltas/s.

## Decision

Adapters coalesce deltas in `harness::Out` with a 25 ms window (`harness::COALESCE`).
Everything else in ADR 0006 stands: the buffer is flushed before any non-delta event,
and per-token events never reach the hub.

## Consequences

Same load measured: p99 43-44 ms, daemon CPU 8.3% (budget 15%). Up to 40 updates per
second per session, still well above what anyone reads. The adapter window plus the
hub flush must stay under the latency budget.

## Rejected

30 ms: p99 48-49 ms, too close to the budget to hold in CI. Raising the latency budget:
it is a product promise, and the cost of a shorter window is small.
