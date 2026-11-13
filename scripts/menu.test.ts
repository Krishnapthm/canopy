import { test, expect } from "bun:test";
import { readFile } from "node:fs/promises";

const source = (await readFile(new URL("../defaults/mods/menu/main.js", import.meta.url), "utf8"))
  .replace(/^import .*;\n/gm, "").replace("export default class Menu", "class Menu");
const items = new Function("View", `${source}; return items;`)(class {}) as (p: unknown, git?: unknown, agents?: unknown[]) => [string, string | null, Record<string, unknown>, unknown?][];

// Argument specs straight from the command registry, as the app validates them.
const registry = await readFile(new URL("../src/commands.rs", import.meta.url), "utf8");
const specs = new Map(
  [...registry.matchAll(/cmd\("([^"]+)",\s*&\[([^\]]*)\]/g)].map(([, name, args]) => [
    name,
    [...args.matchAll(/(req|opt)\("([^"]+)"\)/g)].map(([, kind, arg]) => ({ arg, required: kind === "req" })),
  ]),
);
// Menu-only items that open another popup or dialog, and the command each one ends in.
const local: Record<string, string | null> = { "canvas-colors": null, "canvas-close": "terminal.kill" };
const agents = [{ name: "Claude Code", installed: true, run: ["claude"] }, { name: "Gone", installed: false, run: ["gone"] }];

test("every canvas menu item passes only declared args, with the required ones", () => {
  // Targets as the canvas sends them. `machine` is what e5f3afc dropped: canvas.create never took it.
  const empty = { point: [10, 20], space: "s1", machine: "m1" };
  const card = { terminal: "t1", id: "tab1", popped: false };
  const menus = [
    { tab: "canvas-empty", target: empty },
    { tab: "canvas-empty", target: { ...empty, space: null, machine: null } },
    { tab: "canvas-editors", target: { path: "/tmp/file.rs", editors: [{ key: "vscode", name: "VS Code", icon: "icons/program.svg" }] } },
    { tab: "canvas-space", target: { id: "s1" } },
    { tab: "canvas-colors", target: { id: "s1" } },
    { tab: "canvas-sub", target: { terminal: "t1", id: "abc" } },
    { tab: "canvas-card", target: card },
    { tab: "canvas-card", target: { ...card, popped: true } },
  ];
  let checked = 0;
  for (const menu of menus) {
    for (const [label, command, args] of items(menu, null, agents)) {
      if (command === null) continue;
      const name = command in local ? local[command] : command;
      if (name === null) continue;
      const spec = specs.get(name);
      expect(spec, `${menu.tab} ${label}: unknown command ${name}`).toBeDefined();
      for (const key of Object.keys(args)) expect(spec!.some((a) => a.arg === key), `${menu.tab} ${label}: ${name} does not take ${key}`).toBe(true);
      for (const a of spec!.filter((a) => a.required)) expect(args[a.arg], `${menu.tab} ${label}: ${name} needs ${a.arg}`).toBeDefined();
      checked++;
    }
  }
  expect(checked).toBeGreaterThan(15);
});

test("the canvas menu lists installed agents under a label, with a note while checking or when none", () => {
  const menu = { tab: "canvas-empty", target: { point: [1, 2], space: "s1", machine: null } };
  const label = (list: ReturnType<typeof items>) => list.findIndex(([, command, args]) => command === null && args.label);
  const checking = items(menu);
  expect(checking[label(checking) + 1].slice(0, 2)).toEqual(["Checking installed agents...", null]);
  const none = items(menu, null, [{ name: "Gone", installed: false, run: ["gone"] }]);
  expect(none.slice(label(none) + 1).map(([text, command]) => [text, command])).toEqual([["No agents installed", null]]);
  const listed = items(menu, null, agents);
  expect(listed.slice(label(listed) + 1).map(([text, command, args]) => [text, command, args])).toEqual([
    ["Claude Code", "canvas.create", { point: [1, 2], space: "s1", argv: ["claude"] }],
  ]);
});


test("hook children only offer Message", () => {
  expect(items({ tab: "canvas-sub", target: { terminal: "t1", id: "abc" } })).toEqual([
    ["Message", "canvas.sub_message", { terminal: "t1", id: "abc" }],
  ]);
});
