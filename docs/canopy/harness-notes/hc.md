# hc (herdr core) notes

## Labels, 2026-11-13 (hc 0.1.20, scratch core)

- `hc label <t> k1=v1 k2=v2` sets several labels at once; `hc label <t> k=` removes `k`
  and leaves the others - verified live.
- `hc get <t> --json` returns the terminal record with a `labels` object - verified live.
  The `SessionEnd` hook uses both to clear `canopy.sub.*` on its own terminal.
