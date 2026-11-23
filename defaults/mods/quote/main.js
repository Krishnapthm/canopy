// The quote popover: opened by quote.menu when a selection in a pane is finished.
// Rows: wired agents, other running agents, then installed agents to start with the quote.
// Annotations submit their prompt. Other quotes stay in the prompt for review.
import { View, div, svg, image } from "gpui-kit";
import { v_flex, h_flex, TextareaState, Textarea } from "gpui-base";
import { run, props, closeOverlay, theme, ui, knownAgents, spaces } from "herdr";

export default class Quote extends View {
  init(_props, cx) {
    this.message = TextareaState.new({ placeholder: "Add a message (optional)", rows: 2 });
    this.message.focus();
    this.known = [];
    cx.spawn(async cx => {
      // No "New agent" rows when the check fails: running agents still work.
      try { this.known = (await knownAgents({ on: spaces().find(s => s.active)?.machine })).filter(a => a.installed); }
      catch { this.known = []; }
      cx.notify();
    });
  }

  // target: { terminal } for a running agent, { argv } to start one.
  send(target) {
    const message = this.message.value().trim();
    run("quote.send", { ...props().quote, ...(message ? { message } : {}), ...target });
    closeOverlay(props().overlay);
  }

  label(text) {
    return div().px(8).pt(6).pb(2).text_size(11).text_color(theme().dim).child(text);
  }

  row(id, agent, target) {
    const t = theme();
    const icon = !agent.icon ? svg("icons/square-terminal.svg").size(16).flex_none().text_color(t.muted)
      : agent.tint ? svg(agent.icon).size(16).flex_none().text_color(t.muted)
      : image(agent.icon).w(16).h(16).flex_none();
    return h_flex().id(id).h(ui.size.control).px(8).gap(9).rounded(ui.size.radiusSm).items_center().cursor_pointer()
      .hover(s => s.bg(t.hover)).on_click(() => this.send(target))
      .child(icon)
      .child(div().flex_1().min_w(0).truncate().child(agent.name));
  }

  render() {
    const t = theme();
    // `canopy check` renders without props.
    const agents = props()?.agents ?? [];
    const wired = agents.filter(a => a.wired);
    const running = agents.filter(a => !a.wired);
    const any = agents.length > 0 || this.known.length > 0;
    const { path = "", start, end, kind } = props()?.quote ?? {};
    const annotation = kind === "annotation";
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
        .child(div().font_semibold().child(annotation ? "Send annotation to an agent" : "Send quote to an agent")))
      .child(div().px(8).pb(4).text_size(11).text_color(t.dim).truncate()
        .child(any ? (annotation ? "Submits the screenshot and comments" : `${where} · pasted into the prompt, not sent`) : where))
      .child(div().mx(3).h(1).bg(t.border))
      .when(wired.length > 0, el => el.child(this.label("Wired to this pane"))
        .children(wired.map((a, i) => this.row(`wired-${i}`, a, { terminal: a.terminal }))))
      .when(wired.length > 0 && running.length > 0, el => el.child(this.label("Running agents")))
      .children(running.map((a, i) => this.row(`quote-${i}`, a, { terminal: a.terminal })))
      .when(!agents.length, el => el.child(div().px(8).py(7).text_color(t.dim).child("No running agents")))
      .when(this.known.length > 0, el => el.child(this.label("New agent with this"))
        .children(this.known.map((a, i) => this.row(`new-${i}`, a, { argv: a.run }))))
      .when(any, el => el.child(div().p(3).child(Textarea.new(this.message).w_full().px(8).py(6).rounded(ui.size.radiusSm).border_1().border_color(t.border).bg(t.background).text_size(12))));
  }
}
