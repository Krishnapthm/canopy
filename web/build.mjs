import { readdir, readFile, writeFile } from "node:fs/promises";
import { gzipSync } from "node:zlib";
import assert from "node:assert/strict";
async function compress(dir) {
  let js = 0;
  for (const entry of await readdir(dir,{withFileTypes:true})) {
    const path = dir+"/"+entry.name;
    if (entry.isDirectory()) {js += await compress(path);continue;}
    if (path.endsWith(".gz")) continue;
    const zipped = gzipSync(await readFile(path),{level:9});
    await writeFile(path+".gz",zipped);
    if (path.endsWith(".js")) js += zipped.length;
  }
  return js;
}
const size = await compress("dist");
assert(size <= 50*1024, `All JS gzip ${size} exceeds 50 KB`);
console.log(`All JS gzip: ${size} bytes (budget 51200, including lazy chunks)`);
