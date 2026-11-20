# File and Note Panes

File panes show text files on the canvas. Note panes hold text you type for your agents to read.

## File panes

1. Click **Open file** on the toolbar.
2. Select a text file.

In canvas mode, press `Cmd+O` (`Ctrl+O` on Linux and Windows) to open the same file picker.

An agent can open a file beside itself and wire it:

```sh
canopy pane new file ./README.md --wire
```

Opening a path that already has a pane moves that pane instead of creating another.
Moving it to another space removes its existing wires.

The file pane helper bar has three buttons:

- **Open as tab** opens the file in the [Files panel](Files-Panel) for editing.
- **Connect** starts a wire. Select an agent pane to wire the file to it.
- **Close** closes the pane without a dialog.

Wire a file to an agent so it can find the file as a peer.
See [Wiring](Wiring) for wires and [Agent Pane Commands](Agent-Pane-Commands) for reading pane text.
See [Images and PDFs](Images-and-PDFs) for those file types.

## Note panes

1. Click **Note** on the toolbar.
2. Type in the pane.

Canopy saves each edit in `canvas.json` in the app home.
Wire the note to an agent as you wire a file.

> **Warning:** Closing a note deletes its text. Canopy asks before closing notes with text other than whitespace.

## Troubleshooting

- **An agent cannot close a note with text.** Only you can close it. Empty notes and notes with only whitespace can close without confirmation.
