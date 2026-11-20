# Sub-Agents

A sub-agent is a helper that an agent starts. Canopy shows it beside its parent.

![An agent with two sub-agents beside it](images/sub-agents-1.png)

## Helper agents

An agent uses `canopy spawn` to start a helper in a real terminal, wired to its parent.
The helper starts in the caller's folder and space.
See [CLI](CLI) for the command syntax and [Wiring](Wiring) for messaging with `hc`.

When the agent supplies a task, Canopy appends this sentence.
It replaces `<parent>` with the caller's terminal address:

```text
When you are done or stuck, report to your parent with: hc send <parent> "<what you did or need>"
```

No sentence is added without a task.

## In-process sub-agents

In-process sub-agents run inside their parent agent, without separate terminals.
Their named panes show **working**, then **done**. Done panes dim.

- Click a sub-agent pane to follow its read-only transcript in the [Files panel](Files-Panel).
- Talk to the sub-agent through its parent's terminal.

## Clear finished sub-agents

- Right-click the parent pane and select **Clear finished** to remove its done sub-agent panes.
- To remove one done pane, select it and press `Backspace` or `Delete`.

A working sub-agent refuses deletion because its pane cannot stop it independently.
The toast says **This sub-agent is still working. Stop it through its parent.**
