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
