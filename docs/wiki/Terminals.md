# Terminals

A terminal pane runs a shell or program, collapsed with a name or expanded with live terminal text.

## Open a terminal

- Click **Terminal** on the toolbar, or press `Cmd+T` (`Ctrl+Shift+T` on Linux and Windows).
- Press `Cmd+Shift+T` (`Ctrl+Alt+T` on Linux and Windows) to start the selected pane's program again.
- If that program has no available launch command, this opens a plain terminal instead.
- See [Agents](Agents) for agent-specific behavior.

Right-click a terminal's pill, header or text for its pane menu:

| Item | Action |
|---|---|
| **Rename** | Changes the name, when the pane has a tab. |
| **Pop out**, **Bring back** | Moves the terminal between the canvas and its own window. |
| **Expand**, **Collapse** | Changes between an expanded pane and a collapsed pane. Hidden while the terminal has its own window. |
| **Clear finished** | Appears when finished sub-agents exist. See [Sub-Agents](Sub-Agents). |
| **Close terminal** | Uses the closing rules in [Panes](Panes). |
| **Kill terminal** | Ends the terminal and its sub-agents immediately, except sub-agents kept in their own tabs. |

A custom name takes priority, except generic names such as `terminal` or the program name.
Otherwise, agents show their program name, shells show `terminal`, and commands show their name without flags. Agent session titles stay hidden.

- Double-click an expanded pane's name to rename it.
- Press `Enter` or click elsewhere to save. Press `Esc` to cancel.
- Use **Rename** for a collapsed pane.

## Expand and collapse

- Select **Expand** or **Collapse** in the pane menu.
- Press `Cmd+Enter` (`Ctrl+Shift+Enter` on Linux and Windows) to toggle the selected terminal.
- Drag an expanded pane's edge or corner to resize it.
- Several terminals can stay expanded at once.

The helper bar has **Enter**, **Expand** or **Collapse**, **Connect**, and **Close**.
See [Wiring](Wiring) for **Connect** and [Panes](Panes) for **Close**.

## Enter a terminal

1. Double-click the pane, or select it and press `Enter` in canvas mode.
2. Type in the terminal.
3. Press `Cmd+Esc` (`Ctrl+Esc` on Linux and Windows) to return the keyboard to the canvas.

- `Esc` stays with the entered terminal.
- The wheel scrolls an entered terminal. Over other terminals, it pans the canvas.
- Hold `Cmd` (`Ctrl` on Linux and Windows) while scrolling to zoom the canvas.
- See [Canvas](Canvas) for sync input.

## Text zoom

- Enter a terminal first.
- Press `Cmd+=`, `Cmd+-` or `Cmd+0` (`Ctrl+=`, `Ctrl+-` or `Ctrl+0` on Linux and Windows).

These keys enlarge, reduce or reset that terminal's text. The program's columns and rows refit. Text zoom resets when the app restarts.

## Pop out

- Select **Pop out** in the pane menu to open the terminal in its own window.
- Select **Bring back** to return it to the canvas.
- Closing that window also returns the terminal to the canvas.

## Restored shells

After restart, reattached `zsh`, `bash`, `fish` and `sh` shells show a clean screen and redraw their prompt.
Canopy clears the replayed screen without typing anything, leaving agents and other programs unchanged.
See [herdr-core](herdr-core) for why terminals survive the window.

## Settings

Edit these in [settings.json](Settings-and-Themes):

- `terminal_size`: new expanded pane size in columns and rows. Default: `[100, 30]`. Limits: `[20..500, 5..200]`.
- `canvas_resize_terminals`: `true` refits columns and rows after resizing. With `false`, resizing scales the existing grid to fit.

## Troubleshooting

- **Scrolling moves the canvas.** Enter the terminal first.
- **Resizing scales the text.** Set `canvas_resize_terminals` to `true` to refit columns and rows instead.
