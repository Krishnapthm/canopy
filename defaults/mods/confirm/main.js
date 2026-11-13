// Asks before something destructive, e.g. "Delete space?". Opened as a dialog.
// Props: title, detail, command?, accept?, destructive?; dontAsk offers the terminal-close opt-out.
// Enter confirms (runs the command), Esc cancels.

import { View } from "gpui-kit";
import { run, props, closeOverlay, ui } from "herdr";

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
      .when(p.dontAsk, (el) => el.child(ui.checkbox("dont-ask", this.dontAsk, "Don't ask again", (checked, cx) => { this.dontAsk = checked; cx.notify(); })))
      .child(ui.footer(
        ui.button("no", p.command ? "Cancel" : "Close").on_click(close),
        ...(p.command ? [ui.button("yes", p.accept ?? "Confirm", p.destructive ? "destructive" : "primary").on_click(() => this.yes())] : []),
      ));
  }
}
