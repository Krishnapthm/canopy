# Sub-agents

In-process helpers appear as named cells beside their parent, joined by an
edge. They show working, then done; done cells dim. They are observations,
not independent terminals.

Drag a cell to keep it somewhere else. Click to follow its read-only
transcript in the side panel, including when the file appears later.
"Message through parent" opens the Wire thread: what you type there reaches the
sub-agent through its parent, addressed to it (see wire.md). Pick "Clear finished" on the parent to remove its
done cells, or select a done cell and press Backspace or Delete. Working
cells refuse deletion with a toast. Ending the parent session clears all
its sub-agent cells.

Session hooks publish `canopy.sub.<id>` labels on the parent terminal.
Start marks working, Stop marks done, and restarting the same id marks
working again. Names prefer the transcript sidecar's description, then the
stop payload's description, then the agent type. Names are plain text,
limited to 256 characters. Label events and folder changes refresh them.
`canvas.json` stores each cell's `sub`, `parent`, position and stacking
order; existing terminal cards still load. Transcript changes use the
existing folder watcher and text preview, with agent markup shown literally.
