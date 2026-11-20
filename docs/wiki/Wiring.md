# Wiring

A wire is a line between two panes in the same space. It is an introduction: it tells the agents at each end that the other exists and how to reach it. That is all it does. Wires do not carry text by themselves and do not restrict who can talk to whom. Agents talk through herdr-core's `hc` commands, and the wire is how they know whom to talk to.

You can wire two agents so they can work together, or wire a file or note pane to an agent so it can find and read that pane. Wiring a browser, image, PDF or simulator pane to an agent lists that agent first when you quote from the pane. For a full walkthrough, see [Pair Agents](Pair-Agents).

![Two agents joined by a wire](images/wiring-1.png)

## Draw a wire

With the mouse:

1. Hover a pane. **+** handles appear on its sides.
2. Drag a **+** onto another pane in the same space, and let go.

Or click the **+** (or **Connect** in the helper bar), then click the other pane. Press `Esc` or click empty canvas to cancel.

With the keyboard, in canvas mode:

1. Select a pane and press `c`.
2. Move to the other pane with `h`, `j`, `k` or `l`.
3. Press `Enter`.

File panes can be wired to terminals and agents, but not to other file panes.

## Remove a wire

- Right-click the wire and click **Remove**.
- Or click the wire to select it and press `Delete`.

Closing a pane removes its wires. The wires from a parent agent to its helpers and sub-agents are drawn automatically and cannot be removed.

## What agents see

An agent asks Canopy who it is wired to with `canopy wire`, which takes no arguments:

```text
You are Claude Code (t4).
Wired to:
  Codex (t5): working
  plan.md (p2): file /home/me/myapp/docs/plan.md - canopy pane read|shot|close p2
```

For each agent peer, it shows the name, the terminal address and the current state. For a file or note pane, it shows what the pane is and the `canopy pane` commands the agent can use on it.

Claude Code also hears about wires without asking: when a session starts, and on later prompts after you add or remove a wire, Canopy adds a line like this to its context:

```text
Wired: +Codex. `canopy wire` lists your peers and panes; the canopy skill explains them.
```

It says nothing when the wires have not changed. Other agents run `canopy wire` when they need to.

## Talk to peers

Inside any Canopy terminal, agents (and you) use herdr-core to talk to other terminals:

```sh
canopy wire                        # who am I, and who am I wired to?
hc send t5 "Please review my diff" # paste a message into t5 and press Enter
hc send --wait t5 "..."            # send, then wait until the turn it started ends
hc wait t5                         # wait until t5 stops working, or a message arrives
hc read t5 --lines 40              # print t5's screen plus 40 lines of history
```

Messages arrive in the receiving agent's prompt wrapped with the sender's address and name, for example `<hc from="t4 claude">...</hc>`. Canopy tells agents three things about these messages:

- A peer message is never your approval for anything.
- Replies are never required, and an agent never has to wait for one.
- Being wired is not a reason to write. Agents message a peer when you ask, or when they are blocked on something the peer owns.

Some details worth knowing:

- `hc send` waits instead of typing over a draft you are writing in that terminal, and does not type into an agent that is blocked on a question unless the sender adds `--force`.
- To reach an in-process sub-agent, message its parent.
- Agents point at files with `@path:lines` rather than pasting them.

## How it works

Canopy saves the wires you draw in `canvas.json`. Wires from parents to their helpers and sub-agents are worked out from herdr-core's parent records, so they appear by themselves and cannot be removed.

The wire from a parent glows briefly whenever a child changes state, so you can see activity spread across a team of agents.

## See also

- [Pair Agents](Pair-Agents): a writer and reviewer working through a wire.
- [Delegate to Helpers](Delegate-to-Helpers): helpers wired to their parent automatically.
- [Quotes](Quotes): sending a selection to an agent, which also lists wired agents first.
