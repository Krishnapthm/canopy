// Mounted while Option is held on the canvas (the key list, from keys.json), or while a wire is
// being connected (one line: `pointer` for a wire following the pointer, `connecting` for the keyboard).
import { View, div } from "gpui-kit";
import { v_flex } from "gpui-base";
import { keys, props, theme } from "herdr";

export default class CanvasKeys extends View {
  render() {
    const t = theme();
    const hint = props()?.pointer ? "Click a pane to connect · Esc cancels" : props()?.connecting ? "Arrows pick a pane · Enter connects · Esc cancels" : null;
    if (hint) return div().size_full().flex().items_center().px(12).bg(t.chrome).text_color(t.foreground).text_size(11).child(hint);
    const rows = new Map();
    for (const k of keys().filter(k => k.section === "canvas-mode")) {
      const binding = k.action.startsWith("{") ? JSON.parse(k.action) : { command: k.action };
      const action = binding.args?.picker ? "layout picker" : typeof binding.command === "string" ? binding.command : "invalid binding";
      rows.set(action, [...(rows.get(action) ?? []), k.keys]);
    }
    const label = action => ({ "canvas.enter": "enter", "canvas.leave": "deselect / home", "canvas.fit_all": "fit all", "canvas.select_all": "select all", "app.help": "all keys", "canvas.new": "new terminal", "canvas.agents": "agent list" }[action]
      ?? action.replace(/^canvas\./, "").replace("go:", "").replaceAll(":", " "));
    return v_flex().size_full().overflow_y_scrollbar().px(12).py(8).bg(t.chrome).text_color(t.foreground).text_size(11)
      .child(div().child("Canvas keys"))
      .child(div().flex().flex_wrap().gap(8).children([...rows].map(([action, bindings]) =>
        div().child(`${bindings.join("/")} ${label(action)}`))));
  }
}
