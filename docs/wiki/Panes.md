# Panes

Everything on the canvas is a pane: a terminal, an agent, a file, a note, an image, a web page, a simulator. They all move, select, wire and close the same way, and each kind adds its own tools on top. This page covers what is common to all of them.

## Pane kinds

| Kind | Use it for | Page |
|---|---|---|
| Terminal | A shell or any program. | [Terminals](Terminals) |
| Agent | A coding agent CLI. An agent pane is a terminal pane running an agent. | [Agents](Agents) |
| Sub-agent | A helper an agent started, shown beside it. | [Sub-Agents](Sub-Agents) |
| File | Keeping a text file in view, or showing it to an agent. | [File and Note Panes](File-and-Note-Panes) |
| Note | Plain text you type: a plan, a checklist, a message for agents. | [File and Note Panes](File-and-Note-Panes) |
| Image | Screenshots and designs, with region quotes. | [Images and PDFs](Images-and-PDFs) |
| PDF | Specs and papers you can search, highlight and quote. | [Images and PDFs](Images-and-PDFs) |
| Browser | A web page, such as your dev server. macOS only. | [Browser Pane](Browser-Pane) |
| Simulator | A live iOS Simulator. macOS with Xcode only. | [Simulator Devices](Simulator-Devices) |
| Mod pane | A kind added by an installed mod. | [Layouts and Mods](Layouts-and-Mods) |

## Open a pane

- Click a pane kind on the [toolbar](Toolbar-and-Helper-Bar) at the bottom of the canvas.
- Press `Cmd+O` (`Ctrl+O` on Linux and Windows) in canvas mode to pick a file. Canopy opens text files as file panes, PNG, JPEG, GIF and WebP files as image panes, and PDFs as PDF panes.
- Drag a file from your file manager onto the canvas to open it where you drop it.
- Ask an agent. Agents open panes with [`canopy pane new`](Agent-Pane-Commands), usually wired to themselves so you both see the same thing.
- Browsers and simulators open docked in the [files panel](Files-Panel), not on the canvas. A docked pane is a tab there. **Move to canvas** on its helper bar puts it on the canvas, and **Open in panel** docks it again.

A pane you open on the canvas brings the camera to 100% with the pane in view. A pane an agent opens leaves your view alone, and a toast tells you when it lands out of view (see [Canvas](Canvas#where-the-camera-goes)).

## The helper bar

Hover or select any pane and a small bar of buttons appears above its top-left corner. It holds that pane kind's common actions, such as **Reload** on a browser or **Annotate** on a simulator, plus **Connect** to draw a wire and **Close**. Hover a button to see its name and shortcut. See [Toolbar and Helper Bar](Toolbar-and-Helper-Bar).

## Color a pane

Give a pane its own color to pick it out from the rest of its space. Click the color swatch in its top controls. On a sticky note, the swatch is in the helper bar.

- Choose a named color or **Custom hex color** and enter a value such as `#3a78e8`. Saved colors appear in other pane color menus and stay available after you quit Canopy.
- Choose **Suggest a color** to cycle through colors unused in this space. Canopy generates a new color if all saved colors are in use.
- Choose **Space color** to restore inheritance.

The tint appears in the frame when you deselect the pane. Selection and sync input have their own outlines. Choose **gray** for an explicit gray tint, even in a space with another color.

## Close panes

Select one or more panes in canvas mode and press `Backspace` or `Delete`, or click **Close** in a pane's helper bar. A docked pane closes with its tab's X.

What happens next depends on the kind:

- **Terminals and agents** ask first, because closing one stops the program in it. The dialog lists every terminal about to close. Click **Terminate and close all** to go ahead, or **Cancel** (or `Esc`) to keep them.
- **Files, images, PDFs, browsers and simulators** close at once. They are views, so nothing is lost: the file stays on disk.
- **Notes** ask first when they contain text, because closing a note deletes its text.
- **Sub-agents** close only when they are done. See [Sub-Agents](Sub-Agents).

If you select a mix and cancel the terminal dialog, the other selected panes still close.

Closing the last terminal in a space leaves the space open, with its files, browsers and other panes. Only **Delete space** removes a space (see [Spaces](Spaces)).

To stop being asked about plain terminals, set `confirm_close_terminal` to `false` in [settings](Settings-and-Themes), or tick **Don't ask again** when you close one from its own menu. Agents always ask.

### Which panes an action uses

Terminal actions, such as closing or expanding, use the selected terminal, agent and sub-agent panes in the current space. With nothing selected there, they use the terminal that has the keyboard, if any.

## Stacking

Panes can overlap when `canvas_push` is `"off"`. Click a pane to bring it to the front. Where panes overlap, the frontmost pane takes the clicks. Right-click a terminal for **Bring to front** or **Send to back**. Canopy saves the stacking order.

## What Canopy saves

Your canvas lives in `canvas.json` in the [app home](Files-and-Folders). For each pane it keeps the kind, position, size, stacking order, space and any color override, plus what that kind needs: a file's path, a note's text, a PDF's highlights. The terminals themselves are kept by herdr-core, which is why they survive a restart.

## Pane IDs

Commands and wires name panes by ID. You see these in `canopy wire`, `canopy pane` output and `canopy ls`:

- Terminals and agents use their herdr-core address, such as `t3` (or `mac/t7` on a machine called `mac`).
- Sub-agents use `sub:<parent>:<id>`.
- Every other pane uses `p<N>`, such as `p1`. Canopy never reuses a number, so an old ID cannot point at a new pane.

A pane's header scales with the canvas zoom along with everything in it (see [Canvas](Canvas#how-panes-behave-when-you-zoom)). Resize a pane by dragging an edge or corner. The pointer shows the resize direction when it is over a resize area.

## See also

- [Canvas](Canvas): selecting, moving and arranging panes.
- [Agent Pane Commands](Agent-Pane-Commands): how agents open, read and close panes.
