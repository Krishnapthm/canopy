// Modified for Canopy.
import { test, expect } from "bun:test";
import { readFile, mkdtemp, mkdir, writeFile, rm, symlink } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

const source = await readFile(new URL("../defaults/mods/files/main.js", import.meta.url), "utf8");
const directorySource = source.slice(source.indexOf("async function directory("), source.indexOf("\nexport default class"));
const loadDirectory = (exec: Function, platform: string) => new Function("exec", "window", `${directorySource}; return directory;`)(exec, () => ({ platform }));
const directory = loadDirectory(
  async (command: string, args: string[]) => {
    // Hidden like the app's spawns: a visible console would give PowerShell UTF-8 and hide code page bugs.
    const p = Bun.spawn([command, ...args], { stdout: "pipe", stderr: "pipe", windowsHide: true });
    const [code, stdout, stderr] = await Promise.all([p.exited, new Response(p.stdout).text(), new Response(p.stderr).text()]);
    return { code, stdout, stderr };
  },
  process.platform === "win32" ? "windows" : process.platform,
);

test("Files retries a remote Windows directory with PowerShell on that machine", async () => {
  const calls: { command: string; args: string[]; on: string }[] = [];
  const list = loadDirectory(async (command: string, args: string[], options: { on: string }) => {
    calls.push({ command, args, on: options.on });
    if (command === "sh") throw new Error("herdr.exec: cannot start sh: program not found");
    return { code: 0, stdout: '{"name":"中文.txt","dir":false}', stderr: "" };
  }, "linux");
  const path = "C:/O’Brien's $folder";
  expect(await list(path, "winvm")).toEqual([{ name: "中文.txt", dir: false }]);
  expect(calls.map(c => [c.command, c.on])).toEqual([["sh", "winvm"], ["powershell", "winvm"]]);
  expect(calls[0].args.at(-1)).toBe(path);
  expect(calls[1].args.at(-1)).toContain("'C:/O’’Brien''s $folder'");
  expect(calls[1].args.at(-1)).toContain("[Console]::OutputEncoding");
});

test("Files does not mask remote command failures with PowerShell", async () => {
  for (const failure of [new Error("connection lost"), new Error("cannot start sh: permission denied"), { code: 1, stdout: "", stderr: "folder denied" }]) {
    const calls: string[] = [];
    const list = loadDirectory(async (command: string) => {
      calls.push(command);
      if (failure instanceof Error) throw failure;
      return failure;
    }, "linux");
    await expect(list("/folder", "linuxvm")).rejects.toThrow(failure instanceof Error ? failure.message : failure.stderr);
    expect(calls).toEqual(["sh"]);
  }
});

test("Files keeps remote POSIX directory listings on sh", async () => {
  const calls: string[] = [];
  const list = loadDirectory(async (command: string) => {
    calls.push(command);
    return { code: 0, stdout: "d\0folder\0f\0file.txt\0", stderr: "" };
  }, "windows");
  expect(await list("/folder", "linuxvm")).toEqual([{ name: "folder", dir: true }, { name: "file.txt", dir: false }]);
  expect(calls).toEqual(["sh"]);
});

test("Files lists literal folder names, not shell expressions", async () => {
  const root = await mkdtemp(join(process.platform === "linux" ? "/var/tmp" : tmpdir(), "herdr-files-"));
  try {
    for (const name of ["Test Folder (ACME)", "O'Brien $folder; (ACME)", "O’Brien ‘quoted’ ‚low‛"]) {
      const folder = join(root, name);
      await mkdir(join(folder, "child"), { recursive: true });
      await writeFile(join(folder, "hello.txt"), "hello");
      const entries = await directory(folder, null);
      expect(entries.sort((a: { name: string }, b: { name: string }) => a.name.localeCompare(b.name))).toEqual([
        { name: "child", dir: true }, { name: "hello.txt", dir: false },
      ]);
    }
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("Files preserves Unicode names when listing and expanding folders", async () => {
  const root = await mkdtemp(join(tmpdir(), "herdr-files-unicode-"));
  try {
    const folder = "中文目录 O'Brien (测试)", file = "项目说明.txt", child = "子文件.txt";
    await mkdir(join(root, folder));
    await writeFile(join(root, file), "hello");
    await writeFile(join(root, folder, child), "hello");
    const entries = await directory(root, null);
    expect(entries).toHaveLength(2);
    expect(entries).toContainEqual({ name: folder, dir: true });
    expect(entries).toContainEqual({ name: file, dir: false });
    expect(await directory(join(root, folder), null)).toEqual([{ name: child, dir: false }]);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("Files follows directory links one level at a time", async () => {
  const root = await mkdtemp(join(tmpdir(), "herdr-files-links-"));
  try {
    const target = join(root, "versions");
    await mkdir(target);
    for (let minor = 1; minor <= 9; minor++) {
      await mkdir(join(target, `0.${minor}.0`));
    }
    await writeFile(join(target, "0.9.0", "STORY.md"), "story preview");
    await mkdir(join(root, ".hidden"));
    await writeFile(join(root, "ordinary.txt"), "ordinary");
    await symlink("versions", join(root, ".kb"), "dir");
    if (process.platform === "win32") await symlink(target, join(root, "junction"), "junction");
    await symlink(join(root, "ordinary.txt"), join(root, "file-link"), "file");
    await symlink(join(root, "missing"), join(root, "dangling"), "dir");
    // A cycle must be listed without recursively walking it.
    await symlink(target, join(target, "loop"), "dir");

    const entries = await directory(root, null);
    expect(entries).toHaveLength(process.platform === "win32" ? 7 : 6);
    if (process.platform === "win32") expect(entries).toContainEqual({ name: "junction", dir: true });
    expect(entries).toContainEqual({ name: ".kb", dir: true });
    expect(entries).toContainEqual({ name: "versions", dir: true });
    expect(entries).toContainEqual({ name: ".hidden", dir: true });
    expect(entries).toContainEqual({ name: "ordinary.txt", dir: false });
    expect(entries).toContainEqual({ name: "file-link", dir: false });
    expect(entries).toContainEqual({ name: "dangling", dir: false });
    const versions = await directory(join(root, ".kb"), null);
    expect(versions).toHaveLength(10);
    for (let minor = 1; minor <= 9; minor++) {
      expect(versions).toContainEqual({ name: `0.${minor}.0`, dir: true });
    }
    expect(versions).toContainEqual({ name: "loop", dir: true });
    expect(await directory(join(root, ".kb", "0.9.0"), null)).toEqual([{ name: "STORY.md", dir: false }]);
    expect(await readFile(join(root, ".kb", "0.9.0", "STORY.md"), "utf8")).toBe("story preview");
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});


test("Files filter preserves folders for lazy expansion and matches names without case", () => {
  const body = source.slice(source.indexOf("function matchesFile("), source.indexOf("async function directory("));
  const matches = new Function(body + "; return matchesFile;")();
  expect(matches({ dir: true, name: "src" }, "main")).toBe(true);
  expect(matches({ dir: false, name: "Main.rs" }, "MAIN")).toBe(true);
  expect(matches({ dir: false, name: "other.rs" }, "main")).toBe(false);
  expect(matches({ dir: false, name: "other.rs" }, "")).toBe(true);
});

test("Files saves the snapshot and preserves conflicts and newer edits", async () => {
  let disk = "original", draft = "draft", pending: Promise<void> | undefined;
  const calls: unknown[] = [], toasts: any[] = [];
  const Files = new Function("View", "writeTextFile", "run", "props", "fileMarks", "notify", source.slice(source.indexOf("export default class")).replace("export default ", "") + "; return Files;")(
    class {},
    async (path: string, text: string, expected: string) => {
      expect(path).toBe("/fixture/hello.md");
      if (disk !== expected) throw new Error("File changed outside Canopy");
      disk = text;
    },
    (...args: unknown[]) => calls.push(args), () => ({ editing: true }), async () => [], (toast: unknown) => toasts.push(toast),
  );
  const files = new Files();
  files.root = "/fixture"; files.file = "hello.md"; files.original = disk;
  files.code = { value: () => draft, set_line_marks() {} };
  const cx = { notify() {}, spawn(f: Function) { pending = f(cx); } };
  files.save(false, cx); await pending;
  expect(disk).toBe("draft"); expect(files.original).toBe("draft"); expect(calls).toEqual([]);
  disk = "external"; draft = "my changes";
  files.save(true, cx); await pending;
  expect(files.saveError).toContain("changed outside");
  expect(files.code.value()).toBe("my changes"); expect(files.original).toBe("draft");
  expect(disk).toBe("external"); expect(calls).toEqual([]);
  expect(toasts).toHaveLength(1);
  expect(toasts[0]).toMatchObject({ title: "Could not save hello.md", level: "error" });
  disk = "draft";
  files.save(true, cx); draft = "newer changes"; await pending;
  expect(files.original).toBe("my changes"); expect(calls).toEqual([]);
  files.save(true, cx); await pending;
  expect(disk).toBe("newer changes");
  expect(calls).toEqual([]);
});

test("Files edits full markdown source instead of the bounded read-only preview", async () => {
  const text = "# Source\n" + "a".repeat(400001);
  const options: any[] = [];
  let editing = true, pending: Promise<void> | undefined;
  const Files = new Function("View", "readTextFile", "theme", "settings", "EditorState", "props",
    source.replace(/^import .*;\n/gm, "").replace(/^export (?:default )?/gm, "") + "; return Files;")(
    class {}, async () => text, () => ({}), () => ({}),
    { new(o: any) { options.push(o); return { on() {}, release() {} }; } }, () => ({ editing }),
  );
  const files = new Files();
  files.root = "/fixture"; files.current = () => true;
  const cx = { notify() {}, spawn(f: Function) { pending = f(cx); } };
  files.show("notes.md", cx); await pending;
  expect(options[0].language).toBe("markdown");
  expect(options[0].readonly).toBe(false); expect(options[0].value).toBe(text);
  editing = false;
  files.show("notes.md", cx); await pending;
  expect(options[1].readonly).toBe(true); expect(options[1].value).toContain("file truncated");
});

test("Files reports only dirty transitions to the native tab, including edits during save", async () => {
  let draft = "saved", pending: Promise<void> | undefined;
  const calls: unknown[] = [];
  const Files = new Function("View", "props", "run", "writeTextFile", "fileMarks",
    source.slice(source.indexOf("export default class")).replace("export default ", "") + "; return Files;")(
    class {}, () => ({ canvas: "viewer", editing: true }),
    (...args: unknown[]) => calls.push(args), async () => {}, async () => [],
  );
  const files = new Files();
  Object.assign(files, { root: "/fixture", file: "hello.md", original: "saved", code: { value: () => draft, set_line_marks() {} } });
  const cx = { notify() {}, spawn(f: Function) { pending = f(cx); } };
  files.reportDirty();
  draft = "edit"; files.reportDirty();
  draft = "more edits"; files.reportDirty();
  files.save(false, cx); draft = "newer edit"; await pending;
  files.reportDirty();
  expect(calls).toEqual([
    ["panel.editor", { path: "/fixture/hello.md", dirty: false }],
    ["panel.editor", { path: "/fixture/hello.md", dirty: true }],
  ]);
  files.save(false, cx); await pending;
  expect(calls.at(-1)).toEqual(["panel.editor", { path: "/fixture/hello.md", dirty: false }]);
});

const markSource = source.slice(source.indexOf("export function lineMarks("), source.indexOf("\nasync function fileMarks("));
const lineMarks = new Function(`${markSource.replace("export ", "")}; return lineMarks;`)();

test("Files turns zero-context Git hunks into source gutter marks", () => {
  expect(lineMarks("@@ -1,0 +2,2 @@\n+a\n+b")).toEqual([
    { line: 2, kind: "added" }, { line: 3, kind: "added" },
  ]);
  expect(lineMarks("@@ -2 +2 @@\n-old\n+new")).toEqual([{ line: 2, kind: "modified" }]);
  expect(lineMarks("@@ -1 +0,0 @@")).toEqual([{ line: 1, kind: "removed" }]);
  expect(lineMarks("@@ -4,2 +3,0 @@")).toEqual([{ line: 4, kind: "removed" }]);
  expect(lineMarks("@@ -1 +1 @@ title\n@@ -3,0 +4 @@\n@@ -8 +8,0 @@")).toEqual([
    { line: 1, kind: "modified" }, { line: 4, kind: "added" }, { line: 9, kind: "removed" },
  ]);
  expect(lineMarks("")).toEqual([]);
});

test("Files replaces the pending auto-save timer and cancels it when edits are reverted", async () => {
  const viewerSource = source.slice(source.indexOf("  show(path, cx)"), source.indexOf("\n  // Only transitions"));
  const save = () => { saves++; };
  let saves = 0, value = "original", change: Function;
  const code = { value: () => value, on: (event: string, handler: Function) => { if (event === "change") change = handler; }, set_line_marks() {} };
  const args = {
    readTextFile: async () => value,
    theme: () => ({}), settings: () => ({}), props: () => ({ canvas: "viewer", editing: true }),
    LANG: {}, EditorState: { new: () => code }, autoSave: () => true, fileMarks: async () => [],
  };
  const viewer = new Function(...Object.keys(args), `return ({ ${viewerSource} });`)(...Object.values(args));
  Object.assign(viewer, { current: () => true, reportDirty() {}, save });
  let loaded: Promise<void>;
  const pending = new Set<Function>();
  const cx = {
    notify() {}, spawn(fn: Function) { loaded = fn(cx); },
    timer: {
      after(ms: number, cb: Function) {
        expect(ms).toBe(1000);
        pending.add(cb);
        return { cancel: () => pending.delete(cb) };
      },
    },
  };
  viewer.show("test.txt", cx);
  await loaded!;
  value = "first";
  change!(null, cx);
  const first = [...pending][0];
  value = "second";
  change!(null, cx);
  expect(pending.size).toBe(1);
  expect(pending.has(first)).toBe(false);
  value = "original";
  change!(null, cx);
  expect(pending.size).toBe(0);
  value = "third";
  change!(null, cx);
  const last = [...pending][0];
  pending.delete(last);
  last(cx);
  expect(saves).toBe(1);
});

const { parseFilter, menuItems } = new Function(
  source.slice(source.indexOf("/** @typedef"), source.indexOf("function matchesFile")).replace(/^export /gm, "") +
    "; return { parseFilter, menuItems };",
)();

test("Files recognizes a content filter only at the start", () => {
  expect(parseFilter("main?")).toEqual({ kind: "name", query: "main?" });
  expect(parseFilter("?  a+b  ")).toEqual({ kind: "content", query: "a+b" });
  expect(parseFilter("? ")).toEqual({ kind: "content", query: "" });
});

test("Files scopes menu actions to the row, clipboard, and machine", () => {
  const file = { kind: "file", path: "/project/report.pdf", canvas: true };
  const dir = { kind: "dir", path: "/project/docs" };
  const ids = (items: any[]) => items.filter(Boolean).map(item => item.id);
  expect(ids(menuItems(dir, null, "linux"))).toContain("new-folder");
  expect(ids(menuItems(dir, null, "linux"))).toContain("new-file");
  expect(ids(menuItems(dir, null, "linux"))).not.toContain("paste");
  const clip = { op: "copy", paths: [file.path] };
  const actions = ids(menuItems(file, clip, "macos"));
  expect(actions).toContain("paste");
  expect(actions).toContain("preview");
  expect(actions).toContain("browser");
  expect(actions).not.toContain("new-file");
  expect(actions).not.toContain("new-folder");
  expect(menuItems(file, clip, "linux")).toContainEqual({ id: "reveal", label: "Show in folder" });
  expect(ids(menuItems({ ...file, on: "remote" }, clip, "macos"))).toEqual(["copy-path", "copy-relative"]);
});

test("Files debounces content search and parses numbered Git hits", async () => {
  const calls: unknown[] = [];
  const Files = new Function("View", "HITS", "exec", source.slice(source.indexOf("export default class"))
    .replace("export default ", "") + "; return Files;")(
    class {}, 200,
    async (command: string, args: string[], options: unknown) => {
      calls.push([command, args, options]);
      return { code: 0, stdout: args[0] === "rev-parse" ? "true\n" : "folder/a:1.txt\u000012\u0000   literal + hit   \n", stderr: "" };
    },
  );
  const files = new Files();
  files.context = { cwd: "/project", on: null };
  files.current = () => true;
  const timers: { ms: number; callback: Function; cancelled: boolean }[] = [];
  const cx = {
    notify() {},
    spawn(callback: Function) { return callback(cx); },
    timer: { after(ms: number, callback: Function) {
      const timer = { ms, callback, cancelled: false };
      timers.push(timer);
      return { cancel() { timer.cancelled = true; } };
    } },
  };
  files.filter = parseFilter("? first");
  files.search(cx);
  files.filter = parseFilter("? literal + hit");
  files.search(cx);
  expect(timers[0].cancelled).toBe(true);
  expect(timers[1].ms).toBe(300);
  expect(files.searching).toBe(true);
  await timers[1].callback(cx);
  expect(calls[1]).toEqual(["git", ["grep", "-nzIF", "--max-count=5", "-e", "literal + hit", "--", "."], { cwd: "/project", on: null }]);
  expect(files.hits).toEqual([{ path: "folder/a:1.txt", line: "12", text: "literal + hit" }]);
  expect(files.searching).toBe(false);
});
