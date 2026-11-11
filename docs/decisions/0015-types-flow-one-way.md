# 0015 - Types flow one way

Status: Accepted (2026-11)

## Context

Hand-synced protocol types drift and break the web silently.

## Decision

`crates/protocol` is the source of truth. `ts-rs`, an optional feature used only by `make gen`, generates `web/src/protocol.gen.ts`. A protocol change means: edit Rust, `make gen`, fix what `tsc` reports. The generated file is never edited by hand.

## Consequences

A protocol change that breaks the web fails `tsc`. `ts-rs` stays out of release builds.

## Rejected

A JSON Schema round trip, or hand-written TS types.
