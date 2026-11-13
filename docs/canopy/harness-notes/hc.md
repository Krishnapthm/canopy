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
