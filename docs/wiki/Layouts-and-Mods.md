# Layouts and mods

You can change the whole window, not just its colors. Layouts arrange it; mods draw the panels inside it.

Read the [layout and mod guide](../defaults/AGENTS.md). This is the same guide Canopy writes into your app home on every start, so people and agents work from one source of truth.
It covers layouts, settings, keys, overlays, mod code and the app API.

For live folder updates, `watchFolder(path)` returns an awaitable promise with
`cancel()`. Keep the promise and cancel it when the folder is no longer shown;
it resolves with `[]` and releases that call without stopping other mods' watches.

The generated `herdr.d.ts` and `gpui-kit.d.ts` beside your installed guide describe the full API for your version.

To match the app's look, build from `ui` in the `herdr` module (buttons,
checkbox, input, dialog pieces, rows, sizes); every shipped dialog uses it.

Start with Settings for fonts, colors and sounds.
For deeper changes, duplicate a shipped layout or mod from the command palette, then edit the copy.
Shipped files update on start; personal copies do not.
Save to reload. Check for errors, even when the old screen still looks fine.

```sh
canopy check ~/.config/canopy/mods/my-panel
canopy status
```

The check loads a mod and renders it once. It does not replace looking at the result in the window.

Mods can run programs, read files and use the network. Only install code you trust.
See [security and privacy](security.md).

## Mod panes

A mod declares a canvas pane kind `mod:<name>` in `mod.json`:

```json
"pane": { "label": "Note", "icon": "mods/note/sticky-note.svg", "actions": [] }
```

Open it with `canvas.pane_new {kind: "mod:<name>"}`. The app draws one copy per
pane with `props().pane` set to its id. The copy lives only while its place is
shown. `canvasPanes()` gives its saved props; save state with
`run("canvas.pane_set", {pane, props})`. This shallow-merges the object; a string
`props.text` is what `canopy pane read` returns.

Actions declare `{icon, label, command, args}`. They run app commands with the
pane selected. After them the app adds Connect and Close in the shared helper bar.

## Toolbar state and shortcut hints

`window().hand` is true while the canvas hand tool is on (`canvas.hand`).
Use it to highlight Hand or Select. `window().keys` becomes true after the
person holds Cmd (Ctrl elsewhere) alone for 0.4 s; show each button's key
beside its icon while true. Release or any key press hides the hints.
`keys()` rows include `label`, the platform-formatted binding, such as `⇧⌘N`
on macOS or `Ctrl+Shift+N` elsewhere. Use it in tooltips and inline hints.

Open a popup with `openOverlay({mod, kind: "popup", at: "above"})` to place
its bottom 8 px above the floating block under the pointer, with its left
edge 16 px left of the pointer. The popup snaps inside the window with an
8 px margin. Without a floating block under the pointer it opens at the pointer.

[Back to Canopy](../README.md)
