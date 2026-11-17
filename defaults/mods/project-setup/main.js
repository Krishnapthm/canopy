// Offers to add Canopy's marked block to a project's AGENTS.md or CLAUDE.md, and copies the same block for global files, which Canopy never writes. Props: folder? (default: the active space's). docs/agent-setup.md.
import { View, div } from "gpui-kit";
import { v_flex } from "gpui-base";
import { notify, readTextFile, writeTextFile, spaces, props, closeOverlay, theme, ui } from "herdr";

const BEGIN = "<!-- canopy:begin -->";
const END = "<!-- canopy:end -->";
const BLOCK = `${BEGIN}
## Canopy

This project is open in Canopy, a canvas of terminals, agents and panes (files, notes). When the person mentions panes, the canvas, wiring, quotes or peers, use the \`canopy\` skill.
${END}`;

// Which file gets the block. agents, claude: the files' text, null when missing.
// CLAUDE.md importing AGENTS.md (an `@AGENTS.md` line) means AGENTS.md; else the one that
// exists, AGENTS.md when both do; else a new AGENTS.md.
function target(agents, claude) {
  if (claude != null && /^@(\.\/)?AGENTS\.md[ \t]*$/m.test(claude)) return { file: "AGENTS.md", create: agents == null };
  if (agents != null) return { file: "AGENTS.md", create: false };
  if (claude != null) return { file: "CLAUDE.md", create: false };
  return { file: "AGENTS.md", create: true };
}
// The text with the block: replaced in place when marked, else appended after a blank line.
function withBlock(text) {
  const start = text.indexOf(BEGIN);
  const end = start < 0 ? -1 : text.indexOf(END, start + BEGIN.length);
  if (end >= 0) return text.slice(0, start) + BLOCK + text.slice(end + END.length);
  return text === "" ? BLOCK + "\n" : text.replace(/\n*$/, "\n\n") + BLOCK + "\n";
}
// The text without the block and the blank line before it.
function withoutBlock(text) {
  const start = text.indexOf(BEGIN);
  const end = start < 0 ? -1 : text.indexOf(END, start + BEGIN.length);
  if (end < 0) return text;
  const tail = end + END.length;
  return text.slice(0, start).replace(/\n\n$/, "\n") + text.slice(tail + (text[tail] === "\n" ? 1 : 0));
}

export default class ProjectSetup extends View {
  init(_props, cx) {
    this.handle = cx.focus_handle();
    this.handle.focus();
    const space = spaces().find(s => s.active);
    this.folder = (props()?.folder ?? (space?.machine == null ? space?.folder : null) ?? "").replace(/\/+$/, "");
    if (this.folder) cx.spawn(async cx => {
      try {
        const [agents, claude] = await Promise.all(["AGENTS.md", "CLAUDE.md"].map(f => readTextFile(`${this.folder}/${f}`).catch(() => null)));
        const { file, create } = target(agents, claude);
        const old = (file === "AGENTS.md" ? agents : claude) ?? null;
        this.plan = { path: `${this.folder}/${file}`, file, create, old, present: Boolean(old?.includes(BEGIN) && old.includes(END)) };
      } catch (e) { this.error = String(e?.message ?? e); }
      cx.notify();
    });
  }

  apply(change, cx) {
    const plan = this.plan;
    cx.spawn(async () => {
      try {
        const current = await readTextFile(plan.path);
        // Deleted while the dialog was open: there is no block left to remove.
        if (current == null && plan.present) return closeOverlay(props().overlay);
        await writeTextFile(plan.path, change(current ?? ""), current);
        closeOverlay(props().overlay);
        notify({ title: plan.present ? `Removed Canopy's block from ${plan.file}` : `Added Canopy's block to ${plan.file}`, level: "success" });
      } catch (e) {
        notify({ title: `Could not write ${plan.file}`, body: String(e?.message ?? e), level: "error" });
      }
    });
  }

  render() {
    const t = theme();
    const plan = this.plan;
    const close = () => closeOverlay(props().overlay);
    return ui.panel("project-setup").track_focus(this.handle)
      .on_key_down((e, cx) => { if (e.key === "escape") { close(); cx.stop_propagation(); } })
      .child(ui.title("Teach agents Canopy"))
      .child(ui.description(!this.folder
        ? "This space has no folder. Paste the block into your global AGENTS.md or CLAUDE.md yourself; Canopy never writes those."
        : !plan ? "Checking the project..."
        : plan.present ? `${plan.path} has Canopy's block.`
        : `Add this block to ${plan.path}${plan.create ? " (a new file)" : ""}? Agents that read it learn to use the canopy skill.`))
      .child(v_flex().p(12).rounded(ui.size.radius).bg(t.background).border_1().border_color(t.border).text_size(ui.size.small)
        .children(BLOCK.split("\n").map(line => div().child(line || " "))))
      .child(ui.description("For global AGENTS.md or CLAUDE.md files, copy the block and paste it yourself."))
      .when(Boolean(this.error), el => el.child(ui.description(this.error).when(Boolean(t.error), el => el.text_color(t.error))))
      .child(ui.footer(
        ui.button("ps-copy", this.copied ? "Copied" : "Copy instructions").on_click((_, cx) => { cx.write_to_clipboard(BLOCK); this.copied = true; cx.notify(); }),
        ui.button("ps-no", plan && !plan.present ? "No" : "Close").on_click(close),
        ...(plan ? [plan.present
          ? ui.button("ps-remove", "Remove block", "destructive").on_click((_, cx) => this.apply(withoutBlock, cx))
          : ui.button("ps-yes", `Add to ${plan.file}`, "primary").on_click((_, cx) => this.apply(withBlock, cx))] : []),
      ));
  }
}
