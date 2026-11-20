import { expect, test } from "bun:test";
import { lintStyle } from "./doc-style";

const page = (body: string) => `# Page\n\nA short description.\n\n${body}\n`;

test("a clean page passes", () => {
  expect(lintStyle("docs/wiki/P.md", page("1. Click **Connect**.\n2. Run `just build; canopy ls`.\n\n```sh\ncanopy wire # just; fine in code\n```\n\nSee [Panes](Panes) and [tile view](Canvas#tile-view).\n\n![The toolbar](images/toolbar-1.png)\n\n| a | b; c |\n|---|---|"))).toEqual([]);
});

test("flags style and portability problems with lines", () => {
  const long = Array(26).fill("word").join(" ") + ".";
  const errors = lintStyle("docs/wiki/P.md", page(`${long}\n\nOne. Two. Three. Four.\n\n1. ${Array(21).fill("go").join(" ")}.\n\nIt is a — dash; simply [[Panes]] <b>x</b> [x](panes.md) ![](images/a.png)\n\n> [!NOTE]`));
  const has = (s: string) => expect(errors.some(e => e.includes(s))).toBe(true);
  has("sentence has 26 words (max 25)");
  has("paragraph has 4 sentences");
  has("sentence has 21 words (max 20 in a step)");
  for (const s of ["em dash", "semicolon", 'banned word "simply"', "wiki [[link]]", "raw HTML", 'link target "panes.md"', "image without alt text", "GitHub alert"]) has(s);
  expect(errors[0]).toMatch(/^docs\/wiki\/P\.md:5: /);
});

test("needs a title except in sidebar and footer", () => {
  expect(lintStyle("docs/wiki/P.md", "Text.\n")).toEqual(["docs/wiki/P.md:1: line 1 must be the page title (# Title)"]);
  expect(lintStyle("docs/wiki/_Sidebar.md", "- [Home](Home)\n")).toEqual([]);
});
