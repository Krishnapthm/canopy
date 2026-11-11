# 0029 - 161 KB first-load budget

Status: Accepted (2026-11)

## Context

ADR 0022 set the web first-load JS budget to 160 KB gzip. Harness rows show the
Claude brand mark from @lobehub/icons (ADR 0027). Its Mono SVG path adds 746
bytes gzip over the Claude Code mark, which left the bundle 160 bytes over
budget. Every first-load chunk is already needed on the first screen.

## Decision

Raise the first-load JS budget to 161 KB gzip. `web/build.mjs` asserts it.
This supersedes only the budget number in ADR 0022.

## Consequences

The headroom is small again: the next first-load addition must pay for itself
or move behind a lazy import.

## Rejected

- Keep the Claude Code mark: the user chose the Claude mark.
- Lazy-load the brand icons: they render in the sidebar on first paint.
