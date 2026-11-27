# Settings, keys, layouts and agent setup

Everything here lives in the app home: `~/.config/canopy`, or `$HERDR_APP_HOME`
when set. Read `<app home>/AGENTS.md` first: it is the installed guide to
settings, layouts, keys, commands and mods for this exact version. Do not edit it
or the generated `.d.ts` files; the app rewrites them on start.

Make only the change asked for. Touch nothing else.

## Files and how they apply

| File | Decides | Applies |
|---|---|---|
| `settings.json` | the person: zoom, fonts, sounds, canvas and browser options | live |
| `keys.json` (`keys.macos.json` on a Mac) | what every key does | live |
| `layouts/<name>.json` | the window, its blocks and theme | live |
| `programs.json` | agents and programs: run command, flags, note, skills folder | live |
| `mods/<name>/` | mods ([mods.md](mods.md)) | on save |

- An invalid file keeps the last good version and shows a toast with
  file:line:column; `canopy status` lists the error. Check it after every edit.
- An unknown settings key is a warning toast (with a close spelling) and the rest applies.
- `canopy config` prints every setting with its default and explanation.
- Shipped layouts and mods belong to Canopy: copy first with
  `canopy do layout.clone --name main` or `canopy do mod.clone --name toolbar`,
  then edit the copy.
- `app.settings {section: "shortcuts"}` opens searchable keys for the person.
- `app.quit` exits Canopy while the core keeps terminals and agents running. On macOS,
  `run_in_background` adds a menu bar icon and keeps a Dock tile after Quit while terminals
  run; its menu has **Open Canopy** and **Quit Completely** (stops the core, asks first).
- If a layout or keys edit locks the person out, `ctrl-alt-shift-escape` always
  restores the default layout and sets the broken files aside.

### Keys

```json
{
  "global":   { "ctrl-shift-t": "tab.new", "alt-1": "space.select:1" },
  "terminal": { "ctrl-shift-c": "terminal.copy" },
  "mod:my-timer": { "space": "my-timer.toggle" },
  "canvas-mode": { "x": "canvas.expand" }
}
```

An action is a command (`name`, or `name:value` for its first argument), an object
`{"command": ..., "args": {...}}`, `send:<text>` (types into the terminal),
`<mod>.<command>` (delivered to that mod's `nextCommand()`), or `terminal`. Do not
bind plain ctrl+letters programs rely on (ctrl+c, ctrl+d, ctrl+r, ...) unless asked.

## Agents in `programs.json`

An agent entry's `run` is its command, `flags` are extra argv words Canopy adds
when it starts the agent (`{canopy}` is the app binary, `{note}` and
`{note_json}` the short Canopy note), and `skills` is its global skills folder.
On start Canopy writes this skill into `<skills>/canopy/` for every installed
agent that has one (Claude Code `~/.claude/skills`, Codex `~/.agents/skills`),
touching nothing else there. `canopy skill remove` deletes it.

Typed agents in plain local zsh and fish terminals get the same flags through
generated shell functions; Canopy never edits the person's shell files.

## The project instruction block

When a folder opens as a space, Canopy asks the person once whether to add a
marked block (`<!-- canopy:begin -->` ... `<!-- canopy:end -->`) to the project's
`AGENTS.md` or `CLAUDE.md`. It is theirs: never add, edit or remove it unless
they ask. Canopy never writes global instruction files.
