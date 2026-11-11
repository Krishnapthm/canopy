# 0011 - Hand-rolled line-delimited JSON-RPC peer

Status: Accepted (2026-11)

## Context

Codex, ACP and MCP share the same framing: one JSON object per line.

## Decision

One small module implements the peer, with tolerant `serde` structs for only the fields we use. Unknown fields and variants are counted, never fatal.

## Consequences

No SDK dependency trees to track. We own the subset we use and must verify it against installed CLIs.

## Rejected

`rmcp`, an ACP crate and a Codex SDK: three dependency trees for a subset we can write ourselves.
