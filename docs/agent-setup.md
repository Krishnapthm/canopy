# How agents learn Canopy

The `canopy` skill explains panes, wiring, quotes, visible helper agents and
messaging peers. Harnesses discover it by its name and description;
`canopy --skill` prints the same text as a fallback.

On start, Canopy installs it for installed agents with a `skills` folder in
`programs.json`: Claude Code uses `~/.claude/skills`, Codex uses
`~/.agents/skills`. A leading `~/` expands to the home folder. Other agents
can declare their verified folder with the same field.
Canopy writes only `<skills>/canopy/SKILL.md`. It rewrites the file when the
text changes, only if the file contains its installed-by-Canopy mark, and
leaves an unmarked file alone. Run `canopy skill remove` before uninstalling
to remove marked files, including those for agents no longer installed.
It removes the `canopy` folder only when empty.

Claude and Codex receive a short launch note naming the skill. Claude's
session and prompt hook says `Wired: ...` when peers change (and lists them
on a fresh session). The line points to `canopy wire` for peers and panes
and to the skill for instructions.

## Project setup

When a local space with a folder other than the home folder opens, the toolbar offers **Teach agents
Canopy** unless either instruction file already has Canopy's begin marker.
The dialog shows the target path and this exact block:

```markdown
<!-- canopy:begin -->
## Canopy

This project is open in Canopy, a canvas of terminals, agents and panes (files, notes). When the person mentions panes, the canvas, wiring, quotes or peers, use the `canopy` skill.
<!-- canopy:end -->
```

If `CLAUDE.md` imports `AGENTS.md` on an `@AGENTS.md` or `@./AGENTS.md` line,
the target is `AGENTS.md`. Otherwise it uses the existing file, preferring
`AGENTS.md` when both exist, or offers a new `AGENTS.md` when neither exists.

Yes, shown as **Add to AGENTS.md** or **Add to CLAUDE.md**, writes the block.
**No** or Esc closes without writing. Any answer, including Esc, counts:
`project-setup.asked` in shared state remembers the offer per folder.
`canopy do mod.open --name project-setup` reopens it. A marked target offers
**Remove block**. Adding replaces an existing marked block in place or appends
it after a blank line; removal leaves the surrounding project text.
Writes re-read the file and check it has not changed before saving. A failed
write shows an error toast and keeps the dialog open. Enter does not write.
This setup supports local spaces only.

## Global files

Canopy never writes global `AGENTS.md` or `CLAUDE.md` files. **Copy
instructions** copies the same block for the person to paste themselves.
With no space folder, the dialog offers copying and closing only.
