# 0017 - macOS and Linux only in v1

Status: Accepted (2026-11)

## Context

The design relies on Unix sockets and process groups.

## Decision

Support macOS and Linux only. Windows is a later port.

## Consequences

No Windows code paths or CI. Unix-only APIs may be used freely.

## Rejected

-
