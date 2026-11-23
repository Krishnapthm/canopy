# Canopy

Canopy is a desktop app for running several coding agents at once. Your terminals and agents sit side by side on one canvas, Canopy tells you which one is waiting for you, and the agents can talk to each other and to the things on your screen.

![A canvas with agents, a terminal, a browser pane and a simulator](images/home-1.png)

## Why Canopy

Once you run more than one coding agent, the terminal gets in the way. Agents hide in tabs, one sits blocked on a question for twenty minutes before you notice, and every time you want an agent to look at something (a line of code, a broken button, a screenshot) you copy and paste it by hand.

Canopy is built around that workflow:

- **See everything at once.** Each project gets a space: a canvas where terminals, agents, files, notes, web pages and iOS simulators are all panes you can arrange however you like.
- **Know who needs you.** Every agent shows whether it is working, waiting for you or done, and Canopy tells you with a sound and a toast when one needs attention.
- **Let agents work together.** Draw a wire between two agents and they can message each other. Ask one to review the other's work, or to start visible helpers that report back when they finish.
- **Point instead of paste.** Select lines in a file, a region of an image, an element on a web page or a spot on a simulator screen, and quote it into an agent's prompt.
- **Nothing dies with the window.** Your terminals run in [herdr-core](herdr-core), a small background program. Close Canopy, update it, or let it crash, and your agents keep working.

Canopy runs the agent CLIs you already use, such as Claude Code and Codex, unchanged. It never sees your API keys, and the app itself sends no telemetry and checks for no updates.

## Where to start

**New to Canopy?** Install it with the one-line command in the [README](https://github.com/Krishnapthm/canopy#install), then follow the [Quick Start](Quick-Start). In about fifteen minutes you will have two agents on a canvas, one reviewing the other's work. Read [Concepts](Concepts) next for the ideas behind it.

**Have something specific to do?** The guides walk through the workflows Canopy is built for:

- [Pair two agents](Pair-Agents) so one writes and one reviews.
- [Point agents at things](Point-Agents-at-Things) on your screen instead of describing them.
- [Let agents show you their work](Let-Agents-Show-You) in a browser, a file or a simulator.
- [Delegate to helpers](Delegate-to-Helpers) you can watch and talk to.
- [Work on parallel branches](Parallel-Branches) with one space per Git worktree.

**Looking something up?** Try [Keys and Mouse](Keys-and-Mouse), the [CLI](CLI), [Settings and Themes](Settings-and-Themes) or the [Glossary](Glossary). If something is wrong, start at [Troubleshooting](Troubleshooting).

## Platforms

Canopy runs on macOS (Apple Silicon) and Linux. Browser panes and simulator panes are macOS only. Windows builds from source but has no installer.
