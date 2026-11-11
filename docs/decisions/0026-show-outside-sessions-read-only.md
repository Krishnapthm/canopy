# 0026 - Show outside sessions read-only

Status: Accepted (2026-11)

## Context

Users want to see the Claude Code and Codex sessions already running in a folder
alongside Canopy's own. Those sessions run inside another terminal's process.
Claude Code 2.1.318 offers no way for another program to drive an interactive
session (`claude attach` opens only background sessions, as a TUI), and two
writers on one transcript corrupt it. Codex 0.167.1 cannot tell whether another
process has a thread open. herdr, the closest prior art, never adopts sessions it
did not start: it shows and drives only its own, and restores those by resume id.

## Decision

Add `Discover {cwd?}`, returning `{sessions, errors}`, run only on an explicit
client request and off the hub. It lists live Claude Code sessions with
`claude agents --json [--cwd]` and drops ones Canopy already owns. Each row has
harness, vendor_id, cwd, title, started (ms), busy and pid. Clients show them
read-only. Canopy never closes, resumes or forks a session it did not start.
Codex is not listed: its thread list is history, not what is running.

## Consequences

No discovery work runs while idle. A failed probe becomes a toast. To work with
an outside session in Canopy, the user closes it and starts a Canopy agent.

## Rejected

Closing the other process and resuming it (kills the user's terminal session).
Forking with `--fork-session` (two diverging copies; more surface). Listing Codex
threads as running (misleading). Reading `~/.claude/projects` transcripts (private
storage). Polling (breaks the idle budget).
