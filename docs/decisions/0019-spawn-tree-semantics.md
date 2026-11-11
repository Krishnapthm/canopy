# 0019 - Spawn tree semantics

Status: Accepted (2026-11)

## Context

Agents spawn agents through the MCP tools of ADR 0010. Without fixed rules, children escalate permissions, loop, or go unreported.

## Decision

- `spawn_agent` children inherit the caller's cwd and policy. Policy is min(requested, parent), so a child cannot escalate.
- A child is finished when its status is settled: idle, done, failed, cancelled, interrupted or rate_limited.
- Each settled child is reported to its parent exactly once: either as a `wait_agent` result, or as an "[canopy] agent ..." notice message delivered when the parent is idle.
- Stop on a node stops its whole spawned subtree.
- `wait_agent` is bounded at 240 s. The harness MCP tool timeout is 270 s: Claude via the `MCP_TOOL_TIMEOUT` env var, Codex via `mcp_servers.canopy.tool_timeout_sec`.
- Canopy's read-only tools (`wait_agent`, `list_agents`) are auto-allowed in Claude.
- Limits: max depth 3, max 16 live sessions, optional per-root token budget, and refusal on a rate-limited harness. All are returned as tool errors.

## Consequences

A runaway tree is bounded in depth, count and tokens. The timeout margin (270 vs 240) lets `wait_agent` return cleanly before the harness gives up.

## Rejected

Unbounded waits: harness tool timeouts kill the call. Reporting children on every poll: duplicate notices.
