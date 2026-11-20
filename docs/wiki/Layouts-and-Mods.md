# Layouts and Mods

Almost everything you see in Canopy, from the toolbar to the space tabs to the note pane, is built from two kinds of plain files in your app home (`~/.config/canopy`):

- A **layout** is a JSON file that arranges the window: which parts go where.
- A **mod** is a small JavaScript program that draws one part of the window, adds a pane kind, or adds commands.

Because they are files, you can change them, and so can your agents.

## Ask an agent to change Canopy

The quickest way to customize Canopy is to describe what you want to an agent running inside it. Start with:

```text
Read ~/.config/canopy/AGENTS.md, then tell me what you can change in Canopy.
```

That file is the layout and mod guide, installed with the app. From there, ask for what you want: "move the toolbar to the top", "add a panel that lists my open pull requests", "make a pane kind for TODO lists". The agent edits the files, Canopy reloads them as soon as they are saved, and the agent can check its work with `canopy check` and `canopy shot`.

Canopy replaces its shipped layouts and mods on every start, moving any you edited to personal copies, so it is cleaner to change a copy from the start (see below). Mods run with your full access, so read [Security and Privacy](Security-and-Privacy) before installing one from someone else.

## Built-in layouts

Select a layout in the [command palette](Go-To-and-Palette):

- **Layout: Canopy** (`main`): the canvas with a floating toolbar.
- **Layout: Classic** (`classic`): spaces and agents on the left, tabs above the panes, and a status bar.
- **Layout: Terminals** (`terminals`): a space picker and tabs above the terminal panes.
- **Layout: Canopy 98** (`win98`): a Windows 98 desktop with a taskbar.

## Change a layout

Shipped layouts and mods update on start. Personal copies do not.

1. Select the layout you want to change.
2. Select **Duplicate layout** in the command palette.
3. Edit the copy in `layouts/<name>-copy.json` in your app home.
4. Save the file to reload the layout.

Canopy switches to the copy when you duplicate a layout.
The copy still uses the original mods.
To change their code, duplicate a mod and change the copy's `content.mod` reference.

See [Files and Folders](Files-and-Folders) for your app home and its files.

## Write a mod

Read [Security and Privacy](Security-and-Privacy) before you install a mod.

1. Select **Duplicate mod** in the command palette.
2. Type the mod's folder name.
3. Press `Enter` to create the copy.
4. Edit `mods/<name>-copy/main.js` in your app home.
5. Save the file to reload the mod.

Read the [layout and mod guide](https://github.com/Krishnapthm/canopy/blob/main/defaults/AGENTS.md) for the API.
Canopy installs this guide as `AGENTS.md` in your app home.
The generated `herdr.d.ts` and `gpui-kit.d.ts` beside it describe the full API for your version.

Canopy keeps the upstream names `herdr` for the JavaScript module and `HERDR_APP_HOME` for the app home override.

## Mod panes

Add a `pane` block to `mod.json` to declare a pane kind named `mod:<name>`:

```json
"pane": { "label": "Note", "icon": "mods/note/sticky-note.svg", "actions": [] }
```

Each action declares `{icon, label, command, args}`.
Actions run app commands with the pane selected.
Canopy draws these actions before **Connect** and **Close** in the shared helper bar.

## Check a mod

Run this command after each change:

```sh
canopy check ~/.config/canopy/mods/my-panel
```

The check loads the mod and checks its initial description and eager native elements.
It does not check layout, painting, deferred content or later asynchronous states.
Inspect the result in the app too.
