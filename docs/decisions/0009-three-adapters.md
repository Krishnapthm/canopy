# 0009 - Harness adapters: claude, codex, acp

Status: Accepted (2026-11)

## Context

Canopy drives official CLIs. Native protocols are richer where they exist; ACP covers the rest.

## Decision

Three adapters: `claude` (stream-json, needs `--permission-prompt-tool stdio`, verified on 2.1.318), `codex` (app-server) and `acp`. Built-in harnesses are `claude` and `codex` only. Gemini and OpenCode are NOT built in because their ACP behaviour could not be verified (the CLIs are not installed). Any ACP agent is added at runtime with `canopy daemon --acp name=command args`.

## Consequences

Adding a vendor is configuration, not code, once its ACP behaviour is verified and recorded in `docs/harness-notes/`. Built-in Gemini and OpenCode entries can be added later with fixtures.

## Rejected

ACP for all: adds a Node process per Claude or Codex session and loses steering, cost and sub-agent attribution. Four bespoke adapters: OpenCode's HTTP API buys little over ACP.
