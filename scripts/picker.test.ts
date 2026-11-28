// Modified for Canopy.
import { expect, test } from "bun:test";
import { readFile } from "node:fs/promises";

// Exercise the shipped view and its event handlers without a window or a core.
// Size callbacks use physical pixels, while element dimensions use zoomed units.
class Element {
  nodes: Element[] = [];
  calls: Record<string, any[]> = {};
  constructor() {
    const proxy: Element = new Proxy(this, {
      get: (target, key: string) => key in target ? (target as any)[key] : (...args: any[]) => {
        if (key === "child") target.nodes.push(args[0]);
        else if (key === "children") target.nodes.push(...args[0]);
        else if (key === "when") { if (args[0]) args[1](proxy); }
        else target.calls[key] = args;
        return proxy;
      },
    });
    return proxy;
  }
}
const source = await readFile(new URL("../defaults/mods/picker/main.js", import.meta.url), "utf8");
const element = () => new Element();
const commands: unknown[][] = [];
let testSpaces: any[] = [];
let closed = 0;
const values: Record<string, any> = {
  View: class {}, div: element, svg: element, v_flex: element, h_flex: element,
  Input: { new: element }, InputState: { new: () => ({ handlers: {} as Record<string, any>, on(name: string, handler: any) { this.handlers[name] = handler; }, focus() {} }) },
  ui: { size: { radius: 8, radiusSm: 6, radiusLg: 10, control: 32, gap: 8, pad: 20, text: 13, small: 12, title: 15 } },
  agents: () => [], spaces: () => testSpaces, terminals: () => [], theme: () => ({ hover: "hover" }),
  settings: () => ({ zoom: 1.2 }), props: () => ({}), shared: () => ({}),
  setShared() {}, closeOverlay() { closed++; }, openOverlay() {}, run(...args: unknown[]) { commands.push(args); }, hc() {},
};
const Picker = new Function(...Object.keys(values), source.replace(/^import .*;\n/gm, "").replace("export default class Picker", "return class Picker"))(...Object.values(values));
const cx = { notify() {}, stop_propagation() {}, timer: { after() {} }, focus_handle() {}, spawn() {} };
function picker() {
  testSpaces = [];
  closed = 0;
  const view = new Picker();
  view.init({}, cx);
  return view;
}
function find(root: Element, id: string): Element {
  if (root?.calls?.id?.[0] === id) return root;
  for (const child of root?.nodes ?? []) {
    const found = find(child, id);
    if (found) return found;
  }
  return undefined as any;
}
function measure(view: any, height: number) {
  const list = find(view.render(), "pk-list");
  expect(list).toBeDefined();
  list.calls.on_size_change[0]({ width: 500, height: height * 1.2 }, cx);
}

test("picker reveals keyboard selection in its measured, constrained list", () => {
  for (const height of [40, 48]) {
    const view = picker();
    measure(view, height);
    let tree = view.render();
    tree.calls.on_key_down[0]({ key: "up" }, cx);
    tree = view.render();
    const selected = find(tree, "pk-space");
    expect(selected.calls.top[0]).toBe(height - 32);
    expect(selected.calls.bottom[0]).toBe(0);
    const list = find(tree, "pk-list");
    expect(list.calls.h[0]).toBeLessThanOrEqual(320);
    list.calls.on_scroll_wheel[0]({ delta: { y: -1000 } }, cx);
    expect(find(view.render(), selected.calls.id[0])).toBeDefined();
    tree.calls.on_key_down[0]({ key: "down" }, cx);
    const first = find(view.render(), "pk-new");
    expect(first.calls.top[0]).toBeGreaterThanOrEqual(0);
    expect(first.calls.bottom[0]).toBeGreaterThanOrEqual(0);
  }
});

test("picker opens a local space", () => {
  const view = picker();
  commands.length = 0;
  view.go({ kind: "space" });
  expect(commands).toEqual([["space.new"]]);
});
