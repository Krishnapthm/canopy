// Modified for Canopy.
// Context menus (Canopy's items). Opened as a popup with
// props { tab: "pane" | "tab" | "space" | "global", target }.
// ↑/↓ move, Enter runs, Esc closes. An item is [label, command, args, program?];
// a command "worktree" opens the worktree dialog instead. A null command is a
// section label or a note: shown, never chosen.

import { View, div, svg, image } from "gpui-kit";
import { v_flex, h_flex } from "gpui-base";
import { run, props, closeOverlay, openOverlay, theme, keys, terminals, spaces, panes, exec, knownAgents } from "herdr";

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
    case "canvas-empty": return [
      ["New terminal", "canvas.create", at],
      ["Open file...", "preview.open", { point: tg.point }],
      ["Arrange", "canvas.arrange", {}],
      ["Arrange >", "canvas.arrange", { picker: true }],
      ["Undo layout", "canvas.undo", {}],
      ...(!tg.space ? [["New space here", "canvas.create", { ...at, kind: "space" }]] : []),
      ["Agents", null, { label: true }],
      ...agentItems(agents, at),
    ];
    case "canvas-space": return [
      ["Arrange", "canvas.arrange", { space: tg.id }],
      ["Arrange >", "canvas.arrange", { space: tg.id, picker: true }],
      ["Undo layout", "canvas.undo", {}],
      ["Rename", "space.rename", { id: tg.id }],
      ["Color >", "canvas-colors", tg],
      ["Show frame", "canvas.space_style", { id: tg.id }],
      ["Delete space", "space.close", { id: tg.id }],
    ];
    case "canvas-arrange": return [["Tidy", "tidy"], ["Row", "row"], ["Column", "column"], ["By wires", "wires"]].map(([name, layout]) => [name, "canvas.arrange", { ...tg, layout }]);
    case "canvas-colors": return ["gray", "red", "orange", "yellow", "green", "teal", "blue", "purple"].map(color => [color, "canvas.space_style", { id: tg.id, color }]);
    case "canvas-sub": return [["Message through parent", "canvas.sub_message", { terminal: tg.terminal, id: tg.id }]];
    case "canvas-card": return [
      ["Arrange", "canvas.arrange", {}],
      ["Arrange >", "canvas.arrange", { picker: true }],
      ...(tg.id ? [["Rename", "tab.rename", { id: tg.id }]] : []),
      [tg.popped ? "Bring back" : "Pop out", tg.popped ? "panel.bring_back" : "panel.pop_out", { terminal: tg.terminal }],
      ...(!tg.popped ? [["Open in panel", "panel.open", { terminal: tg.terminal }]] : []),
      ...(tg.finished ? [["Clear finished", "canvas.clear_finished", { terminal: tg.terminal }]] : []),
      ["Close terminal", "canvas.close", { terminal: tg.terminal }],
    ];
    case "pane": {
      // A kept sub-agent can go back under its parent, still running.
      const all = terminals();
      const term = all.find((t) => t.address === tg.terminal);
      const parent = term?.parent && all.find((t) => t.address === term.parent);
      // Once they go to the program, Shift+right-click still opens this menu.
      const toProgram = term?.labels?.["herdr.right-click"] === "program";
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
        ...(term
          ? [
              toProgram
                ? ["Use Canopy right-click menu", "terminal.right_click", { terminal: term.address, to: "menu" }]
                : ["Send right-clicks to pane", "terminal.right_click", { terminal: term.address, to: "program" }],
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
        ["New space", "space.new", {}],
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
  if (!agents) return [["Checking installed agents...", null, {}]];
  const installed = agents.filter(a => a.installed);
  if (!installed.length) return [["No agents installed", null, {}]];
  return installed.map(a => [a.name, "canvas.create", { ...at, argv: a.run }, a]);
}

function menuIcon(command) {
  const name = command.endsWith(".close") ? "x" : command.endsWith(".rename") ? "square-pen" : command.endsWith(".new") ? "plus" : command === "worktree" ? "git-branch" : command === "app.help" ? "terminal" : command === "app.theme" ? "settings" : command === "pane.split" ? "panel-right" : command === "pane.zoom" ? "square" : "terminal";
  return `mods/workspace-chrome/${name}.svg`;
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
    if (p?.tab === "canvas-empty")
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

  choose(i, cx) {
    const it = items(props(), this.git, this.agents)[i];
    if (!it?.[1]) return;
    closeOverlay(props()?.overlay);
    if (it[1] === "worktree") openOverlay({ mod: "worktree", kind: "dialog", ...(it[2].action === "open" ? { width: 560, height: 480 } : { width: 440, height: 260 }), props: it[2] });
    else if (it[1] === "canvas-colors") openOverlay({ mod: "menu", kind: "popup", width: 220, props: { tab: "canvas-colors", target: it[2] } });
    else run(it[1], it[2]);
  }

  render(cx) {
    const t = theme();
    const list = items(props(), this.git, this.agents);
    // Arrow keys step over labels and notes.
    const step = (d) => { for (let n = 1; n <= list.length; n++) { const j = (this.at + d * n + list.length * n) % list.length; if (list[j][1]) return j; } return this.at; };
    return v_flex()
      .id("menu")
      .track_focus(this.handle)
      .w_full()
      .min_h(0)
      .p(5)
      .bg(t.popover)
      .text_size(12.8)
      .on_key_down((e, cx) => {
        if (e.key === "down") this.at = step(1);
        else if (e.key === "up") this.at = step(-1);
        else if (e.key === "enter") this.choose(this.at, cx);
        else if (e.key === "escape") closeOverlay(props().overlay);
        else return;
        cx.notify();
      })
      .children(
        list.map((it, i) => !it[1]
          // shadcn ContextMenuSeparator + ContextMenuLabel; a note is a dim row.
          ? v_flex()
            .when(it[2].label, el => el.child(div().mx(7).my(4).h(1).bg(t.border)))
            .child(div().px(8).py(it[2].label ? 5 : 7).text_size(it[2].label ? 11.5 : 12.8).text_color(t.dim)
              .when(it[2].label, el => el.font_semibold()).child(it[0]))
          : v_flex()
            .when(i > 0 && (it[1].endsWith(".close") || it[1] === "app.help" || (it[1] === "app.theme" && list[i - 1][1] !== "app.theme")), el => el.child(div().mx(7).my(4).h(1).bg(t.border)))
            .child(h_flex()
            .id(`mi-${i}`)
            .h(31)
            .px(8)
            .gap(9)
            .rounded(7)
            .items_center()
            .justify_between()
            .text_color(t.foreground)
            .when(i === this.at, (el) => el.bg(t.selected))
            .hover((s) => s.bg(t.hover))
            .on_click((_e, cx) => this.choose(i, cx))
            .child(it[3]?.icon
              ? (it[3].tint ? svg(it[3].icon).size(14).flex_none().text_color(t.foreground) : image(it[3].icon).w(14).h(14).flex_none())
              : svg(menuIcon(it[1])).size(14).flex_none().text_color(t.muted))
            .child(div().flex_1().child(it[0]))
            .child(div().text_size(11).text_color(t.dim).child(shortcut(it[1], it[2])))),
        ),
      );
  }
}
