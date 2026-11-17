# Sub-agents

There are two kinds. Real helpers are agents another agent started with
`canopy spawn <program> --name <n> [--model <m>] [-- <task>]` when you asked it
for sub-agents, helpers or peers: each is a full terminal in its parent's folder
and space, wired to the parent, that you can open and type into. `canopy wire`
lists their addresses; the parent messages them with `hc send`, reads their
screens with `hc read` and waits with `hc wait --children any`. When a task is
given, Canopy appends a request to report back with `hc send <parent> "..."`
when done or stuck (see [Wire](wire.md)). The rest of this page is about the
other kind.

In-process helpers appear as named cells beside their parent, joined by an
edge. They show working, then done; done cells dim. They are observations,
not independent terminals.

Drag a cell to keep it somewhere else. Click to follow its read-only
transcript in the side panel, including when the file appears later.
Talk to a sub-agent through its parent's terminal. Pick "Clear finished" on
the parent to remove its done cells, or select a done cell and press Backspace
or Delete. Working
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
