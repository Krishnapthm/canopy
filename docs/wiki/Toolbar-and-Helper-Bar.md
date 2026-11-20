# Toolbar and Helper Bar

The toolbar holds canvas tools and buttons for new panes.
A helper bar holds actions for a pane.

## Toolbar

![The toolbar at the bottom of the canvas](images/toolbar-and-helper-bar-1.png)

The main canvas layout places the toolbar at the bottom center.
Its buttons appear in this order:

| Button | Use |
|---|---|
| **Select** | Turns the Hand tool off. |
| **Hand (pan)** | Turns the Hand tool on. |
| **Terminal** | Opens a [terminal](Terminals). |
| **Agent** | Opens the [agent menu](Agents). |
| **Open file** | Opens a [file pane](File-and-Note-Panes), [image or PDF](Images-and-PDFs). |
| **Browser** | Opens a [browser pane](Browser-Pane). Shows on macOS only. |
| **Simulator** | Opens a [simulator](Simulator-Devices). Shows only on macOS with Xcode's `xcrun simctl`. |
| Mod pane kinds, such as **Note** | Opens the corresponding pane. Only installed [mod pane kinds](Layouts-and-Mods) appear. |
| **Connect (select a pane first)** | Starts a connection from the selected pane. |

- With the Hand tool on, drag with the left mouse button anywhere on the canvas to pan.
- Click **Select** to turn the Hand tool off.
- Before clicking **Connect (select a pane first)**, select a pane.
- Press the arrow keys to choose another pane. Press `Enter` to connect, or `Esc` to cancel.

The toolbar can offer **Teach agents Canopy** for a local project. See [Agent Setup](Agent-Setup).

## Helper bar

![A helper bar above a terminal pane](images/toolbar-and-helper-bar-2.png)

- Hover or select a pane to show its helper bar above its top-left corner.
- Sub-agent panes have no helper bar.
- Bars hide during dragging, resizing, box selection, image region selection or a pending layout.
- A bar stays visible while its clicked wire follows the pointer.

To use **Connect** in a helper bar:

1. Click **Connect**. A wire follows the pointer.
2. Click another pane to connect. Press `Esc` or click empty canvas to cancel.

**Connect** stays highlighted while its wire follows the pointer or the pane has a wire, including a parent connection.
See [Wiring](Wiring) for what a wire does.

Each pane kind owns its other helper bar buttons:

- [Terminals](Terminals), including agent terminals.
- [File and Note Panes](File-and-Note-Panes).
- [Images and PDFs](Images-and-PDFs).
- [Browser Pane](Browser-Pane).
- [Simulator Devices](Simulator-Devices).
- [Layouts and Mods](Layouts-and-Mods) for mod-declared actions.

## Shortcut hints

- Hover a button to see its name and its shortcut, when assigned.
- To show every key beside its button at once, see [Show keys](Keys-and-Mouse#show-keys).
