# 0028 - Web canvas of project frames

Status: Accepted (2026-11)

## Context

Developers need to see agents across projects together, including read-only
sessions started outside Canopy (ADR 0026).

## Decision

Group plain DOM frames by project root, computed by walking cwd ancestors for
the first .git entry (file or directory), otherwise cwd. Fill missing project
roots when loading old nodes. Reuse Tree and the outside-session rows.

Implement pan and zoom with one CSS transform on the world element. Drag frames
by their headers. Store frame positions in localStorage per browser. Load the
canvas, Card and Sheet lazily. Open selected sessions in a right-side Sheet.

## Consequences

No new dependency or idle discovery polling is needed. Layout belongs to each
browser. Outside sessions remain read-only with no take-over or fork operation.

## Rejected

- React Flow: designed for node-and-edge graphs and adds a dependency.
- tldraw: license is not on the allowed list.
- git rev-parse per spawn: adds a process per spawn.
- Daemon-stored layout: shared layout is not needed yet.
