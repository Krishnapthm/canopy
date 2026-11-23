# Sub-Agents

When an agent starts other agents to help it, Canopy shows them on the canvas next to the agent that started them (the parent), joined by a wire. There are two kinds, and they behave differently:

- **Helpers** are full agents started with `canopy spawn`. Each runs in its own terminal pane, and you can enter it and talk to it.
- **In-process sub-agents** run inside the parent, such as Claude Code's built-in sub-agents. Canopy shows each one as a small pane you can follow but not type to.

For the workflow, with examples of what to ask for, read [Delegate to Helpers](Delegate-to-Helpers). This page is the reference.

![An agent with two sub-agents beside it](images/sub-agents-1.png)

## Helpers

An agent starts a helper from its terminal:

```sh
canopy spawn codex --name docs -- Update the README for the new verbose flag.
```

- `<program>` is any agent key from `programs.json`, such as `claude` or `codex`.
- `--name` names the helper's pane.
- `--model <model>` picks a model, when the agent CLI supports one.
- Everything after `--` is the helper's first message, its task.

The helper starts in the parent's folder and space, wired to the parent. When a task is given, Canopy adds one sentence to the end of it, with `<parent>` replaced by the parent's terminal address:

```text
When you are done or stuck, report to your parent with: hc send <parent> "<what you did or need>"
```

Without a task, nothing is added.

The parent waits for its helpers with `hc wait --children any`, which returns when a helper finishes a turn, blocks, fails or exits. herdr-core also types a short line into the parent when a helper exits. The wire between parent and helper glows briefly whenever the helper's state changes.

Helpers are ordinary agent panes. Everything on [Agents](Agents) and [Terminals](Terminals) applies to them.

## In-process sub-agents

Claude Code reports its built-in sub-agents to Canopy through session hooks that Canopy adds at launch. Each sub-agent appears as a pane named after its task (or its type, until the task arrives). It shows **working** while it runs, then **done**, and dims once done.

- Click a sub-agent pane to follow its read-only transcript in the [Files panel](Files-Panel).
- To talk to a sub-agent, talk to its parent. The sub-agent has no terminal of its own.
- These panes describe work, not processes. When the parent's session ends, its sub-agent panes are cleared.

## Clear finished sub-agents

- Right-click the parent pane and select **Clear finished** to remove all its done sub-agent panes.
- To remove one, select it and press `Backspace` or `Delete`.

A sub-agent that is still working cannot be removed, because closing its pane cannot stop it. Canopy shows **This sub-agent is still working. Stop it through its parent.** Stop it from the parent, then clear it.

## See also

- [Delegate to Helpers](Delegate-to-Helpers): the workflow.
- [Wiring](Wiring): the `hc` commands helpers and parents use to talk.
- [CLI](CLI): every `canopy` command.
