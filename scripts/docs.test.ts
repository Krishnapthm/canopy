import { describe, expect, test } from "bun:test";
import { checkExample, checkMarkdown, commandPage, type Reference } from "./docs";

const ref: Reference = {
  help: "Usage:\n  canopy status\n  canopy ls\n  canopy do <command> [args]\n  canopy commands\n  canopy --skill\n  canopy --version\n",
  commands: [{ name: "pane.split", about: "Split a pane.", args: [{ name: "direction", required: true }] }],
};

describe("CLI examples", () => {
  test("validates commands and named arguments without executing examples", () => {
    for (const source of ["canopy", "canopy ls", "canopy --help", "canopy --skill", "canopy do pane.split --direction right", `canopy do pane.split '{"direction":"right"}'`]) {
      expect(checkExample(source, ref)).toEqual([]);
    }
    expect(checkExample("canopy wat", ref)).toEqual(["unknown command or flag: wat"]);
    expect(checkExample("canopy ls && canopy wat", ref)).toEqual(["unknown command or flag: wat"]);
    expect(checkExample("canopy --wat", ref)).toHaveLength(1);
    expect(checkExample("canopy ls --wat", ref)).toHaveLength(1);
    expect(checkExample("canopy do pane.wat", ref)).toHaveLength(1);
    expect(checkExample("canopy do pane.split --wat right", ref)).toHaveLength(1);
    expect(checkExample("canopy do pane.split --direction=right", ref)).toContain("unknown flag for do: --direction=right");
    expect(checkExample("canopy do pane.split --direction='right'", ref)).toContain("unknown flag for do: --direction=right");
    expect(checkExample('canopy ls "--wat"', ref)).toEqual(["unknown flag for ls: --wat"]);
    expect(checkExample('canopy do pane.split "--wat" right', ref)).toEqual(["unknown flag for do: --wat"]);
    expect(checkExample('canopy do pane.split "--direction" "right"', ref)).toEqual([]);
    expect(checkExample("canopy do pane.split --direction", ref)).toEqual(["missing value for --direction"]);
  });

  test("ignores flags inside quotes and comments, and other programs", () => {
    expect(checkExample(`canopy do pane.split --direction '--not-a-flag' # --comment`, ref)).toEqual([]);
    expect(checkExample("curl --not-our-flag https://example.org", ref)).toEqual([]);
    expect(checkExample("canopy ls | other --not-our-flag", ref)).toEqual([]);
    expect(checkExample("canopy do pane.split --direction canopy", ref)).toEqual([]);
  });

  test("reports inline and fenced examples with file and line", () => {
    expect(checkMarkdown("README.md", "Text `canopy wat`.\n\n```sh\ncanopy ls --wat\n```", ref)).toEqual([
      "README.md:1: unknown command or flag: wat",
      "README.md:4: unknown flag for ls: --wat",
    ]);
    expect(checkMarkdown("docs/example.md", "~~~sh\ncanopy do pane.split \\\n  --wat right\n~~~", ref)).toEqual(["docs/example.md:2: unknown flag for do: --wat"]);
    expect(checkMarkdown("README.md", "Plain prose canopy wat is not code.", ref)).toEqual([]);
  });
});

test("command reference comes from the catalog", () => {
  const page = commandPage(ref.commands);
  expect(page).toContain("## `pane.split`");
  expect(page).toContain("| `direction` | yes |");
  expect(page).toContain("Split a pane.");
  expect(checkMarkdown("docs/commands.md", page, ref)).toEqual([]);
});

