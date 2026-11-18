// Modified for Canopy.
// A text prompt. Enter saves, Esc cancels. props: title, value, hint, command {name, args};
// optional label (default "Name"), field (the argument the text goes in, default "name")
// and submit (the button, default "Save").
import { View, div } from "gpui-kit";
import { v_flex, InputState } from "gpui-base";
import { run, props, closeOverlay, theme, ui } from "herdr";

export default class Prompt extends View {
  init(_props, cx) {
    this.text = props()?.value ?? "";
    this.input = InputState.new({ value: this.text });
    if (this.text) this.input.set_selected_range({ start: 0, end: this.text.length });
    this.input.on("change", (_e, cx) => { this.text = this.input.value(); cx.notify(); });
    this.input.on("submit", () => this.submit());
    this.inputFocused = true;
    this.input.on("focus", (_e, cx) => { this.inputFocused = true; cx.notify(); });
    this.input.on("blur", (_e, cx) => { this.inputFocused = false; cx.notify(); });
    this.input.focus();
    cx.timer.after(600, () => this.input.focus());
    this.handle = cx.focus_handle();
  }

  submit() {
    const p = props();
    closeOverlay(p.overlay);
    run(p.command.name, { ...(p.command.args ?? {}), [p.field ?? "name"]: this.text });
  }

  key(e, cx) {
    if (e.key === "escape") { closeOverlay(props().overlay); cx.stop_propagation(); }
  }

  render() {
    const p = props() ?? {};
    return ui.panel("prompt").track_focus(this.handle).on_key_down((e, cx) => this.key(e, cx))
      .child(ui.title((p.title ?? "").replace(/^./, (c) => c.toUpperCase())))
      .child(v_flex().gap(6).child(ui.label(p.label ?? "Name")).child(ui.input(this.input, this.inputFocused))
        .when(Boolean(p.hint), (el) => el.child(div().text_size(ui.size.small).text_color(theme().dim).child(p.hint))))
      .child(ui.footer(
        ui.button("pm-cancel", "Cancel").on_click(() => closeOverlay(p.overlay)),
        ui.button("pm-save", p.submit ?? "Save", "primary").on_click(() => this.submit())));
  }
}
