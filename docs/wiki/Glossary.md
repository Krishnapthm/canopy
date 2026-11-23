# Glossary

The words Canopy uses, each with one meaning. [Concepts](Concepts) explains how they fit together.

| Word | Meaning |
|---|---|
| Agent | A coding agent CLI, such as Claude Code or Codex, running in a terminal pane. Canopy lists only the agents installed on your computer. See [Agents](Agents). |
| Agent state | Working, blocked (needs you), done (finished, not yet seen) or idle. Reported by herdr-core. |
| App home | The folder with your settings, keys, layouts, mods and canvas, `~/.config/canopy` by default. See [Files and Folders](Files-and-Folders). |
| Canvas | The open, zoomable surface in a space that holds its panes. See [Canvas](Canvas). |
| Canvas mode | When the canvas has the keyboard, so single keys act on panes. The opposite of an entered pane. |
| Collapsed pane | A terminal pane shown as a small pill with its name. |
| Core | [herdr-core](herdr-core), the background program (`hc`) that runs your terminals. |
| Entered pane | The pane that has the keyboard. You enter a pane by double-clicking it. |
| Envelope | The `<pane-content>` wrapper around pane text an agent reads, marking it as data rather than a request. |
| Expanded pane | A terminal pane shown as a full, live terminal. |
| Files panel | The side panel with the file tree, previews and transcripts. See [Files Panel](Files-Panel). |
| Helper | An agent another agent started with `canopy spawn`, in its own terminal pane and wired to its parent. |
| Helper bar | The small row of buttons above a pane you hover or select. |
| Home | The view of all your spaces as cards. |
| In-process sub-agent | A sub-agent that runs inside its parent agent, shown as a read-only pane. |
| Label | A small value herdr-core stores on a terminal, shared by every app connected to the core. See [Labels](Labels). |
| Layout | A JSON file that arranges the window. See [Layouts and Mods](Layouts-and-Mods). |
| Machine | Another computer running herdr-core that Canopy reaches over SSH. See [Machines](Machines). |
| Mod | A JavaScript add-on that draws part of the window, adds a pane kind or adds commands. |
| Pane | Anything on the canvas: a terminal, agent, sub-agent, file, note, image, PDF, browser, simulator or mod pane. |
| Parent | The agent that started a helper or sub-agent. |
| Peer | A pane wired to your pane. |
| Quote | A selection from a pane, pasted into an agent's prompt with its source, without pressing `Enter`. See [Quotes](Quotes). |
| Skill | The `canopy` instructions agents load to learn Canopy. See [Agent Setup](Agent-Setup). |
| Space | A room for one project folder, with its own canvas, name and color. See [Spaces](Spaces). |
| Sub-agent | A helper or in-process sub-agent, shown beside its parent. See [Sub-Agents](Sub-Agents). |
| Terminal address | herdr-core's name for a terminal, such as `t3`, or `mac/t7` on a machine called `mac`. |
| Tile view | A temporary view that fits your panes into an even grid. |
| Toolbar | The bar at the bottom of the canvas with tools and new-pane buttons. |
| Top strip | The bar at the top of the window with Home, the space tabs and view buttons. |
| Wire | A line between two panes that introduces them as peers. It does not carry or limit messages. See [Wiring](Wiring). |
| Worktree space | A space for a separate Git worktree of a repository, on its own branch. See [Parallel Branches](Parallel-Branches). |
