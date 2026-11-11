# 0010 - Cross-vendor spawn via an MCP server in the daemon

Status: Accepted (2026-11)

## Context

MCP is the only extension point every harness shares, and ACP mandates stdio MCP support.

## Decision

The daemon implements the MCP tools once (`crates/canopy/src/daemon/mcp.rs`). Harnesses reach it through a stdio shim: `canopy mcp-stdio --sock <path> --session <id>`. The shim sends `{"mcp":"<session>"}` as its first line on the daemon socket, then pipes bytes. It uses the same socket as clients.

## Consequences

The shim is a dumb pipe in the same binary (about 1-2 MB RSS). The calling session is identified by the first line, so tools know the parent. Spawn semantics are in ADR 0019.

## Rejected

HTTP MCP: needs streamable-HTTP semantics and is optional in ACP. Wrapping each vendor's native sub-agent tool: impossible across vendors.
