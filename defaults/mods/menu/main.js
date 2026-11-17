// Modified for Canopy.
// Context menus (Canopy's items). Opened as a popup with
// props { tab: "pane" | "tab" | "space" | "global", target }.
// ↑/↓ move, Enter runs, → opens a submenu, ← or Esc closes. An item is
// [label, command, args, program?]; a command "worktree" opens the worktree
// dialog instead, and "sub:<tab>" is a submenu of that tab with args as its
// target, shown beside the row on hover (shadcn ContextMenuSub). A null
// command is a section label or a note: shown, never chosen.

import { View, div, svg, image } from "gpui-kit";
import { v_flex, h_flex } from "gpui-base";
import { run, props, closeOverlay, openOverlay, theme, keys, terminals, spaces, panes, exec, knownAgents, ui } from "herdr";

// Where a space is now: its first terminal's live folder, else where it was made.
function place(s) {
  const tabs = new Set((s.tabs ?? []).map((c) => c.id));
  return panes().find((p) => tabs.has(p.tab) && p.cwd)?.cwd || s.folder;
}

// A space's checkout: "main" (a repo's own), "linked" (a worktree), or null.
async function checkout(folder, on) {
  const top = await exec("git", ["rev-parse", "--show-toplevel"], { cwd: folder, on });
  const common = await exec("git", ["rev-parse", "--path-format=absolute", "--git-common-dir"], { cwd: folder, on });
  if (top.code !== 0 || common.code !== 0) return null;
  return common.stdout.trim() === `${top.stdout.trim()}/.git` ? "main" : "linked";
}

// `agents`: knownAgents() for canvas-empty, undefined while it is still checking.
function items(p, git, agents) {
  p = p ?? {};
  const tg = p.target ?? {};
  // `machine` only picks which agents to list; canvas.create does not take it.
  const { machine: _, ...at } = tg;
  switch (p.tab) {
    case "primary": return tg.items ?? [];
    case "canvas-editors": return [["Open here", "panel.editor", { choice: "here", path: tg.path }, { icon: "mods/workspace-chrome/square-pen.svg", tint: true }],
      ...(tg.editors ?? []).map(e => [`Open in ${e.name}`, "panel.editor", { choice: e.key, path: tg.path }, e])];
    case "canvas-agents": return agentItems(agents, at);
    case "canvas-empty": return [
      ["New terminal", "canvas.create", at],
      ["Open file", "preview.open", { point: tg.point }],
      ["Arrange", "canvas.arrange", {}],
      ["Arrange as", "sub:canvas-arrange", {}],
      ["Tile view", "canvas.tile", {}],
      ["Undo layout", "canvas.undo", {}],
      ...(!tg.space ? [["Open space here", "canvas.create", { ...at, kind: "space" }]] : []),
      ["Agents", null, { label: true }],
      ...agentItems(agents, at),
    ];
    case "canvas-space": return [
      ["Arrange", "canvas.arrange", { space: tg.id }],
      ["Arrange as", "sub:canvas-arrange", { space: tg.id }],
      ["Undo layout", "canvas.undo", {}],
      ["Rename", "space.rename", { id: tg.id }],
      ["Color", "sub:canvas-colors", tg],
      ["Delete space", "space.close", { id: tg.id }],
    ];
    case "canvas-arrange": return [["Tidy", "tidy"], ["Row", "row"], ["Column", "column"], ["By wires", "wires"]].map(([name, layout]) => [name, "canvas.arrange", { ...tg, layout }]);
    case "canvas-colors": return ["gray", "red", "orange", "yellow", "green", "teal", "blue", "purple"].map(color => [color, "canvas.space_style", { id: tg.id, color }]);
    case "canvas-wire": return [["Remove", "wire.remove", { a: tg.a, b: tg.b }]];
    // A cell's own menu: a pill or header right-click, or a right-click inside its terminal.
    case "canvas-card": return [
      ...(tg.id ? [["Rename", "tab.rename", { id: tg.id }]] : []),
      [tg.popped ? "Bring back" : "Pop out", tg.popped ? "panel.bring_back" : "panel.pop_out", { terminal: tg.terminal }],
      ...(!tg.popped ? [[tg.expanded ? "Collapse" : "Expand", "canvas.expand", { terminal: tg.terminal }]] : []),
      ...(tg.finished ? [["Clear finished", "canvas.clear_finished", { terminal: tg.terminal }]] : []),
      ["Bring to front", "canvas.front", { terminal: tg.terminal }],
      ["Send to back", "canvas.back", { terminal: tg.terminal }],
      ["Close terminal", "canvas.close", { terminal: tg.terminal }],
      ["Kill terminal", "terminal.kill", { terminal: tg.terminal }],
    ];
    case "pane": {
      // A kept sub-agent can go back under its parent, still running.
      const all = terminals();
      const term = all.find((t) => t.address === tg.terminal);
      const parent = term?.parent && all.find((t) => t.address === term.parent);
      // A terminal shown outside a tab (a layout terminal block, a window) has
      // no pane to split, zoom or close: its pane is the block's id.
      const inTab = panes().some((pane) => pane.pane === tg.pane);
      return [
        ...(inTab
          ? [
              ["Split right", "pane.split", { direction: "right", pane: tg.pane }],
              ["Split down", "pane.split", { direction: "down", pane: tg.pane }],
              ["Zoom", "pane.zoom", { pane: tg.pane }],
            ]
          : []),
        ...(parent ? [[`Send back to ${parent.name || parent.address}`, "terminal.park", { terminal: term.address }]] : []),
        ...(inTab ? [["Close pane", "pane.close", { pane: tg.pane }]] : []),
      ];
    }
    case "tab":
      return [
        ["Rename", "tab.rename", { id: tg.id }],
        ["New tab", "tab.new", { space: tg.space }],
        ["Close", "tab.close", { id: tg.id }],
      ];
    case "space":
      return [
        ["Rename", "space.rename", { id: tg.id }],
        ...(git ? [["New worktree", "worktree", { space: tg.id, action: "new" }], ["Open worktree", "worktree", { space: tg.id, action: "open" }]] : []),
        ...(git === "linked" ? [["Remove worktree", "worktree", { space: tg.id, action: "remove" }]] : []),
        ["Close", "space.close", { id: tg.id }],
      ];
    default:
      return [
        ["Open space", "space.new", {}],
        ["Keybinds", "app.help", {}],
        ["Theme: graphite", "app.theme", { name: "graphite" }],
        ["Theme: ink", "app.theme", { name: "ink" }],
        ["Theme: moss", "app.theme", { name: "moss" }],
        ["Theme: atom one dark", "app.theme", { name: "atom-one-dark" }],
      ];
  }
}

// Installed agents from the program table, so a newly listed agent appears with no code change.
function agentItems(agents, at) {
  if (!agents) return [["Checking installed agents", null, {}]];
  const installed = agents.filter(a => a.installed);
  if (!installed.length) return [["No agents installed", null, {}]];
  return installed.map(a => [a.name, "canvas.create", { ...at, argv: a.run }, a]);
}

// One menu box's width and row heights, so a submenu can line up with its row.
const WIDTH = 220, ROW = 32, LABEL = 25;

// A rule above row i: before closing, help and the first theme.
function rule(list, i) {
  const c = list[i][1];
  return i > 0 && Boolean(c) && (c.endsWith(".close") || c === "app.help" || (c === "app.theme" && list[i - 1][1] !== "app.theme"));
}

// The key bound to a command, for the hint on the right.
function shortcut(name, args) {
  // `pane.split:right` binds pane.split with { direction: "right" }.
  const value = args && Object.values(args)[0];
  const k = keys().find((b) => b.section === "global" && (b.action === name || b.action === `${name}:${value}`));
  return k ? k.keys : "";
}

export default class Menu extends View {
  init(_props, cx) {
    this.git = null;
    this.handle = cx.focus_handle();
    this.handle.focus();
    const p = props();
    // Checked when the menu opens: it runs a login shell, so it can take seconds.
    if (["canvas-empty", "canvas-agents"].includes(p?.tab))
      cx.spawn(async (cx) => {
        this.agents = await knownAgents({ on: p.target?.machine }).catch(() => []);
        cx.notify();
      });
    this.at = 0;
    const space = p?.tab === "space" && spaces().find((s) => s.id === p.target?.id);
    const folder = space && place(space);
    if (folder)
      cx.spawn(async (cx) => {
        this.git = await checkout(folder, space.machine).catch(() => null);
        cx.notify();
      });
  }

  // `sub`: the open submenu {i, tab, target}; `subAt`: its keyboard row, or -1 while the keys stay on the menu.
  choose(i, cx) {
    const it = items(props(), this.git, this.agents)[i];
    if (!it?.[1]) return;
    if (it[1].startsWith("sub:")) return this.open(i, it, 0, cx);
    this.run(it);
  }

  run(it) {
    closeOverlay(props()?.overlay);
    if (it[1] === "worktree") openOverlay({ mod: "worktree", kind: "dialog", ...(it[2].action === "open" ? { width: 560, height: 480 } : { width: 440, height: 260 }), props: it[2] });
    else run(it[1], it[2]);
  }

  open(i, it, at, cx) {
    this.sub = { i, tab: it[1].slice(4), target: it[2] };
    this.subAt = at;
    cx.notify();
  }

  // One menu box: the rows of `list`, `at` highlighted.
  box(t, list, at, row) {
    return v_flex().w(WIDTH).p(5).bg(t.popover).border_1().border_color(t.border).rounded(ui.size.radius).shadow_lg().text_size(ui.size.text)
      // Clicks inside the box never reach the root, which closes the menu.
      .on_mouse_down("left", (_e, cx) => cx.stop_propagation())
      .children(list.map((it, i) => !it[1]
        // shadcn ContextMenuSeparator + ContextMenuLabel; a note is a dim row.
        ? v_flex()
          .when(it[2].label, el => el.child(div().mx(7).my(4).h(1).bg(t.border)))
          .child(div().h(it[2].label ? LABEL : ROW).px(8).flex().items_center().text_size(it[2].label ? 11.5 : 12.8).text_color(t.dim)
            .when(it[2].label, el => el.font_semibold()).child(it[0]))
        : v_flex()
          .when(rule(list, i), el => el.child(div().mx(7).my(4).h(1).bg(t.border)))
          .child(row(h_flex()
            .id(`mi-${list === this.list ? "" : "s"}${i}`)
            .h(ROW).px(8).gap(9).rounded(ui.size.radiusSm).items_center().justify_between()
            .text_color(t.foreground)
            .when(i === at, (el) => el.bg(t.selected))
            .hover((s) => s.bg(t.hover))
            // Only agents carry an icon: the program's own.
            .when(it[1] === "canvas.create" && it[3]?.icon, el => el.child(it[3].tint ? svg(it[3].icon).size(14).flex_none().text_color(t.foreground) : image(it[3].icon).w(14).h(14).flex_none()))
            .child(div().flex_1().child(it[0]))
            .child(it[1].startsWith("sub:")
              ? svg("mods/workspace-chrome/chevron-right.svg").size(12).flex_none().text_color(t.muted)
              : div().text_size(11).text_color(t.dim).child(shortcut(it[1], it[2]))), it, i))));
  }

  render(cx) {
    const t = theme();
    const list = this.list = items(props(), this.git, this.agents);
    const sub = this.sub && items({ tab: this.sub.tab, target: this.sub.target }, null, null);
    // Arrow keys step over labels and notes.
    const step = (rows, from, d) => { for (let n = 1; n <= rows.length; n++) { const j = (from + d * n + rows.length * n) % rows.length; if (rows[j][1]) return j; } return from; };
    const inSub = sub && this.subAt >= 0;
    // The submenu hangs beside its row: rows above it, their rules and the box padding.
    const top = this.sub ? list.slice(0, this.sub.i + 1).reduce((y, it, i) => y + (it[1] || !it[2]?.label ? ROW : LABEL) + (rule(list, i) || it[2]?.label ? 9 : 0), 5) - ROW - 5 : 0;
    return h_flex()
      .id("menu")
      .track_focus(this.handle)
      .w_full()
      .min_h(0)
      .items_start()
      .gap(4)
      // The overlay is wider than the menu, for the submenu: a click on the empty part closes it.
      .on_mouse_down("left", () => closeOverlay(props().overlay))
      .on_key_down((e, cx) => {
        if (inSub) {
          if (e.key === "down") this.subAt = step(sub, this.subAt, 1);
          else if (e.key === "up") this.subAt = step(sub, this.subAt, -1);
          else if (e.key === "enter") return sub[this.subAt]?.[1] && this.run(sub[this.subAt]);
          else if (e.key === "left" || e.key === "escape") this.subAt = -1;
          else return;
        } else if (e.key === "down") { this.at = step(list, this.at, 1); this.sub = null; }
        else if (e.key === "up") { this.at = step(list, this.at, -1); this.sub = null; }
        else if (e.key === "right" && list[this.at]?.[1]?.startsWith("sub:")) return this.open(this.at, list[this.at], 0, cx);
        else if (e.key === "enter") this.choose(this.at, cx);
        else if (e.key === "escape") closeOverlay(props().overlay);
        else return;
        cx.notify();
      })
      .child(this.box(t, list, this.at, (row, it, i) => row
        .on_hover((on, cx) => {
          if (!on) return;
          this.at = i;
          if (it[1].startsWith("sub:")) { if (this.sub?.i !== i) this.open(i, it, -1, cx); }
          else if (this.sub) { this.sub = null; cx.notify(); }
          else cx.notify();
        })
        .on_click((_e, cx) => this.choose(i, cx))))
      .when(Boolean(sub), el => el.child(div().mt(top)
        .child(this.box(t, sub ?? [], this.subAt, (row, it) => row.on_click(() => this.run(it))))));
  }
}
