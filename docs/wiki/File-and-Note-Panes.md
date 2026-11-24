# File and Note Panes

File panes and note panes put text on the canvas next to your agents. A **file pane** shows a file from disk, such as the module you are discussing or a plan an agent wrote. A **note pane** holds text you type straight into the canvas, such as a checklist, a plan or instructions you want several agents to share.

Both can be wired to agents, which lets those agents find and read them.

## File panes

### Open a file

- Click **Open file** on the toolbar, or press `Cmd+O` (`Ctrl+O` on Linux and Windows) in canvas mode, and pick a file.
- Drag a file onto the canvas.
- Ask an agent to open one next to itself. It runs:

  ```sh
  canopy pane new file ./README.md --wire
  ```

If the file already has a pane, Canopy moves that pane to you instead of opening a second copy. Moving a file pane to another space removes its wires, because wires only join panes in the same space.

### What you can do with it

- **Read it.** Scroll through the file's text right on the canvas.
- **Quote from it.** Select lines and click the quote icon to send them to an agent with the path and line numbers. See [Point Agents at Things](Point-Agents-at-Things#lines-in-a-file).
- **Edit it.** Click **Open as tab** in the helper bar to open the file in the [Files panel](Files-Panel), where you can edit and save it.
- **Wire it.** Click **Connect** and then an agent. The agent then sees the file in `canopy wire` and can read it with `canopy pane read`. A file can be wired to terminals and agents, but not to another file.

**Close** in the helper bar closes the pane without asking. The file on disk is untouched.

## Note panes

### Make a note

1. Right-click empty canvas and choose **New Sticky Note**, click **Note** on the toolbar, or press `Option+Cmd+N` (`Ctrl+Alt+N` on Linux and Windows).
2. Click the note and type. To move it, hover over the note and drag the grip in its helper bar.

The `t` shortcut also opens a note while the canvas has focus.

Every edit is saved as you type, in `canvas.json` in the app home, so notes survive restarts. A note is not a file in your project, so it never shows up in Git.

### Use notes with agents

Wire a note to an agent the same way you wire a file. The agent reads it with `canopy pane read`. Some ways people use notes:

- A plan that two paired agents both follow, so you can edit the plan in one place.
- A running list of things you noticed while testing, to hand over in one go.
- A scratchpad an agent writes to: agents can open a note next to themselves with `canopy pane new mod:note --wire`.

### Closing a note

> **Warning:** Closing a note deletes its text. Canopy asks before closing a note that has any text other than spaces.

Agents cannot close a note that has text in it, even if you ask them to. Only you can.

## See also

- [Images and PDFs](Images-and-PDFs) for other files you open on the canvas.
- [Agent Pane Commands](Agent-Pane-Commands) for what agents can do with these panes.
- [Wiring](Wiring) for what a wire does.
