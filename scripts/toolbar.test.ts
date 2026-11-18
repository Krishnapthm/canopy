import { test, expect } from "bun:test";
import { readFile } from "node:fs/promises";

const source = (await readFile(new URL("../defaults/mods/toolbar/main.js", import.meta.url), "utf8"))
  .replace(/^import .*;\n/gm, "").replace("export default class Toolbar", "class Toolbar");

test("toolbar keys use host labels and match serialized object bindings", () => {
  const bindings = [
    { section: "global", keys: "cmd-t", action: "canvas.new", label: "⌘T" },
    { section: "layout:main", keys: "cmd-n", action: "canvas.new", label: "⌘N" },
    { section: "canvas-mode", keys: "n", action: "canvas.new", label: "N" },
    { section: "canvas-mode", keys: "v", action: '{"args":{"on":false},"command":"canvas.hand"}', label: "V" },
    { section: "canvas-mode", keys: "t", action: '{"args":{"kind":"mod:note"},"command":"canvas.pane_new"}', label: "T" },
  ];
  const key = new Function("View", "keys", `${source}; return key;`)(class {}, () => bindings);
  expect(key("canvas.new")).toBe("N");
  expect(key("canvas.new", {})).toBe("N");
  expect(key("canvas.hand", { on: false })).toBe("V");
  expect(key("canvas.pane_new", { kind: "mod:note" })).toBe("T");
  expect(key("canvas.pane_new", { kind: "mod:other" })).toBe("");
  expect(key("missing")).toBe("");
});
