# Agent Setup

An agent can only use Canopy's features if it knows they exist. Without help, an agent asked to "open the dev server next to you" or "ask the reviewer" has no idea what you mean. Canopy teaches agents in three ways, from automatic to opt-in:

1. **The `canopy` skill**, installed automatically for Claude Code and Codex.
2. **A launch note**, passed automatically when Canopy starts Claude Code or Codex.
3. **A project block** in your project's `AGENTS.md` or `CLAUDE.md`, added only with your permission.

None of this changes how the agent CLI is configured. Canopy never edits your global instruction files or the agents' settings.

## The canopy skill

A skill is a set of instructions an agent loads when a task calls for it, discovered by its name and description. The `canopy` skill gives an agent the exact commands for the jobs people ask for most: showing a site in a browser pane and on the iOS simulator, opening files and notes, starting helpers, messaging wired peers, and writing a mod. Its `SKILL.md` stays short. Deeper detail sits in `references/` files that the agent opens only when the task needs them: `simulator.md`, `browser.md`, `panes-and-canvas.md`, `mods.md` (a step-by-step guide to writing a mod, with examples that pass `canopy check`) and `customize.md`. So a person who knows no Rust can still hand the skill to any coding agent and ask for a new pane or panel.

Canopy installs it every time it starts, for each installed agent whose `programs.json` entry names a skills folder:

| Agent | Skill file |
|---|---|
| Claude Code | `~/.claude/skills/canopy/SKILL.md` and `references/` |
| Codex | `~/.agents/skills/canopy/SKILL.md` and `references/` |
| Others with a `skills` folder | `<skills>/canopy/SKILL.md` and `references/` |

Canopy writes only its own `canopy` folder. It marks each file it installs and checks each file for that mark before updating or removing it. If you provide an unmarked `SKILL.md`, Canopy leaves the entire folder alone.

To print the skill, for an agent CLI without skill support, run `canopy --skill`. It prints `SKILL.md` followed by every reference file.

`CANOPY_SKILL_HOME` replaces your home folder when Canopy works out these paths. It is useful for testing in a scratch folder. See [Files and Folders](Files-and-Folders) for other environment variables.

## The launch note

When Canopy starts Claude Code or Codex, it adds a short note through the CLI's own flags (`--append-system-prompt` for Claude Code, `developer_instructions` for Codex). The note tells the agent that it runs inside Canopy, gives its terminal address, shows how to message and wait for peers, says that peer messages are never your approval, and names the `canopy` skill. Claude Code also gets session hooks that report wires and in-process sub-agents.

These flags apply only to that session. Nothing is written to the agent's configuration.

## Teach a project

![The Teach agents Canopy dialog](images/agent-setup-1.png)

The project block helps every agent that works in the project, including ones Canopy does not start or know about. It is a short marked block:

```text
<!-- canopy:begin -->
## Canopy

This project is open in Canopy, a canvas of terminals, agents and panes (files, notes). When the person mentions panes, the canvas, wiring, quotes or peers, use the `canopy` skill.
<!-- canopy:end -->
```

The first time you open a space for a local project folder (other than your home folder), Canopy shows **Teach agents Canopy** with the target file and the block.

1. Read the file path and block in the dialog.
2. Click **Add to AGENTS.md** or **Add to CLAUDE.md** to write it.

Or click **No** (or press `Esc`) to skip. Closing the dialog counts as an answer, so Canopy does not ask again for that folder. `Enter` alone never writes anything.

Canopy chooses the file like this:

- If `CLAUDE.md` imports `AGENTS.md` (an `@AGENTS.md` or `@./AGENTS.md` line), it uses `AGENTS.md`.
- Otherwise it uses the file that exists, preferring `AGENTS.md` if both do.
- If neither exists, it offers to create `AGENTS.md`.

Adding replaces an existing marked block, or appends the block after a blank line. Canopy checks that the file has not changed since it read it before saving. If the write fails, an error toast says why and the dialog stays open.

If either file already contains the block, Canopy does not ask. To open the dialog again, for example to remove the block, run:

```sh
canopy do mod.open --name project-setup
```

When a block exists, the dialog offers **Remove block**, which removes it and keeps the rest of the file, and **Close**.

The project block can be written for local spaces only, not for spaces on [machines](Machines).

## Global instruction files

Canopy never writes your global `AGENTS.md` or `CLAUDE.md`. If you want every agent everywhere to know about Canopy, click **Copy instructions** in the dialog and paste the block into your global file yourself. The button changes to **Copied**.

When the current space has no folder, the dialog offers only **Copy instructions** and **Close**.

## Remove the skill

Before you uninstall Canopy, run:

```sh
canopy skill remove
```

This removes the skill files Canopy marked as its own from every configured skills folder, including those for agents you have since uninstalled. It leaves unmarked files and any other files alone, and removes each `canopy` folder only if it is then empty.

## Troubleshooting

- **The dialog offers "a new file", then a toast says "Could not write AGENTS.md" and "Text file exceeds 1 MiB".** Your instruction file is over 1 MiB, which is more than Canopy reads. Make it smaller, then open the dialog again.
- **An agent does not use Canopy features.** Check which layers it gets: see the table in [Agents](Agents#which-agents-canopy-knows). For agents without a skill or launch note, add the project block, or tell the agent to run `canopy --skill`.

## See also

- [Agents](Agents): which agents get which integration.
- [Concepts](Concepts#how-agents-learn-canopy): the three layers in context.
