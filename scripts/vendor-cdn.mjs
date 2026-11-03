#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const clientDir = path.join(root, "js", "client");
const pagePath = path.join(clientDir, "play.html");
const vendorDir = path.join(clientDir, "vendor");

const dryRun = process.argv.includes("--dry-run");
const html = fs.readFileSync(pagePath, "utf8");
const mapMatch = html.match(/(<script\b[^>]*\btype\s*=\s*"importmap"[^>]*>)([\s\S]*?)(<\/script>)/);
if (!mapMatch) {
  console.error("play.html has no import map");
  process.exit(2);
}
const map = JSON.parse(mapMatch[2]).imports ?? {};

const isRemote = (u) => typeof u === "string" && /^https?:\/\//.test(u);
const ORIGIN_OF = (u) => new URL(u).origin;

const localName = (url) => {
  const { pathname } = new URL(url);
  const m = pathname.match(/^\/npm\/(.+?)\/([^/]*)$/);
  const tail = m ? `${m[1]}` : pathname.replace(/^\/+/, "");
  const stem = tail.replace(/[@^~]/g, "-").replace(/[^A-Za-z0-9._-]+/g, "-").replace(/^-+|-+$/g, "");
  return `${stem || "module"}.js`;
};

const refresh = process.argv.includes("--refresh");
const fetches = [];
for (const [name, url] of Object.entries(map)) {
  if (isRemote(url)) { fetches.push({ name, url }); continue; }
  if (!refresh) continue;
  const local = path.join(clientDir, url.replace(/^\.\//, ""));
  if (!fs.existsSync(local)) continue;
  const recorded = fs.readFileSync(local, "utf8").match(/^\/\/ Vendored from (\S+) by /m);
  if (!recorded) continue;
  fetches.push({ name, url: recorded[1] });
}
if (fetches.length === 0) {
  console.log("nothing remote in the import map");
  process.exit(0);
}

const seen = new Map();
const wrote = [];

for (const { name, url } of fetches) {
  const dir = path.join(vendorDir, name.replace(/[^A-Za-z0-9._-]+/g, "-"));
  const queue = [{ url, entry: true }];
  while (queue.length) {
    const { url: at, entry } = queue.shift();
    if (seen.has(at)) {
      if (entry) seen.get(at).entry = true;
      continue;
    }
    const file = entry ? "index.js" : localName(at);
    seen.set(at, { dir, file, entry });
    const res = await fetch(at);
    if (!res.ok) throw new Error(`${at}: HTTP ${res.status}`);
    let src = await res.text();
    const deps = [...src.matchAll(/((?:^|[\s,;{}()])(?:import|export)[\s\S]{0,400}?from\s*|import\s*)["'](\/npm\/[^"']+)["']/g)]
      .map((m) => m[2]);
    for (const dep of new Set(deps)) {
      const abs = new URL(dep, ORIGIN_OF(at)).href;
      if (!seen.has(abs)) queue.push({ url: abs });
    }
    src = src.replace(/(from\s*|import\s*)["'](\/npm\/[^"']+)["']/g, (all, head, dep) => {
      const abs = new URL(dep, ORIGIN_OF(at)).href;
      const target = seen.get(abs) ?? { file: localName(abs) };
      return `${head}"./${String(target.file).replace(/^\.\//, "")}"`;
    });
    const header =
      "// Vendored from " + at + " by scripts/vendor-cdn.mjs. Bytes as published, with each /npm/...\n" +
      "// specifier rewritten to the file beside this one. Re-run that script to refresh.\n";
    wrote.push({ dir, file, bytes: Buffer.byteLength(src), url: at });
    if (!dryRun) {
      fs.mkdirSync(dir, { recursive: true });
      fs.writeFileSync(path.join(dir, file), `${header}${src}`);
    }
  }
}

if (!dryRun) {
  const next = { ...map };
  for (const { name, url } of fetches) next[name] = `./vendor/${name.replace(/[^A-Za-z0-9._-]+/g, "-")}/index.js`;
  const body = JSON.stringify({ imports: next }, null, 2)
    .split("\n")
    .map((line, i) => (i === 0 ? line : `  ${line}`))
    .join("\n");
  fs.writeFileSync(pagePath, html.replace(mapMatch[2], `\n${body}\n  `));
}

const total = wrote.reduce((n, w) => n + w.bytes, 0);
for (const w of wrote) console.log(`${path.relative(root, path.join(w.dir, w.file))}: ${w.bytes} bytes <- ${w.url}`);
console.log(
  `\n${dryRun ? "would vendor" : "vendored"} ${wrote.length} module(s) for ${fetches.length} import-map entr(ies),` +
    ` ${(total / 1024).toFixed(1)} KiB total`,
);
