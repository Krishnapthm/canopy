import { View, div, svg } from "gpui-kit";
import { h_flex } from "gpui-base";
import { spaces, agents, theme, ui, run, window, keys } from "herdr";

const RANK = { blocked: 4, done: 3, working: 2, idle: 1 };

// The agents to show as chips: needs-you first, then most recently changed; the rest are counted.
function chips(list, max = 6) {
  const sorted = [...list].sort((a, b) => (RANK[b.status] ?? 0) - (RANK[a.status] ?? 0) || b.seq - a.seq);
  return { shown: sorted.slice(0, max), more: Math.max(0, sorted.length - max) };
}

export default class WorkspaceStatus extends View {
  // The palette key as actually bound, not a hardcoded one; without a
  // binding the hint stays useful ("commands") instead of lying.
  hint(t) {
    const mac = window()?.platform === "macos";
    const mod = { cmd: mac ? "⌘" : "Ctrl+", ctrl: mac ? "⌃" : "Ctrl+", shift: mac ? "⇧" : "Shift+", alt: mac ? "⌥" : "Alt+", option: mac ? "⌥" : "Alt+" };
    const b = keys().find((k) => k.section === "global" && k.action === "app.palette");
    const parts = b && b.keys.endsWith("--") ? [...b.keys.slice(0, -2).split("-"), "-"] : b?.keys.split("-");
    const key = parts && parts.map((p) => mod[p.toLowerCase()] ?? p.toUpperCase()).join("");
    return key ? `${key}  commands` : "commands";
  }

  render() {
    const t = theme();
    const space = spaces().find(s => s.active);
    const { shown, more } = chips(agents());
    const dot = { blocked: t.blocked, working: t.working, done: t.done ?? t.accent, idle: t.dim };
    const chip = (id) => h_flex().id(id).gap(6).items_center().px(6).h(18).rounded(ui.size.radiusSm).cursor_pointer().hover(s => s.bg(t.hover));
    return h_flex().size_full().px(12).gap(12).items_center().bg(t.chrome).border_t(1).border_color(t.border).text_size(11.8).text_color(t.muted)
      .when(Boolean(space?.branch), el => el.child(h_flex().gap(6).items_center().child(svg("mods/workspace-chrome/branch.svg").size(13)).child(space.branch)))
      .children(shown.map(a => chip(`agent-${a.address}`)
        .tooltip(`${a.space_name} · ${a.tab_name}`)
        .on_click(() => run("agent.focus", { terminal: a.address }))
        .child(div().size(6).rounded(3).flex_none().bg(dot[a.status] ?? t.dim))
        .child(div().max_w(120).truncate().text_color(a.status === "blocked" ? t.blocked : t.muted).child(a.program?.name || a.title || a.address))))
      .when(more > 0, el => el.child(chip("agents-more").on_click(() => run("mod.open", { name: "picker" })).child(`+${more}`)))
      .child(div().flex_1())
      .child(div().id("palette").text_color(t.dim).on_click(() => run("app.palette")).child(this.hint(t)));
  }
}
