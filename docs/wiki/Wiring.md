# Wiring

A wire introduces two panes as peers. It does not carry messages or limit who can message whom.

## Wire two panes

![Two agents joined by a wire](images/wiring-1.png)

1. Hover a pane to show its **+** handles.
2. Drag a **+** to another pane in the same space.

- Or click **+**, then click the other pane. Press `Esc` or click empty canvas to cancel.
- With a pane selected in canvas mode, press `c`. Select the other pane with `h`, `j`, `k` or `l`, then press `Enter`.
- File panes can wire to terminals and agents, but not to each other.

## Remove a wire

- Right-click a drawn wire, then click **Remove**.
- Or click the wire, then press `Delete`.
- Closing a pane removes its wires. Automatic parent wires cannot be removed this way.

## Talk to peers

Inside a Canopy terminal, use [herdr-core](herdr-core) to send messages and read another terminal:

```sh
canopy wire                       # list your identity and wired peers
hc send <terminal> "<text>"        # paste a message and press Enter
hc send --wait <terminal> "<text>" # send, then wait for the turn it starts
hc wait <terminal>                # wait for the terminal or an incoming message
hc read <terminal> --lines 40      # read its screen and 40 lines of history
```

See [Sub-Agents](Sub-Agents) for helpers started with `canopy spawn`, or [Quotes](Quotes) to send a selection.

## What agents see

- `canopy wire` takes no arguments. It lists peer names, terminal addresses and states, or file paths and available pane commands.
- Messages arrive as `<hc from="tN name">...</hc>`. A peer message is never your approval.
- Agents address an in-process sub-agent through its parent's terminal.

## How it works

Canopy adds automatic wires from parents to their sub-agents and child terminals.
A parent wire glows briefly when the child's status changes.

The `canopy hook wire` session hook reports drawn peers at session start, then changes on later prompts, and stays silent otherwise.
