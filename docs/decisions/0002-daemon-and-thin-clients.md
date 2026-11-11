# 0002 - Daemon plus thin clients, one binary

Status: Accepted (2026-11)

## Context

Agents must outlive the terminal and be visible from the TUI and the web app at once.

## Decision

One `canopy` binary. A daemon owns every session; the CLI, TUI and web app are peers that connect to it. The daemon is auto-started like a tmux server and exits when idle.

## Consequences

TUI and web are true peers sharing live state. Sessions survive a closed terminal. The daemon lifecycle (start, detach, idle exit) is code we own and test.

## Rejected

Core as a library inside the TUI: sessions die with the terminal and the web cannot see them.
