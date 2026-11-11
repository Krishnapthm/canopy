# 0018 - Short session ids are suffixes

Status: Accepted (2026-11)

## Context

A ULID begins with a 48-bit millisecond timestamp, so agents spawned together share a long common prefix.

## Decision

UIs show the last 8 characters of a session id. The daemon resolves any unique suffix to a session (`resolve` in `crates/canopy/src/daemon/hub.rs`); an ambiguous suffix is an error.

## Consequences

Short ids are distinct in practice and need no extra state. Commands and tools accept the short form.

## Rejected

Prefixes: collide in practice. A separate short-id counter: more state to persist and sync.
