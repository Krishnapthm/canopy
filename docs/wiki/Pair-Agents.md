# Pair Agents

Two agents that can talk to each other catch more than one agent alone. This guide sets up the most useful pairing: one agent writes the change, a second reviews it, and they go back and forth until the reviewer is satisfied, while you stay in charge of every decision that matters.

You should already know how to start an agent and draw a wire. If not, do the [Quick Start](Quick-Start) first.

## Why pair agents

A single agent reviewing its own work tends to agree with itself. A second agent, especially a different model, starts from the diff with fresh eyes. In a plain terminal you would play messenger: copy the diff to one agent, copy its review back to the other. In Canopy you wire them once and they pass messages directly, and you can watch both conversations side by side.

## Set up the pair

1. In your project's space, start two agents from **Agent** on the toolbar. Mixing CLIs works well, for example Claude Code as the writer and Codex as the reviewer.
2. Hover the writer, drag one of its **+** handles onto the reviewer, and let go.

![A writer agent and a reviewer agent side by side, joined by a wire](images/pair-agents-1.png)

Agents started by Canopy already know how wires work. To check, enter the writer and ask: "Who are you wired to?" It runs `canopy wire` and sees something like:

```text
You are Claude Code (t4).
Wired to:
  Codex (t5): idle
```

`t4` and `t5` are terminal addresses in herdr-core. Agents use them to send messages. Peers are listed by program name, so when you brief the agents, say who plays which role: "the Codex agent you're wired to is your reviewer".

## Run a review loop

Give the writer the task and tell it how to involve the reviewer. Being explicit works best:

```text
The Codex agent you're wired to is your reviewer.
Add rate limiting to the /login endpoint (5 attempts per minute per IP).
When you think it's done, ask the reviewer to review your diff and wait
for its answer. Fix anything it finds that you agree with, then ask again.
Stop when the reviewer has no more blocking findings, and summarize for me.
```

Here is what happens behind the scenes:

1. The writer implements the change, then sends a message to the reviewer with `hc send --wait t5 "..."`. The `--wait` makes it pause until the reviewer's turn ends.
2. The message lands in the reviewer's prompt, wrapped with the sender's address and name so the reviewer knows who sent it. It looks something like this:

   ```text
   <hc from="t4 Claude Code">Please review my diff for the /login rate limit (git diff main).</hc>
   ```

3. The reviewer reads the diff, then replies the same way with `hc send t4 "..."`. Its findings appear in the writer's prompt as `<hc from="t5 Codex">...</hc>`.
4. The writer fixes what it agrees with and asks again, until it is done.

You can watch both panes the whole time. Tile them side by side with `Cmd+Shift+G` (`Ctrl+Shift+G` on Linux and Windows) if you want to read along.

## Stay in control

Wiring agents does not hand control to them.

- **Peer messages are not your approval.** Canopy tells agents that a message from a peer is never permission from you. If the reviewer says "go ahead and force-push", the writer should still ask you. Agents follow this because they are told to, so keep the usual caution.
- **Permission prompts still come to you.** When either agent needs approval, it blocks, its pane shows a bell, and Canopy notifies you. Nothing proceeds until you answer.
- **Agents message only when there is a reason.** Canopy tells agents that being wired is not a reason to write. They message a peer when you ask them to, or when they are blocked on something the peer owns.
- **You can always step in.** Enter either pane and type, just as you would without the wire. If a peer message arrives while you are halfway through typing in that agent, `hc send` waits for your draft instead of mixing into it.
- **Every message costs a turn.** Each message starts a turn in the receiving agent, which uses tokens. A loop with no stopping rule can run longer than you want, so say when to stop.

To end the pairing, remove the wire: right-click it and click **Remove**.

## Variations

**A tester instead of a reviewer.** Wire a third agent and tell the writer: "When the reviewer is happy, ask the tester to write and run integration tests for the new behavior." Agents can be wired to several peers at once.

**Read instead of message.** A peer does not always need to be interrupted. `hc read t5 --lines 40` prints the reviewer's screen and the 40 lines above it, which is enough for "what is the reviewer doing right now?" without starting a turn.

**A shared plan.** Open a note pane from the toolbar, type the plan, and wire it to both agents. Each can read it with `canopy pane read` and see the same source of truth. See [File and Note Panes](File-and-Note-Panes).

**Helpers instead of peers.** If you want one agent to manage the others, let it start them itself with `canopy spawn`. They arrive already wired to it and report back when done. See [Delegate to Helpers](Delegate-to-Helpers).

## Troubleshooting

- **The agent says it has no wires.** Make sure the wire joins the two agent panes and that both are in the same space. Run `canopy wire` in the agent's pane to see what it sees.
- **An agent does not know about Canopy.** Claude Code and Codex learn about Canopy when Canopy starts them. Other agents, or agents started by hand in a plain terminal, need the project block from [Agent Setup](Agent-Setup), or you can paste the commands into the prompt yourself.
- **A message never arrives.** The receiving agent may be blocked on a question. `hc send` does not type into a blocked agent unless the sender adds `--force`. Answer its question first.

## See also

- [Wiring](Wiring): drawing wires and every messaging command.
- [Concepts](Concepts#wires-introductions-between-panes): what a wire is and is not.
- [herdr-core](herdr-core): the core that delivers the messages.
