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

Open **Settings** > **Look** and choose a card under **Layout**. The **Canopy**
card shows the canvas with a wired agent, terminal, and simulator.

Select a layout in the [command palette](Go-To-and-Palette):

- **Layout: Canopy** (`main`): the canvas with a floating toolbar and a status strip.
- **Layout: Classic** (`classic`): spaces and agents on the left, tabs above the panes, and a status bar.
- **Layout: Terminals** (`terminals`): a space picker and tabs above the terminal panes.

## Change a layout

Shipped layouts and mods update on start. Personal copies do not. When Canopy
retires a shipped default, startup removes its original copy and keeps edited
files. If your saved layout is gone, Canopy opens its main layout.

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

Mod panes declare the same geometry traits as built-in panes. Add a `pane` block to `mod.json` to register the kind `mod:<name>`, where `<name>` is the mod's folder. The shipped note mod hides the standard header and uses the default sizes:

```json
{
  "place": "none",
  "title": "Note",
  "pane": { "label": "Note", "icon": "mods/note/sticky-note.svg", "header": false }
}
```

| Field | What it does | Default |
|---|---|---|
| `header` | Shows the standard header when `true`. | `true` |
| `size` | The opening size, as `[width, height]`. | `[320, 240]` |
| `min` | The smallest resize, as `[width, height]`. | `[240, 120]` |

Sizes are in canvas pixels, the size at 100% zoom; the pane and its header scale with the canvas like every pane (see [Canvas](Canvas#how-panes-behave-when-you-zoom)). Each number must be positive and finite, and `min` must fit inside `size`. An invalid declaration shows a toast that opens the `mod.json`, and the pane uses the defaults for all three fields.

Each entry in `actions` declares `{icon, label, command, args}` and runs an app command with the pane selected. The shared helper bar shows these actions before **Connect** and **Close**.

## Context menus

A mod opens a context menu with `openMenu({ tab, target })`. Every menu in Canopy opens this way, so they all behave alike: the frame is sized for its rows, moves left at the right window edge so a submenu still fits, scrolls when it is taller than the window, and replaces any other open menu. Escape, a click outside it (even over a web page), resizing the window, or panning or zooming the canvas closes it.

```js
import { openMenu } from "herdr";

// A right-click menu with your own rows: [label, command, args].
openMenu({ tab: "primary", target: { items: [["Reload", "canvas.browser_reload", { pane: "p3" }]] } });
```

`tab` picks a page of the shipped `menu` mod. The names are the `MenuTab` type in `herdr.d.ts`. `primary` shows the rows you give in `target.items`; the others, such as `tab` or `canvas-agents`, build their rows from the ids in `target`. An unknown `tab` shows an error toast. Use `at: "above"` to open the menu over the floating block under the pointer, as the toolbar's agent button does.

## Check a mod

Run this command after each change:

```sh
canopy check ~/.config/canopy/mods/my-panel
```

The check loads the mod and checks its initial description and eager native elements.
It does not check layout, painting, deferred content or later asynchronous states.
Inspect the result in the app too.
