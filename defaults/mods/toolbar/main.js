// The canvas's floating bottom toolbar: tools and new panes. Mouse first; docs/panes.md.
import { View, div, svg, image } from "gpui-kit";
import { h_flex, v_flex } from "gpui-base";
import { props, run, theme, ui, paneKinds, openOverlay, closeOverlay, spaces, shared, setShared, knownAgents, readTextFile, home } from "herdr";

export default class Toolbar extends View {
  init(_props, cx) {
    this.tool = "select";
    // A reloaded toolbar shows Select, so the canvas must be in Select too.
    if (props()?.menu !== "agents") run("canvas.hand", { on: false });
    // paneKinds() scans the mods folder: read it once, not every render.
    this.kinds = paneKinds().filter(p => p.kind.startsWith("mod:"));
    if (props()?.menu !== "agents") return;
    const machine = spaces().find(s => s.active)?.machine;
    const where = machine ?? "";
    this.agents = (shared("new-menu.agents")?.[where] ?? []).filter(a => a.installed);
    this.checking = true;
    cx.spawn(async cx => {
      try {
        const agents = await knownAgents({ on: machine });
        setShared("new-menu.agents", { ...shared("new-menu.agents"), [where]: agents });
        this.agents = agents.filter(a => a.installed);
      } catch { this.failed = true; }
      this.checking = false;
      cx.notify();
    });
  }
  // Offers project setup once per local folder: the dialog appears when neither AGENTS.md nor
  // CLAUDE.md holds Canopy's block, and any answer (Esc included) counts. docs/agent-setup.md.
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
    if (props()?.menu === "agents") {
      return v_flex().w_full().min_h(0).p(5).bg(t.popover).text_size(ui.size.text)
        .children((this.failed ? [] : this.agents).map(a => h_flex().id(a.key).h(ui.size.control).px(9).gap(10)
          .rounded(ui.size.radiusSm).items_center().text_color(t.foreground).hover(s => s.bg(t.selected))
          .child(a.tint ? svg(a.icon).size(14).text_color(t.foreground) : image(a.icon).size(14))
          .child(div().flex_1().child(a.name))
          .on_click(() => { closeOverlay(props().overlay); run("canvas.create", { argv: a.run, name: a.name }); })))
        .when(this.failed || this.agents.length === 0, el => el.child(div().px(9).py(6).text_color(t.dim)
          .child(this.checking && !this.failed ? "Checking installed agents" : "No agent CLIs found")));
    }
    this.offerSetup(cx);
    const button = (id, icon, label, action) => ui.iconButton(id, icon, 28).tooltip(label).on_click(action);
    const tool = (name, icon, label) => button(name, icon, label, () => {
      this.tool = name;
      run("canvas.hand", { on: name === "hand" });
      cx.notify();
    }).when(this.tool === name, el => el.bg(t.selected).text_color(t.foreground));
    const divider = () => div().w(1).h(18).mx(4).bg(t.border);
    return h_flex().gap(2).items_center().p(2)
      .child(tool("select", "mods/toolbar/pointer.svg", "Select"))
      .child(tool("hand", "mods/toolbar/hand.svg", "Hand (pan)"))
      .child(divider())
      .child(button("terminal", "icons/square-terminal.svg", "Terminal", () => run("canvas.create", {})))
      .child(button("agent", "icons/bot.svg", "Agent", () => openOverlay({ mod: "toolbar", kind: "popup", at: "pointer", width: 220, props: { menu: "agents" } })))
      .child(button("file", "mods/toolbar/file-plus.svg", "Open file", () => run("preview.open", { place: "canvas" })))
      .children(this.kinds.map(p => button(p.kind, p.icon, p.label, () => run("canvas.pane_new", { kind: p.kind }))))
      .child(divider())
      .child(button("wire", "icons/link.svg", "Connect (select a pane first)", () => run("canvas.connect", {})));
  }
}
