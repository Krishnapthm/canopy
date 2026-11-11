# Canopy

Canopy runs coding agents from different vendors (Claude Code, Codex, any ACP agent) as one
live tree. Any agent can spawn a child on another vendor through canopy's MCP tools, and
every node shows its status, tokens, and files touched. It is local only and drives the
official CLIs you are already logged into. See [PLAN.md](PLAN.md) for the design.

Status: milestones M0 to M3 (daemon, CLI, Claude and Codex adapters, cross-vendor spawn
tree). The TUI (M4) and web app (M5) are not built yet.

## Requirements

- macOS or Linux, Rust (pinned in `rust-toolchain.toml`), Node 20+ for `make check`
- `claude` and/or `codex` on `PATH`, logged in. Check with `canopy doctor`.

## Build

```
make build                 # target/release/canopy
```

## Run

The daemon starts on demand and exits after 10 idle minutes. Start it yourself to add ACP
agents or change limits:

```
canopy daemon                                   # detached, logs to $CANOPY_HOME/daemon.log
canopy daemon --foreground                      # in this terminal, Ctrl-C to stop
canopy daemon --acp gemini="gemini --acp"       # register any ACP agent by name
canopy daemon --max-depth 3 --max-live 16 --budget 2000000
```

Stop a detached daemon with `kill $(pgrep -f "canopy daemon")`; every agent it started stops with it.

```
canopy run claude "fix the failing test in src/lib.rs"     # prints the session id
canopy run --policy auto_edit codex "add a --json flag"
canopy ls                                                  # the tree
canopy tail <id> -f                                        # follow until it settles
canopy send <id> "also update the README"                  # prints steered|queued|started_turn
canopy permit <id> <request> allow|deny|always
canopy stop <id>                                           # stops it and everything it spawned
```

Ids can be shortened to the last characters shown by `canopy ls`. State lives in
`$CANOPY_HOME` (default `~/.canopy`).

Agents get four MCP tools: `spawn_agent`, `wait_agent`, `message_agent`, `list_agents`.
Ask an agent to use them, for example: "use spawn_agent to have codex write the tests".

## Develop

```
make check          # fmt, clippy -D warnings, tsc
make test           # unit + fixture replay tests
make e2e            # real daemon + fake agents in a temp CANOPY_HOME
make bench          # budget assertions (release build)
make gen            # regenerate web/src/protocol.gen.ts
make check-docs     # lint AGENTS.md files and the ADR index
```

## License

MIT
