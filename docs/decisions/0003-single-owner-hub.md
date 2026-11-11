# 0003 - Single-owner hub task

Status: Accepted (2026-11)

## Context

The workload is I/O multiplexing of many JSON streams. Shared mutable state across threads buys bugs, not speed, at this load.

## Decision

The tokio runtime is `current_thread`. `crates/canopy/src/daemon/hub.rs` owns all state and is fed by one channel; nothing else mutates it and there is no `Mutex` around session state. The daemon does no work when idle. Only two timers exist, each only while needed: a 16 ms flush timer while output is pending, and a single 10 minute idle-exit timer. No polling, no periodic timers, no file watchers.

## Consequences

Ordering is trivially correct and there is no lock contention. Anything slow must be pushed to a task that reports back by message. Revisit the runtime only if the M6 bench saturates one core.

## Rejected

Multi-thread runtime with `Arc<Mutex>`: more memory, more bugs, no gain at this load.
