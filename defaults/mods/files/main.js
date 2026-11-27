// Modified for Canopy.
// A lazy file browser for the focused pane's folder, with bounded read-only previews.
import { View, div, svg } from "gpui-kit";
import { v_flex, h_flex, Editor, EditorState, Input, InputState } from "gpui-base";
import { spaces, panes, focused, exec, readTextFile, writeTextFile, watchFolder, theme, settings, window, props, run, openOverlay, nextCommand, shared, setShared, notify, ui } from "herdr";

const autoSave = () => shared("files.auto_save") !== false;

// Zero-context hunk headers describe changes in the working copy's line numbers.
export function lineMarks(diff) {
  const marks = [];
  for (const [, oldCount = "1", start, newCount = "1"] of diff.matchAll(/^@@ -\d+(?:,(\d+))? \+(\d+)(?:,(\d+))? @@/gm)) {
    const line = Number(start), count = Number(newCount);
    if (count === 0) marks.push({ line: line + 1, kind: "removed" });
    else for (let i = 0; i < count; i++) marks.push({ line: line + i, kind: oldCount === "0" ? "added" : "modified" });
  }
  return marks;
}

async function fileMarks(file, text, options) {
  try {
    const git = (...args) => exec("git", args, options);
    if ((await git("rev-parse", "--is-inside-work-tree")).stdout.trim() !== "true") return [];
    const lines = text.split("\n").length;
    const path = `:(literal)${file}`;
    if ((await git("ls-files", "--error-unmatch", "--", path)).code === 1) {
      return Array.from({ length: lines }, (_, i) => ({ line: i + 1, kind: "added" }));
    }
    const diff = await git("diff", "-U0", "--no-color", "--no-ext-diff", "--no-textconv", "--", path);
    return diff.code === 0 ? lineMarks(diff.stdout).map(mark =>
      mark.kind === "removed" ? { ...mark, line: Math.min(mark.line, lines) } : mark) : [];
  } catch {
    // Git is optional for the viewer.
    return [];
  }
}

function location() {
  if (props()?.canvas) return { cwd: props().root, on: props().on ?? null };
  const space = spaces().find(s => s.active), f = focused();
  const pane = f?.pane ? panes().find(p => p.pane === f.pane) : null;
  const on = f?.pane === null ? (f.terminal?.includes("/") ? f.terminal.split("/")[0] : null) : space?.machine ?? null;
  return { cwd: pane?.cwd || f?.cwd || space?.folder, on, space: space?.id };
}
const key = () => JSON.stringify(location());
const LANG = { md: "markdown", mdx: "markdown", ts: "js", tsx: "js", mts: "js", cts: "js", jsx: "js", mjs: "js", toml: "yaml", jsonc: "json", svelte: "html", vue: "html" };
const PER_FOLDER = 500, TOTAL = 5000, HITS = 200;
const CODE = new Set(["c", "cc", "cpp", "cs", "css", "go", "h", "hpp", "html", "java", "js", "json", "jsonc", "jsx", "kt", "lua", "mjs", "cjs", "mts", "cts", "php", "py", "rb", "rs", "scss", "sh", "sql", "svelte", "swift", "toml", "ts", "tsx", "vue", "xml", "yaml", "yml", "zig", "zsh"]);

// Lucide icon for a tree row: folders, markdown notes, code, else a plain file.
function icon(entry) {
  if (entry.dir) return "folder";
  const ext = entry.name.includes(".") ? entry.name.split(".").pop().toLowerCase() : "";
  return ext === "md" || ext === "mdx" ? "sticky-note" : CODE.has(ext) ? "code" : "file";
}

/** @typedef {{kind: "file" | "dir", path: string, on?: string | null, canvas?: boolean}} TreeTarget */
/** @type {ReadonlyArray<{id: string, label: string, when: (target: TreeTarget, clip: unknown) => boolean} | null>} */
export const TREE_ACTIONS = [
  { id: "copy-path", label: "Copy path", when: () => true },
  { id: "copy-relative", label: "Copy relative path", when: () => true },
  null,
  { id: "copy", label: "Copy", when: () => true },
  { id: "cut", label: "Cut", when: () => true },
  { id: "paste", label: "Paste", when: (_t, clip) => Boolean(clip) },
  null,
  { id: "new-file", label: "New file", when: t => t.kind === "dir" },
  { id: "new-folder", label: "New folder", when: t => t.kind === "dir" },
  { id: "rename", label: "Rename", when: () => true },
  { id: "trash", label: "Move to Trash", when: () => true },
  null,
  { id: "reveal", label: "Reveal in Finder", when: () => true },
  { id: "terminal", label: "Open terminal here", when: () => true },
  { id: "browser", label: "Open in Canopy browser", when: t => t.canvas && t.kind === "file" },
  { id: "preview", label: "Open in Preview", when: t => t.kind === "file" && /\.(pdf|png|jpe?g|gif|webp)$/i.test(t.path) },
];

export function menuItems(target, clip, platform) {
  return TREE_ACTIONS.flatMap(action => {
    if (!action) return target.on ? [] : [null];
    const { id, label, when } = action;
    if ((target.on && id !== "copy-path" && id !== "copy-relative") || (id === "preview" && platform !== "macos") || !when(target, clip)) return [];
    return [{ id, label: id === "reveal" && platform !== "macos" ? "Show in folder" : label }];
  });
}

export function parseFilter(text) {
  return text.startsWith("?") ? { kind: "content", query: text.slice(1).trim() } : { kind: "name", query: text };
}

function matchesFile(entry, filter) {
  return entry.dir || !filter || entry.name.toLowerCase().includes(filter.toLowerCase());
}

async function directory(path, on) {
  const windowsDirectory = async () => {
    // Windows PowerShell otherwise uses the console code page; exec decodes UTF-8.
    // Links keep directory attributes even when dangling; check their target.
    // Other reparse points (OneDrive's folders) have no LinkType and are folders.
    // PowerShell reads curly single quotes as quotes too, so they're doubled like '.
    const r = await exec("powershell", ["-NoProfile", "-Command", "[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false); & { param($p) @(Get-ChildItem -LiteralPath $p -Force -ErrorAction Stop | Where-Object Name -ne '.git' | Select-Object -First 501 | ForEach-Object { @{ name=$_.Name; dir=($_.PSIsContainer -and (-not ($_.Attributes -band [IO.FileAttributes]::ReparsePoint) -or -not $_.LinkType -or ($_.Target -and [IO.Directory]::Exists(([IO.Path]::Combine([IO.Path]::GetDirectoryName($_.FullName), $_.Target[0])))))) } }) | ConvertTo-Json -Compress } '" + path.replace(/['‘’‚‛]/g, "$&$&") + "'"], { on });
    if (r.code !== 0) throw new Error(r.stderr || "Cannot list this folder");
    const result = JSON.parse(r.stdout || "[]");
    return Array.isArray(result) ? result : [result];
  };
  if (!on && window()?.platform === "windows") return windowsDirectory();
  const script = 'n=0; for p in "$1"/* "$1"/.[!.]* "$1"/..?*; do [ -e "$p" ] || [ -L "$p" ] || continue; name=${p##*/}; [ "$name" = .git ] && continue; if [ -d "$p" ]; then kind=d; else kind=f; fi; printf "%s\\0%s\\0" "$kind" "$name"; n=$((n+1)); [ "$n" -gt 500 ] && break; done; test -d "$1"';
  let r;
  try { r = await exec("sh", ["-c", script, "files", path], { on }); }
  catch (e) {
    if (on && String(e).includes("cannot start sh: program not found")) return windowsDirectory();
    throw e;
  }
  if (r.code !== 0) throw new Error(r.stderr || "Cannot list this folder");
  const fields = r.stdout.split("\0"), out = [];
  for (let i = 0; i + 1 < fields.length; i += 2) out.push({ name: fields[i + 1], dir: fields[i] === "d" });
  return out;
}

export default class Files extends View {
  init(_props, cx) {
    this.filter = parseFilter("");
    this.treeId = Math.random().toString(36);
    this.input = InputState.new({ placeholder: "Filter, or ? to search in files" });
    this.input.on("change", (_e, cx) => {
      this.filter = parseFilter(this.input.value());
      this.search(cx);
      cx.notify();
    });
    if (props()?.canvas !== "viewer") cx.spawn(async cx => {
      while (true) {
        const message = await nextCommand();
        try {
          // A command with arguments arrives as `{"name","args"}` JSON, without them as its name.
          const { name, args = {} } = message.startsWith("{") ? JSON.parse(message) : { name: message };
          if (name === "selected" && args.path !== this.selected) {
            this.selected = args.path;
            cx.notify();
          } else if (name === "collapse_all" && (!args?.treeId || args.treeId === this.treeId)) {
            this.folders.clear();
            for (const path of this.entries.keys()) if (path) this.entries.delete(path);
            cx.notify();
          } else if (name === "tree_action" && args.treeId === this.treeId && args.context === this.context?.key && this.current(this.context)) {
            this.treeAction(args, cx);
          } else if (name === "file_changed" && !this.on) {
            const clip = shared("files.clipboard");
            if (args.moved && clip?.op === "cut" && clip.paths.includes(args.moved)) {
              const paths = clip.paths.filter(path => path !== args.moved);
              setShared("files.clipboard", paths.length ? { ...clip, paths } : null);
            }
            this.refresh(cx);
          }
        } catch (e) {
          notify({ title: "File action failed", body: String(e), level: "error" });
        }
      }
    });
    this.saved = new Map();
    this.entries = new Map(); this.folders = new Set(); this.pending = new Set(); this.marks = {};
    this.refresh(cx);
    cx.timer.every(10000, cx => this.refresh(cx));
  }

  watch(root, cx) {
    if (this.on || this.watched === root) return;
    this.watched = root;
    const context = this.context;
    cx.spawn(async cx => {
      try { while (this.context === context) { await watchFolder(root); if (this.current(context)) { this.refresh(cx); if (this.file && !props()?.editing) this.show(this.file, cx); } } }
      catch { if (this.context === context) this.watched = null; }
    });
  }

  current(context) {
    return this.context === context && key() === context.key;
  }

  async loadFolder(path, context) {
    const { on, cwd: root } = context;
    const entries = await directory(path ? `${root}/${path}` : root, on);
    if (!this.current(context) || (path && !this.folders.has(path))) return;
    const used = [...this.entries.entries()].filter(([p]) => p !== path).reduce((n, [, e]) => n + e.length, 0);
    this.entries.set(path, entries.slice(0, Math.max(0, Math.min(PER_FOLDER, TOTAL - used))).sort((a, b) => Number(b.dir) - Number(a.dir) || a.name.localeCompare(b.name)));
    if (entries.length > PER_FOLDER || used + entries.length > TOTAL) this.limited = true;
  }

  refresh(cx) {
    const { cwd, on } = location(), next = key();
    if (next !== this.context?.key) {
      if (this.context) this.saved.set(this.context.key, { folders: this.folders, file: this.file });
      const saved = this.saved.get(next);
      this.context = { key: next, cwd, on }; this.root = cwd; this.on = on; this.entries = new Map(); this.folders = new Set(saved?.folders);
      this.pending = new Set(); this.marks = {}; this.all = null; this.show(null); this.watched = null; this.error = null; this.limited = false;
      this.loading = false;
      if (props()?.file || saved?.file) this.show(props()?.file || saved.file, cx);
    }
    if (!cwd || this.loading) return;
    const context = this.context;
    this.loading = true;
    this.limited = false;
    cx.spawn(async cx => {
      try {
        for (const path of ["", ...this.folders]) {
          if (!this.current(context)) return;
          await this.loadFolder(path, context);
        }
        const top = await exec("git", ["rev-parse", "--show-toplevel"], { cwd, on }).catch(() => null);
        const marks = {};
        if (top?.code === 0) {
          // Writes nothing under .git (index lock, fsmonitor cookie): the folder watch would refresh again.
          const status = await exec("git", ["--no-optional-locks", "-c", "core.fsmonitor=false", "status", "--porcelain=v1", "-z", "--no-renames", "--untracked-files=all"], { cwd, on }).catch(() => null);
          if (status?.code === 0) for (const entry of status.stdout.split("\0").filter(Boolean).slice(0, TOTAL)) {
            const xy = entry.slice(0, 2);
            marks[`${top.stdout.trim()}/${entry.slice(3)}`] = xy === "??" || xy.includes("A") ? "A" : xy.includes("U") ? "U" : xy.includes("D") ? "D" : "M";
          }
        }
        // Search covers every file, not just loaded folders; git lists them fast and honors .gitignore.
        const listed = top?.code === 0 ? await exec("git", ["ls-files", "-z", "--cached", "--others", "--exclude-standard"], { cwd, on }).catch(() => null) : null;
        if (!this.current(context)) return;
        this.all = listed?.code === 0 ? listed.stdout.split("\0").filter(Boolean) : null;
        this.marks = marks; this.error = null; this.watch(cwd, cx);

      } catch (e) { if (this.current(context)) this.error = String(e); }
      finally { if (this.context === context) this.loading = false; cx.notify(); }
    });
  }

  toggle(path, cx) {
    if (this.folders.has(path)) {
      for (const folder of [...this.folders]) if (folder === path || folder.startsWith(path + "/")) {
        this.folders.delete(folder);
        this.entries.delete(folder);
      }
    } else {
      this.folders.add(path);
      if (!this.entries.has(path) && !this.pending.has(path)) {
        const context = this.context;
        this.pending.add(path);
        cx.spawn(async cx => {
          try { await this.loadFolder(path, context); }
          catch (e) { if (this.current(context)) this.error = String(e); }
          finally { if (this.context === context) this.pending.delete(path); cx.notify(); }
        });
      }
    }
    cx.notify();
  }

  show(path, cx) {
    this.saveTimer?.cancel(); this.saveTimer = null;
    this.saveError = null;
    const load = this.fileLoad = {};
    this.code?.release(); this.code = null; this.file = path; this.fileError = null; this.reportedDirty = undefined;
    if (!path || !cx) return;
    const context = this.context, root = this.root, on = this.on;
    cx.spawn(async cx => {
      try {
        const text = await readTextFile(`${root}/${path}`, { on });
        if (this.fileLoad !== load || !this.current(context)) return;
        if (text === null) { this.fileError = "File deleted"; return; }
        if (text.includes("\0")) { this.fileError = "Can't preview"; return; }
        const t = theme(), ext = path.includes(".") ? path.split(".").pop() : "txt";
        this.original = text;
        const viewer = props()?.canvas === "viewer";
        const code = this.code = EditorState.new({ language: LANG[ext] ?? ext, readonly: !props()?.editing,
          value: !props()?.editing && text.length > 400000 ? text.slice(0, 400000) + "\n… (file truncated)" : text,
          font_family: settings().font_family, font_size: 12,
          ...(viewer ? { soft_wrap: true, active_line: true, mark_colors: { added: t.ansi2, modified: t.ansi4, removed: t.ansi1 } } : {}),
          syntax: { keyword: t.ansi5, string: t.ansi2, comment: t.dim, function: t.ansi4, type: t.ansi3, number: t.ansi1, constant: t.ansi6 } });
        code.on("change", (_e, cx) => {
          this.reportDirty();
          this.saveTimer?.cancel();
          this.saveTimer = null;
          this.queuedSave = null;
          if (viewer && props()?.editing && autoSave() && (this.saving || code.value() !== this.original)) {
            this.saveTimer = cx.timer.after(1000, cx => {
              this.saveTimer = null;
              if (this.code === code && props()?.editing && autoSave()) this.save(false, cx);
            });
          }
          cx.notify();
        });
        this.reportDirty();
        if (viewer) {
          code.on("gutter_quote", e => run("quote.menu", {
            text: code.value().split("\n").slice(e.start - 1, e.end).join("\n"),
            start: e.start, end: e.end, path: `${root}/${path}`, point: { x: e.x, y: e.y },
          }));
          cx.notify();
          const marks = await fileMarks(path, text, { cwd: root, on });
          if (this.code === code && this.original === text) code.set_line_marks(marks);
        }
      } catch { if (this.fileLoad === load && this.current(context)) this.fileError = "Cannot preview this file (binary, too large, or unreadable)"; }
      finally { cx.notify(); }
    });
  }

  // Only transitions reach the native strip, never a timer or the file contents.
  reportDirty() {
    if (props()?.canvas !== "viewer" || !props()?.editing || !this.code) return;
    const dirty = this.code.value() !== this.original;
    if (dirty === this.reportedDirty) return;
    this.reportedDirty = dirty;
    run("panel.editor", { path: `${this.root}/${this.file}`, dirty });
  }

  save(manual, cx) {
    const code = this.code;
    if (!code || !props()?.editing) return;
    this.saveTimer?.cancel();
    this.saveTimer = null;
    if (this.saving) {
      this.queuedSave = { code, manual };
      return;
    }
    const text = code.value();
    if (text === this.original) return;
    const { root, file, on } = this;
    const path = `${root}/${file}`;
    this.saving = true; this.saveError = null; cx.notify();
    cx.spawn(async cx => {
      try {
        await writeTextFile(path, text, this.original);
        if (this.code !== code) return;
        this.original = text;
        this.reportDirty();
        cx.spawn(async () => {
          const marks = await fileMarks(file, text, { cwd: root, on });
          if (this.code === code && this.original === text) code.set_line_marks(marks);
        });
      } catch (e) {
        if (this.code === code) this.saveError = String(e);
        notify({ title: `Could not save ${file.split("/").pop()}`, body: String(e), level: "error",
          action: { name: "preview.open", args: { path } } });
      } finally {
        this.saving = false;
        const queued = this.queuedSave;
        this.queuedSave = null;
        if (queued && queued.code === this.code && (queued.manual || autoSave())) this.save(queued.manual, cx);
        cx.notify();
      }
    });
  }

  treeAction({ id, path, kind, name }, cx) {
    const prefix = `${this.root}/`;
    if (typeof path !== "string" || !path.startsWith(prefix) || !["file", "dir"].includes(kind)) {
      throw new Error("Select a file or folder in this tree");
    }
    const relative = path.slice(prefix.length);
    if (id === "copy-path" || id === "copy-relative") {
      const text = id === "copy-path" ? path : relative;
      cx.write_to_clipboard(text);
      notify({ title: "Path copied", body: text, level: "success" });
      return;
    }
    if (this.on) return;
    const clip = shared("files.clipboard");
    const leaf = path.split("/").at(-1);
    const parent = path.slice(0, -leaf.length);
    const folder = kind === "dir" ? path : parent;
    if (id === "copy" || id === "cut") {
      setShared("files.clipboard", { op: id, paths: [path] });
    } else if (id === "paste" && clip) {
      for (const from of clip.paths) run(clip.op === "cut" ? "file.move" : "file.copy", { from, into: folder });
    } else if (id === "new-file" || id === "new-folder" || id === "rename") {
      if (name === undefined) {
        openOverlay({ mod: "prompt", kind: "dialog", width: 380, props: {
          title: id === "rename" ? "Rename" : id === "new-file" ? "New file" : "New folder",
          value: id === "rename" ? leaf : "",
          command: { name: "files.tree_action", args: { id, path, kind, treeId: this.treeId, context: this.context.key } },
        } });
        return;
      }
      if (!name.trim() || name === "." || name === ".." || /[\/\\\0]/.test(name) || (window()?.platform === "windows" && name.includes(":"))) {
        notify({ title: "Enter a file or folder name", level: "error" });
        return;
      }
      if (id === "rename" && name !== leaf) run("file.rename", { from: path, to: parent + name });
      if (id !== "rename") run(id === "new-file" ? "file.new" : "file.mkdir", { path: folder + "/" + name });
    } else if (id === "trash") {
      openOverlay({ mod: "confirm", kind: "dialog", width: 380, props: {
        title: `Move ${leaf} to the Trash?`, accept: "Move to Trash", destructive: true,
        command: { name: "file.trash", args: { path } },
      } });
    } else if (id === "reveal") run("file.reveal", { path });
    else if (id === "terminal") run("terminal.open", { folder });
    else if (id === "browser") run("canvas.pane_new", { kind: "browser", url: path });
    else if (id === "preview") run("preview.external", { path });
  }

  search(cx) {
    this.searchTimer?.cancel();
    const filter = this.filter, context = this.searchContext = this.context;
    this.hits = [];
    this.searching = filter.kind === "content" && filter.query.length >= 2 && Boolean(context?.cwd);
    if (!this.searching) return;
    this.searchTimer = cx.timer.after(300, cx => cx.spawn(async cx => {
      const current = () => this.filter === filter && this.current(context);
      if (!current()) return;
      const { cwd, on } = context;
      try {
        const top = await exec("git", ["rev-parse", "--is-inside-work-tree"], { cwd, on }).catch(() => null);
        if (!current()) return;
        const git = top?.code === 0 && top.stdout.trim() === "true";
        const options = ["--max-count=5", "-e", filter.query];
        const result = git
          ? await exec("git", ["grep", "-nzIF", ...options, "--", "."], { cwd, on })
          : await exec("grep", ["-rHnIF", "--null", ...options, "."], { cwd, on });
        if (!current()) return;
        if (result.code !== 0 && result.code !== 1) throw new Error(result.stderr || "Cannot search files");
        const pattern = git ? /([^\0]+)\0(\d+)\0([^\n]*)\n?/g : /([^\0]+)\0(\d+):([^\n]*)\n?/g;
        for (const [, path, line, text] of result.stdout.matchAll(pattern)) {
          this.hits.push({ path: path.replace(/^\.\//, ""), line, text: text.trim() });
          if (this.hits.length === HITS) break;
        }
      } catch (e) {
        if (current()) notify({ title: "File search failed", body: String(e), level: "error" });
      } finally {
        if (current()) { this.searching = false; cx.notify(); }
      }
    }));
  }

  rows(prefix, depth, out, t) {
    if (out.length >= TOTAL || depth > 40) return out;
    if (this.filter.kind === "content") {
      if (this.searchContext !== this.context) return out;
      if (!this.hits.length) out.push(div().p(8).text_size(12).text_color(t.dim).child(this.searching ? "Searching" : "No matches"));
      for (const { path, line, text } of this.hits) {
        out.push(this.row(path, { name: path, dir: false }, 0, t, `${line}  ${text}`, line));
      }
      return out;
    }
    if (this.filter.query && this.all && !depth) {
      // shortcut: substring match on the relative path, outside git the tree filter below applies; add fuzzy ranking if lists get noisy.
      const q = this.filter.query.toLowerCase();
      for (const path of this.all) {
        if (!path.toLowerCase().includes(q)) continue;
        // Rows are built every render, so a short query stays fast.
        if (out.length >= HITS) { out.push(div().p(8).text_size(11).text_color(t.dim).child(`First ${HITS} matches; type more to narrow`)); break; }
        const cut = path.lastIndexOf("/");
        out.push(this.row(path, { name: path.slice(cut + 1), dir: false }, 0, t, cut > 0 ? path.slice(0, cut) : ""));
      }
      return out;
    }
    for (const entry of this.entries.get(prefix) ?? []) {
      if (out.length >= TOTAL) break;
      if (!matchesFile(entry, this.filter.query)) continue;
      const path = prefix ? `${prefix}/${entry.name}` : entry.name;
      out.push(this.row(path, entry, depth, t, ""));
      if (entry.dir && this.folders.has(path)) this.rows(path, depth + 1, out, t);
    }
    return out;
  }

  // One tree or search row; `hint` is the dimmed folder shown beside a search hit.
  row(path, entry, depth, t, hint, line) {
    const open = this.folders.has(path), mark = this.marks[`${this.root}/${path}`];
    const filePath = `${this.root}/${path}`;
    const kind = entry.dir ? "dir" : "file";
    const tone = mark === "A" ? t.ansi2 : mark === "D" || mark === "U" ? t.ansi1 : mark ? t.blocked : null;
    return h_flex().id(`file-${path}${line ? `:${line}` : ""}`).h(28).flex_none().pl(8 + depth * 18).pr(8).gap(7).items_center().rounded(7).cursor_pointer().hover(s => s.bg(t.hover)).when((this.file === path || this.selected === `${this.root}/${path}`), el => el.bg(t.hover))
      .on_mouse_down("right", (_e, cx) => {
        const actions = menuItems({ kind, path: filePath, on: this.on, canvas: Boolean(props()?.canvas) }, shared("files.clipboard"), window()?.platform)
          .map(item => item ? [item.label, "files.tree_action", { id: item.id, path: filePath, kind, treeId: this.treeId, context: this.context.key }] : ["", null, { label: true }]);
        cx.stop_propagation();
        openOverlay({ mod: "menu", kind: "popup", width: 230, frame: "none",
          props: { tab: "primary", target: { items: actions } } });
      })
      .on_click((_e, cx) => entry.dir ? this.toggle(path, cx) : props()?.canvas && !this.on ? (this.selected = filePath, run("preview.open", { path: filePath }), cx.notify()) : this.show(path, cx))
      .child(entry.dir ? svg(`mods/files/chevron-${open ? "down" : "right"}.svg`).size(12).flex_none().text_color(t.dim) : div().w(12).flex_none())
      .child(svg(`mods/files/${icon(entry)}.svg`).size(14).flex_none().text_color(t.muted))
      .child(div().min_w(0).truncate().text_color(tone ?? t.foreground).child(entry.name))
      .child(div().flex_1().min_w(0).truncate().text_size(11).text_color(t.dim).child(hint))
      .when(Boolean(mark), el => el.child(div().text_size(11).text_color(tone).child(mark)));
  }

  render(cx) {
    if (key() !== this.context?.key) this.refresh(cx);
    const t = theme();
    if (props()?.canvas === "viewer") {
      const dismissTip = (_e, cx) => { setShared("files.auto_save_tip", "seen"); cx.notify(); };
      return v_flex().relative().size_full().min_h(0).bg(t.canvas).text_color(t.foreground)
        .on_key_down((e, cx) => {
          if (props()?.editing && e.keystroke === "cmd-s") { this.save(true, cx); cx.stop_propagation(); }
        })
        .when(Boolean(props()?.editing), el => el.child(h_flex().h(32).flex_none().px(12).gap(8).items_center().border_b(1).border_color(t.border).text_size(12)
          .child(div().flex_1().text_color(t.dim).child(this.saving ? "Saving" : this.code && this.code.value() !== this.original ? "Unsaved" : "Saved"))
          .when(!autoSave(), bar => bar.child(div().id("file-Save").px(8).py(3).rounded(ui.size.radiusSm).border_1().border_color(t.border)
            .cursor_pointer().hover(s => s.bg(t.hover)).on_click((_e, cx) => this.save(true, cx)).child("Save")))))
        .when(Boolean(this.saveError), el => el.child(div().px(12).py(6).text_size(12).text_color(t.foreground).child(this.saveError)))
        // A finished selection opens the quote popover where the mouse came up.
        .on_mouse_up("left", (e) => {
          if (props()?.editing) return;
          const quote = this.code?.selection();
          if (quote?.text.trim()) run("quote.menu", { ...quote, path: `${this.root}/${this.file}`, point: e.position });
        })
        .child(this.code ? Editor.new(this.code).flex_1().min_h(0) : div().p(16).text_color(t.dim).child(this.fileError || "Loading file"))
        .when(Boolean(props()?.editing) && autoSave() && shared("files.auto_save_tip") !== "seen", el =>
          el.child(h_flex().absolute().top(40).right(8).left(72).justify_end()
            .child(v_flex().max_w(320).min_w(0).p(12).gap(8).rounded(ui.size.radiusSm).border_1().border_color(t.border)
              .bg(t.popover).text_color(t.foreground).text_size(12)
              .child(h_flex().items_center().gap(8)
                .child(div().flex_1().font_semibold().child("Auto save is on"))
                .child(div().id("auto-save-tip-close").px(6).cursor_pointer().hover(s => s.bg(t.hover)).on_click(dismissTip).child("×")))
              .child(div().child("Changes you make to a file now save on their own. To turn this off, open the More menu and uncheck Auto save."))
              .child(h_flex().justify_end().child(div().id("auto-save-tip-ok").px(8).py(4).rounded(ui.size.radiusSm)
                .bg(t.hover).cursor_pointer().on_click(dismissTip).child("Got it"))))));
    }
    if (props()?.canvas === "tree") {
      if (this.filter.kind === "content" && this.searchContext !== this.context) this.search(cx);
      if (this.on && this.file) return v_flex().size_full().min_h(0)
        .child(div().id("files-remote-back").p(8).cursor_pointer().hover(s => s.bg(t.hover))
          .on_click((_e, cx) => { this.show(null); cx.notify(); }).child("Back to files"))
        .child(this.code ? Editor.new(this.code).flex_1().min_h(0) : div().p(8).child(this.fileError || "Loading file"));
      const list = this.rows("", 0, [], t);
      return v_flex().size_full().min_h(0).text_size(13).text_color(t.muted)
        .child(div().p(8).flex_none().child(Input.new(this.input).w_full().h(30).px(10).gap(8).rounded(8).border_1().border_color(t.border).text_size(12)
          .child(svg("mods/files/search.svg").size(14).flex_none().text_color(t.dim))))
        .child(v_flex().id("canvas-files-tree").flex_1().min_h(0).overflow_y_scrollbar().px(6).pb(6).children(list)
          .when(this.filter.kind === "name" && Boolean(this.filter.query) && !list.length && Boolean(this.entries.get("")?.length), el => el.child(div().p(8).text_size(12).text_color(t.dim).child("No matching files")))
          .when(this.filter.kind === "name" && (Boolean(this.error) || !this.entries.get("")?.length), el => el.child(div().p(8).text_size(12).text_color(t.dim).child(this.error || (this.loading ? "Loading files" : this.root ? "No files" : "Click a space to see its files"))))
          .when(this.limited, el => el.child(div().p(8).text_size(11).child("File list limited"))));
    }
    return v_flex().id("files").size_full().min_h(0).rounded(10).overflow_hidden().bg(t.background).border_1().border_color(t.border).text_color(t.foreground).text_size(13)
      .child(h_flex().h(42).flex_none().px(14).gap(8).items_center().border_b(1).border_color(t.border)
        .when(Boolean(this.file), el => el.child(div().id("files-back").w(20).cursor_pointer().text_color(t.muted).on_click((_e, cx) => { this.show(null); cx.notify(); }).child("←")))
        .child(div().font_semibold().child("Files"))
        .child(div().flex_1().min_w(0).truncate().text_right().text_size(12).text_color(t.dim).child(this.file ?? this.root?.split(/[\\/]/).pop() ?? "")))
      .when(!this.file, el => el.child(v_flex().id("files-tree").flex_1().min_h(0).overflow_y_scrollbar().p(6).children(this.rows("", 0, [], t))
        .when(Boolean(this.error) || !this.entries.get("")?.length, el => el.child(div().p(8).text_size(12).text_color(t.dim).child(this.error || (this.loading ? "Loading files" : this.root ? "No files" : "No folder"))))
        .when(this.limited, el => el.child(div().p(8).text_size(11).text_color(t.dim).child("File list limited to 500 entries per folder and 5,000 total")))))
      .when(Boolean(this.file), el => el.child(this.code ? Editor.new(this.code).flex_1().min_h(0) : div().p(12).text_color(t.dim).child(this.fileError || "Loading file")));
  }
}
