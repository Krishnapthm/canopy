import { readdir, readFile, writeFile } from "node:fs/promises";
import { gzipSync } from "node:zlib";
import assert from "node:assert/strict";
async function compress(dir) {
  for (const entry of await readdir(dir,{withFileTypes:true})) {
    const path = dir+"/"+entry.name;
    if (entry.isDirectory()) {await compress(path);continue;}
    if (!path.endsWith(".gz")) await writeFile(path+".gz",gzipSync(await readFile(path),{level:9}));
  }
}
await compress("dist");
const manifest = JSON.parse(await readFile("dist/.vite/manifest.json","utf8"));
const visited = new Set();
async function size(key) {
  if (visited.has(key)) return 0;
  visited.add(key);
  const chunk = manifest[key];
  let bytes = chunk.file.endsWith(".js") ? (await readFile("dist/"+chunk.file+".gz")).length : 0;
  for (const dependency of chunk.imports ?? []) bytes += await size(dependency);
  return bytes;
}
let bytes = 0;
for (const [key, chunk] of Object.entries(manifest)) if (chunk.isEntry) bytes += await size(key);
assert(bytes <= 120*1024, `Initial JS gzip ${bytes} exceeds 120 KB`);
console.log(`Initial JS gzip: ${bytes} bytes (budget 122880, excludes lazy chunks)`);
