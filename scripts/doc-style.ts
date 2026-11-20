// Style lint for the wiki pages in docs/wiki: the mechanical part of
// docs/canopy/doc-style.md (ASD-STE100 limits, banned words, portable Markdown
// for a later Fumadocs move). Run on files: `bun scripts/doc-style.ts docs/wiki/Panes.md`;
// with no files it checks every hand-written page. `bun scripts/docs.ts check` runs it too.
import { readdir, readFile } from "node:fs/promises";

const banned = ["simply", "just", "easily", "seamless", "seamlessly", "powerful", "robust", "leverage", "utilize", "please", "note that", "currently", "soon"];
const bannedRe = new RegExp(`\\b(${banned.join("|")})\\b`, "i");

// Prose with code, link targets and images reduced to plain words.
const plain = (line: string) =>
  line
    .replace(/`[^`]*`/g, "X")
    .replace(/!\[([^\]]*)\]\([^)]*\)/g, "$1")
    .replace(/\[([^\]]*)\]\([^)]*\)/g, "$1")
    .replace(/[*_]/g, "");

const sentences = (text: string) => text.split(/(?<=[.!?])\s+(?=[A-Z0-9`"(*])/).map(s => s.trim()).filter(s => /\w/.test(s));
const words = (sentence: string) => sentence.split(/\s+/).filter(w => /\w/.test(w)).length;

/** Every style problem in one page, as `file:line: problem`. */
export function lintStyle(file: string, source: string): string[] {
  const errors: string[] = [];
  const lines = source.split("\n");
  const at = (i: number, problem: string) => errors.push(`${file}:${i + 1}: ${problem}`);
  const special = /\/_(Sidebar|Footer)\.md$/.test(file);
  if (!special && !/^# \S/.test(lines[0] ?? "")) at(0, "line 1 must be the page title (# Title)");

  let fence = false;
  let para: { start: number; text: string; step: boolean } | undefined;
  const endPara = () => {
    if (!para) return;
    const list = sentences(para.text);
    if (list.length > 3) at(para.start, `paragraph has ${list.length} sentences (max 3)`);
    const max = para.step ? 20 : 25;
    for (const s of list) if (words(s) > max) at(para.start, `sentence has ${words(s)} words (max ${max}${para.step ? " in a step" : ""}): "${s.slice(0, 60)}..."`);
    para = undefined;
  };

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (line.includes("—")) at(i, "em dash: use a plain dash or two sentences");
    if (/^\s*(```|~~~)/.test(line)) { endPara(); fence = !fence; continue; }
    if (fence) continue;
    const text = line.replace(/`[^`]*`/g, "X");
    if (text.includes("[[")) at(i, "wiki [[link]]: use [text](Page-Name)");
    if (/^\s*>\s*\[!/.test(text)) at(i, "GitHub alert: use > **Warning:** ...");
    if (/<\/?[a-zA-Z][^>]*>/.test(text) && !/^<!--/.test(line)) at(i, "raw HTML: use Markdown");
    if (/!\[\]\(/.test(text)) at(i, "image without alt text");
    for (const [, target] of text.matchAll(/\]\(([^)\s]+)\)/g))
      if (!/^(https?:|#|images\/|mailto:)/.test(target) && !/^[A-Za-z][A-Za-z0-9-]*(#[\w-]+)?$/.test(target))
        at(i, `link target "${target}": use Page-Name, Page-Name#heading, images/... or a full URL`);
    const bad = text.replace(/\]\([^)]*\)/g, "]").match(bannedRe);
    if (bad) at(i, `banned word "${bad[1]}"`);

    if (/^\s*(#|\||$|<!--)/.test(line)) { endPara(); continue; }
    const item = line.match(/^\s*(\d+\.|[-*])\s+(.*)$/);
    if (item) { endPara(); para = { start: i, text: plain(item[2]), step: /\d/.test(item[1]) }; continue; }
    if (/;/.test(plain(line).replace(/&\w+;/g, ""))) at(i, "semicolon: split into two sentences");
    const body = plain(line.replace(/^\s*>\s?/, ""));
    if (para) para.text += " " + body;
    else para = { start: i, text: body, step: false };
  }
  endPara();
  return errors;
}

/** The hand-written wiki pages (Commands.md is generated). */
export async function wikiPages(): Promise<string[]> {
  return (await readdir("docs/wiki")).filter(f => f.endsWith(".md") && f !== "Commands.md").sort().map(f => `docs/wiki/${f}`);
}

if (import.meta.main) {
  const files = process.argv.slice(2).length ? process.argv.slice(2) : await wikiPages();
  const errors = (await Promise.all(files.map(async f => lintStyle(f, await readFile(f, "utf8"))))).flat();
  if (errors.length) { console.error(errors.join("\n")); process.exit(1); }
  console.log(`Style clean in ${files.length} page${files.length === 1 ? "" : "s"}.`);
}
