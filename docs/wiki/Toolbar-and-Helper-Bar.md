# Toolbar and Helper Bar

Canopy has two kinds of button bars. The **toolbar** at the bottom of the canvas creates panes and switches tools. The **helper bar** appears on panes with actions outside their header. Between them, nearly everything has a button, and every button shows its keyboard shortcut when you hover it.

The sticky note's helper bar holds its move grip and color swatch.

## Toolbar

![The toolbar at the bottom of the canvas](images/toolbar-and-helper-bar-1.png)

The toolbar floats over the bottom center of the canvas, beside the files panel when it is open. The canvas runs underneath it to the window edge, so panes can pan behind it. Drag the toolbar frame to move it to another canvas edge.

From left to right, or top to bottom on a side edge:

| Button | What it does |
|---|---|
| **Select** | The normal pointer: click to select, drag to move. |
| **Hand (pan)** | Drag anywhere with the left button to pan. Click **Select** to go back. |
| **Terminal** | Opens a [terminal](Terminals) in the current space. |
| **Agent** | Opens a menu of the agent CLIs installed on your computer. Pick one to start it. See [Agents](Agents). |
| **Open file** | Opens a [file](File-and-Note-Panes), [image or PDF](Images-and-PDFs) as a pane. |
| Mod pane kinds, such as **Note** | Opens that kind of pane. Each installed [mod pane kind](Layouts-and-Mods#mod-panes) gets a button. |
| **Connect (select a pane first)** | Starts a wire from the selected pane. Select a pane before you click it. |

**Browser** and **Simulator** sit in the bar at the top right of the window, beside **Tile view** and **Files**. They open the pane docked as a tab in the [files panel](Files-Panel). Choose **Move to canvas** from the browser’s **⋯** menu or from the simulator’s helper bar to place it on the canvas. On the canvas, **Open in panel** docks it again, and **Pin to screen** pins it to the top right corner while you pan and zoom (**Unpin** undoes it). **Browser** shows on macOS only, and **Simulator** only on a Mac where Xcode's `xcrun simctl` is installed.

To wire from the toolbar: select a pane, click **Connect**, use the arrow keys to choose the other pane, and press `Enter` (or `Esc` to cancel). Dragging a pane's **+** handle does the same with the mouse. See [Wiring](Wiring).

The first time you open a space for a local project folder, the toolbar opens **Teach agents Canopy**. See [Agent Setup](Agent-Setup).

## Helper bar

![A helper bar above a terminal pane](images/toolbar-and-helper-bar-2.png)

Hover or select a pane to see its helper bar near its top-left corner. If another pane or the canvas edge is in the way, Canopy tries the other sides. Right-click the pane for its actions if no side has enough room. The bar hides while you drag, resize, draw a selection box or select an image region.

Every pane kind's bar ends with the same two buttons:

- **Connect** starts a wire. A line follows the pointer. Click another pane to connect, or press `Esc` or click empty canvas to cancel. **Connect** stays highlighted while the pane has any wire, including the automatic one to a parent agent. In the right-click menu, **Connect** starts the keyboard [wiring](Wiring#draw-a-wire) flow.
- **Close** closes the pane, with the same rules as anywhere else ([Panes](Panes#close-panes)).

The other buttons depend on the kind. Each kind's page lists them:

- [Terminals](Terminals): enter, expand or collapse.
- [File and Note Panes](File-and-Note-Panes): open as a tab.
- [Images and PDFs](Images-and-PDFs): quote, find, highlight, pages.
- [Simulator Devices](Simulator-Devices): zoom, pop out, choose simulator and open Device Hub. The device actions appear in a bottom bar inside the pane, hidden while annotating.
- Mod panes show the actions their mod declares. See [Layouts and Mods](Layouts-and-Mods#mod-panes).

Browser actions are in the [browser header](Browser-Pane#the-header). Sub-agent panes have no helper bar.

## Shortcut hints

- Hover any toolbar or helper bar button to see its name and its shortcut, when it has one.
- Hold `Cmd` (`Ctrl` on Linux and Windows) for a moment to show every shortcut next to its button at once. See [Keys and Mouse](Keys-and-Mouse#show-keys).
