// The canvas's floating bottom toolbar: tools and new panes. Mouse first; docs/wiki/Toolbar-and-Helper-Bar.md.
import { View, div } from "gpui-kit";
import { h_flex } from "gpui-base";
import { run, theme, ui, paneKinds, openOverlay, spaces, shared, setShared, readTextFile, home, window, keys } from "herdr";

// Object bindings are serialized by serde_json with sorted field names.
function key(action, args) {
  const object = JSON.stringify({ args, command: action });
  const rows = keys().filter(b => b.action === action || b.action === object);
  const rank = section => ({ "canvas-mode": 0, "layout:main": 1, global: 2 }[section] ?? 3);
  rows.sort((a, b) => rank(a.section) - rank(b.section));
  return rows[0]?.label ?? "";
}

export default class Toolbar extends View {
  init() {
    // paneKinds() scans the mods folder: read it once, not every render.
    const kinds = paneKinds();
    // Browser and simulator panes open from the canvas bar's top right, pinned.
    this.kinds = kinds.filter(p => p.kind.startsWith("mod:"));
  }
  // Offers project setup once per local folder: the dialog appears when neither AGENTS.md nor
  // CLAUDE.md holds Canopy's block, and any answer (Esc included) counts. docs/wiki/Agent-Setup.md.
  // shortcut: local spaces only; remote folders need writeTextFile on another machine.
  offerSetup(cx) {
    const space = spaces().find(s => s.active);
    const folder = space && space.machine == null ? (space.folder ?? "").replace(/\/+$/, "") : "";
    // The home folder is no project: its AGENTS.md would read like a global file.
    if (!folder || folder === home().replace(/\/+$/, "") || folder === this.offeredFor) return;
    this.offeredFor = folder;
    if (shared("project-setup.asked")?.[folder]) return;
    cx.spawn(async () => {
      const texts = await Promise.all(["AGENTS.md", "CLAUDE.md"].map(f => readTextFile(`${folder}/${f}`).catch(() => null)));
      if (texts.some(t => t?.includes("<!-- canopy:begin -->"))) return;
      setShared("project-setup.asked", { ...(shared("project-setup.asked") ?? {}), [folder]: true });
      openOverlay({ mod: "project-setup", kind: "dialog", width: 560, props: { folder } });
    });
  }
  render(cx) {
    const t = theme();
    this.offerSetup(cx);
    const win = window() ?? {};
    const space = spaces().find(s => s.active);
    const button = (id, icon, label, action, hint) => ui.iconButton(id, icon, 28)
      .tooltip(hint ? `${label}  ${hint}` : label).on_click(action)
      .when(win.keys && Boolean(hint), el => el.w_auto().px(4).gap(4)
        .child(div().text_size(11).text_color(t.muted).child(hint)));
    const tool = (name, icon, label) => button(name, icon, label,
      () => run("canvas.hand", { on: name === "hand" }),
      name === "hand" ? "Space" : key("canvas.hand", { on: false }))
      .when(win.hand === (name === "hand"), el => el.bg(t.selected).text_color(t.foreground));
    const divider = () => div().w(1).h(18).mx(2).bg(t.border);
    return h_flex().gap(2).items_center()
      .child(tool("select", "mods/toolbar/pointer.svg", "Select"))
      .child(tool("hand", "mods/toolbar/hand.svg", "Hand (pan)"))
      .child(divider())
      .child(button("terminal", "icons/square-terminal.svg", "Terminal", () => run("canvas.new"), key("canvas.new")))
      .child(button("agent", "icons/bot.svg", "Agent", () => openOverlay({ mod: "menu", kind: "popup", at: "above", width: 220, props: { tab: "canvas-agents", target: { ...(space ? { space: space.id } : {}), ...(space?.machine ? { machine: space.machine } : {}) } } }), key("canvas.agents")))
      .child(button("file", "mods/toolbar/file-plus.svg", "Open file", () => run("preview.open", { place: "canvas" }), key("preview.open", { place: "canvas" })))
      .children(this.kinds.map(p => button(p.kind, p.icon, p.label, () => run("canvas.pane_new", { kind: p.kind }), key("canvas.pane_new", { kind: p.kind }))))
      .child(divider())
      .child(button("wire", "icons/link.svg", "Connect (select a pane first)", () => run("canvas.connect", {}), key("canvas.connect")));
  }
}
