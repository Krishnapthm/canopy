// Mounted while Option is held on the canvas, or while a wire is being connected. Bindings come from keys.json.
import { View, div } from "gpui-kit";
import { v_flex } from "gpui-base";
import { keys, props, theme } from "herdr";

export default class CanvasKeys extends View {
  render() {
    const t = theme();
    const rows = new Map();
    for (const k of keys().filter(k => k.section === "canvas-mode")) {
      const binding = k.action.startsWith("{") ? JSON.parse(k.action) : { command: k.action };
      const action = binding.args?.picker ? "layout picker" : typeof binding.command === "string" ? binding.command : "invalid binding";
      rows.set(action, [...(rows.get(action) ?? []), k.keys]);
    }
    const label = action => ({ "canvas.enter": "enter", "canvas.leave": "deselect / home", "canvas.fit_all": "fit all", "canvas.select_all": "select all", "app.help": "all keys", "canvas.new": "new terminal", "canvas.agents": "agent list" }[action]
      ?? action.replace(/^canvas\./, "").replace("go:", "").replaceAll(":", " "));
    return v_flex().size_full().overflow_y_scrollbar().px(12).py(8).bg(t.chrome).text_color(t.foreground).text_size(11)
      .child(div().child(props()?.connecting ? "Connect: select the other cell, then confirm or cancel" : "Canvas keys"))
      .child(div().flex().flex_wrap().gap(8).children([...rows].map(([action, bindings]) =>
        div().child(`${bindings.join("/")} ${label(action)}`))));
  }
}
