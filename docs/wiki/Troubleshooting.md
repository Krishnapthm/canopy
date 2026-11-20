# Troubleshooting

Find a fix for a problem, or collect information for a bug report.

## Start here

Read the troubleshooting section for the feature:

- [Machines](Machines)
- [Terminals](Terminals)
- [Agents](Agents)
- [File and Note Panes](File-and-Note-Panes)
- [Images and PDFs](Images-and-PDFs)
- [Browser Pane](Browser-Pane)
- [Simulator Devices](Simulator-Devices)
- [herdr-core](herdr-core)

## The window

- **Canopy does not open.** Check the [install instructions](https://github.com/Krishnapthm/canopy#install) and [core troubleshooting](herdr-core).
- **Settings errors appear.** See [Settings and Themes](Settings-and-Themes) for settings file errors.
- **The Mac display was asleep.** Wake the display. Canopy resumes drawing when the window becomes visible.

## Terminals and the core

- **The core is missing or too old.** Follow [herdr-core](herdr-core) for the instructions shown before the main window opens.
- **Canopy cannot connect to the core.** Resolve the displayed error, then click **Retry**.
- **`canopy` in a terminal opens another copy.** Check that the terminal uses the same [app home](Files-and-Folders).
  On Unix, use `canopy` by name because an absolute path bypasses the handoff to the terminal's app.

See [Terminals](Terminals) for scrolling and resizing problems.

## Agents

- **An agent CLI does not appear.** See [Agents](Agents) for command detection, `programs.json` and aliases.

## Panes

See the pane pages under **Start here** for file, note, image, PDF, browser and simulator problems.
See [Agent Pane Commands](Agent-Pane-Commands) for reading and closing panes from an agent.

## Get help

1. Keep the Canopy window open if it can open.
2. Run `canopy status` in a terminal that uses the same app home.
3. Copy the JSON output into a [bug report](https://github.com/Krishnapthm/canopy/issues/new?template=bug.yml).
4. Include the steps to reproduce, expected behavior, Canopy version and operating system.

`canopy status` reports the window state and errors. It needs a running window.
If the window cannot open, include the displayed error instead.
