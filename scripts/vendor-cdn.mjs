#!/usr/bin/env node
// Vendors the import map's remote modules into js/client/vendor, so the page fetches nothing from a
// second origin.
//
// Usage:
//   node scripts/vendor-cdn.mjs            # vendor every remote entry of play.html's import map
//   node scripts/vendor-cdn.mjs --dry-run  # report what would be fetched, write nothing
//
// play.html names nostr-tools and xstate at a CDN. That costs the page a DNS lookup and a TLS
// handshake to another host, and then one request per module the bundle imports: nostr-tools' ESM
// build pulls nine more (@noble/curves, @noble/hashes, @noble/ciphers, @scure/base), so the graph the
// browser has to unroll before it can run anything is ten cross-origin fetches, discovered one level
// at a time. It also makes first draw depend on a host this project does not control.
//
// This walks each bundle's own imports, fetches what they name, rewrites those specifiers to the
// vendored files, and points the import map at the local entry. The bytes are unchanged apart from
// the rewritten specifiers and one header line naming where they came from, so nothing is rebuilt or
// re-minified. Re-run it after a version bump in the import map; the previous files are replaced.
import fs from "node:fs";
import path from "node:path";

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), "..");
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

// A jsdelivr /npm/<pkg>@<version>/<subpath>/+esm path, and the bare /npm/... form a bundle imports.
const isRemote = (u) => typeof u === "string" && /^https?:\/\//.test(u);
const ORIGIN_OF = (u) => new URL(u).origin;

// One vendored file per remote module, named after the package, version and subpath it came from.
const localName = (url) => {
  const { pathname } = new URL(url);
  const m = pathname.match(/^\/npm\/(.+?)\/([^/]*)$/);
  const tail = m ? `${m[1]}` : pathname.replace(/^\/+/, "");
  const stem = tail.replace(/[@^~]/g, "-").replace(/[^A-Za-z0-9._-]+/g, "-").replace(/^-+|-+$/g, "");
  return `${stem || "module"}.js`;
};

// An entry is vendored again when --refresh is passed: the URL it came from is recorded in the header
// of the file already there, so a re-run repairs a bad rewrite without touching the import map.
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

const seen = new Map(); // remote URL -> { dir, file }
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
    // Rewrite each jsdelivr specifier this bundle imports to the vendored file beside it.
    const deps = [...src.matchAll(/((?:^|[\s,;{}()])(?:import|export)[\s\S]{0,400}?from\s*|import\s*)["'](\/npm\/[^"']+)["']/g)]
      .map((m) => m[2]);
    for (const dep of new Set(deps)) {
      const abs = new URL(dep, ORIGIN_OF(at)).href;
      if (!seen.has(abs)) queue.push({ url: abs });
    }
    src = src.replace(/(from\s*|import\s*)["'](\/npm\/[^"']+)["']/g, (all, head, dep) => {
      const abs = new URL(dep, ORIGIN_OF(at)).href;
      const target = seen.get(abs) ?? { file: localName(abs) };
      // A bare specifier resolves through the import map, and these chunks have no entry there, so
      // the rewritten specifier has to keep its relative form.
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
  // Two-space JSON indented to the <script> tag, which is how the block is written by hand.
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
