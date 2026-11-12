import { test, expect } from "bun:test";
import { readFile } from "node:fs/promises";

const source = (await readFile(new URL("../defaults/mods/menu/main.js", import.meta.url), "utf8"))
  .replace(/^import .*;\n/gm, "").replace("export default class Menu", "class Menu");
const items = new Function("View", `${source}; return items;`)(class {}) as (p: unknown) => [string, string, Record<string, unknown>][];

// Argument specs straight from the command registry, as the app validates them.
const registry = await readFile(new URL("../src/commands.rs", import.meta.url), "utf8");
const specs = new Map(
  [...registry.matchAll(/cmd\("([^"]+)",\s*&\[([^\]]*)\]/g)].map(([, name, args]) => [
    name,
    [...args.matchAll(/(req|opt)\("([^"]+)"\)/g)].map(([, kind, arg]) => ({ arg, required: kind === "req" })),
  ]),
);
// Menu-only items that open another popup or dialog, and the command each one ends in.
const local: Record<string, string | null> = { "canvas-agents": null, "canvas-colors": null, "canvas-close": "terminal.kill" };

test("every canvas menu item passes only declared args, with the required ones", () => {
  // Targets as the canvas sends them. `machine` is what e5f3afc dropped: canvas.create never took it.
  const empty = { point: [10, 20], space: "s1", machine: "m1" };
  const card = { terminal: "t1", id: "tab1", popped: false };
  const menus = [
    { tab: "canvas-empty", target: empty },
    { tab: "canvas-empty", target: { ...empty, space: null, machine: null } },
    { tab: "canvas-agents", target: empty, agents: [{ name: "Claude Code", installed: true, run: ["claude"] }] },
    { tab: "canvas-space", target: { id: "s1" } },
    { tab: "canvas-colors", target: { id: "s1" } },
    { tab: "canvas-card", target: card },
    { tab: "canvas-card", target: { ...card, popped: true } },
  ];
  let checked = 0;
  for (const menu of menus) {
    for (const [label, command, args] of items(menu)) {
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
