import fs from "node:fs";
import { spawnSync } from "node:child_process";
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

function rebuildImage() {
  const exe = path.join(root, "rust", "target", "debug", "mapexport");
  if (!fs.existsSync(exe)) {
    console.error(`hot-sync: mapexport is not built (${exe}); the map image was not rebuilt`);
    return;
  }
  const scenario = path.join(target, "scenarios", "moba");
  const r = spawnSync(exe, [scenario, path.join(scenario, "moba.osdt")], { encoding: "utf8" });
  if (r.status !== 0) console.error(`hot-sync: mapexport FAIL: ${(r.stderr || "").trim()}`);
  else console.log(`hot-sync: map image rebuilt (${(r.stdout || "").trim()})`);
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
  if (files.some((rel) => rel.startsWith("scenarios/moba/") && /\.(osd|tsv|json|txt)$/.test(rel))) rebuildImage();
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
