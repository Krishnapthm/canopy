# 0013 - Browser-native virtualization

Status: Accepted (2026-11)

## Context

Long transcripts need windowing without a virtual-list library.

## Decision

Use `content-visibility: auto` plus a 300-item window with paging. Implemented in M5.

## Consequences

Zero JS and zero dependency; variable heights just work. Not built yet.

## Rejected

A virtual-list library: handles variable heights badly and adds a dependency.
