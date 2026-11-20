# Shared state in labels

Core clients share which agents you have seen and pinned through terminal labels.
This protocol lets other clients share that state with Canopy.

- Labels belong to the terminal. The shared state ends with it.
- `events.subscribe` snapshots contain `labels` and `turn`. Each `labels` event contains the whole label map.
- Remote terminals use the same rules.
- These labels are optional. Clients can ignore them and keep their own behavior.

## `herdr.seen=<turn>`: the last turn a person looked at

The core increases `turn` each time the terminal leaves `working`.
A change from `blocked` to `idle` does not increase it.

Show an agent in this order:

1. For `blocked` or `error`, show that it needs you.
2. For `working`, show that it is working.
3. Otherwise, show `done` when `turn > herdr.seen`. Show `idle` when it is not.

With no `herdr.seen` label, use the client's own seen rules.
A missing label is not `0`.

Write the label with these rules:

- Write `herdr.seen=<turn>` when you look at an agent. Raise an existing value, or write it when missing.
- Count an agent only when its app or window is in the foreground and the agent appears on screen.
- A peek or floating view counts. A terminal that replaces another pane's terminal hides that terminal from the seen check.
- Write once per turn while the agent stays on screen. Do not write on every update.
- This rule preserves another client's mark unread. Looking again later can raise the value again.
- To mark unread, write `herdr.seen=<turn - 1>`. This is the only lower write. Do not mark unread at turn `0`.
- Give a terminal that your client starts `herdr.seen=0`.

The last writer wins.
Each client chooses how long a done row stays in place while you look at it.

## `herdr.pinned=1`: pinned

- A present, non-empty value means pinned.
- To unpin, write an empty value: `herdr.pinned=`. This removes the label.
- Show pinned agents first, in the client's usual order.
- In Canopy mods, use `terminal.pin {terminal, pinned}` and read `pinned` from `agents()`.
  Do not write the label directly from a mod.

## Adding more

- Use labels for small values that change at human speed.
- Use the `herdr.` prefix.
- Use documents (`doc.get`, `doc.set`) for larger values or app-wide state.
- Keep state for one device on that device.

See [Agents](Agents) for using agents in Canopy.
