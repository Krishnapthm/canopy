# hc (herdr core) notes

## Labels, 2026-11-13 (hc 0.1.20, scratch core)

- `hc label <t> k1=v1 k2=v2` sets several labels at once; `hc label <t> k=` removes `k`
  and leaves the others - verified live.
- `hc get <t> --json` returns the terminal record with a `labels` object - verified live.
  The `SessionEnd` hook uses both to clear `canopy.sub.*` on its own terminal.

## Children and send, 2026-11-13 (hc 0.1.20, scratch core)

- `hc run` gives the child the core's environment (PATH included), not the caller's -
  verified: a PATH set on the calling `hc run` did not reach the child.
- `hc send` run inside a terminal wraps the text as `<hc from="t1">...</hc>` - verified;
  without `HC_TERMINAL_ID` it types the text as is.
- When a child exits, hc types `<hc>t4 luna exited (exit 129). Read: hc read t4</hc>` into
  an agent parent (`--no-notify` turns it off) - verified. A plain shell parent gets nothing.

## Core contract, 2026-11-16 (hc 0.1.20)

`scripts/core-check ~/.local/bin/hc` passed in full against a scratch core:

- Help for every subcommand and flag in `docs/canopy/core-contract.md`.
- Hello: generation 1, version 0.1.20, every `USES` method. Features:
  framed, attach, events, status, shell, resize_notifications,
  automatic_input, restart_environment.
- `run --bg --name --label --cwd`, then `ls --json` lists it and
  `get --json` has its labels (`ls --json` omits labels).
- `send` text reaches a plain `sh` reader; `read --lines` shows its answer.
- `run --parent <t>` shows up as `parent` in `ls --json`.
- `hc` is on PATH inside a terminal started by `hc run`. An `hc run` from
  inside a core terminal, with no `--parent` or `--cwd`, gets the caller as
  parent and the caller's cwd.

Not covered: `<hc from>` tagging and `hc wait --children any` need both ends
to report status (OSC 7501), so they need a live agent.
