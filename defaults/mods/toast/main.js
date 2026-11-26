// How notifications look. Anything can post one with notify({ title, body,
// level, icon, action }); replace this mod to restyle them all. Click to
// run its action (e.g. go to the agent) and dismiss it. A long message
// expands in place to its full text, with Copy; an expanded toast stays
// until you close it (keepOverlay). Hovering any toast pauses its timer.

import { View, div, image, svg } from "gpui-kit";
import { v_flex, h_flex } from "gpui-base";
import { props, closeOverlay, keepOverlay, run, theme } from "herdr";

const DOT = { error: "blocked", warning: "working", success: "done", info: "accent" };

// shortcut: a mod can't measure whether a line was cut off, so this guesses from length at
// the toast's 340 px width; measure the text once gpui-kit reports truncation.
export const long = (p) => (p.title ?? "").length > 40 || (p.body ?? "").length > 45 || /\n/.test(p.body ?? "");

// What Copy puts on the clipboard: the whole message.
export const fullText = (p) => [p.title, p.body].filter(Boolean).join("\n");

export default class Toast extends View {
  render(cx) {
    const t = theme();
    const p = props() ?? {};
    const login = /^([a-z0-9][a-z0-9_.-]{0,31}) needs login; run hc machine login \1$/.exec(p.body ?? "");
    const command = login ? `hc machine login ${login[1]}` : null;
    const open = Boolean(this.expanded);
    // A small icon button; its click never reaches the toast's own action.
    const button = (id, icon, label, onClick) => div().id(id).flex_none().p(4).rounded(4).cursor_pointer()
      .hover((s) => s.bg(t.raised)).tooltip(label)
      .on_click((_e, cx) => { cx.stop_propagation(); onClick(cx); })
      .child(svg(icon).size(14).text_color(t.muted));
    const line = (el) => open ? el : el.truncate();
    return h_flex()
      .id("toast")
      .w_full()
      .min_h(72)
      .px(18)
      .py(12)
      .gap(12)
      .when(open, (el) => el.items_start())
      .when(!open, (el) => el.items_center())
      .bg(t.popover)
      .cursor_pointer()
      .hover((s) => s.bg(t.hover))
      .on_click(() => {
        if (p.action?.name) run(p.action.name, p.action.args ?? {});
        closeOverlay(p.overlay);
      })
      .child(p.icon ? image(p.icon).w(18).h(18).flex_none() : div().size(8).rounded(4).flex_none().bg(t[DOT[p.level] ?? "accent"]))
      .child(
        v_flex()
          .id("toast-text")
          .flex_1()
          .min_w(0)
          .when(open, (el) => el.max_h(360).overflow_y_scroll().gap(4))
          .when(!open, (el) => el.overflow_hidden())
          .child(line(div().font_semibold().text_size(13).text_color(t.foreground)).child(login ? `${login[1]} needs a login: run` : p.title ?? ""))
          .when(!login && Boolean(p.body), (el) => el.child(line(div().text_size(12).text_color(t.muted)).child(p.body ?? "")))
          .when(Boolean(login), (el) => el.child(h_flex().gap(8).items_center()
            .child(div().flex_1().min_w(0).truncate().tooltip(command).text_size(12).text_color(t.muted).child(command))
            .child(div().id("toast-login-copy").flex_none().px(6).py(3).rounded(4).bg(t.raised)
              .text_size(12).text_color(t.foreground).cursor_pointer().hover(s => s.bg(t.hover))
              .on_click((_e, cx) => { cx.stop_propagation(); cx.write_to_clipboard(command); this.copied = true; cx.notify(); })
              .child(this.copied ? "Copied" : "Copy")))),
      )
      .when(!login && (open || long(p)), (el) => el.child(h_flex().flex_none().gap(2).items_center()
        .when(open, (row) => row.child(div().id("toast-copy").flex_none().px(6).py(3).rounded(4).bg(t.raised)
          .text_size(12).text_color(t.foreground).cursor_pointer().hover(s => s.bg(t.hover))
          .on_click((_e, cx) => { cx.stop_propagation(); cx.write_to_clipboard(fullText(p)); this.copied = true; cx.notify(); })
          .child(this.copied ? "Copied" : "Copy")))
        .child(button("toast-expand", open ? "icons/chevron-up.svg" : "icons/chevron-down.svg", open ? "Show less" : "Show all", (cx) => {
          // Reading it: the toast stays until closed.
          if (!this.expanded) keepOverlay(p.overlay);
          this.expanded = !open;
          cx.notify();
        }))
        .when(open, (row) => row.child(button("toast-close", "icons/x.svg", "Close", () => closeOverlay(p.overlay))))));
  }
}
