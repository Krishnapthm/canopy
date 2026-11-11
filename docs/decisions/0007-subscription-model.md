# 0007 - Subscription model

Status: Accepted (2026-11)

## Context

With dozens of agents streaming, sending every transcript to every client is the main source of lag.

## Decision

Tree updates go to every client. Transcript events go only to clients subscribed to that session. Tree rows carry a throttled one-line activity summary instead of the stream.

## Consequences

Cost scales with what a user is looking at, not with the number of agents. Clients must subscribe explicitly and re-fetch on subscribe.

## Rejected

Broadcast everything and filter client-side.
