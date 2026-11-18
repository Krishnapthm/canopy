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
const local: Record<string, string | null> = { "canvas-close": "terminal.kill" };
const agents = [{ name: "Claude Code", installed: true, run: ["claude"] }, { name: "Gone", installed: false, run: ["gone"] }];

test("every canvas menu item passes only declared args, with the required ones", () => {
  // Targets as the canvas sends them. `machine` is what e5f3afc dropped: canvas.create never took it.
  const empty = { point: [10, 20], space: "s1", machine: "m1" };
  const card = { terminal: "t1", id: "tab1", popped: false };
  const menus = [
    { tab: "canvas-empty", target: empty },
    { tab: "canvas-agents", target: empty },
    { tab: "canvas-empty", target: { ...empty, space: null, machine: null } },
    { tab: "canvas-editors", target: { path: "/tmp/file.rs", editors: [{ key: "vscode", name: "VS Code", icon: "icons/program.svg" }] } },
    { tab: "canvas-space", target: { id: "s1" } },
    { tab: "canvas-colors", target: { id: "s1" } },
    { tab: "canvas-arrange", target: { space: "s1" } },
    { tab: "canvas-wire", target: { a: "t1", b: "t2" } },
    { tab: "canvas-card", target: card },
    { tab: "canvas-card", target: { ...card, popped: true, finished: true } },
  ];
  let checked = 0;
  for (const menu of menus) {
    for (const [label, command, args] of items(menu, null, agents)) {
      if (command === null) continue;
      // A submenu is checked as the menu it opens, with its args as the target.
      if (command.startsWith("sub:")) { menus.push({ tab: command.slice(4), target: args }); continue; }
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
  expect(checking[label(checking) + 1].slice(0, 2)).toEqual(["Checking installed agents", null]);
  const none = items(menu, null, [{ name: "Gone", installed: false, run: ["gone"] }]);
  expect(none.slice(label(none) + 1).map(([text, command]) => [text, command])).toEqual([["No agents installed", null]]);
  const listed = items(menu, null, agents);
  expect(listed.slice(label(listed) + 1).map(([text, command, args]) => [text, command, args])).toEqual([
    ["Claude Code", "canvas.create", { point: [1, 2], space: "s1", argv: ["claude"] }],
  ]);
});


test("a drawn wire can be removed", () => {
  expect(items({ tab: "canvas-wire", target: { a: "t1", b: "t2" } })).toEqual([["Remove", "wire.remove", { a: "t1", b: "t2" }]]);
});

test("arrange picker lists the four layouts and preserves its space target", () => {
  expect(items({ tab: "canvas-arrange", target: { space: "s1" } }).map(([,command,args])=>[command,args])).toEqual(
    ["tidy","row","column","wires"].map(layout=>["canvas.arrange",{space:"s1",layout}])
  );
});

test("confirm forwards the terminal opt-out only when the dialog offers it", async () => {
  const source = (await readFile(new URL("../defaults/mods/confirm/main.js", import.meta.url), "utf8"))
    .replace(/^import .*;\n/gm, "").replace("export default class Confirm", "class Confirm");
  let current: Record<string, unknown> = {};
  const sent: unknown[] = [];
  const Confirm = new Function("View", "props", "closeOverlay", "run", `${source}; return Confirm;`)(class {}, () => current, () => {}, (...args: unknown[]) => sent.push(args));
  const confirm = new Confirm();
  confirm.dontAsk = true;
  current = { dontAsk: true, command: { name: "canvas.close", args: { terminal: "t1", confirmed: "terminal" } } };
  confirm.yes();
  expect(sent.pop()).toEqual(["canvas.close", { terminal: "t1", confirmed: "terminal", dont_ask: true }]);
  current = { command: { name: "canvas.close", args: { terminal: "t1", confirmed: "agent" } } };
  confirm.yes();
  expect(sent.pop()).toEqual(["canvas.close", { terminal: "t1", confirmed: "agent" }]);
});

test("canvas hint bar reads rebound mode keys and distinguishes the layout picker", async () => {
  const source = (await readFile(new URL("../defaults/mods/canvas-keys/main.js", import.meta.url), "utf8"))
    .replace(/^import .*;\n/gm, "").replace("export default class CanvasKeys", "class CanvasKeys");
  let shown: string[] = [];
  let shownProps: Record<string, unknown> = {};
  const element: unknown = new Proxy({}, { get: (_, name) => (...args: unknown[]) => {
    if (name === "child" && typeof args[0] === "string") shown.push(args[0]);
    return element;
  } });
  const Hint = new Function("View", "div", "v_flex", "keys", "props", "theme", `${source}; return CanvasKeys;`)(class {}, () => element, () => element, () => [
    {section:"global", keys:"q", label:"Q", action:"app.quit"},
    {section:"canvas-mode", keys:"z", label:"Z", action:"canvas.new"},
    {section:"canvas-mode", keys:"b", label:"B", action:"{}"},
    {section:"canvas-mode", keys:"shift-z", label:"⇧Z", action:JSON.stringify({command:"canvas.arrange",args:{picker:true}})},
  ], () => shownProps, () => ({}));
  new Hint().render();
  expect(shown).toContain("z new terminal");
  expect(shown).toContain("b invalid binding");
  expect(shown).toContain("shift-z layout picker");
  expect(shown.some(s => s.includes("app.quit"))).toBe(false);
  // A connection shows one line instead of the key list.
  [shown, shownProps] = [[], {pointer:true}];
  new Hint().render();
  expect(shown).toEqual(["Click a pane to connect · Esc cancels"]);
  [shown, shownProps] = [[], {connecting:true}];
  new Hint().render();
  expect(shown).toEqual(["Arrows pick a pane · Enter connects · Esc cancels"]);
});

test("full key help names object bindings without splitting their JSON", async () => {
  const source = (await readFile(new URL("../defaults/mods/help/main.js", import.meta.url), "utf8"))
    .replace(/^import .*;\n/gm, "").replace("export default class Help", "class Help");
  const label = new Function("View", `${source}; return label;`)(class {});
  expect(label('{"command":"canvas.arrange","args":{"picker":true}}')).toBe("choose canvas layout");
  expect(label("canvas.go:left")).toBe("canvas go left");
  expect(label('{}')).toBe("invalid binding");
});


test("pane confirmation lists each pane and cancels without running its captured command", async () => {
  const source = (await readFile(new URL("../defaults/mods/confirm/main.js", import.meta.url), "utf8"))
    .replace(/^import .*;\n/gm, "").replace("export default class Confirm", "class Confirm");
  const labels: string[] = [], icons: string[] = [], text: string[] = [], sent: unknown[] = [];
  let keydown: (event: { key: string }, cx: unknown) => void;
  let cancel: () => void;
  let closed = 0;
  const element: any = new Proxy({}, { get: (_, name) => (...args: any[]) => {
    if (name === "when" && args[0]) args[1](element);
    if (name === "child" && typeof args[0] === "string") text.push(args[0]);
    if (name === "on_key_down") keydown = args[0];
    return element;
  } });
  const ui = { panel: () => element, title: () => element, description: () => element, footer: () => element,
    button: (id: string, label: string) => { labels.push(label); return { on_click: (fn: () => void) => { if (id === "no") cancel = fn; return element; } }; } };
  const current = { overlay: "dialog", title: "Terminate panes?", accept: "Terminate and close all", destructive: true,
    panes: [{ name: "review <script>", icon: "icons/agent.svg", tint: true }, { name: "shell", icon: "icons/terminal.svg", tint: true }],
    command: { name: "canvas.close", args: { terminals: ["t1", "t2"], confirmed: "all" } } };
  const Confirm = new Function("View", "div", "svg", "image", "props", "closeOverlay", "run", "theme", "ui", `${source}; return Confirm;`)(
    class {}, () => element, (path: string) => { icons.push(path); return element; }, () => element, () => current,
    () => closed++, (...args: unknown[]) => sent.push(args), () => ({ muted: "gray" }), ui);
  const confirm = new Confirm();
  confirm.render();
  expect(labels).toEqual(["Cancel", "Terminate and close all"]);
  expect(text).toEqual(["review <script>", "shell"]);
  expect(icons).toEqual(["icons/agent.svg", "icons/terminal.svg"]);
  cancel!();
  keydown!({ key: "escape" }, { stop_propagation() {} });
  expect(closed).toBe(2);
  expect(sent).toEqual([]);
  confirm.yes();
  expect(sent).toEqual([["canvas.close", current.command.args]]);
});


test("canvas-agents numbers installed rows and starts the numbered agent", () => {
  const shown: string[] = [], sent: unknown[] = [];
  let keydown: (e: unknown, cx: unknown) => void;
  const element: any = new Proxy({}, { get: (_, name) => (...args: any[]) => {
    if (name === "when" && args[0]) args[1](element);
    if (name === "child" && typeof args[0] === "string") shown.push(args[0]);
    if (name === "on_key_down") keydown = args[0];
    return element;
  } });
  const current = { tab: "canvas-agents", target: { space: "s1", machine: "m1" } };
  const Menu = new Function("View", "div", "svg", "image", "h_flex", "v_flex", "props", "theme", "ui", "keys", "closeOverlay", "run", `${source}; return Menu;`)(
    class {}, () => element, () => element, () => element, () => element, () => element,
    () => current, () => ({}), { size: {} }, () => [], () => {}, (...args: unknown[]) => sent.push(args));
  const menu = new Menu();
  menu.agents = [
    { name: "First", installed: true, run: ["first"] },
    { name: "Missing", installed: false, run: ["missing"] },
    { name: "Second", installed: true, run: ["second", "--flag"] },
    { name: "Third", installed: true, run: ["third"] },
  ];
  menu.render();
  expect(shown.filter(text => /^[1-9]$/.test(text))).toEqual(["1", "2", "3"]);
  let stopped = 0;
  const cx = { stop_propagation() { stopped++; } };
  keydown!({ key: "2", modifiers: {} }, cx);
  expect(sent).toEqual([["canvas.create", { space: "s1", argv: ["second", "--flag"] }]]);
  keydown!({ key: "7", modifiers: {} }, cx);
  keydown!({ key: "1", modifiers: { shift: true } }, cx);
  expect(sent).toHaveLength(1);
  expect(stopped).toBe(2);
});
