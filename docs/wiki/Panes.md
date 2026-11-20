# Panes

A pane is anything on the canvas: a terminal, agent, file or another kind below.

## Pane kinds

- [Terminal](Terminals): a shell or program running in a terminal.
- [Agent](Agents): a coding agent running in a terminal pane.
- [Sub-agent](Sub-Agents): a helper shown beside its parent agent.
- [File](File-and-Note-Panes): a text file preview.
- [Note](File-and-Note-Panes): plain text you type into a pane.
- [Image](Images-and-PDFs): an image file shown on the canvas.
- [PDF](Images-and-PDFs): a document you can read, search and highlight.
- [Browser](Browser-Pane): a web page on the canvas (macOS only).
- [Simulator](Simulator-Devices): an iOS Simulator on the canvas (macOS only).
- [Mod pane](Layouts-and-Mods): a pane drawn by an installed mod.

## Open a pane

- Click a pane kind on the [toolbar](Toolbar-and-Helper-Bar).
- Press `Cmd+O` (`Ctrl+O` on Linux and Windows) to choose a file.
- Drag a file onto the canvas to open it there.
- Ask an agent to open a pane with [`canopy pane`](Agent-Pane-Commands).

Canopy chooses a text, image or PDF pane from the file's extension.

## Select, close and stack panes

See [Canvas](Canvas) for selection and movement.

Terminal actions use selected terminal, agent and sub-agent panes in the current space.
Without a selection there, they use the terminal pane holding keyboard focus, if any.
Space cards and other pane kinds are not terminal action targets.

1. Give the canvas the keyboard.
2. Press `Backspace` or `Delete` to close the target panes.
3. Click **Terminate and close all** to stop the listed terminals and their programs.

The dialog lists each terminal with its icon and name.
Click **Cancel** or press `Esc` to cancel terminal closing.
Selected file, image, PDF, browser and simulator panes close without a dialog, even when you cancel terminal closing.

See [File and Note Panes](File-and-Note-Panes) for note closing and [Sub-Agents](Sub-Agents) for sub-agent closing.

`confirm_close_terminal` defaults to `true` in [settings](Settings-and-Themes).
Set it to `false` to close plain terminals without a prompt. Agents still require confirmation.

Closing a plain terminal through its own menu offers **Don't ask again**, which saves this setting as `false`.

Click a pane to bring it to the front. Canopy saves the stacking order.
See [Canvas](Canvas) for **Bring to front** and **Send to back**.

## Saved panes

`canvas.json` keeps each pane's ID, kind, position, size, stacking order, space and kind-specific properties.
These properties include file paths, note text and PDF highlights.
See [Files and Folders](Files-and-Folders) for the file's location.

## How it works

Pane IDs identify the same panes in commands and wires:

- Terminal and agent panes use their core addresses, such as `t3`.
- Sub-agent panes use `sub:<parent>:<id>`.
- All other pane kinds use `p<N>`, such as `p1`.

Canopy keeps a saved counter so closing a pane does not reuse its `p<N>` ID.
