#!/usr/bin/env node
// Keeps play.html's modulepreload hints equal to the module graph the page actually fetches.
//
// The page is buildless, so nothing generates these hints at serve time: they are committed in
// play.html. Without them the browser walks the graph one level per round trip and reaches the
// sim wasm last. A hint that names a module no longer imported is a wasted request, and a module
// that is imported but not listed silently loses the hint, so this script compares the two sets.
//
// Usage:
//   node scripts/check-modulepreload.mjs           # exit 1 and print the difference
//   node scripts/check-modulepreload.mjs --fix     # rewrite play.html with the current graph
//
// Only static imports are followed. A dynamic import() is a deliberate deferral, so preloading it
// would pull the bytes back onto the critical path.

import fs from "node:fs";
import path from "node:path";

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), "..");
const clientDir = path.join(root, "js", "client");
const pagePath = path.join(clientDir, "play.html");

const html = fs.readFileSync(pagePath, "utf8");

// The entries are the module scripts the document itself loads; the browser fetches these as it
// parses, so they are never hinted.
const entries = [...html.matchAll(/<script\b[^>]*\btype\s*=\s*"module"[^>]*\bsrc\s*=\s*"([^"]+)"/g)]
  .map((m) => m[1])
  .filter((src) => src.startsWith("./") || !src.startsWith("/"))
  .map((src) => src.replace(/^\.\//, ""));

const hinted = [...html.matchAll(/<link\b[^>]*\brel\s*=\s*"modulepreload"[^>]*\bhref\s*=\s*"([^"]+)"/g)]
  .map((m) => m[1].replace(/^\.\//, ""));

// A static import or re-export: `import ... from "x"`, `import "x"`, `export ... from "x"`.
const STATIC = /^\s*(?:import|export)\s(?:[^"']*?\bfrom\s*)?["']([^"']+)["']/gm;

const reachable = new Set();
const walk = (file) => {
  if (reachable.has(file)) return;
  reachable.add(file);
  let src;
  try {
    src = fs.readFileSync(path.join(clientDir, file), "utf8");
  } catch {
    return; // a path the loader resolves elsewhere (bare specifier, vendor file); not hinted here
  }
  STATIC.lastIndex = 0;
  let m;
  while ((m = STATIC.exec(src))) {
    const spec = m[1];
    if (!spec.startsWith(".")) continue; // bare specifiers resolve through the import map and the CDN
    const rel = path.normalize(path.join(path.dirname(file), spec)).split(path.sep).join("/");
    if (rel.startsWith("..")) continue;
    walk(rel);
  }
};
for (const entry of entries) walk(entry);
for (const entry of entries) reachable.delete(entry);

const listed = [...reachable].sort();
const fix = process.argv.includes("--fix");

const missing = listed.filter((f) => !hinted.includes(f));
const stale = hinted.filter((f) => !listed.includes(f));

if (missing.length === 0 && stale.length === 0) {
  console.log(`modulepreload ok entries=${entries.length} hinted=${hinted.length}`);
  process.exit(0);
}

const show = (label, files) => `${label}:\n${files.map((f) => `  ${f}`).join("\n")}`;
console.error(`modulepreload drift (${missing.length} missing, ${stale.length} stale)`);
if (missing.length) console.error(show("imported but not hinted", missing));
if (stale.length) console.error(show("hinted but not imported", stale));

if (!fix) {
  console.error("\nrun with --fix to rewrite js/client/play.html");
  process.exit(1);
}

// Replace the contiguous run of modulepreload links with the current graph, or insert one.
const lines = listed.map((f) => `  <link rel="modulepreload" href="./${f}">`).join("\n");
const block = new RegExp(
  `(?:^[ \\t]*<link\\b[^>]*\\brel\\s*=\\s*"modulepreload"[^>]*>\\n)+`,
  "m",
);
const next = block.test(html)
  ? html.replace(block, `${lines}\n`)
  : html.replace(/([ \t]*)<link rel="preload" href="\.\/pkg\/web_bg\.wasm"/, `${lines}\n$1<link rel="preload" href="./pkg/web_bg.wasm"`);
fs.writeFileSync(pagePath, next);
console.log(`fixed: ${listed.length} hints written to js/client/play.html`);
