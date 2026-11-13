// The quote popover: opened by quote.menu when a preview selection is finished.
// Each row is a running agent (its icon and pill name); clicking one pastes the
// quote and the optional message into it, never pressing Enter.
import { View, div, svg, image } from "gpui-kit";
import { v_flex, h_flex, TextareaState, Textarea } from "gpui-base";
import { run, props, closeOverlay, theme, ui } from "herdr";

export default class Quote extends View {
  init() {
    this.message = TextareaState.new({ placeholder: "Add a message (optional)", rows: 2 });
    this.message.focus();
  }

  send(terminal) {
    const message = this.message.value().trim();
    run("quote.send", { ...props().quote, ...(message ? { message } : {}), terminal });
    closeOverlay(props().overlay);
  }

  render() {
    const t = theme();
    const agents = props().agents ?? [];
    const { path = "", start, end } = props().quote ?? {};
    const where = path.split("/").pop() + (start ? `:${start}${end && end !== start ? `-${end}` : ""}` : "");
    return v_flex().id("quote").w_full().p(5).gap(4).bg(t.popover).text_size(ui.size.text).text_color(t.foreground)
      .on_key_down((e, cx) => {
        if (e.key !== "escape") return;
        closeOverlay(props().overlay);
        cx.stop_propagation();
      })
      // Says what the popover does: the selection goes to an agent's prompt.
      .child(h_flex().px(8).pt(4).pb(2).gap(8).items_center()
        .child(svg("icons/quote.svg").size(14).flex_none().text_color(t.muted))
        .child(div().font_semibold().child("Send quote to an agent")))
      .child(div().px(8).pb(4).text_size(11).text_color(t.dim).truncate()
        .child(agents.length ? `${where} · pasted into the prompt, not sent` : where))
      .child(div().mx(3).h(1).bg(t.border))
      .when(!agents.length, el => el.child(div().px(8).py(7).text_color(t.dim).child("No running agents")))
      .children(agents.map((a, i) => h_flex().id(`quote-${i}`).h(ui.size.control).px(8).gap(9).rounded(ui.size.radiusSm).items_center().cursor_pointer()
        .hover(s => s.bg(t.hover)).on_click(() => this.send(a.terminal))
        .child(a.tint ? svg(a.icon).size(16).flex_none().text_color(t.muted) : image(a.icon).w(16).h(16).flex_none())
        .child(div().flex_1().min_w(0).truncate().child(a.name))))
      .when(agents.length > 0, el => el.child(div().p(3).child(Textarea.new(this.message).w_full().px(8).py(6).rounded(ui.size.radiusSm).border_1().border_color(t.border).bg(t.background).text_size(12))));
  }
}
