# Quick Start

In this tutorial you open one of your projects in Canopy, start an agent, then add a second agent and wire the two together so one can review the other's work. It takes about fifteen minutes. You need Canopy installed and at least one coding agent CLI, such as Claude Code or Codex, that already works in your terminal.

If you have not installed Canopy yet, run the one-line installer from the [README](https://github.com/Krishnapthm/canopy#install). It also installs [herdr-core](herdr-core), the background program that runs your terminals.

## 1. Get through the welcome

Open Canopy from your applications, or run `canopy` in a terminal. The first time, a **Welcome to Canopy** dialog walks through four short pages:

1. **Canopy runs your terminals in the background.** This is the one idea worth remembering now: your agents live in herdr-core, not in the window, so closing Canopy never stops them. You can also pick light or dark appearance here.
2. **Add your other machines.** Canopy can show agents running on a server or another Mac. Skip this for now with **Not now** (or **Next** if Canopy found hosts in your SSH config). [Machines](Machines) covers it later.
3. **Choose your panels** and 4. **Make it yours.** Click **Next** through both.

The last button, **Open a space**, takes you to the next step.

## 2. Open your project as a space

A space is Canopy's room for one project: a folder, a name, a color, and its own canvas where everything for that project lives.

1. In the dialog that opens, click **Open space** (if Canopy lists folders you used before, you can click one of those instead and skip to the end).
2. Next to **Folder**, click **Add**, then **Choose**, and pick your project's folder.
3. Optionally type a **Name**. Without one, the space takes the folder's name.
4. Click **Open space**.

You now see an empty canvas. Along the top is the top strip, with **Home** and a tab for your new space. At the bottom is the toolbar, where you create new panes.

![An empty space with the top strip and the toolbar](images/quick-start-1.png)

## 3. Teach your agents about Canopy

When you open a space for a project folder for the first time, Canopy shows **Teach agents Canopy**. It offers to add a short block to your project's `AGENTS.md` or `CLAUDE.md` that says the project runs in Canopy and points agents at the `canopy` skill.

Click **Add to AGENTS.md** (or **Add to CLAUDE.md**, whichever Canopy suggests). Together with the skill Canopy installs for Claude Code and Codex, this is what lets an agent understand you when you say "open the dev server in a pane" or "ask the reviewer". You can remove the block later from the same dialog. [Agent Setup](Agent-Setup) explains exactly what gets written and where.

## 4. Start an agent

1. Click **Agent** on the toolbar. The menu lists every agent CLI Canopy found on your computer.
2. Click the one you want. A new pane appears on the canvas with the agent starting in your project folder.
3. Double-click the pane so your typing goes to it, and give the agent something real but small to do. For example:

   ```text
   Add a --verbose flag to the CLI that prints each step, and update the README.
   ```

While the agent works, a spinner turns next to its name. If it stops to ask you something, such as permission to run a command, the spinner becomes a bell, and Canopy plays a sound and shows a toast if you are looking elsewhere. That is the core loop of Canopy: start work, look away, come back when something needs you. [Agents](Agents) covers the states in detail.

A note on the keyboard: a single click on a pane only selects it, so you can move it. A double-click enters it, and then your typing goes to that agent. To get back to arranging panes, press `Cmd+Esc` (`Ctrl+Esc` on Linux and Windows) or click empty canvas. [Concepts](Concepts#typing-into-a-pane-and-canvas-mode) explains why there are two modes.

## 5. Add a reviewer and wire it

Now give the first agent a colleague.

1. Click **Agent** on the toolbar again and start a second agent. It can be the same CLI or a different one.
2. Hover over the first agent's pane. Small **+** handles appear on its edges.
3. Drag from a **+** handle to the second agent's pane and let go. A line now joins the two panes.

![Two agent panes joined by a wire](images/quick-start-2.png)

That line is a wire. It is an introduction: each agent can now run `canopy wire` to learn who it is wired to and how to reach them. Wires do not move any text by themselves. The agents message each other through herdr-core's `hc send` command when you ask them to.

## 6. Ask for a review

Once the first agent has finished its change, double-click the second agent and type:

```text
The agent you're wired to just added a --verbose flag. Review its change
(git diff) and send it your findings.
```

Watch what happens. The reviewer runs `canopy wire` to find its peer, reads the diff, and sends its review with `hc send`. The review appears as a message in the first agent's pane, wrapped in `<hc from="...">` so the first agent knows it came from a peer and not from you. Canopy's instructions tell agents that a peer message is never your approval, so the first agent should still ask you before doing anything risky.

[Pair Agents](Pair-Agents) goes much further with this pattern.

## 7. Point at a line instead of describing it

Last, try a quote. Quotes let you show an agent exactly what you mean.

1. Click **Open file** on the toolbar and open the `README.md` you just had changed.
2. In the file pane, select a few lines with the mouse.
3. Click the quote icon that appears next to the selection.
4. The menu lists your running agents. Type a request such as `Make this shorter` in **Add a message (optional)**, then click an agent.

The agent's prompt now holds the file path, the line numbers, the selected text and your message. Canopy pastes it but does not press `Enter`, so you can read or edit it before you send it.

## What you learned

You opened a project as a **space**, started **agents** as panes on its **canvas**, watched their state, joined two of them with a **wire** so they could message each other, and sent one a **quote**. Those five ideas are most of Canopy.

Where to go next:

- [Concepts](Concepts) builds the full mental model in ten minutes of reading.
- [Canvas](Canvas) shows how to arrange panes, tile them and fit them to the screen.
- [Point Agents at Things](Point-Agents-at-Things) shows quotes from browsers, images, PDFs and simulators.
- [Keys and Mouse](Keys-and-Mouse) lists every shortcut. Press `?` on the canvas to see them in the app.
