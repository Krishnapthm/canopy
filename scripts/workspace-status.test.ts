import { test, expect } from "bun:test";
import { readFile } from "node:fs/promises";

const source = (await readFile(new URL("../defaults/mods/workspace-status/main.js", import.meta.url), "utf8"))
  .replace(/^import .*;\n/gm, "").replace("export default class", "class");
const chips = new Function("View", `${source}; return chips;`)(class {});

const agent = (address: string, status: string, seq: number) => ({ address, status, seq });

test("chips put blocked first, then done, working, idle, newest first within a status", () => {
  const { shown, more } = chips([
    agent("idle-new", "idle", 9), agent("work", "working", 5), agent("done", "done", 1),
    agent("block-old", "blocked", 2), agent("block-new", "blocked", 7), agent("idle-old", "idle", 3),
  ]);
  expect(shown.map((a: any) => a.address)).toEqual(["block-new", "block-old", "done", "work", "idle-new", "idle-old"]);
  expect(more).toBe(0);
});

test("chips cap at six and count the rest", () => {
  const list = Array.from({ length: 8 }, (_, i) => agent(`a${i}`, "idle", i));
  const { shown, more } = chips(list);
  expect(shown).toHaveLength(6);
  expect(more).toBe(2);
});

test("agent dots use effective pane colors, falling back to status", () => {
  const colors: unknown[] = [];
  const element = new Proxy({}, { get: (_, name) => (...args: any[]) => {
    if (name === "when" && args[0]) args[1](element);
    if (name === "bg" || name === "text_color") colors.push([name, args[0]]);
    return element;
  } });
  const node = () => element;
  const Status = new Function("View", "div", "svg", "h_flex", "theme", "spaces", "agents", "panes", "ui", "window", "keys", `${source}; return WorkspaceStatus;`)(
    class {}, node, node, node,
    () => ({ "tint-blue": "#abc", "tint-teal": "#def", blocked: "#123" }),
    () => [{ active: true, branch: "main", color: "blue" }],
    () => [
      { ...agent("t1", "blocked", 4), pane: "p1" },
      { ...agent("t2", "blocked", 3), pane: "p2" },
      { ...agent("t3", "blocked", 2), pane: "p3" },
      { ...agent("t4", "blocked", 1), pane: "p4" },
    ],
    () => [
      { pane: "p1", color: "teal" }, { pane: "p2", color: "blue" },
      { pane: "p3", color: "#aabbcc" }, { pane: "p4", color: null },
    ],
    { size: { radiusSm: 4 } }, () => ({}), () => [],
  );
  new Status().render();
  expect(colors.filter(color => JSON.stringify(color) === JSON.stringify(["bg", "#abc"]))).toHaveLength(2);
  expect(colors).toContainEqual(["bg", "#def"]);
  expect(colors).toContainEqual(["bg", "#aabbcc"]);
  expect(colors).toContainEqual(["bg", "#123"]);
  expect(colors).toContainEqual(["text_color", "#def"]);
});
