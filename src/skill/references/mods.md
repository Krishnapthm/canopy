# Writing a Canopy mod

A mod is a small JavaScript program that adds to Canopy's window: a pane kind on
the canvas (like the sticky note), a panel, a toolbar, a dialog. You need no Rust,
no build step and no knowledge of GPUI. Save the file and the app reloads it.

## What a mod is

```
~/.config/canopy/mods/<name>/     ($HERDR_APP_HOME/mods/<name>/ when that is set)
  main.js     the mod: default-exports a class extending View (required)
  mod.json    where it appears, and its pane kind if it has one (optional)
  *.js *.svg  more modules (import "./ui.js") and its own icons
```

- `main.js` describes UI with chained builders, `div().p(8).child("hi")`, and
  native Rust draws it. There is no HTML, CSS, DOM, Node API or `setTimeout`.
- Three modules: `gpui-kit` (elements: `View`, `div`, `svg`, `image`),
  `gpui-base` (`h_flex`, `v_flex`, `Button`, `Input`, `Editor`, `ScrollHandle`)
  and `herdr` (the app: data, commands, toasts, state). Their exact types are in
  `gpui-kit.d.ts` and `herdr.d.ts` in the app home, regenerated for this version
  on every start. When unsure of a name or argument, read them; do not guess.
- The app home's `AGENTS.md` is the full guide; this page is the fast path.
- Mods are trusted code: they can run programs and fetch URLs. Make only the mod
  the person asked for.

## The loop (do every step, every change)

1. Start from a copy, never a shipped mod: `canopy do mod.clone --name note --as my-note`,
   or create a new folder under `mods/`. Shipped mods are replaced on every start.
2. Write `main.js` (and `mod.json` for a pane kind).
3. `canopy check ~/.config/canopy/mods/<name>`: loads the mod once headlessly and
   exits nonzero with the exact error. Never call a mod done without a pass.
4. Saving reloads it in the running window. `canopy status` shows a load error; a
   broken save keeps the old version running, so a working screen proves nothing.
5. Show it: a pane kind with `canopy pane new mod:<name> --wire`; then
   `canopy shot /tmp/mod.png` (or `canopy pane shot <pane>`) and look at the picture.

## Example: a pane kind with helper bar buttons

A 25-minute countdown on the canvas. Its helper bar gets **Start or pause** and
**Reset**, and an agent can read the time left with `canopy pane read`. This
example passes `canopy check` as written.

`mods/countdown/mod.json`:

```json
{
  "place": "none",
  "title": "Countdown",
  "pane": {
    "label": "Countdown",
    "icon": "mods/countdown/timer.svg",
    "size": [260, 160],
    "min": [180, 120],
    "actions": [
      { "icon": "mods/countdown/play.svg", "label": "Start or pause", "command": "countdown.toggle" },
      { "icon": "mods/countdown/reset.svg", "label": "Reset", "command": "countdown.reset" }
    ]
  }
}
```

`mods/countdown/main.js`:

```js
// A countdown pane. Its helper bar has Start or pause and Reset (declared in mod.json),
// and `canopy pane read <pane>` returns the time left.
import { View, div } from "gpui-kit";
import { v_flex } from "gpui-base";
import { nextCommand, notify, props, run, theme } from "herdr";

const LENGTH = 25 * 60; // seconds

export default class Countdown extends View {
  init(_props, cx) {
    this.pane = props()?.pane; // this pane's id, like "p4"
    this.left = LENGTH;
    this.ticking = null; // the timer while running; null when paused, so nothing runs
    // Helper bar buttons and keys send "countdown.toggle" / "countdown.reset"; we get "toggle" / "reset".
    cx.spawn(async (cx) => {
      for (;;) {
        const command = await nextCommand();
        if (command === "toggle") this.toggle(cx);
        if (command === "reset") { this.stop(); this.left = LENGTH; this.save(); }
        cx.notify();
      }
    });
  }

  toggle(cx) {
    if (this.ticking) { this.stop(); this.save(); return; }
    this.ticking = cx.timer.every(1000, (cx) => {
      this.left -= 1;
      if (this.left <= 0) {
        this.stop();
        this.left = LENGTH;
        notify({ title: "Countdown finished", level: "success" });
      }
      this.save();
      cx.notify();
    });
  }

  stop() {
    this.ticking?.cancel();
    this.ticking = null;
  }

  // Saved in the pane (canvas.json, written debounced): survives restarts, and is what `pane read` returns.
  save() {
    if (this.pane) run("canvas.pane_set", { pane: this.pane, props: { text: `${this.label()} ${this.ticking ? "running" : "paused"}` } });
  }

  label() {
    const m = Math.floor(this.left / 60), s = this.left % 60;
    return `${m}:${String(s).padStart(2, "0")}`;
  }

  render() {
    const t = theme();
    return v_flex().size_full().items_center().justify_center().gap(4).bg(t.surface)
      .child(div().text_size(40).font_semibold().text_color(this.ticking ? t.foreground : t.muted).child(this.label()))
      .child(div().text_size(12).text_color(t.dim).child(this.ticking ? "running" : "paused"));
  }
}
```

Icons are paths from the app home. `stroke="currentColor"` makes them follow the theme.

`mods/countdown/timer.svg`:

```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="13" r="8"/><path d="M12 9v4l2 2M9 2h6"/></svg>
```

`mods/countdown/play.svg`:

```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round"><path d="M7 4l13 8-13 8z"/></svg>
```

`mods/countdown/reset.svg`:

```svg
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M3 12a9 9 0 1 0 3-6.7L3 8M3 3v5h5"/></svg>
```

Then `canopy check ~/.config/canopy/mods/countdown` and
`canopy pane new mod:countdown --wire`.

What it shows:

- `"pane"` in `mod.json` declares the kind `mod:countdown`. `size` and `min` are
  world pixels at 100% zoom; `"header": false` hides the standard header.
- Each `actions` entry is a helper bar button `{icon, label, command, args}` that
  runs a command with the pane selected. The app draws the bar; never draw your own.
- `"<mod>.<command>"` (from a button, a key in `keys.json`, or `canopy do` with
  that name) arrives in `nextCommand()` as `"toggle"`. Every
  shown copy of the mod receives it.
- `props().pane` is this pane's id. `run("canvas.pane_set", {pane, props})` saves
  state in the pane; a string `props.text` is what `canopy pane read` returns, and
  `canvasPanes()` gives saved props back after a restart.
- The timer exists only while counting. Idle costs nothing.

## Example: the smallest pane (shipped `note`)

```js
import { View, div } from "gpui-kit";
import { Editor, EditorState } from "gpui-base";
import { canvasPanes, props, run, settings, theme } from "herdr";

export default class Note extends View {
  init(_props, cx) {
    this.pane = props()?.pane;
    const text = canvasPanes().find(p => p.id === this.pane)?.props?.text ?? "";
    this.code = EditorState.new({ language: "markdown", value: text, font_family: settings().font_family,
      font_size: 13, line_numbers: false, folding: false, soft_wrap: true });
    this.code.on("change", () => { if (this.pane) run("canvas.pane_set", { pane: this.pane, props: { text: this.code.value() } }); });
  }
  render() {
    const t = theme();
    return div().size_full().min_h(0).p(8).bg(t.canvas).text_color(t.foreground).child(Editor.new(this.code).size_full());
  }
}
```

with `mod.json` `{"place": "none", "title": "Note", "pane": {"label": "Note", "icon": "mods/note/sticky-note.svg", "header": false}}`.

## Example: a dialog

Open a context menu with `openMenu({ tab, target })` (see `MenuTab` in `herdr.d.ts`); it sizes,
flips and scrolls the menu and closes any other open one.

Open any mod as a dialog, popup or floating window with `openOverlay`; it closes
itself with `closeOverlay(props().overlay)`. The shipped `confirm` mod is a
complete dialog built from `ui` pieces, so it looks like the app:

```js
import { View } from "gpui-kit";
import { closeOverlay, props, run, ui } from "herdr";

export default class Confirm extends View {
  render() {
    const p = props() ?? {};
    const close = () => closeOverlay(p.overlay);
    return ui.panel("confirm")
      .child(ui.title(p.title ?? "Are you sure?"))
      .child(ui.footer(
        ui.button("no", "Cancel").on_click(close),
        ui.button("yes", "Confirm", "primary").on_click(() => { close(); run(p.command.name, p.command.args ?? {}); }),
      ));
  }
}
```

Open it from another mod: `openOverlay({ mod: "my-confirm", kind: "dialog", props: { title: "Delete it?", command: { name: "...", args: {} } } })`.

## Recipes

| To | Use |
|---|---|
| Run an app command | `run("canvas.fit", {})`; `commands()` lists them all |
| Show a toast | `notify({ title, body?, level?: "info" \| "success" \| "warning" \| "error", action?: { name, args } })` |
| Run a program | `await exec("git", ["status"], { cwd })` inside `cx.spawn(async (cx) => ...)` |
| Fetch a URL | `await fetch(url)` inside `cx.spawn` |
| Share state between mods | `shared(key)`, `setShared(key, value)` (persisted; everyone re-renders) |
| Per-copy storage | `localStorage` |
| Read live app data | `spaces()`, `agents()`, `terminals()`, `canvasPanes()`, `focused()`, `theme()`, `settings()`: `render` runs again when they change, so never poll them |
| Watch a folder | `const w = watchFolder(dir); await w;` then reload; `w.cancel()` when you stop showing it |
| Read or write a file | `readTextFile(path)`, `writeTextFile(path, text, expected)` |
| Know your size | `.id("root").on_size_change((size, cx) => { this.size = size; cx.notify(); })` on the root |
| Take the keyboard | `"focusable": true` in mod.json, `this.h = cx.focus_handle()` in init, `.track_focus(this.h).on_key_down((e, cx) => ...)` |
| Clean up | optional `async close(cx)` gets one call (at most 250 ms) when the copy goes |
| Look like the app | `ui.button`, `ui.iconButton`, `ui.checkbox`, `ui.input`, `ui.panel`, `ui.title`, `ui.row` |

## Rules that keep mods correct and cheap

- Never keep a `cx` for later. Every callback gets its own:
  `on_click((event, cx) => ...)`, `cx.spawn(async (cx) => ...)`, `cx.timer.every(ms, (cx) => ...)`.
- State lives on `this`, set in `init`. Module top-level code runs once per source
  version and is shared by every copy.
- Call `cx.notify()` after changing state that `render` shows.
- Idle costs nothing: no timer unless something is pending, and never a timer to
  animate. Use `.frames([...], ms)`, `svg(icon).spin(ms)`, `.loop(...)` and
  `.transition(...)`, which run natively and stop when hidden.
- Colors come from `theme()` (`foreground`, `muted`, `dim`, `surface`, `canvas`,
  `accent`, `border`, ...), so the mod follows light and dark themes.
- Show errors the person causes with `notify({ level: "error", ... })`, not only `console.log`.
- `ctrl-alt-shift-escape` resets a layout or keys file that locked the person out.
