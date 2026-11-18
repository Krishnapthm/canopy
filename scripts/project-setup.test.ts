import { test, expect } from "bun:test";
import { readFile } from "node:fs/promises";

const source = (await readFile(new URL("../defaults/mods/project-setup/main.js", import.meta.url), "utf8"))
  .replace(/^import .*;\n/gm, "").replace("export default class ProjectSetup", "class ProjectSetup");
const globals = { View: class {} };
const { target, withBlock, withoutBlock, BLOCK, BEGIN } = new Function(...Object.keys(globals),
  `${source}; return { target, withBlock, withoutBlock, BLOCK, BEGIN };`)(...Object.values(globals));

test("target follows imports and existing project instruction files", () => {
  for (const [agents, claude, file, create] of [
    ["# P\n", "@AGENTS.md\n", "AGENTS.md", false],
    [null, "@AGENTS.md\n", "AGENTS.md", true],
    ["# P\n", "# Claude\n@./AGENTS.md \t\n", "AGENTS.md", false],
    [null, "@./AGENTS.md\n", "AGENTS.md", true],
    [null, "# Claude\n", "CLAUDE.md", false],
    ["# P\n", null, "AGENTS.md", false],
    ["# P\n", "# Claude\n", "AGENTS.md", false],
    [null, null, "AGENTS.md", true],
    [null, "Read @AGENTS.md for details\n", "CLAUDE.md", false],
  ]) expect(target(agents, claude)).toEqual({ file, create });
});

test("withBlock appends once and replaces a marked block in place", () => {
  expect(withBlock("")).toBe(BLOCK + "\n");
  expect(withBlock("# P\n")).toBe("# P\n\n" + BLOCK + "\n");
  expect(withBlock("# P")).toBe("# P\n\n" + BLOCK + "\n");
  const old = "# P\n\n<!-- canopy:begin -->\nold text\n<!-- canopy:end -->\n\nafter\n";
  expect(withBlock(old)).toBe("# P\n\n" + BLOCK + "\n\nafter\n");
  expect(withBlock(old).split(BEGIN)).toHaveLength(2);
  for (const text of ["", "# P\n", "# P", old]) {
    expect(withBlock(withBlock(text))).toBe(withBlock(text));
  }
});

test("withoutBlock restores the project text and leaves unmarked text alone", () => {
  expect(withoutBlock(withBlock("# P\n"))).toBe("# P\n");
  expect(withoutBlock(withBlock(""))).toBe("");
  expect(withoutBlock("# P\n")).toBe("# P\n");
});
