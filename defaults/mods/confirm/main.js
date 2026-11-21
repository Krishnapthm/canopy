// Modified for Canopy.
// Asks before something destructive, e.g. "Delete space?". Opened as a dialog.
// Props: title, detail, panes? (icon, tint, name), command?, accept?, cancel?, destructive?, dontAsk?.
// Enter confirms (runs the command), Esc cancels.

import { View, div, svg, image } from "gpui-kit";
import { run, props, closeOverlay, theme, ui } from "herdr";

export default class Confirm extends View {
  init(_props, cx) {
    this.dontAsk = false;
    this.handle = cx.focus_handle();
    this.handle.focus();
  }

  yes() {
    const p = props();
    closeOverlay(p.overlay);
    if (p.command) run(p.command.name, { ...(p.command.args ?? {}), ...(p.dontAsk && this.dontAsk ? { dont_ask: true } : {}) });
  }

  render() {
    const p = props() ?? {};
    const close = () => closeOverlay(props().overlay);
    return ui.panel("confirm")
      .track_focus(this.handle)
      .on_key_down((e, cx) => {
        if (e.key === "enter") this.yes();
        else if (e.key === "escape") close();
        else return;
        cx.stop_propagation();
      })
      .child(ui.title(p.title ?? "Are you sure?"))
      .when(Boolean(p.detail), (el) => el.child(ui.description(p.detail)))
      .when(Boolean(p.panes?.length), el => el.child(div().id("confirm-panes").max_h(320).overflow_y_scroll()
        .children(p.panes.map((pane, i) => div().id(`confirm-pane-${i}`).flex().items_center().gap(8).py(4)
          .child(pane.tint ? svg(pane.icon).size(16).flex_none().text_color(theme().muted) : image(pane.icon).w(16).h(16).flex_none())
          .child(div().flex_1().min_w(0).child(pane.name))))))
      .when(p.dontAsk, (el) => el.child(ui.checkbox("dont-ask", this.dontAsk, "Don't ask again", (checked, cx) => { this.dontAsk = checked; cx.notify(); })))
      .child(ui.footer(
        ui.button("no", p.cancel ?? (p.command ? "Cancel" : "Close")).on_click(close),
        ...(p.command ? [ui.button("yes", p.accept ?? "Confirm", p.destructive ? "destructive" : "primary").on_click(() => this.yes())] : []),
      ));
  }
}
