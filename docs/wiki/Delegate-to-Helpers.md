# Delegate to Helpers

Big tasks often split into pieces that can run in parallel: research three libraries, fix the tests in four packages, write the backend and frontend of a feature at the same time. Agents can already start sub-agents, but usually those run invisibly inside the parent, and you only see the summary at the end. Canopy puts helpers on the canvas where you can watch them, talk to them and stop them.

This guide covers the two kinds of helpers and how to work with each.

## Two kinds of helpers

| | Visible helpers | In-process sub-agents |
|---|---|---|
| Started by | The agent running `canopy spawn` | The agent's own sub-agent feature |
| Runs in | Its own terminal pane | Inside the parent agent |
| You can | Enter it, type to it, stop it | Follow its transcript, read only |
| Talks back | `hc send` to the parent | Through the parent |
| Available for | Any agent Canopy knows | Claude Code |

Use visible helpers for real work you might want to steer. In-process sub-agents are fine for quick lookups the parent handles on its own.

## Ask for visible helpers

You do not need to know the command. Ask the agent in plain words, and say how many helpers and what each should do:

```text
Start three helpers, one per package (api, web, cli). Each should fix the
failing tests in its package and report back. When all three are done,
run the full test suite and summarize.
```

An agent that knows Canopy starts each helper with `canopy spawn`:

```sh
canopy spawn claude --name api-tests -- Fix the failing tests in packages/api.
```

What happens next:

1. A new agent pane appears on the canvas next to the parent, in the same space and folder, already wired to it.
2. The helper receives its task, with one sentence added at the end telling it how to report back:

   ```text
   When you are done or stuck, report to your parent with: hc send t4 "<what you did or need>"
   ```

3. The parent waits with `hc wait --children any`, which returns each time a helper finishes, blocks or exits.
4. When a helper's status changes, the wire to its parent glows briefly, so you can see activity at a glance.

![An agent with two helpers beside it, joined by wires](images/sub-agents-1.png)

The program name after `canopy spawn` is any agent Canopy knows (`claude`, `codex` and so on), so a Claude Code parent can start Codex helpers, or the other way round. Add `--model <model>` to choose a model for the helper.

## Work with your helpers

Helpers are ordinary agent panes, so everything you know about agents applies:

- **Watch.** Each helper shows a spinner while it works and a bell when it needs you. Press `Cmd+Shift+G` (`Ctrl+Shift+G` on Linux and Windows) to tile them, or right-click the canvas and choose **Arrange as** > **By wires** to line up each parent to the left of its helpers.
- **Steer.** Double-click a helper and type to it directly: "Skip the flaky snapshot test, it's tracked elsewhere."
- **Stop.** Enter the helper and use the agent's own interrupt key, or close its pane. Closing an agent always asks for confirmation.

When a helper exits, herdr-core also types a short line into the parent so it knows, even if it was not waiting.

## Follow in-process sub-agents

When Claude Code starts one of its built-in sub-agents, Canopy shows it as a small pane next to the parent, named after its task. It shows **working** while it runs, then **done**, and dims when it finishes.

- Click a sub-agent pane to follow its read-only transcript in the [Files panel](Files-Panel).
- To ask it something, talk to the parent. The sub-agent has no terminal of its own.
- When you have a pile of finished ones, right-click the parent and select **Clear finished**. To remove one, select it and press `Backspace`.

A sub-agent that is still working cannot be removed, because its pane cannot stop it. Stop it through the parent.

## Tips

- **Say how to finish.** "When all helpers are done, run the tests and summarize" gives the parent a clear end. Without it, the parent may stop after starting them.
- **Mind the cost.** Every helper is a full agent session using tokens. Three helpers cost roughly three times as much as one.
- **Keep work separate.** Helpers share the parent's folder. Give them separate files or packages, or have them work in [separate worktrees](Parallel-Branches) if they could step on each other.

## See also

- [Sub-Agents](Sub-Agents): the reference for helper and sub-agent panes.
- [Pair Agents](Pair-Agents): two agents you start yourself, as equals.
- [Wiring](Wiring): the messaging commands helpers use.
