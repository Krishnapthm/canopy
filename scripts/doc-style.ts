// Style lint for the wiki pages in docs/wiki: the mechanical part of
// docs/canopy/doc-style.md (filler words, em dashes, portable Markdown for a
// later Fumadocs move). How well a page reads is left to review.
// Run on files: `bun scripts/doc-style.ts docs/wiki/Panes.md`; with no files it
// checks every hand-written page. `bun scripts/docs.ts check` runs it too.
import { readdir, readFile } from "node:fs/promises";

const banned = ["simply", "easily", "seamless", "seamlessly", "powerful", "robust", "leverage", "utilize", "note that"];
const bannedRe = new RegExp(`\\b(${banned.join("|")})\\b`, "i");

/** Every style problem in one page, as `file:line: problem`. */
export function lintStyle(file: string, source: string): string[] {
  const errors: string[] = [];
  const lines = source.split("\n");
  const at = (i: number, problem: string) => errors.push(`${file}:${i + 1}: ${problem}`);
  const special = /\/_(Sidebar|Footer)\.md$/.test(file);
  if (!special && !/^# \S/.test(lines[0] ?? "")) at(0, "line 1 must be the page title (# Title)");

  let fence = false;
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (line.includes("\u2014")) at(i, "em dash: use a plain dash or two sentences");
    if (/^\s*(```|~~~)/.test(line)) { fence = !fence; continue; }
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
    if (bad) at(i, `filler word "${bad[1]}"`);
  }
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
