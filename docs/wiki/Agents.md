# Agents

An agent is a coding agent CLI, such as Claude Code or Codex, running in a terminal pane. Canopy runs the same CLI you would run in any terminal, in your space's folder, with your own login and settings. What Canopy adds is around it: you can see at a glance which agents are working, which are done and which are waiting for you, and agents can talk to each other and to the panes on your canvas.

## Start an agent

1. Click **Agent** on the toolbar, or press `Shift+N` in canvas mode.
2. Click an agent in the menu, or press its number (`1` to `9`).

The menu lists only the agent CLIs installed on your computer. The agent starts in a new pane, in the space's folder. Double-click the pane to type to it.

To start another copy of the selected agent, press `Cmd+Shift+T` (`Ctrl+Alt+T` on Linux and Windows).

## Know when an agent needs you

The reason to run agents in Canopy is that you can stop watching them. herdr-core tracks each agent's state, and Canopy shows it everywhere the agent appears:

![An agent pane showing it is blocked and needs you](images/agents-1.png)

| State | Meaning | Where you see it |
|---|---|---|
| Working | The agent is busy. | A spinner next to its name. A dot on its space's tab. |
| Blocked | The agent is waiting for you: a question or a permission prompt. | A bell next to its name and on its space's card on Home. **Needs you** in the agents list. |
| Done | It finished, and you have not looked at the result yet. | **Done** in the agents list. |
| Idle | Nothing new to see. | Nothing. |

When an agent becomes blocked, or finishes while you are not looking at it, Canopy also:

- shows a toast. Click it to jump to that agent.
- plays a sound, if sounds are on in [Settings](Settings-and-Themes).

"Looking at it" means its pane is on screen in the window you are using. Once you have seen a done agent, it goes back to idle.

## The agents list

The [Classic layout](Classic-Layout) has an agents list in its sidebar that shows every agent with its state. To switch to it, open the [command palette](Go-To-and-Palette) and select **Layout: Classic**.

- Click an agent to go to its pane.
- Click the sort icon beside **Agents** to choose **Priority** (grouped under **Needs you**, **Done**, **Working** and **Idle**) or **Grouped by space**.
- Hover an agent and click **Pin to the top** to keep it first.
- Click a group heading to fold or unfold it.
- Click a helper that has no pane of its own to peek at its terminal. In the peek, **↗ go to** focuses its pane and **↗ keep** keeps it as a tab.

These keys filter which terminal panes the canvas shows:

| macOS | Linux and Windows | Shows |
|---|---|---|
| `Cmd+Shift+1` | `Ctrl+Shift+1` | Agents only |
| `Cmd+Shift+2` | `Ctrl+Shift+2` | All terminal panes |
| `Cmd+Shift+3` | `Ctrl+Shift+3` | Terminal panes without agents |

## Which agents Canopy knows

Canopy recognizes agent CLIs from a built-in table, `programs.json`. It includes Claude Code, Codex, Pi, PiG, OpenCode, Cursor, Copilot, Amp, Gemini, Grok, DeepSeek Harness, Antigravity, Kimi, Kiro, Hermes, Qoder and Crush. Any of them that is installed shows up in the **Agent** menu.

Every agent works in Canopy as a terminal program with states and notifications. Claude Code and Codex get more, because Canopy can pass them flags at launch:

| | Claude Code | Codex | Other agents |
|---|---|---|---|
| States and notifications | Yes | Yes | Yes |
| `canopy` skill installed | Yes | Yes | No |
| Launch note about Canopy | Yes | Yes | No |
| Told when wires change | Yes | No, runs `canopy wire` | No |
| In-process sub-agents shown | Yes | No | No |

Other agents learn about Canopy from the project block in `AGENTS.md` (see [Agent Setup](Agent-Setup)), or from you.

### Add your own agent

Add an entry inside `programs` in `programs.json` in your [app home](Files-and-Folders):

```json
"my-agent": { "name": "My Agent", "run": ["my-agent"] }
```

`run` is the command that starts it. Add `match` with other process names if the program shows up under a different name once running.

## Troubleshooting

- **The Agent menu says No agents installed.** Canopy cannot find any CLI from its table on your `PATH`. Install one, or check the `run` command of your entry in `programs.json`.
- **My shell alias does not appear in the menu.** The launcher does not expand shell aliases. Use a program or a script on your `PATH`, or a shell function.
- **The agents list says No agent status yet.** Agents appear once they report a state. Older agent versions, and agents running inside tmux, may never report one.

## See also

- [Terminals](Terminals): everything about the pane an agent runs in.
- [Agent Setup](Agent-Setup): how agents learn about Canopy.
- [Pair Agents](Pair-Agents) and [Delegate to Helpers](Delegate-to-Helpers): agents working together.
