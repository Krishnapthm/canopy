# 0016 - A scripted fake agent

Status: Accepted (2026-11)

## Context

E2E tests, benches and UI work must be deterministic and spend no tokens, while still exercising the real adapter.

## Decision

`canopy fake-agent` speaks ACP and is scripted by prompt lines: `say`, `think`, `sleep`, `tool`, `ask`, `edit`, `usage`, `fail`. Tests register it as two ACP harnesses, `fake` and `fake2`, to cover cross-vendor behaviour.

## Consequences

Tests run through the real daemon and the real ACP adapter. The fake must be kept in step with the ACP subset we use.

## Rejected

Mocks inside the adapter layer: they skip the code that breaks.
