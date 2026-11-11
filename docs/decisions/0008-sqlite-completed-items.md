# 0008 - SQLite, completed items only

Status: Accepted (2026-11)

## Context

We need cross-session queries (history, cost rollups) and crash safety without heavy write load.

## Decision

Bundled SQLite in WAL mode via `rusqlite`. Only completed items are written; in-flight text stays in memory. The `items` table has primary key `(session, seq)`, WITHOUT ROWID, with a per-session `seq` (the PLAN's global autoincrement was dropped). One writer thread batches whatever is queued, up to 256 messages per transaction.

## Consequences

About 50x fewer writes than persisting deltas. A crash loses at most the in-flight item. Per-session `seq` makes paging a session a range scan on the primary key.

## Rejected

Per-session JSONL: no queries, hand-rolled indexing. Persisting every delta: pointless write load. Global autoincrement id: a second index for no benefit.
