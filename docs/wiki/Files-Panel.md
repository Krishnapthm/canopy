# Files Panel

The files panel is a side panel with your space's file tree and file previews. Use it when you want to browse the project, read what an agent changed, or make a quick edit yourself, without putting a pane on the canvas. It is also where sub-agent transcripts open.

![The files panel beside the canvas](images/files-panel-1.png)

## Open the panel

1. Click the folder button, **Files**, in the [top strip](Spaces#the-top-strip).
2. Drag the panel's left edge to resize it.

- Click **Expand panel** to fill the window. Click **Restore panel** to return to the canvas.
- Resizing also ends expanded mode.
- Click **Close panel** to close it. The folder button restores its file tabs.

## Browse and preview files

The canvas tree shows the selected space's folder.
In the classic layout, the tree follows the focused pane's folder, or the active space's folder when none is focused.

1. Click a folder to expand or collapse it.
2. Type in **Search files** to filter the list.
3. Click a file to preview it.

- On the canvas, file tabs appear in the top strip above the panel.
- Click a tab to select it. Hover for its full path. Click its X to close it.
- Markdown and transcript previews accept text files up to 2 MiB.
- Other text previews accept files up to 1 MiB and show the first 400,000 UTF-16 code units.
- The tree loads at most 500 entries per folder and 5,000 entries in total.
- For files on the canvas, see [File and Note Panes](File-and-Note-Panes).

## Edit a file

1. Open the arrow menu beside the editor button.
2. Select **Open here** to edit in the panel.
3. Edit the text.
4. Click **Save** to save it, or **Done** to save and return to the preview.

An unsaved dot replaces the tab's X until you hover over it.
Editing accepts text files up to 1 MiB.
If the file changes outside Canopy, saving fails and keeps your draft.

## Changes

Changes is a quick review screen for what your agents did: every uncommitted file in the repository, with its diff. It is available in the [Classic layout](Classic-Layout). Click **Changes** at the right of the tab row to see uncommitted changes in the focused folder's Git repository.
Click a file to see its diff, with added and removed lines.

- The list shows at most 1,000 changed files.
- A diff shows at most 1,500 lines, with each line limited to 4,000 characters.

> **Warning:** **Commit** stages and commits all changes in the repository.

1. Type a message in **Commit message**.
2. Click **Commit**.

## Sub-agent transcripts

A transcript opened from a [sub-agent](Sub-Agents) appears as a file tab in the panel.
It is read-only and updates when the transcript changes.
Answer the sub-agent's questions in its parent agent.
