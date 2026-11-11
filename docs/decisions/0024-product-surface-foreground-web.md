# 0024 - Product surface: control room and foreground web

Status: Accepted (2026-11)

## Context

Canopy is a shared control room for coding agents. A second scripting interface adds
commands and maintenance without helping that workflow. A daemon-owned HTTP server
also hides the web server's lifetime from the terminal that opened it.

## Decision

The public interface is bare `canopy` (TUI), `canopy web`, `canopy doctor`,
`canopy demo`, `canopy help` (also `-h` and `--help`), and `--version`.
Remove `run`, `ls`, `send`, `resume`, `stop`, `permit`, and `tail`.
Keep `daemon`, `mcp-stdio`, and `fake-agent` as hidden internal commands.

HTTP leaves the daemon and runs in the foreground `canopy web` process until
Ctrl+C or SIGTERM. It serves embedded assets on loopback, authenticates WebSockets,
and bridges them to the daemon socket using the shared connect-or-autostart path.
An exclusive standard-library file lock permits one web server per CANOPY_HOME.
The private web.json records URL, port, and pid and is removed on clean shutdown.
Each run gets a fresh token. The daemon owns sessions and serves only its socket.

This supersedes ADR 0002 in part for the scripting CLI, ADR 0004 in part for the
client surface, and ADR 0014 in part for HTTP process ownership. ADR 0005's
WebSocket text frames and batching remain unchanged.

## Consequences

TUI and browser clients share the live tree and keep their own view. Agents survive
closing the web server. Scripts using removed commands must use the socket protocol.
Tests and the disposable demo drive that protocol directly. No dependency is added.

## Rejected

Keeping scripting aliases would preserve a second public interface. Daemon-hosted
HTTP makes stopping the browser server depend on stopping agents too.
