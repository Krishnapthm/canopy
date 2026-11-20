# Agent Setup

The `canopy` skill teaches agents about panes, wiring, quotes, helpers and peer messages.

## The canopy skill

Canopy installs the skill on start for installed agents whose `programs.json` entry has a `skills` folder.

- Claude Code: `~/.claude/skills/canopy/SKILL.md`.
- Codex: `~/.agents/skills/canopy/SKILL.md`.
- Other agents: `<skills>/canopy/SKILL.md`, using their entry's `skills` folder.

Canopy writes only its own `canopy` skill folder.
It updates changed skill text only when the file has Canopy's installation marker.
It leaves unmarked files alone.

A leading `~/` means your home folder.
`CANOPY_SKILL_HOME` replaces that home folder when Canopy expands `~/` in skill paths.
See [Files and Folders](Files-and-Folders) for other environment variables.

Agents discover the skill by its name and description.
Claude Code and Codex also receive a launch message naming it.
See [Agents](Agents) to start an agent.

## Teach a project

![The Teach agents Canopy dialog](images/agent-setup-1.png)

Canopy opens **Teach agents Canopy** once per local project folder, except your home folder.
It skips the offer if either instruction file contains `<!-- canopy:begin -->`.
Closing the dialog counts as an answer.

1. Read the target path and the block in the dialog.
2. Click **Add to AGENTS.md** or **Add to CLAUDE.md** to write the block.

The block tells agents to use the `canopy` skill for panes, the canvas, wiring, quotes and peers.
Canopy chooses the file as follows:

- If `CLAUDE.md` has an `@AGENTS.md` or `@./AGENTS.md` line, Canopy uses `AGENTS.md`.
- Otherwise, it uses the existing file, preferring `AGENTS.md` when both exist.
- If neither exists, it offers to create `AGENTS.md`.

Click **No** or press `Esc` to close without writing. Pressing `Enter` does not write.
To open the dialog again, run:

```sh
canopy do mod.open --name project-setup
```

- **Remove block** removes an existing marked block and keeps the surrounding text.
- **Close** closes the dialog when a block exists.
- Adding replaces a marked block or appends it after a blank line.
- Canopy checks for file changes before saving. A failed write shows an error toast and leaves the dialog open.

Project writes support local spaces only.

## Global instruction files

Canopy never writes your global `AGENTS.md` or `CLAUDE.md`.
Click **Copy instructions** to copy the block, then paste it yourself. The button changes to **Copied**.

With no space folder, the dialog offers only **Copy instructions** and **Close**.

## Remove the skill

Before uninstalling Canopy, run:

```sh
canopy skill remove
```

This removes marked skill files from configured skill folders, including those for agents no longer installed.
It keeps unmarked files and other files. It removes each `canopy` folder only when empty.
