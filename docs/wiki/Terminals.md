# Terminals

A terminal pane runs a shell or any program, exactly like a terminal window, but it lives on the canvas next to everything else. Agents are terminal panes too, so everything on this page also applies to them. [Agents](Agents) covers what is specific to agents.

Terminal panes come in two sizes:

- **Expanded:** a full, live terminal you can read and type into. Use this for things you are watching or working in.
- **Collapsed:** a small pill with the terminal's name. Use this for things that run in the background, like a dev server or a long build, so they stay on the canvas without taking up room. A collapsed agent still shows its spinner or bell.

## Open a terminal

- Click **Terminal** on the toolbar, press `n` in canvas mode, or press `Cmd+T` (`Ctrl+Shift+T` on Linux and Windows).
- To start another copy of what the selected pane runs, such as a second Claude Code, press `Cmd+Shift+T` (`Ctrl+Alt+T` on Linux and Windows). If that program cannot be started again, you get a plain terminal.

New terminals start in the space's folder and appear near the last one you opened.

## Enter a terminal and type

Clicking a terminal only selects it, so you can move it around without typing into it. To type:

1. Double-click the pane, or select it and press `Enter`.
2. Type as usual. Every key goes to the program, including `Esc`, and the mouse wheel scrolls its output.
3. When you are done, press `Cmd+Esc` (`Ctrl+Esc` on Linux and Windows) or click empty canvas to give the keyboard back to the canvas.

While you are in a terminal, the wheel over *other* terminals still pans the canvas, and `Cmd`+scroll (`Ctrl`+scroll) still zooms.

## Expand, collapse and resize

- Select a terminal and press `Cmd+Enter` (`Ctrl+Shift+Enter` on Linux and Windows) to switch between expanded and collapsed. In canvas mode, `x` collapses the selected terminal.
- Or right-click the terminal and choose **Expand** or **Collapse**.
- Drag an expanded pane's edge or corner to resize it. By default, the program's columns and rows refit to the new size.

You can keep as many terminals expanded as you like.

## Rename a terminal

A terminal is named after its program: agents show their program name (such as **Claude Code**), shells show `terminal`, and other commands show their name without flags. To give one a better name, such as `dev server`:

- Double-click the name on an expanded pane, type, and press `Enter`. `Esc` cancels.
- Or right-click and choose **Rename**, which also works on collapsed panes.

## The terminal menu

Right-click a terminal's name, header or text for its menu:

| Item | What it does |
|---|---|
| **Rename** | Changes the name. |
| **Pop out**, **Bring back** | Moves the terminal into its own window, and back onto the canvas. |
| **Expand**, **Collapse** | Switches size. Hidden while the terminal is popped out. |
| **Bring to front**, **Send to back** | Changes the stacking order when panes overlap. |
| **Clear finished** | Removes finished sub-agent panes. Shown when there are some. See [Sub-Agents](Sub-Agents). |
| **Close terminal** | Closes it, asking first. See [Panes](Panes#close-panes). |
| **Kill terminal** | Ends the terminal and its sub-agents immediately, without asking. Helpers kept in their own tabs keep running. |

The helper bar on a terminal has **Enter**, **Expand** or **Collapse**, **Connect** and **Close**.

## Pop out a terminal

Sometimes you want one terminal in its own window, for example on a second monitor. Choose **Pop out** from the terminal menu. The pane on the canvas shows that it is popped out. Choose **Bring back**, or close the window, to return it to the canvas.

## Text size

Enter a terminal, then press `Cmd+=`, `Cmd+-` or `Cmd+0` (`Ctrl+=`, `Ctrl+-` or `Ctrl+0` on Linux and Windows) to make that terminal's text larger, smaller or default. The program refits its columns and rows. Text size resets when Canopy restarts.

## After a restart

Because [herdr-core](herdr-core) runs your terminals, they are still there when you reopen Canopy. Reattached `zsh`, `bash`, `fish` and `sh` shells show a clean screen and redraw their prompt instead of replaying old output. Agents and other programs are left exactly as they were.

## Settings

Edit these in [settings.json](Settings-and-Themes):

| Setting | Default | What it does |
|---|---|---|
| `terminal_size` | `[100, 30]` | Columns and rows for new expanded terminals and for `=`. Columns 20 to 500, rows 5 to 200. |
| `canvas_resize_terminals` | `true` | Refits columns and rows when you resize a pane. With `false`, resizing scales the existing text instead. |
| `confirm_close_terminal` | `true` | Asks before closing a plain terminal. Agents always ask. |

## Troubleshooting

- **Scrolling moves the canvas instead of the output.** The terminal is not entered. Double-click it first.
- **Typing does nothing.** The terminal is selected but not entered. Double-click it, or press `Enter`.
- **Resizing makes the text bigger instead of adding columns.** Set `canvas_resize_terminals` to `true`.

## See also

- [Agents](Agents): agent states, notifications and the agents list.
- [Canvas](Canvas#sync-input): typing into several terminals at once.
- [Wiring](Wiring#talk-to-peers): reading or messaging another terminal with `hc`.
