import { expect, test } from "bun:test";
import { readFile } from "node:fs/promises";

const source = await readFile(new URL("../defaults/frame/settings/main.js", import.meta.url), "utf8");
const { conflicts, shortcutRows, parseShortcuts, matchesShortcut, bindShortcut, resetShortcuts } = new Function("window", source.slice(source.indexOf("function actionName"), source.indexOf("export default class Settings")) + "return { conflicts, shortcutRows, parseShortcuts, matchesShortcut, bindShortcut, resetShortcuts };")(() => ({ platform: "macos" }));

test("shortcuts list unbound commands and distinguish sections and arguments", () => {
  const rows = shortcutRows({ global: { "ctrl-a": "pane.go:left" } }, { terminal: { "ctrl-a": "pane.go:right" } }, [{ name: "pane.go", about: "Move" }, { name: "app.help", about: "Help" }]);
  expect(rows).toHaveLength(3);
  expect(rows.find((r: any) => r.name === "app.help").current).toEqual([]);
  expect(rows.find((r: any) => r.name === "pane.go:left").about).toBe("Move");
});

test("conflicts compare equivalent keys only against other actions in this section", () => {
  expect(conflicts({ "shift-ctrl-a": "app.help", "cmd-a": "terminal.copy" }, "ctrl-shift-a", "app.settings")).toEqual(["app.help"]);
  expect(conflicts({ "ctrl-a": "app.help" }, "ctrl-a", "app.help")).toEqual([]);
});

test("invalid shortcut documents cannot enter the editor", () => {
  for (const raw of ["null", "[]", '{"global":null}', '{"global":{"a":false}}']) expect(() => parseShortcuts(raw)).toThrow();
});

test("search matches names, descriptions and current or default keys", () => {
  const row = { name: "app.help", about: "Open keyboard help", current: ["cmd-k"], defaults: ["cmd-/"] };
  for (const q of ["HELP", "keyboard", "cmd-k", "cmd-/", " "]) expect(matchesShortcut(row, q)).toBe(true);
  expect(matchesShortcut(row, "quit")).toBe(false);
});

test("conflicts handle platform aliases, modifier order, uppercase keys and minus", () => {
  expect(conflicts({ "super-A": "app.help" }, "shift-cmd-a", "app.settings")).toEqual(["app.help"]);
  expect(conflicts({ "ctrl--": "app.help" }, "ctrl--", "app.settings")).toEqual(["app.help"]);
  expect(conflicts({ "secondary-k": "app.help" }, "cmd-k", "app.settings")).toEqual(["app.help"]);
});

test("binding preserves sections and object arguments, and refuses collisions and the reserved key", () => {
  const action = { command: "canvas.new", args: { like: true } };
  const row = { section: "layout:main", action, name: JSON.stringify(action) };
  const current = { "layout:main": { "cmd-t": action, "cmd-n": "canvas.new" }, global: { "cmd-t": "tab.new" } };
  bindShortcut(current, row, ["cmd-k"]);
  expect(current).toEqual({ "layout:main": { "cmd-k": action, "cmd-n": "canvas.new" }, global: { "cmd-t": "tab.new" } });
  expect(() => bindShortcut(structuredClone(current), row, ["cmd-n"])).toThrow("already binds");
  expect(() => bindShortcut(structuredClone(current), row, ["ctrl-alt-shift-escape"])).toThrow("reserved");
});

test("row reset restores all defaults, clears unshipped actions and keeps unrelated bindings", () => {
  const current = { global: { "cmd-k": "app.help", "cmd-f": "editor.find" }, "mod:editor": { "cmd-s": "editor.save" } };
  const defaults = { global: { "cmd-/": "app.help", "cmd-shift-/": "app.help" } };
  resetShortcuts(current, defaults, [], { section: "global", name: "app.help", action: "app.help" });
  expect(current).toEqual({ global: { "cmd-/": "app.help", "cmd-shift-/": "app.help", "cmd-f": "editor.find" }, "mod:editor": { "cmd-s": "editor.save" } });
  resetShortcuts(current, defaults, [], { section: "global", name: "editor.find", action: "editor.find" });
  expect(current.global["cmd-f"]).toBeUndefined();
});

test("reset all restores OS defaults and preserves unrelated actions and sections", async () => {
  for (const file of ["keys.json", "keys.macos.json"]) {
    const defaults = JSON.parse(await readFile(new URL(`../defaults/${file}`, import.meta.url), "utf8"));
    const current: any = { global: { "f12": "app.help", "f11": "editor.find", "f10": "app.unbound" }, "mod:editor": { "cmd-s": "editor.save" } };
    resetShortcuts(current, defaults, [{ name: "app.help" }, { name: "app.unbound" }]);
    expect(current.global.f12).toBeUndefined();
    expect(current.global.f10).toBeUndefined();
    expect(current.global.f11).toBe("editor.find");
    expect(current["mod:editor"]).toEqual({ "cmd-s": "editor.save" });
    for (const [section, bindings] of Object.entries(defaults)) {
      for (const [key, action] of Object.entries(bindings as any)) expect(current[section][key]).toEqual(action);
    }
  }
});

test("reset refuses to replace an unrelated action occupying a default key", () => {
  expect(() => resetShortcuts({ global: { "cmd-/": "editor.find" } }, { global: { "cmd-/": "app.help" } }, [{ name: "app.help" }])).toThrow("already binds editor.find");
});

function editor(raw: string | null, writeError?: string, concurrent?: string) {
  let disk = raw;
  const writes: any[] = [], toasts: any[] = [], tasks: Promise<void>[] = [];
  const store = new Map<string, unknown>();
  const values = {
    View: class {}, window: () => ({ platform: "macos" }), props: () => ({ overlay: "settings-test" }),
    shared: (key: string) => store.get(key), setShared: (key: string, value: unknown) => store.set(key, value),
    appHome: () => "/isolated", keys: () => [], notify: (note: any) => toasts.push(note),
    readTextFile: async () => disk,
    writeTextFile: async (file: string, text: string, expected: string | null) => {
      writes.push({ file, text, expected });
      if (concurrent && writes.length === 1) { disk = concurrent; throw new Error("stale snapshot"); }
      if (writeError) throw new Error(writeError);
      expect(expected).toBe(disk);
      disk = text;
    },
  };
  const Settings = new Function(...Object.keys(values), source.replace(/^import .*;\n/gm, "").replace("export default class Settings", "return class Settings"))(...Object.values(values));
  const view = new Settings();
  Object.assign(view, { shortcutDefaults: { global: { "cmd-/": "app.help" } }, shortcutKeys: {}, shortcutBusy: false });
  const cx = { notify() {}, spawn(fn: any) { tasks.push(fn(cx)); } };
  return { view, cx, writes, toasts, tasks, disk: () => disk, setDisk: (value: string) => { disk = value; } };
}

const helpRow = { section: "global", name: "app.help", action: "app.help" };

test("recording cancels on Esc, clears on Backspace and saves a combination immediately", async () => {
  for (const key of ["escape", "backspace", "cmd-k"]) {
    const e = editor('{"global":{"cmd-/":"app.help","f12":"editor.find"}}');
    e.view.recording = helpRow;
    e.view.captureShortcut(key, e.cx);
    await Promise.all(e.tasks);
    expect(e.view.recording).toBeNull();
    expect(e.writes).toHaveLength(key === "escape" ? 0 : 1);
    if (key !== "escape") {
      expect(e.writes[0].file).toBe("/isolated/keys.json");
      expect(JSON.parse(e.disk()!).global).toEqual(key === "backspace" ? { f12: "editor.find" } : { f12: "editor.find", "cmd-k": "app.help" });
    }
  }
});

test("save preserves fresh external edits and toasts malformed files and write failures", async () => {
  const good = editor('{}');
  good.setDisk('{"global":{"f12":"editor.find"},"mod:editor":{"ctrl-s":"editor.save"}}');
  good.view.saveShortcuts((current: any) => bindShortcut(current, helpRow, ["cmd-k"]), good.cx);
  await Promise.all(good.tasks);
  expect(JSON.parse(good.disk()!)).toEqual({ global: { f12: "editor.find", "cmd-k": "app.help" }, "mod:editor": { "ctrl-s": "editor.save" } });
  for (const [raw, error] of [["{broken", undefined], ["{}", "Disk full"]]) {
    const e = editor(raw!, error);
    e.view.saveShortcuts((current: any) => bindShortcut(current, helpRow, ["cmd-k"]), e.cx);
    await Promise.all(e.tasks);
    expect(e.disk()).toBe(raw!);
    expect(e.toasts).toHaveLength(1);
    expect(e.toasts[0].level).toBe("error");
    expect(e.view.shortcutBusy).toBe(false);
  }
});

test("a concurrent edit is retried from the fresh file without losing unrelated keys", async () => {
  const e = editor('{}', undefined, '{"global":{"f12":"editor.find"}}');
  e.view.saveShortcuts((current: any) => bindShortcut(current, helpRow, ["cmd-k"]), e.cx);
  await Promise.all(e.tasks);
  expect(e.writes).toHaveLength(2);
  expect(JSON.parse(e.disk()!)).toEqual({ global: { f12: "editor.find", "cmd-k": "app.help" } });
  expect(e.toasts).toEqual([]);
});
