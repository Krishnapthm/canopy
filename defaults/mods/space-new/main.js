// Modified for Canopy.
// Space dialog, props { args, folders }: first "Choose space" lists the folders of
// earlier spaces (a click makes a space there); "New space" shows the full form.
import { View, div, svg } from "gpui-kit";
import { v_flex, h_flex, InputState, Input } from "gpui-base";
import { run, props, closeOverlay, theme, pickFolder, spaces } from "herdr";
const basename = path => path.split(/[\\/]/).filter(Boolean).pop() ?? "";
export default class SpaceNew extends View {
  init(_props, cx) {
    this.cx = cx;
    this.folder = "";
    this.open = false;
    // With nothing to choose from, skip straight to the form.
    this.choosing = (props().folders ?? []).length > 0;
    this.name = InputState.new({ placeholder: "My app" });
    this.search = InputState.new({ placeholder: "Search folders" });
    this.find = InputState.new({ placeholder: "Search spaces" });
    for (const input of [this.name, this.search, this.find]) input.on("change", () => cx.notify());
    this.name.on("submit", () => this.create());
    this.find.on("submit", () => { const [first] = this.matches(this.find); if (first) this.make("", first); });
    (this.choosing ? this.find : this.name).focus();
    this.handle = cx.focus_handle();
  }
  matches(input) {
    const q = input.value().toLowerCase();
    return (props().folders ?? []).filter(f => f.toLowerCase().includes(q));
  }
  make(name, folder) {
    run("canvas.create", { ...props().args, kind: "space", name, folder });
    closeOverlay(props().overlay);
  }
  create() {
    const name = this.name.value().trim() || basename(this.folder);
    if (name) this.make(name, this.folder);
  }
  render() {
    const t = theme();
    const icon = name => svg(`mods/workspace-chrome/${name}.svg`).size(14);
    const close = () => closeOverlay(props().overlay);
    const button = (id, text, action, primary = false, enabled = true) => h_flex().id(id).h(32).px(12).gap(5).items_center().rounded(8)
      .bg(primary ? t.solid : t.raised).text_color(primary ? t.solid_foreground : t.foreground)
      .opacity(enabled ? 1 : 0.4).cursor_pointer().on_click(() => { if (enabled) action(); }).child(text);
    const row = (id, label, action) => h_flex().id(id).h(32).px(10).gap(8).items_center().rounded(6).cursor_pointer()
      .hover(s => s.bg(t.raised)).on_click(action).child(label);
    const shell = v_flex().id("space-new").track_focus(this.handle).on_key_down((e, cx) => {
      if (e.key === "escape") { close(); cx.stop_propagation(); }
    }).w_full().bg(t.popover).text_color(t.foreground).text_size(12);
    if (this.choosing) {
      const list = this.matches(this.find);
      return shell.p(6).gap(2)
        .child(Input.new(this.find).h(36).px(10).text_size(13))
        .child(div().h(1).mx(4).bg(t.input))
        .child(div().px(10).pt(6).pb(2).text_size(11).text_color(t.dim).child("Recent"))
        .child(v_flex().id("space-recent").max_h(280).overflow_y_scroll().gap(2)
          .children(list.map((f, i) => row(`space-recent-${i}`,
            h_flex().flex_1().min_w(0).gap(8).items_center()
              .child(div().flex_none().child(basename(f) || f))
              .child(div().flex_1().min_w(0).truncate().text_size(11).text_color(t.dim).child(f)),
            () => this.make("", f))))
          .when(list.length === 0, el => el.child(div().px(10).py(8).text_color(t.dim).child("No matching spaces"))))
        .child(div().h(1).mx(4).my(2).bg(t.input))
        .child(row("space-new-form", h_flex().gap(8).items_center().child(icon("plus")).child("New space"), () => {
          this.choosing = false;
          this.name.focus();
          this.cx.notify();
        }));
    }
    const choose = folder => { this.folder = folder; this.open = false; this.cx.notify(); };
    // Names of the spaces already on a folder; two spaces may share one.
    const using = folder => folder && spaces().filter(s => s.folder === folder).map(s => s.name).join(", ");
    return shell.p(20).gap(16)
      .child(h_flex().items_center().child(div().flex_1().text_size(16).font_semibold().child("New space"))
        .child(button("space-close", icon("x"), close)))
      .child(v_flex().gap(8).child("Name").child(Input.new(this.name).h(36).px(10).rounded(8).bg(t.background)))
      .child(h_flex().items_center().gap(5).child("Folder")
        .child(div().id("folder-help").child(svg("mods/space-new/circle-help.svg").size(13).text_color(t.dim))
          .tooltip("Optional. With no folder, the space opens at the root (your home folder)."))
        .child(div().flex_1())
        .child(button("space-add", h_flex().gap(4).items_center().child(icon("plus")).child("Add"), () => {
          this.open = !this.open;
          if (this.open) this.search.focus();
          this.cx.notify();
        }).border_1().border_color(t.input)))
      // Inline under the Folder row, so the dialog grows instead of clipping a floating list.
      .when(this.open, el => el.child(v_flex().id("folder-dropdown").max_h(260).overflow_y_scroll().p(8).gap(6).rounded(8).border_1().border_color(t.input).bg(t.background)
        .child(Input.new(this.search).h(32).px(8))
        .children(this.matches(this.search).map((f, i) =>
          v_flex().id(`recent-${i}`).p(6).rounded(4).cursor_pointer().hover(s => s.bg(t.raised)).on_click(() => choose(f))
            .child(h_flex().gap(6).child(basename(f)).when(Boolean(using(f)), el => el.child(div().text_color(t.dim).child(`· space ${using(f)}`))))
            .child(div().text_size(10).text_color(t.dim).child(f))))
        .child(div().h(1).bg(t.input))
        .child(button("folder-choose", h_flex().gap(6).child(icon("folder")).child("Choose..."), async () => {
          const folder = await pickFolder({ prompt: "Select folder" });
          if (folder) choose(folder);
        }))))
      .child(v_flex().p(14).gap(4).rounded(8).border_1().border_color(t.input)
        .child(this.folder ? basename(this.folder) : "No folder: the space opens at the root.")
        .when(Boolean(this.folder), el => el.child(div().text_color(t.dim).text_size(10).child(this.folder)))
        .when(Boolean(using(this.folder)), el => el.child(div().text_color(t.dim).text_size(10).child(`Space ${using(this.folder)} already uses this folder. Another space here is fine.`))))
      .child(h_flex().gap(8).justify_end().child(button("space-cancel", "Cancel", close))
        .child(button("space-create", "Create space", () => this.create(), true, Boolean(this.name.value().trim() || basename(this.folder)))));
  }
}
