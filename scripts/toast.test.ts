import { test, expect } from "bun:test";
import { readFile } from "node:fs/promises";

const source = (await readFile(new URL("../defaults/mods/toast/main.js", import.meta.url), "utf8"))
  .replace(/^import .*;\n/gm, "").replace("export default class Toast", "class Toast").replace(/^export /gm, "");

// Renders the toast with `p` as its props; click handlers are keyed by the id set before them.
function harness(p: Record<string, unknown>) {
  const clicks: Record<string, (e: unknown, cx: unknown) => void> = {};
  const kept: string[] = [], closed: string[] = [], copied: string[] = [];
  let id = "";
  const element: any = new Proxy({}, { get: (_, name) => (...args: any[]) => {
    if (name === "id") id = args[0];
    if (name === "on_click") clicks[id] = args[0];
    if (name === "when" && args[0]) args[1](element);
    return element;
  } });
  const make = () => element;
  const Toast = new Function("View", "div", "image", "svg", "v_flex", "h_flex", "props", "closeOverlay", "keepOverlay", "run", "theme", `${source}; return Toast;`)(
    class {}, make, make, make, make, make, () => p, (o: string) => closed.push(o), (o: string) => kept.push(o), () => {}, () => ({}));
  const toast = new Toast();
  const render = () => { for (const k in clicks) delete clicks[k]; toast.render(); };
  const cx = { stop_propagation() {}, notify() {}, write_to_clipboard: (t: string) => copied.push(t) };
  render();
  return { toast, clicks, kept, closed, copied, render, cx };
}

test("a short toast has nothing to expand", () => {
  const { clicks } = harness({ overlay: "o1", title: "Saved" });
  expect(clicks["toast-expand"]).toBeUndefined();
});

test("a long toast expands to its full text, stays, and copies all of it", () => {
  const body = "Live simulator: The simulator helper stopped (signal 9). Showing screenshots.";
  const h = harness({ overlay: "o1", title: "Simulator", body, level: "error" });
  h.clicks["toast-expand"](null, h.cx);
  expect(h.kept).toEqual(["o1"]);
  h.render();
  h.clicks["toast-copy"](null, h.cx);
  expect(h.copied).toEqual([`Simulator\n${body}`]);
  h.clicks["toast-close"](null, h.cx);
  expect(h.closed).toEqual(["o1"]);
  // Collapsing again does not ask to keep it twice.
  h.clicks["toast-expand"](null, h.cx);
  expect(h.kept).toEqual(["o1"]);
});
