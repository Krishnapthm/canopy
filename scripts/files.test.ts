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

test("Files saves the snapshot, keeps conflicts and newer edits, and Done exits only a saved draft", async () => {
  let disk = "original", draft = "draft", pending: Promise<void> | undefined;
  const calls: unknown[] = [];
  const Files = new Function("View", "writeTextFile", "run", "props", source.slice(source.indexOf("export default class")).replace("export default ", "") + "; return Files;")(
    class {},
    async (path: string, text: string, expected: string) => {
      expect(path).toBe("/fixture/hello.md");
      if (disk !== expected) throw new Error("File changed outside Canopy");
      disk = text;
    },
    (...args: unknown[]) => calls.push(args), () => ({}),
  );
  const files = new Files();
  files.root = "/fixture"; files.file = "hello.md"; files.original = disk;
  files.code = { value: () => draft };
  const cx = { notify() {}, spawn(f: Function) { pending = f(cx); } };
  files.save(false, cx); await pending;
  expect(disk).toBe("draft"); expect(files.original).toBe("draft"); expect(calls).toEqual([]);
  disk = "external"; draft = "my changes";
  files.save(true, cx); await pending;
  expect(files.saveError).toContain("changed outside");
  expect(files.code.value()).toBe("my changes"); expect(files.original).toBe("draft");
  expect(disk).toBe("external"); expect(calls).toEqual([]);
  disk = "draft";
  files.save(true, cx); draft = "newer changes"; await pending;
  expect(files.original).toBe("my changes"); expect(calls).toEqual([]);
  files.save(true, cx); await pending;
  expect(disk).toBe("newer changes");
  expect(calls).toEqual([["panel.editor", { done: true, path: "/fixture/hello.md" }]]);
});

test("Files edits full markdown source instead of the bounded read-only preview", async () => {
  const text = "# Source\n" + "a".repeat(400001);
  const options: any[] = [];
  let editing = true, pending: Promise<void> | undefined;
  const Files = new Function("View", "readTextFile", "theme", "settings", "EditorState", "props",
    source.replace(/^import .*;\n/gm, "").replace("export default class", "class") + "; return Files;")(
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
  const Files = new Function("View", "props", "run", "writeTextFile",
    source.slice(source.indexOf("export default class")).replace("export default ", "") + "; return Files;")(
    class {}, () => ({ canvas: "viewer", editing: true }),
    (...args: unknown[]) => calls.push(args), async () => {},
  );
  const files = new Files();
  Object.assign(files, { root: "/fixture", file: "hello.md", original: "saved", code: { value: () => draft } });
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
