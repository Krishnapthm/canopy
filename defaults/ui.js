// The shared look of shipped mods: one set of sizes and a few styled pieces,
// exported as `ui` from "herdr" (`import { ui } from "herdr"`). Each piece is an
// ordinary element, so callers keep chaining (`ui.button(...).on_click(...)`).
// Colors come from theme() on every call, so pieces follow theme changes.
import { div, svg } from "gpui-kit";
import { h_flex, v_flex, Checkbox, Input } from "gpui-base";

export const ui = {
  /** Corner radii, control height, spacing and text sizes, in px. */
  size: { radius: 8, radiusSm: 6, radiusLg: 10, control: 32, gap: 8, pad: 20, text: 13, small: 12, title: 15 },

  /** A button. variant: "secondary" (default), "primary", "ghost" or "destructive". */
  button(id, label, variant = "secondary") {
    const t = theme();
    const [bg, edge, fg] = {
      primary: [t.solid, t.solid, t.solid_foreground],
      ghost: ["#00000000", "#00000000", t.foreground],
      destructive: [t.red, t.red, t.background],
    }[variant] ?? [t.raised, t.input, t.foreground];
    return h_flex().id(id).h(ui.size.control).px(14).gap(6).flex_none().items_center().justify_center()
      .rounded(ui.size.radius).border_1().border_color(edge).bg(bg).text_color(fg)
      .text_size(ui.size.text).font_medium().cursor_pointer()
      .hover((s) => (variant === "ghost" || variant === "secondary" ? s.bg(t.hover) : s.opacity(0.88)))
      .child(label);
  },

  /** A square icon-only button; `icon` is an svg path. */
  iconButton(id, icon, size = 28) {
    const t = theme();
    return h_flex().id(id).size(size).flex_none().items_center().justify_center().rounded(ui.size.radiusSm)
      .text_color(t.muted).cursor_pointer().hover((s) => s.bg(t.hover).text_color(t.foreground))
      .child(svg(icon).size(Math.round(size * 0.55)));
  },

  /** A labelled checkbox with a real 16 px box. onChange(checked, cx). */
  checkbox(id, checked, label, onChange) {
    const t = theme();
    const box = h_flex().size(16).flex_none().items_center().justify_center().rounded(4).border_1()
      .border_color(checked ? t.solid : t.input).bg(checked ? t.solid : t.background)
      .when(checked, (el) => el.child(svg("frame/settings/check.svg").size(12).text_color(t.solid_foreground)));
    return Checkbox.new(id).checked(checked).accessibility_label(label).flex().gap(8).items_center()
      .text_size(ui.size.text).text_color(t.foreground).cursor_pointer().on_change(onChange)
      .child(box).child(label);
  },

  /** A text field for an InputState; `focused` draws the ring. */
  input(state, focused = false) {
    const t = theme();
    return Input.new(state).h(ui.size.control).px(10).rounded(ui.size.radius).border_1()
      .border_color(focused ? t.ring : t.input).bg(t.background).text_size(ui.size.text).text_color(t.foreground);
  },

  /** The body of a dialog or panel overlay: padding, gap and popover color. */
  panel(id) {
    const t = theme();
    return v_flex().id(id).w_full().min_h(0).p(ui.size.pad).gap(12).rounded(ui.size.radius).bg(t.popover)
      .text_size(ui.size.text).text_color(t.foreground);
  },

  /** A dialog heading that wraps; `hint` (e.g. "Esc") sits at the right. */
  title(text, hint = "Esc") {
    const t = theme();
    return h_flex().items_start().gap(12)
      .child(div().flex_1().min_w(0).text_size(ui.size.title).font_semibold().text_color(t.foreground).child(text))
      .when(Boolean(hint), (el) => el.child(ui.kbd(hint)));
  },

  /** Secondary text under a title, at its natural height. */
  description(text) {
    return div().min_w(0).text_size(ui.size.text).text_color(theme().muted).child(text);
  },

  /** A small field label above an input. */
  label(text) {
    return div().text_size(ui.size.small).text_color(theme().dim).child(text);
  },

  /** Right-aligned actions at the bottom of a dialog. */
  footer(...buttons) {
    return h_flex().pt(8).gap(8).justify_end().children(buttons);
  },

  /** A key or short hint, like "Esc" or "⌘K". */
  kbd(text) {
    const t = theme();
    return div().flex_none().px(6).py(1).rounded(4).border_1().border_color(t.border)
      .text_size(11).text_color(t.dim).child(text);
  },

  /** One row of a list or menu. */
  row(id, selected = false) {
    const t = theme();
    return h_flex().id(id).h(ui.size.control).px(8).gap(8).items_center().rounded(ui.size.radiusSm)
      .text_size(ui.size.text).text_color(t.foreground).cursor_pointer()
      .when(selected, (el) => el.bg(t.selected)).hover((s) => s.bg(t.hover));
  },
};
