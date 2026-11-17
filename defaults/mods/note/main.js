// A note pane: plain text the person types, saved in the pane and readable by wired agents.
import { View, div } from "gpui-kit";
import { Editor, EditorState } from "gpui-base";
import { canvasPanes, props, run, settings, theme } from "herdr";

export default class Note extends View {
  init(_props, cx) {
    this.pane = props()?.pane;
    const text = canvasPanes().find(p => p.id === this.pane)?.props?.text ?? "";
    this.code = EditorState.new({ language: "markdown", value: text, font_family: settings().font_family,
      font_size: 13, line_numbers: false, folding: false, soft_wrap: true });
    // Each edit saves into the pane; the app debounces the canvas.json write.
    this.code.on("change", () => { if (this.pane) run("canvas.pane_set", { pane: this.pane, props: { text: this.code.value() } }); });
  }
  render() {
    const t = theme();
    return div().size_full().min_h(0).p(8).bg(t.canvas).text_color(t.foreground).child(Editor.new(this.code).size_full());
  }
}
