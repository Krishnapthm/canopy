# Agents

An agent is a coding agent CLI running in a terminal pane.
The agents list helps you find the agent that needs you.

## Start an agent

1. Click **Agent** on the toolbar, or press `Shift+N` in canvas mode.
2. Click an agent, or press its number (`1` to `9`) in the menu.

The agent menu lists only installed CLIs. In canvas mode, `n` starts a terminal.
See [Agent Setup](Agent-Setup) for the canopy skill and [Wiring](Wiring) for connecting agents.

## See which agent needs you

![An agent pane showing it is blocked and needs you](images/agents-1.png)

| State | What you see in the agents list |
|---|---|
| Working | A spinner. The agent is working. |
| Blocked | **Needs you**, or **!** under that heading. The agent needs your input. |
| Done | A filled dot. The agent finished, and you have not seen the result. |
| Idle | A ring. The agent has no unseen result. |

A blocked agent also shows a bell on its collapsed pane.
Canopy plays a sound when an agent needs you, or finishes while you are not looking, if sounds are enabled.

When you are not looking at that agent, a toast also appears. Click the toast to focus the agent.
See [Settings and Themes](Settings-and-Themes) for sounds and [Labels](Labels) for seen and pinned labels.

## The agents list

The agents list is in the sidebar of the **Classic** layout. Select **Layout: Classic** in the [command palette](Go-To-and-Palette) to use it.

- Click an agent in the sidebar to focus its pane.
- Click the sort icon beside **Agents** to select **Priority** or **Grouped by space**.
- **Priority** orders agents under **Needs you**, **Done**, **Working** and **Idle**.
- Hover an agent to show **Pin to the top**. Pinned agents appear first.
- Click a group heading to fold or expand it.
- Click a helper without its own pane to peek at its terminal.
- In the peek, click **↗ go to** to focus its pane, or **↗ keep** to keep it as a tab.

See [Sub-Agents](Sub-Agents) for helper agents.

Use these keys to filter terminal panes on the canvas:

| macOS | Linux and Windows | View |
|---|---|---|
| `Cmd+Shift+1` | `Ctrl+Shift+1` | Agents |
| `Cmd+Shift+2` | `Ctrl+Shift+2` | All terminal panes |
| `Cmd+Shift+3` | `Ctrl+Shift+3` | Terminal panes without agents |

## Supported agents

Canopy detects installed CLIs from its `programs.json` table.
Built-in entries include Claude Code, Codex, Pi, PiG, OpenCode, Cursor, Copilot, Amp, Gemini and Grok.
They also include DeepSeek Harness, Antigravity, Kimi, Kiro, Hermes, Qoder and Crush.

Add an entry inside `programs` in your app home's `programs.json`, for example `"my-agent": { "name": "My Agent", "run": ["my-agent"] }`.
Use `run` for the launch command and `match` for other process names.

## Troubleshooting

- **The agent menu says No agents installed.** Canopy cannot find a CLI from its table. Install one or check its `run` command.
- **The agents list says No agent status yet.** Agents appear when they report status. Older versions and agents inside tmux may not report it.
- **An alias does not appear in the menu.** The launcher does not expand aliases. Use a program or shell function.
