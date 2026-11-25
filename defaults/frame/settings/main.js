// Modified for Canopy.
// App-owned settings mod. Writes changes immediately; no Save step.

import { View, div, svg, image } from "gpui-kit";
import { v_flex, h_flex, Scrollbar, InputState, Input, Popup } from "gpui-base";
import { commands, keys, nextCommand, openOverlay, setShared, notify, hc, run, theme, themes, fonts, settings, exec, layouts, shared, appHome, readTextFile, writeTextFile, playSound, props, closeOverlay, window, ui } from "herdr";

const I = (name) => `frame/settings/${name}.svg`;

const SECTIONS = [
  { id: "look", title: "Look", icon: "palette" },
  ...(window()?.platform === "macos" ? [{ id: "keyboard", title: "Keyboard", icon: "keyboard" }] : []),
  { id: "shortcuts", title: "Shortcuts", icon: "keyboard" },
  { id: "machines", title: "Machines", icon: "server" },
  { id: "alerts", title: "Alerts", icon: "bell" },
];

const LOOKS = { main: "Canopy", terminals: "Terminals", win98: "Canopy 98" };
const APPEARANCES = [
  ["system", "System"],
  ["light", "Light"],
  ["dark", "Dark"],
];

// What the app uses when settings.json names no font (settings.rs).
const DEFAULT_FONT = "JetBrains Mono";
const DEFAULT_UI_FONT = "Geist";

// Keep argument-bearing actions distinct, including object bindings.
function actionName(action) {
  return typeof action === "string" ? action : JSON.stringify(action);
}

function commandName(action) {
  return typeof action === "string" ? action.split(":")[0] : action.command;
}

function normalizedKey(key) {
  return key.trim().split(/\s+/).map(chord => {
    const parts = chord.split("-");
    const key = parts.pop();
    if (/^[A-Z]$/.test(key)) parts.push("shift");
    const modifiers = parts.map(p => ({ super: "cmd", win: "cmd", secondary: window()?.platform === "macos" ? "cmd" : "ctrl" })[p.toLowerCase()] ?? p.toLowerCase());
    return [...new Set(modifiers)].sort().concat(key.toLowerCase()).join("-");
  }).join(" ");
}

function conflicts(bindings, key, action) {
  return Object.entries(bindings).filter(([k, a]) => normalizedKey(k) === normalizedKey(key) && actionName(a) !== actionName(action)).map(([, a]) => actionName(a));
}

function parseShortcuts(raw) {
  const value = JSON.parse(raw);
  const object = v => v !== null && typeof v === "object" && !Array.isArray(v);
  if (!object(value) || Object.values(value).some(section => !object(section) || Object.values(section).some(action => typeof action !== "string" && !(object(action) && typeof action.command === "string")))) {
    throw new Error("keys.json must contain sections of key actions.");
  }
  return value;
}

function matchesShortcut(row, query) {
  return `${row.name} ${row.about} ${row.current.join(" ")} ${row.defaults.join(" ")}`.toLowerCase().includes(query.trim().toLowerCase());
}

// Replace only this action; a collision never takes another action's key away.
function bindShortcut(current, row, nextKeys) {
  const bindings = current[row.section] ?? (current[row.section] = {});
  for (const [key, action] of Object.entries(bindings)) {
    if (actionName(action) === row.name) delete bindings[key];
  }
  for (const key of nextKeys) {
    if (normalizedKey(key) === normalizedKey("ctrl-alt-shift-escape")) throw new Error("Ctrl+Alt+Shift+Esc is reserved for restoring Canopy.");
    const other = conflicts(bindings, key, row.action);
    if (other.length) throw new Error(`${row.section}: ${key} already binds ${other.join(", ")}. Clear that binding first.`);
    bindings[key] = row.action;
  }
}

function resetShortcuts(current, defaults, catalog, row = null) {
  if (row) {
    const keys = Object.entries(defaults[row.section] ?? {}).filter(([, action]) => actionName(action) === row.name).map(([key]) => key);
    bindShortcut(current, row, keys);
    return;
  }
  const known = new Set(catalog.map(c => c.name));
  for (const [section, bindings] of Object.entries(current)) {
    const shipped = new Set(Object.values(defaults[section] ?? {}).map(actionName));
    for (const [key, action] of Object.entries(bindings)) {
      if (known.has(commandName(action)) || shipped.has(actionName(action))) delete bindings[key];
    }
  }
  for (const row of shortcutRows({}, defaults, [])) bindShortcut(current, row, row.defaults);
}

function shortcutRows(current, defaults, catalog) {
  const rows = new Map();
  for (const [source, field] of [[current, "current"], [defaults, "defaults"]]) {
    for (const [section, bindings] of Object.entries(source)) {
      for (const [key, action] of Object.entries(bindings)) {
        const id = JSON.stringify([section, actionName(action)]);
        if (!rows.has(id)) rows.set(id, { id, section, action, name: actionName(action), about: catalog.find(c => c.name === commandName(action))?.about ?? "Custom key action", current: [], defaults: [] });
        rows.get(id)[field].push(key);
      }
    }
  }
  for (const c of catalog) {
    if ([...rows.values()].some(row => commandName(row.action) === c.name)) continue;
    const id = JSON.stringify(["global", c.name]);
    rows.set(id, { id, section: "global", action: c.name, name: c.name, about: c.about, current: [], defaults: [] });
  }
  return [...rows.values()].sort((a, b) => a.section.localeCompare(b.section) || a.name.localeCompare(b.name));
}

export default class Settings extends View {
  init(_props, cx) {
    const asked = props()?.section;
    this.section = SECTIONS.some((s) => s.id === asked) ? asked : "look";
    this.picking = null; // "font" | "ui_font" while its list is open
    this.filter = "";
    this.fontInput = InputState.new({ placeholder: "Type to search fonts" });
    this.fontInput.on("change", (_e, cx) => { this.filter = this.fontInput.value(); cx.notify(); });
    this.canvasColorInput = InputState.new({ placeholder: "#1E1E1E" });
    this.canvasColorInput.on("change", (_e, cx) => { const color = this.canvasColorInput.value(); if (/^#[0-9a-f]{6}([0-9a-f]{2})?$/i.test(color)) this.change((q) => { q.canvas_color = color; }, cx); });
    this.renameInput = InputState.new();
    this.renameInput.on("focus", (_e, cx) => { this.renameFocused = true; cx.notify(); });
    this.renameInput.on("blur", (_e, cx) => { this.renameFocused = false; cx.notify(); });
    this.renameInput.on("change", (_e, cx) => { if (this.machineEdit?.action === "rename") this.machineEdit.name = this.renameInput.value(); cx.notify(); });
    this.renameInput.on("submit", (_e, cx) => {
      const edit = this.machineEdit;
      if (edit?.action === "rename") this.machineAction(edit.machine, "rename", cx, edit.name);
    });
    this.shortcutQuery = "";
    this.shortcutInput = InputState.new({ placeholder: "Search actions, descriptions or keys" });
    this.shortcutInput.on("change", (_e, cx) => { this.shortcutQuery = this.shortcutInput.value(); cx.notify(); });
    this.shortcutInput.on("focus", (_e, cx) => { this.cancelShortcut(); cx.notify(); });
    this.recording = null;
    this.shortcutBusy = false;
    this.shortcutRevision = null;
    this.shortcutKeys = null;
    this.shortcutDefaults = {};
    this.shortcutError = null;
    this.person = {};
    this.raw = null;
    this.previews = {}; // layout name -> its "preview" picture
    this.previewsOf = "";
    this.machineList = null;
    this.machineLoading = false;
    this.machineError = null;
    this.machineBusy = {};
    this.machineResults = {};
    this.machineEdit = null;
    this.machineNotice = null;
    this.handle = cx.focus_handle();
    this.handle.focus();
    this.cancelShortcut();
    cx.spawn(async (cx) => {
      await this.load();
      if (this.section === "shortcuts") await this.loadShortcuts();
      if (this.section === "machines") await this.loadMachines();
      cx.notify();
    });
    cx.spawn(async (cx) => {
      for (;;) {
        const command = await nextCommand();
        if (command === "reset_shortcuts") this.saveShortcuts(current => resetShortcuts(current, this.shortcutDefaults, commands()), cx);
        else if (command === "focus" && this.section === "shortcuts") { await this.loadShortcuts(); cx.notify(); }
        else if (command.startsWith('{"shortcut":')) this.captureShortcut(JSON.parse(command).shortcut, cx);
      }
    });
    cx.timer.every(3000, (cx) => {
      if (this.section !== "machines") return;
      cx.spawn(async (cx) => {
        await this.loadMachines();
        cx.notify();
      });
    });
  }

  file() {
    return `${appHome()}/settings.json`;
  }

  // Each layout file may name a picture of itself (`"preview"` on its root).
  loadPreviews(names, cx) {
    this.previewsOf = names.join("\n");
    cx.spawn(async (cx) => {
      const found = {};
      for (const name of names) {
        try {
          const preview = JSON.parse((await readTextFile(`${appHome()}/layouts/${name}.json`)) ?? "{}").preview;
          if (typeof preview === "string" && preview) found[name] = preview;
        } catch {}
      }
      this.previews = found;
      cx.notify();
    });
  }

  async load() {
    this.raw = await readTextFile(this.file());
    try {
      this.person = this.raw ? JSON.parse(this.raw) : {};
    } catch {
      this.person = {};
    }
  }

  // Changes settings.json the way an agent would: read, change, write back
  // only if nobody changed it meanwhile (else read again and redo).
  change(edit, cx) {
    cx.spawn(async (cx) => {
      try {
        for (let attempt = 0; attempt < 3; attempt++) {
          await this.load();
          const next = JSON.parse(JSON.stringify(this.person));
          edit(next);
          try {
            await writeTextFile(this.file(), JSON.stringify(next, null, 2) + "\n", this.raw);
            this.person = next;
            break;
          } catch (e) {
            if (attempt === 2) throw e;
          }
        }
      } catch (e) {
        notify({ title: "Couldn't save settings", body: String(e), level: "error" });
      }
      cx.notify();
    });
  }

  close() {
    this.cancelShortcut();
    closeOverlay(props().overlay);
  }

  // Pieces ---------------------------------------------------------------

  heading(t, text) {
    return div().pt(6).pb(8).text_size(12).font_semibold().text_color(t.dim).child(text.toUpperCase());
  }

  // One setting: what it is on the left, the control on the right.
  row(t, title, about, control) {
    return h_flex()
      .w_full()
      .gap(16)
      .py(12)
      .items_center()
      .border_b(1)
      .border_color(t.border)
      .child(
        v_flex()
          .flex_1()
          .min_w(0)
          .gap(3)
          .child(div().text_size(14).text_color(t.foreground).child(title))
          .when(Boolean(about), (el) => el.child(div().text_size(12.5).line_height("18px").text_color(t.dim).child(about))),
      )
      .child(div().flex_none().child(control));
  }

  button(t, id, label, onClick) {
    return div()
      .id(id)
      .px(12)
      .py(5)
      .rounded(ui.size.radiusSm)
      .border_1()
      .border_color(t.border)
      .bg(t.raised)
      .cursor_pointer()
      .text_size(13)
      .text_color(t.foreground)
      .hover((s) => s.bg(t.hover))
      .on_click(onClick)
      .child(label);
  }

  // − value +
  stepper(t, id, value, onMinus, onPlus) {
    const b = (k, label, f) =>
      div()
        .id(`${id}-${k}`)
        .w(28)
        .h(28)
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .text_size(15)
        .text_color(t.muted)
        .hover((s) => s.bg(t.hover).text_color(t.foreground))
        .on_click(f)
        .child(label);
    return h_flex()
      .items_center()
      .rounded(ui.size.radiusSm)
      .border_1()
      .border_color(t.border)
      .bg(t.raised)
      .overflow_hidden()
      .child(b("minus", "\u2212", onMinus))
      .child(div().w(54).text_size(13).text_color(t.foreground).flex().justify_center().child(value))
      .child(b("plus", "+", onPlus));
  }

  toggle(t, id, on, onClick) {
    return div()
      .id(id)
      .w(36)
      .h(20)
      .rounded(10)
      .p(2)
      .cursor_pointer()
      .bg(on ? t.accent : t.hover)
      .flex()
      .when(on, (el) => el.justify_end())
      .on_click(onClick)
      .child(div().w(16).h(16).rounded(ui.size.radius).bg(on ? t.accent_foreground : t.muted));
  }

  // Look -----------------------------------------------------------------

  // A small drawing of each layout, in the theme's colors (as the welcome draws them).
  preview(t, name) {
    const row = () => div().flex().flex_row();
    const col = () => div().flex().flex_col();
    const term = (bg = t.background, edge = t.border) => div().flex_1().rounded(2).bg(bg).border_1().border_color(edge);
    const line = (w, c = t.dim) => div().h(3).w(w).flex_none().rounded(2).bg(c);
    const lines = (n, c) => col().gap(4).p(5).children([...Array(n)].map((_, i) => line(i % 2 ? 22 : 32, c)));
    const frame = (el) => el.size_full().rounded(6).overflow_hidden().border_1().border_color(t.border).bg(t.surface);
    if (name === "main")
      return frame(row())
        .child(col().w(42).border_r(1).border_color(t.border).child(lines(4)))
        .child(
          col()
            .flex_1()
            .child(row().h(11).gap(2).p(2).border_b(1).border_color(t.border).child(div().w(20).rounded(2).bg(t.accent)).child(div().w(16).rounded(2).bg(t.raised)).child(div().w(16).rounded(2).bg(t.raised)))
            .child(row().flex_1().p(4).gap(3).child(term()).child(term())),
        );
    if (name === "terminals") {
      const card = () => col().flex_1().rounded(4).border_1().border_color("#2c2f34").bg("#17191c").child(row().h(7).px(3).items_center().border_b(1).border_color("#25282c").child(line(14, "#5d6168")));
      return frame(col())
        .bg("#131619")
        .p(4)
        .gap(4)
        .child(row().h(10).gap(3).p(1).rounded(4).bg("#1d1f22").child(div().w(30).rounded(3).bg("#34373c").border_1().border_color("#4a4d53")).child(div().w(30)))
        .child(row().flex_1().gap(4).child(card()).child(card()));
    }
    // A layout of the person's own: a plain frame.
    return frame(col()).child(row().h(11).border_b(1).border_color(t.border)).child(row().flex_1().p(4).child(term()));
  }

  layoutPicker(t, cx) {
    const current = shared("layout") ?? "main";
    // Shipped looks in the welcome's order, then the person's own; new files show up by themselves.
    const shipped = Object.keys(LOOKS);
    const rank = (n) => (shipped.includes(n) ? shipped.indexOf(n) : shipped.length);
    const names = [...layouts()].sort((a, b) => rank(a) - rank(b) || a.localeCompare(b));
    if (names.join("\n") !== this.previewsOf) this.loadPreviews(names, cx);
    return h_flex()
      .w_full()
      .flex_wrap()
      .gap(8)
      .children(
        names.map((name) => {
          const on = name === current;
          const preview = this.previews[name];
          const picture = name === "main" && preview === "layouts/main.svg" && !settings().dark
            ? "layouts/main-light.svg"
            : preview;
          return v_flex()
            .id(`st-l-${name}`)
            .w(124)
            .gap(6)
            .p(6)
            .rounded(10)
            .border_2()
            .border_color(on ? t.accent : "#00000000")
            .bg(on ? t.raised : "#00000000")
            .cursor_pointer()
            .hover((s) => s.bg(t.raised))
            .on_click((_e, cx) => {
              run("layout.switch", { name });
              cx.notify();
            })
            .child(
              div()
                .h(62)
                .w_full()
                .overflow_hidden()
                .child(
                  picture
                    ? image(picture).w_full().h(62).rounded(6).border_1().border_color(t.border)
                    : this.preview(t, name),
                ),
            )
            .child(
              h_flex()
                .gap(5)
                .items_center()
                .child(div().text_size(12.5).text_color(on ? t.foreground : t.muted).child(LOOKS[name] ?? name))
                .when(on, (el) => el.child(svg(I("check")).size(12).text_color(t.accent))),
            );
        }),
      );
  }

  settingPicker(t, key, choices) {
    const defaultValue = choices[0][0];
    const now = this.person[key] ?? defaultValue;
    return h_flex()
      .p(2)
      .gap(2)
      .rounded(ui.size.radius)
      .border_1()
      .border_color(t.border)
      .bg(t.background)
      .children(
        choices.map(([value, label]) =>
          div()
            .id(`st-${key}-${value}`)
            .px(12)
            .py(4)
            .rounded(6)
            .text_size(13)
            .cursor_pointer()
            .when(now === value, (el) => el.bg(t.raised).text_color(t.foreground).border_1().border_color(t.border))
            .when(now !== value, (el) => el.text_color(t.muted).hover((s) => s.text_color(t.foreground)))
            .on_click((_e, cx) =>
              this.change((p) => {
                if (value === defaultValue) delete p[key];
                else p[key] = value;
              }, cx),
            )
            .child(label),
        ),
      );
  }

  themePicker(t, light) {
    const s = settings();
    const current = light ? s.theme_light : s.theme_dark;
    return h_flex()
      .w_full()
      .flex_wrap()
      .gap(8)
      .children(
        // A theme named like a layout (themes/win98.json) is that layout's own.
        themes()
          .filter((x) => Boolean(x.light) === light && (x.name === shared("layout") || !layouts().includes(x.name)))
          .map(({ name, colors: c }) => {
          const on = name === current || (current === "graphite" && name === "herdr-dark") || (current === "paper" && name === "herdr-light");
          const label = ["canopy-dark", "canopy-light", "herdr-dark", "herdr-light"].includes(name)
            ? name.split("-").map(word => word[0].toUpperCase() + word.slice(1)).join(" ")
            : name.replace(/-/g, " ");
          const dot = (color) => div().w(8).h(8).rounded(4).bg(color);
          return v_flex()
            .id(`st-t-${name}`)
            .w(124)
            .gap(6)
            .p(6)
            .rounded(10)
            .border_2()
            .border_color(on ? t.accent : "#00000000")
            .cursor_pointer()
            .hover((s) => s.bg(t.raised))
            .on_click((_e, cx) => {
              run("app.theme", { name });
              cx.notify();
            })
            .child(
              v_flex()
                .h(46)
                .w_full()
                .p(7)
                .gap(6)
                .rounded(6)
                .border_1()
                .border_color(t.border)
                .bg(c.background)
                .child(div().h(4).w(44).rounded(2).bg(c.terminal_foreground))
                .child(h_flex().gap(4).child(dot(c.accent)).child(dot(c.ansi1)).child(dot(c.ansi2)).child(dot(c.ansi3)).child(dot(c.ansi5))),
            )
            .child(
              h_flex()
                .w_full()
                .min_w(0)
                .gap(5)
                .items_center()
                .child(div().min_w(0).truncate().tooltip(label).text_size(12.5).text_color(on ? t.foreground : t.muted).child(label))
                .when(on, (el) => el.child(svg(I("check")).size(12).flex_none().text_color(t.accent))),
            );
        }),
      );
  }

  // The font in use, written in itself; a click opens the list.
  fontButton(t, which, family, def) {
    const open = this.picking === which;
    const trigger = div()
      .id(`st-f-${which}`)
      .w(220)
      .px(12)
      .py(6)
      .rounded(ui.size.radiusSm)
      .border_1()
      .border_color(open ? t.accent : t.border)
      .bg(t.raised)
      .cursor_pointer()
      .hover((s) => s.bg(t.hover))
      .on_click((_e, cx) => {
        this.picking = open ? null : which;
        this.filter = "";
        this.fontInput.set_value("");
        if (this.picking) this.fontInput.focus();
        else this.handle.focus();
        cx.notify();
      })
      .child(div().font_family(family).text_size(13.5).text_color(t.foreground).child(family));
    return Popup.new(`st-fp-${which}`, trigger)
      .when(open, (el) => el.content(this.fontList(t, which, def)));
  }

  fontList(t, which, def) {
    const q = this.filter.toLowerCase();
    const matches = fonts().filter((f) => f.toLowerCase().includes(q)).slice(0, 200);
    const choose = (name, cx) => {
      this.change((p) => {
        if (name === null) delete p[which];
        else p[which] = name;
      }, cx);
      this.picking = null;
      this.handle.focus();
      cx.notify();
    };
    const item = (id, label, family, onClick) =>
      div()
        .id(id)
        .px(10)
        .py(5)
        .rounded(6)
        .cursor_pointer()
        .hover((s) => s.bg(t.hover))
        .on_click(onClick)
        .child(div().font_family(family).text_size(13.5).text_color(t.foreground).child(label));
    return v_flex()
      .id(`st-font-list-${which}`)
      .w(360)
      .on_mouse_down_out((_e, cx) => {
        this.picking = null;
        this.handle.focus();
        cx.notify();
      })
      .rounded(ui.size.radius)
      .border_1()
      .border_color(t.border)
      .bg(t.background)
      // Let the child's native scroll run before stopping the page's wheel event.
      .on_scroll_wheel((_e, cx) => cx.stop_propagation())
      .child(
        div()
          .px(12)
          .py(8)
          .border_b(1)
          .border_color(t.border)
          .text_size(13)
          .child(Input.new(this.fontInput).h(28).text_color(t.foreground)),
      )
      .child(
        div()
          .id(`st-fl-${which}`)
          .h(200)
          .p(4)
          .overflow_y_scrollbar()
          .when(!q, (el) => el.child(item(`st-fd-${which}`, `Default (${def})`, def, (_e, cx) => choose(null, cx))))
          .children(matches.map((f, i) => item(`st-fi-${which}-${i}`, f, f, (_e, cx) => choose(f, cx)))),
      );
  }

  look(t, cx) {
    const s = settings();
    const p = this.person;
    // Numbers come back from the host as 32-bit floats: 1.1 reads as 1.100000023841858.
    const same = (a, b) => (typeof a === "number" && typeof b === "number" ? Math.abs(a - b) < 1e-4 : a === b);
    const own = (key, composed, def) => !same(p[key] ?? def, composed);
    const size = (key, value, d, cx) => this.change((q) => (q[key] = Math.max(8, Math.min(32, (q[key] ?? value) + d))), cx);
    const fontRow = (which, sizeKey, title, family, def, sizeNow) =>
      v_flex()
        .w_full()
        .child(
          this.row(
            t,
            title,
            own(which, family, def) ? "This layout sets its own; yours shows in the others." : null,
            h_flex()
              .gap(8)
              .items_center()
              .child(this.fontButton(t, which, family, def))
              // The panels' text follows zoom: they size their rows to it.
              .when(Boolean(sizeKey), (el) => el.child(this.stepper(t, `st-${sizeKey}`, `${Math.round(sizeNow)}`, (_e, cx) => size(sizeKey, sizeNow, -1, cx), (_e, cx) => size(sizeKey, sizeNow, 1, cx)))),
          ),
        );
    return v_flex()
      .w_full()
      .child(this.heading(t, "Layout"))
      .child(this.layoutPicker(t, cx))
      .child(div().h(18))
      .child(this.heading(t, "Theme"))
      .child(this.row(t, "Appearance", "Light or dark, or as your system is now.", this.settingPicker(t, "appearance", APPEARANCES)))
      .child(div().pt(12).pb(8).text_size(12.5).text_color(t.dim).child("For the layout you're in: each layout keeps its own light and dark theme."))
      .child(div().pb(6).text_size(12.5).font_medium().text_color(t.muted).child("Dark"))
      .child(this.themePicker(t, false))
      .child(div().pt(10).pb(6).text_size(12.5).font_medium().text_color(t.muted).child("Light"))
      .child(this.themePicker(t, true))
      .child(div().h(18))
      .child(this.row(t, "Canvas color", "Hex color or theme default.", h_flex().gap(8).child(Input.new(this.canvasColorInput).w(120).h(28)).child(this.button(t, "canvas-color-default", "Default", (_e, cx) => this.change((q) => { q.canvas_color = null; }, cx)))))
      .child(this.heading(t, "Canvas"))
      .child(this.row(t, "Push neighbours", "Cells nudge each other apart. Off lets them overlap. For drop-on-release, set canvas_push to \"drop\" in settings.json.", this.toggle(t, "st-canvas-push", this.person.canvas_push !== "off", (_e, cx) => this.change((q) => { if (q.canvas_push === "off") delete q.canvas_push; else q.canvas_push = "off"; }, cx))))
      .child(this.row(t, "Focused tile grows", "In tile view the selected tile takes more room. Off keeps every tile the same size.", this.toggle(t, "st-tile-grows", this.person.tile_focus_grows === true, (_e, cx) => this.change((q) => { if (q.tile_focus_grows) delete q.tile_focus_grows; else q.tile_focus_grows = true; }, cx))))
      .child(this.heading(t, "Browser"))
      .child(this.row(t, "Keep cookies", "Until quit deletes cookies and site data when Canopy quits. Kept between runs on macOS 14 or later.", this.settingPicker(t, "browser_keep_cookies", [["until_quit", "Until quit"], ["forever", "Forever"]])))
      .child(this.row(t, "Open links", "Open web links clicked in terminals in a new browser tab or in the system browser.", this.settingPicker(t, "browser_open_links", [["built_in", "Built-in browser"], ["default", "Default browser"]])))
      .when(window()?.platform === "macos", (el) => el
        .child(this.heading(t, "App"))
        .child(this.row(t, "Run in Background", "Keep Canopy running in the menu bar when you quit.",
          this.toggle(t, "st-run-in-background", this.person.run_in_background !== false, (_e, cx) => this.change((p) => {
            if (p.run_in_background === false) delete p.run_in_background;
            else p.run_in_background = false;
          }, cx)))))
      .child(this.heading(t, "Size and fonts"))
      .child(
        this.row(
          t,
          "Zoom",
          "Everything: terminals, panels, text.",
          h_flex()
            .gap(8)
            .items_center()
            .child(this.stepper(t, "st-zoom", `${Math.round(s.zoom * 100)}%`, () => run("app.zoom_out"), () => run("app.zoom_in")))
            .child(this.button(t, "st-zoom-reset", "Reset", () => run("app.zoom_reset"))),
        ),
      )
      .child(fontRow("font", "font_size", "Terminal font", s.font_family, DEFAULT_FONT, s.font_size))
      .children([
        ["terminal_line_height", "Terminal line height", 0.8, 2],
        ["terminal_scroll_speed", "Terminal scroll speed", 0.1, 5],
      ].map(([key, title, min, max]) => this.row(
        t, title,
        own(key, s[key], 1) ? "This layout sets its own; yours shows in the others." : null,
        this.stepper(t, `st-${key}`, `${Math.round(s[key] * 100)}%`,
          (_e, cx) => this.change((q) => (q[key] = Math.max(min, Math.min(max, Math.round(((q[key] ?? s[key]) - 0.1) * 10) / 10))), cx),
          (_e, cx) => this.change((q) => (q[key] = Math.max(min, Math.min(max, Math.round(((q[key] ?? s[key]) + 0.1) * 10) / 10))), cx)),
      )))
      .child(fontRow("ui_font", null, "Interface font", s.ui_font_family, DEFAULT_UI_FONT));
  }

  async loadShortcuts() {
    try {
      const name = window()?.platform === "macos" ? "keys.macos.json" : "keys.json";
      const shipped = await readTextFile(`${appHome()}/frame/settings/${name}`);
      if (shipped === null) throw new Error("Shipped shortcuts are missing. Restart Canopy to install them.");
      this.shortcutDefaults = parseShortcuts(shipped);
      const raw = await readTextFile(`${appHome()}/keys.json`);
      this.shortcutKeys = parseShortcuts(raw ?? shipped);
      this.shortcutRevision = JSON.stringify(keys());
      this.shortcutError = null;
    } catch (e) {
      this.shortcutKeys = null;
      this.shortcutError = String(e);
      notify({ title: "Couldn't read shortcuts", body: String(e), level: "error" });
    }
  }

  cancelShortcut() {
    this.recording = null;
    if (shared("settings.record-shortcut") === props().overlay) setShared("settings.record-shortcut", null);
  }

  captureShortcut(key, cx) {
    const row = this.recording;
    this.cancelShortcut();
    if (row && key !== "escape") this.saveShortcuts(current => bindShortcut(current, row, key === "backspace" ? [] : [key]), cx);
    cx.notify();
  }

  // Re-read before every edit, then use the existing atomic, expected-text write.
  saveShortcuts(edit, cx) {
    if (this.shortcutBusy) return;
    this.shortcutBusy = true;
    this.cancelShortcut();
    cx.notify();
    cx.spawn(async (cx) => {
      try {
        const file = `${appHome()}/keys.json`;
        for (let attempt = 0; ; attempt++) {
          const raw = await readTextFile(file);
          const current = parseShortcuts(raw ?? JSON.stringify(this.shortcutDefaults));
          edit(current);
          try {
            await writeTextFile(file, JSON.stringify(current, null, 2) + "\n", raw);
            this.shortcutKeys = current;
            this.shortcutRevision = JSON.stringify(keys());
            break;
          } catch (e) {
            if (attempt === 2 || await readTextFile(file) === raw) throw e;
          }
        }
        this.shortcutError = null;
      } catch (e) {
        this.shortcutError = String(e);
        notify({ title: "Couldn't save shortcuts", body: String(e), level: "error" });
      } finally {
        this.shortcutBusy = false;
        cx.notify();
      }
    });
  }

  shortcuts(t, cx) {
    // Config reloads already redraw mods. Compare the listing instead of adding a watcher.
    if (this.shortcutRevision !== null && !this.shortcutBusy && this.shortcutRevision !== JSON.stringify(keys())) {
      this.shortcutRevision = JSON.stringify(keys());
      cx.spawn(async (cx) => { await this.loadShortcuts(); cx.notify(); });
    }
    const page = v_flex().w_full().gap(8)
      .child(Input.new(this.shortcutInput).h(32).text_color(t.foreground))
      .child(this.heading(t, "Shortcuts"));
    if (!this.shortcutKeys) return page.child(div().text_color(t.dim).child(this.shortcutError ?? "Reading shortcuts"));
    const rows = shortcutRows(this.shortcutKeys, this.shortcutDefaults, commands()).filter(row => matchesShortcut(row, this.shortcutQuery));
    return page
      .child(this.button(t, "st-shortcuts-reset-all", this.shortcutBusy ? "Saving shortcuts" : "Reset all shortcuts", () => {
        if (this.shortcutBusy) return;
        this.cancelShortcut();
        openOverlay({ mod: "confirm", kind: "dialog", width: 400, props: {
          title: "Reset all shortcuts?", detail: "Restore shipped keys for this OS. Custom mod actions and unrelated sections are kept. Conflicting custom keys must be cleared first.",
          accept: "Reset all shortcuts", destructive: true, command: { name: "frame-settings.reset_shortcuts" },
        } });
      }))
      .child(div().text_size(12).text_color(t.dim).child("Click a key, then press a combination. Esc cancels; Backspace clears."))
      .when(Boolean(this.shortcutError), el => el.child(div().text_size(12).text_color(t.blocked).child(this.shortcutError)))
      .when(rows.length === 0, el => el.child(div().text_color(t.dim).child("No matching shortcuts")))
      .children(rows.map((row, i) => {
        const warnings = row.current.flatMap(key => conflicts(this.shortcutKeys[row.section], key, row.action).map(other => `${key} also binds ${other}`));
        return this.row(t, row.name, `${row.section}: ${row.about}`,
          v_flex().w(210).gap(4)
            .child(this.button(t, `st-shortcut-key-${i}`, this.recording?.id === row.id ? "Press a key (Esc cancels)" : row.current.join(", ") || "Unbound", (_e, cx) => {
              if (this.shortcutBusy) return;
              this.recording = row;
              this.handle.focus();
              setShared("settings.record-shortcut", props().overlay);
              cx.notify();
            }))
            .child(div().text_size(12).text_color(t.dim).child(`Default: ${row.defaults.join(", ") || "Unbound"}`))
            .child(this.button(t, `st-shortcut-reset-${i}`, "Reset to default", (_e, cx) => this.saveShortcuts(current => resetShortcuts(current, this.shortcutDefaults, commands(), row), cx)))
            .when(warnings.length > 0, el => el.child(div().text_size(12).text_color(t.blocked).child(warnings.join("; ")))));
      }));
  }

  keyboard(t) {
    const choices = [
      ["auto", "Automatic (Alt on US keyboards)"],
      ["on", "Alt"],
      ["off", "Types characters"],
      ["left", "Left Option is Alt"],
      ["right", "Right Option is Alt"],
    ];
    return v_flex()
      .w_full()
      .child(this.heading(t, "Option key"))
      .child(div().pb(12).text_size(13).text_color(t.dim).child("Use Option for terminal shortcuts or to type accents and special characters."))
      .children(choices.map(([value, label]) => {
        const on = (this.person.option_as_alt ?? "auto") === value;
        return h_flex()
          .id(`st-option-${value}`)
          .gap(10)
          .px(12)
          .py(10)
          .rounded(ui.size.radiusSm)
          .items_center()
          .cursor_pointer()
          .hover((s) => s.bg(t.hover))
          .on_click((_e, cx) => this.change((p) => { p.option_as_alt = value; }, cx))
          .child(div().text_color(on ? t.accent : t.muted).child(on ? "◉" : "○"))
          .child(div().text_size(14).text_color(t.foreground).child(label));
      }));
  }

  // Machines -------------------------------------------------------------

  async loadMachines() {
    if (this.machineLoading) return;
    this.machineLoading = true;
    try {
      const r = await hc(["machine", "ls", "--json"]);
      if (r.code !== 0) throw new Error(r.stderr || r.stdout || "Couldn't read machines.");
      const list = JSON.parse(r.stdout).machines;
      if (!Array.isArray(list)) throw new Error("Couldn't read machines.");
      this.machineList = list.filter((m) => !m.temporary);
      this.machineError = null;
    } catch (e) {
      this.machineError = String(e);
    } finally {
      this.machineLoading = false;
    }
  }

  machineAction(machine, action, cx, name) {
    if (this.machineBusy[machine]) return;
    if (action === "rename" && (!name?.trim() || name.trim() === machine)) return;
    this.machineBusy[machine] = action;
    delete this.machineResults[machine];
    this.machineEdit = null;
    this.handle.focus();
    cx.notify();
    cx.spawn(async (cx) => {
      try {
        const args = ["machine", action, machine];
        if (action === "rename") args.push(name.trim());
        const r = await hc(args);
        let payload = null;
        try { payload = JSON.parse(r.stdout || r.stderr); } catch {}
        if (r.code !== 0) throw new Error(payload?.error?.message || (r.stderr || r.stdout || "It didn't work.").trim());
        this.machineNotice = action === "rename" ? `Renamed ${machine} to ${name.trim()}.` : `Removed ${machine}; its terminals keep running there.`;
      } catch (e) {
        this.machineResults[machine] = { message: String(e), ok: false };
      } finally {
        delete this.machineBusy[machine];
      }
      await this.loadMachines();
      cx.notify();
    });
  }

  machineRow(t, m) {
    const busy = this.machineBusy[m.name];
    const edit = this.machineEdit?.machine === m.name ? this.machineEdit : null;
    const state = String(m.state ?? "unknown").replace(/[_-]/g, " ");
    const color = ["connected", "online"].includes(m.state) ? t.ansi2 : ["connecting", "reconnecting"].includes(m.state) ? t.working : t.blocked;
    const result = this.machineResults[m.name];
    const controls = h_flex().gap(6).items_center();
    if (busy) controls.child(svg("mods/workspace-chrome/spinner.svg").size(12).text_color(t.working).spin(900)).child(div().text_size(12).text_color(t.muted).child(busy === "rename" ? "Renaming" : "Removing"));
    else {
      for (const [action, label] of [["rename", "Rename"], ["rm", "Remove"]]) {
        controls.child(this.button(t, `st-m-${action}-${m.name}`, label, (_e, cx) => {
          this.machineEdit = { machine: m.name, action, name: m.name };
          if (action === "rename") {
            this.renameInput.set_value(m.name);
            this.renameInput.set_selected_range({ start: 0, end: encodeURIComponent(m.name).replace(/%[0-9A-F]{2}/g, "_").length });
            this.renameFocused = true;
            this.renameInput.focus();
          }
          cx.notify();
        }));
      }
    }
    const row = v_flex().w_full().py(12).gap(8).border_b(1).border_color(t.border)
      .child(h_flex().gap(10).items_center()
        .child(div().w(7).h(7).flex_none().rounded(4).bg(color))
        .child(v_flex().flex_1().min_w(0).gap(3)
          .child(div().text_size(14).text_color(t.foreground).child(m.name))
          .child(div().text_size(12.5).text_color(t.dim).child(`${m.target ?? ""} · ${state} · ${m.version ? `hc ${m.version}` : "hc version unknown"}`)))
        .when(m.keeps_running_after_logout === false, el => el.child(div().id(`st-m-logout-${m.name}`)
          .text_color(t.blocked).cursor_pointer().tooltip("Agents stop when you disconnect. Click for the fix.")
          .on_click(() => run("mod.open", { name: "machine-add" })).child("⚠")))
        .child(controls.flex_none()));
    if (m.state === "needs_login") {
      const command = `hc machine login ${m.name}`;
      row.child(v_flex().gap(5)
        .child(div().text_size(12.5).text_color(t.blocked).child("needs a login: run"))
        .child(h_flex().gap(8).items_center()
          .child(div().flex_1().min_w(0).text_size(12.5).text_color(t.foreground).child(command))
          .child(this.button(t, `st-m-login-copy-${m.name}`, this.copiedLogin === command ? "Copied" : "Copy", (_e, cx) => {
            cx.write_to_clipboard(command);
            this.copiedLogin = command;
            cx.notify();
          }))));
    }
    if (edit && !busy) {
      const cancel = this.button(t, `st-m-cancel-${m.name}`, "Cancel", (_e, cx) => { this.machineEdit = null; this.handle.focus(); cx.notify(); });
      if (edit.action === "rm") row.child(v_flex().gap(8)
        .child(div().text_size(12.5).text_color(t.muted).child(`Remove ${m.name}? Its terminals keep running there.`))
        .child(h_flex().gap(6).child(cancel).child(this.button(t, `st-m-confirm-${m.name}`, "Remove machine", (_e, cx) => this.machineAction(m.name, "rm", cx)))));
      else row.child(v_flex().gap(6)
        .child(div().text_size(12).text_color(t.dim).child("New name (addresses using the old name stop working)"))
        .child(h_flex().gap(6).items_center()
          .child(Input.new(this.renameInput).flex_1().min_w(0).h(32).px(10).rounded(ui.size.radiusSm).border_1().border_color(this.renameFocused ? t.accent : t.input).bg(t.background).text_size(13).text_color(t.foreground))
          .child(cancel).child(this.button(t, `st-m-save-${m.name}`, "Save", (_e, cx) => this.machineAction(m.name, "rename", cx, edit.name)))));
    }
    if (result) row.child(div().text_size(12.5).text_color(result.ok ? t.ansi2 : t.blocked).child(result.message));
    return row;
  }

  machines(t) {
    return v_flex().w_full()
      .child(this.heading(t, "Machines"))
      .child(div().pb(8).text_size(12.5).line_height("18px").text_color(t.dim).child("Your computers, reached over ssh. Their terminals keep running when you disconnect."))
      .child(this.row(t, "This computer", "Connected", div().w(7).h(7).rounded(4).bg(t.ansi2)))
      .children((this.machineList ?? []).map((m) => this.machineRow(t, m)))
      .when(this.machineList === null && !this.machineError, (el) => el.child(h_flex().py(14).gap(8).items_center().child(svg("mods/workspace-chrome/spinner.svg").size(12).text_color(t.working).spin(900)).child(div().text_size(12.5).text_color(t.dim).child("Reading machines"))))
      .when(this.machineList?.length === 0, (el) => el.child(div().py(14).text_size(12.5).text_color(t.dim).child("No other machines yet.")))
      .when(Boolean(this.machineError), (el) => el.child(div().py(10).text_size(12.5).text_color(t.blocked).child(this.machineError)))
      .when(Boolean(this.machineNotice), (el) => el.child(div().py(10).text_size(12.5).text_color(t.ansi2).child(this.machineNotice)))
      .child(h_flex().pt(16).child(this.button(t, "st-m-add", "Add a machine", () => run("mod.open", { name: "machine-add" }))));
  }

  // Alerts ---------------------------------------------------------------

  alerts(t) {
    const sounds = this.person.sounds ?? {};
    const on = sounds.enabled !== false;
    const file = (f) => (f ? f.split(/[\\/]/).pop() : "Canopy's sound");
    const desktop = this.person.desktop_notifications !== false;
    const page = v_flex().w_full();
    if (window()?.platform === "macos") {
      page.child(this.heading(t, "Notifications"))
        .child(this.row(t, "Desktop notifications", "Show agent alerts while Canopy is in the background.",
          this.toggle(t, "st-desktop-notifications", desktop, (_e, cx) => this.change((p) => (p.desktop_notifications = !desktop), cx))))
        .child(this.row(t, "macOS notification style", "Banners disappear automatically. Alerts stay until you dismiss them.",
          this.button(t, "st-notification-style", "Notification style", (_e, cx) => cx.spawn(async () => {
            try {
              const result = await exec("/usr/bin/open", ["x-apple.systempreferences:com.apple.Notifications-Settings.extension?id=dev.canopy.app"]);
              if (result.code !== 0) throw new Error(result.stderr.trim() || "System Settings could not be opened.");
            } catch (e) {
              notify({ title: "Couldn't open notification settings", body: String(e), level: "error" });
            }
          }))))
        .child(div().pt(12).text_size(12.5).line_height("18px").text_color(t.dim)
          .child("Agents include a message when their own terminal notifications are on."));
    }
    return page
      .child(this.heading(t, "Sounds"))
      .child(
        this.row(
          t,
          "Play sounds",
          "When an agent needs you, and when one finishes while you're not looking.",
          this.toggle(t, "st-sounds", on, (_e, cx) => this.change((p) => (p.sounds = { ...(p.sounds ?? {}), enabled: !on }), cx)),
        ),
      )
      .child(this.row(t, "An agent needs you", file(sounds.blocked), this.button(t, "st-play-blocked", "Play", () => playSound("blocked"))))
      .child(this.row(t, "An agent finished", file(sounds.done), this.button(t, "st-play-done", "Play", () => playSound("done"))))
      .child(div().pt(12).text_size(12.5).line_height("18px").text_color(t.dim).child("Want your own sounds? Ask your agent to set sounds.done and sounds.blocked in settings.json to an MP3."));
  }

  // ----------------------------------------------------------------------

  render(cx) {
    const t = theme();
    const nav = v_flex()
      .flex_none()
      .w(190)
      .p(10)
      .gap(2)
      .border_r(1)
      .border_color(t.border)
      .child(div().px(10).pt(6).pb(12).text_size(16).font_semibold().text_color(t.foreground).child("Settings"))
      .children(
        SECTIONS.map((s) => {
          const on = s.id === this.section;
          return h_flex()
            .id(`st-s-${s.id}`)
            .gap(10)
            .px(10)
            .py(7)
            .items_center()
            .rounded(ui.size.radiusSm)
            .cursor_pointer()
            .bg(on ? t.raised : "#00000000")
            .hover((st) => st.bg(t.raised))
            .on_click((_e, cx) => {
              this.cancelShortcut();
              this.section = s.id;
              this.picking = null;
              this.machineEdit = null;
              this.handle.focus();
              if (s.id === "shortcuts") cx.spawn(async (cx) => { await this.loadShortcuts(); cx.notify(); });
              if (s.id === "machines") cx.spawn(async (cx) => { await this.loadMachines(); cx.notify(); });
              cx.notify();
            })
            .child(svg(I(s.icon)).size(15).text_color(on ? t.accent : t.dim))
            .child(div().flex_1().text_size(13.5).text_color(on ? t.foreground : t.muted).child(s.title))
        }),
      );
    return h_flex()
      .id("settings")
      .track_focus(this.handle)
      .when(Boolean(this.picking) && !this.recording, (el) => el.on_action("overlay.close", (_e, cx) => {
        this.picking = null;
        this.handle.focus();
        cx.stop_propagation();
        cx.notify();
      }))
      .w_full()
      .h(620)
      .min_h(0)
      .items_start()
      .on_key_down((e, cx) => {
        if (this.recording) { cx.stop_propagation(); return; }
        const k = e.key;
        if (this.machineEdit) {
          if (k === "escape") this.machineEdit = null;
          else return;
          this.handle.focus();
          cx.stop_propagation();
          cx.notify();
          return;
        }
        if (k === "escape") {
          if (this.picking) this.picking = null;
          else return this.close();
          this.handle.focus();
        } else return;
        cx.stop_propagation();
        cx.notify();
      })
      .child(nav.h_full().child(div().flex_1()))
      .child(
        // A bar that always shows: the pages are taller than the dialog and
        // people should see that they scroll.
        div()
          .relative()
          .flex_1()
          .min_w(0)
          .h_full()
          .child(
            div()
              .id("st-body")
              .size_full()
              .overflow_y_scroll()
              // Padding inside, not on the scroll area: there it counts as
              // content and a page that fits still shows a bar.
              .child(
                div()
                  .px(28)
                  .py(22)
                  .child(
                    this.section === "look"
                      ? this.look(t, cx)
                      : this.section === "shortcuts"
                        ? this.shortcuts(t, cx)
                      : this.section === "keyboard"
                        ? this.keyboard(t)
                      : this.section === "machines"
                        ? this.machines(t)
                      : this.section === "alerts"
                        ? this.alerts(t)
                        : this.look(t, cx),
                  ),
              ),
          )
          .child(Scrollbar.vertical("st-body").mode("always").absolute().inset_0()),
      );
  }
}
