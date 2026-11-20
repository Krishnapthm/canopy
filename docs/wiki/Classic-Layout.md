# Classic Layout

The Classic layout arranges the window like an editor. Spaces and agents are on the left, tabs are above the panes, and a status bar is at the bottom.

## Switch to Classic

1. Open the [command palette](Go-To-and-Palette).
2. Select **Layout: Classic**.

See [Layouts and Mods](Layouts-and-Mods) for the other layouts and for how to change one.

## The sidebar

The sidebar has the **Spaces** list on top and the agents list below it.
See [Agents](Agents#the-agents-list) for the agents list.

- Click a space to open it. Click **+** beside **Spaces** to create a space.
- Drag a space onto another space to reorder them. A line shows where it lands.
- Drag a space only within its own machine.
- Right-click a space for its menu. [Spaces](Spaces#worktrees) describes worktrees.
- Once you add a machine, a header per machine groups its spaces. **This computer** comes first.
- Click a machine header to fold its spaces. Click **+** on the header to create a space on that machine.
- Click **Add machine** at the bottom to add a machine. See [Machines](Machines). **Settings** is above it.

- Press `Cmd+B` (`Ctrl+Shift+B` on Linux and Windows) to hide or show the sidebar.
- Click the sidebar button in the top row to narrow the sidebar to a rail. Click it again to widen it.
- The rail shows each space as a number. A dot marks a space with a blocked or done agent.
- On macOS the rail has no sidebar button. Click the **»** on the agents list to widen it.

## Tabs and panes

Each space has tabs above its panes. A tab shows the program's icon and its name.

A working agent shows a spinner on its tab. A blocked or done agent shows a dot.

- Click **+** to open a new tab. Select **Terminal** or an agent. Press `Cmd+T` (`Ctrl+Shift+T` on Linux and Windows) for the same.
- Click a tab to show it. Scroll over the tabs to move to the next or previous tab.
- Double-click a tab or press `F2` to rename it. Close the active tab with its close icon or with `Cmd+W` (`Ctrl+Shift+W` on Linux and Windows).
- Right-click a tab for **Rename**, **New tab** and **Close**.
- Drag a tab onto another tab to move it to that position.
- **Files** and **Changes** at the right of the tab row open the side panel. See [Files Panel](Files-Panel).

Hover a pane header to show four icons: split right, split down, zoom and close.

- Press `Cmd+D` to split right and `Cmd+Shift+D` to split down. On Linux and Windows press `Ctrl+Shift+O` and `Ctrl+Shift+E`.
- Zoom shows only that pane of the tab. A zoomed tab shows **Z** after its name.
- Press `Cmd+Shift+Enter` (`Ctrl+Shift+Z` on Linux and Windows) to zoom.
- The pane header also shows the folder, and **Needs you** when the agent is blocked.

## The status bar

From left to right, the status bar shows:

- The machine of the open space, or **This computer**.
- The branch of the space, when it has one.
- The number of agents, and **· N needs you** when agents are blocked.
- The palette key, with the word **commands**. Click it to open the command palette.
