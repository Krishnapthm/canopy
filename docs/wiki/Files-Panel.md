# Files Panel

The files panel is a side panel with your space's file tree and file previews. Use it when you want to browse the project, read what an agent changed, or make a quick edit yourself, without putting a pane on the canvas. It is also where sub-agent transcripts open, and where browser and simulator panes dock by default.

![The files panel beside the canvas](images/files-panel-1.png)


On macOS, right-click a local image or PDF in the file tree and choose **Open in Preview**. The preview tab also has an **Open in Preview** button.
## Open the panel

1. click the folder button, **Files**, in the [top strip](Spaces#the-top-strip). it restores the file tab that was active when you closed the panel if that tab is still open. otherwise it opens the file tree.
2. drag the panel's left edge to resize it.

- click **Files** while a tab is active to return to the tree. click it again from the tree to close the panel.
- click **Expand panel** to fill the window. click **Restore panel** or resize the panel to return to the canvas.
- click **Close panel** to hide it. docked panes stay docked while it is hidden. to return to a docked browser or simulator, open the panel and select the pane's tab.

## File tree

The canvas tree shows the folder of the space you are in. On Home it shows the space whose card you last clicked, or else the space you entered most recently.
In the classic layout, the tree follows the focused pane's folder, or the active space's folder when none is focused.

1. Click a folder to expand or collapse it.
2. Type a file name in **Filter, or ? to search in files**. To find text inside files, type `?` followed by at least two characters, such as `?TODO`.
3. Click a file or search result to open its preview. Right-click a local file in the canvas tree and choose **Open in Canopy browser** to render it. [Supported browser formats](Browser-Pane#open-a-local-file)

Use **Collapse all** beside the canvas filter to close every expanded folder and keep the root entries visible. Right-click a local row to copy its absolute or relative path, rename it, move it to the system trash, show it in the system file browser, or open a terminal in its folder. Moving to the trash asks for confirmation. Folder rows also let you create a file or subfolder.

Choose **Copy** or **Cut** on a row, then **Paste** on a folder or any file inside it. If the name is taken, Canopy uses a name like `notes copy.txt` or `notes copy 2.txt`. A successful cut and paste clears that item from the Files clipboard. Remote rows offer **Copy path** and **Copy relative path**. Click a remote file to read it in the tree panel, then choose **Back to files** to return.

Content search starts 300 ms after you stop typing. In a Git working tree it searches tracked files using Git. Elsewhere it uses recursive grep. It matches literal text, skips binary files, and shows up to five matching lines per file and 200 lines total.

- On the canvas, file tabs appear in the top strip above the panel.
- Click a tab to select it. Hover for its full path. Click its X to close it.
- Markdown and transcript previews accept text files up to 2 MiB.
- Other text previews accept files up to 1 MiB and show the first 400,000 UTF-16 code units.
- The tree loads at most 500 entries per folder and 5,000 entries in total.
- For files on the canvas, see [File and Note Panes](File-and-Note-Panes).

## Switch between Source and Preview

Markdown (`.md`, `.markdown`, `.mdx`), JSONL, HTML and SVG open in **Preview**.
Click **Source** in the file header to see the text, and **Preview** to return.
The same controls appear in a canvas file pane's helper bar. JSONL source is
read-only. HTML previews on macOS use a local browser pane in the same tab or
canvas position. Click **Source** in its header or helper bar to return to the file.

- Click **Outline** in a Markdown preview to show the heading rail. Select a heading to scroll to it.
- Click a web link to open a browser pane. A file link opens a Files tab. Relative links resolve from the Markdown file's folder.
- Click **Collapse folders** while the tree is visible to fold open folders.
- Open **More** to find **Auto save**. The checked setting is the default and is shared across spaces.

## Browsers and simulators in the panel

A browser or simulator opens as a tab in this panel, so it sits beside your work without taking canvas space. Click **Browser** or **Simulator** in the [top strip](Spaces#the-top-strip), or let an agent run `canopy pane new browser localhost:3000`. If you have an unsaved edit, the pane opens in a background tab and your editor stays active.

- A docked pane's tab shows its kind's icon and its title (the page title or the device name). A pane wired to an agent adds a small dot in the agent's pane or inherited space color.
- Choose **Move to canvas** from the browser’s **⋯** menu or on the simulator’s [helper bar](Toolbar-and-Helper-Bar) to show the pane at its saved spot in its space. A pane without a saved spot opens in the middle of the view.
- Click **Connect** in the header and drop the wire on an agent to wire the pane. A docked pane has no place on the canvas, so no line is drawn. A dot marks the tab, and the simulator header also shows the agent’s name.
- When you select a browser, its web tabs take the place of its pane tab. Scroll the strip horizontally when there are more tabs than fit. The page stays within the panel below the browser controls as you resize it.
- Click a simulator's or inactive browser's tab X to close its pane. For the selected browser, choose **Close browser pane** from **⋯**. **Close panel** hides the panel until you need it again.

A docked pane is not on the canvas: layout, tiling and wire lines skip it. Canopy remembers docked panes between runs. On the canvas, use **Open in panel** from the browser’s **⋯** menu or its equivalent on the simulator’s helper bar. The pages themselves work as described in [Browser Pane](Browser-Pane) and [Simulator Devices](Simulator-Devices).

## Editing

Use **Source** to make a quick edit or quote an exact line without leaving the panel. For files with a rendered view, use **Preview** to switch back. You can also choose **Open here** from the menu beside the editor button.

- Edits save automatically one second after you stop typing. To turn this off, open **More** and uncheck **Auto save**. The toolbar then shows **Save**. On macOS, **Cmd+S** saves in either setting.
- Hover a line number and click **+** to open the quote menu for that line. Drag from **+** to another line to quote the whole range. This also works in read-only source views.
- The bar beside a line number shows a Git change. Green marks additions, blue marks edits, and a short red bar shows where lines were deleted. The bars refresh when you open or save the file.

The editor wraps long lines and highlights the line with the cursor. It accepts text files up to 1 MiB. If another program changes the file before you save, Canopy shows an error and keeps your draft. An unsaved dot appears on the tab, and leaving a dirty tab asks whether to discard your changes.

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
