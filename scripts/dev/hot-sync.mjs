// Local dev loop: copies the client files you edit into a served site directory as soon as they change, so a page
// reload shows the change. Usage: node scripts/dev/hot-sync.mjs [targetDir]. targetDir is a copy of the Pages layout
// (js/client inside it); the default is ../oscraft-site next to the repo. Only js/client and scenarios/moba are watched.
// The wasm package and the map images are not rebuilt here: they change only when rust/ changes (rebuild with
// wasm-pack and mapexport, then copy them into the target).
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const target = path.resolve(process.argv[2] ?? path.join(root, "..", "oscraft-site"));
const watched = ["js/client", "scenarios/moba"];
const DEBOUNCE_MS = 80;
const pending = new Map();
let timer = null;

const skip = (rel) => /(^|\/)(pkg|node_modules|vendor\/wireweave\/\.git)(\/|$)/.test(rel) || rel.endsWith(".gitignore");

function copyOne(rel) {
  const src = path.join(root, rel);
  const dst = path.join(target, rel);
  if (!fs.existsSync(src)) {
    if (fs.existsSync(dst)) fs.rmSync(dst, { force: true });
    return;
  }
  fs.mkdirSync(path.dirname(dst), { recursive: true });
  fs.copyFileSync(src, dst);
}

function flush() {
  const t0 = Date.now();
  const files = [...pending.keys()];
  pending.clear();
  for (const rel of files) {
    try {
      copyOne(rel);
    } catch (err) {
      console.error(`hot-sync FAIL ${rel}: ${err.message}`);
    }
  }
  console.log(`hot-sync ${files.length} file(s) in ${Date.now() - t0} ms: ${files.slice(0, 3).join(", ")}${files.length > 3 ? " ..." : ""}`);
}

function queue(rel) {
  if (skip(rel)) return;
  pending.set(rel, true);
  clearTimeout(timer);
  timer = setTimeout(flush, DEBOUNCE_MS);
}

function fullSync() {
  const t0 = Date.now();
  let n = 0;
  for (const dir of watched) {
    const base = path.join(root, dir);
    if (!fs.existsSync(base)) continue;
    const walk = (d) => {
      for (const ent of fs.readdirSync(d, { withFileTypes: true })) {
        const abs = path.join(d, ent.name);
        const rel = path.relative(root, abs).split(path.sep).join("/");
        if (skip(rel)) continue;
        if (ent.isDirectory()) walk(abs);
        else if (ent.isFile()) {
          copyOne(rel);
          n++;
        }
      }
    };
    walk(base);
  }
  console.log(`hot-sync full copy: ${n} file(s) to ${target} in ${Date.now() - t0} ms`);
}

fullSync();
for (const dir of watched) {
  const base = path.join(root, dir);
  if (!fs.existsSync(base)) continue;
  fs.watch(base, { recursive: true }, (_event, name) => {
    if (!name) return;
    queue(path.join(dir, name.toString()).split(path.sep).join("/"));
  });
}
console.log(`hot-sync watching ${watched.join(", ")} -> ${target} (Ctrl-C to stop)`);
