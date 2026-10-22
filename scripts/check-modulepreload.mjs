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
//
// Bare specifiers (nostr-tools, xstate) resolve through the import map to a CDN, a different origin
// the browser cannot reach until it has parsed play.js: a cross-origin DNS lookup, TCP and TLS
// handshake, and then the download. So they are checked two ways: a modulepreload for each resolved
// URL, so the download starts with the document's first byte, and a preconnect to each origin, so the
// handshake overlaps the local graph instead of following it.

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

// The import map: bare specifier -> URL, and a key ending in "/" is a prefix mapping that maps the
// rest of the specifier onto it. Read from the page, so a version bump there is picked up here.
const importMap = JSON.parse(
  html.match(/<script\b[^>]*\btype\s*=\s*"importmap"[^>]*>([\s\S]*?)<\/script>/)?.[1] ?? '{"imports":{}}',
).imports ?? {};

// Resolves a bare specifier the way the browser does: an exact key first, then the longest prefix key.
const resolveSpec = (spec) => {
  if (typeof importMap[spec] === "string") return importMap[spec];
  const prefixes = Object.keys(importMap).filter((k) => k.endsWith("/") && spec.startsWith(k));
  if (prefixes.length === 0) return null;
  const key = prefixes.sort((a, b) => b.length - a.length)[0];
  const tail = importMap[key];
  if (!/^(\.{1,2}\/|https?:\/\/)/.test(tail)) return null;
  return tail + spec.slice(key.length);
};

const preconnected = [...html.matchAll(/<link\b[^>]*\brel\s*=\s*"preconnect"[^>]*\bhref\s*=\s*"([^"]+)"/g)]
  .map((m) => m[1]);

// A static import or re-export, in source or in a bundled module. Bundles are one long line with no
// space after `import`, so the two shapes are matched anywhere rather than anchored at a line start:
// `... from "x"` (import, export, and `export ... from`) and `import "x"`. A dynamic import() is
// neither, which is what makes it a deliberate deferral.
const FROM = /\bfrom\s*["']([^"']+)["']/g;
const BARE_IMPORT = /\bimport\s*["']([^"']+)["']/g;

const reachable = new Set();
const bare = new Set();
const walk = (file) => {
  if (reachable.has(file)) return;
  reachable.add(file);
  let src;
  try {
    src = fs.readFileSync(path.join(clientDir, file), "utf8");
  } catch {
    return; // a path the loader resolves elsewhere (bare specifier, vendor file); not hinted here
  }
  for (const re of [FROM, BARE_IMPORT]) {
    re.lastIndex = 0;
    let m;
    while ((m = re.exec(src))) {
      const spec = m[1];
      if (!spec.startsWith(".")) { bare.add(spec); continue; } // resolved through the import map below
      const rel = path.normalize(path.join(path.dirname(file), spec)).split(path.sep).join("/");
      if (rel.startsWith("..")) continue;
      walk(rel);
    }
  }
};
for (const entry of entries) walk(entry);
for (const entry of entries) reachable.delete(entry);

// Import-map entries that resolve to a file beside the page are walked into the graph too: the
// browser reaches them only after it has parsed play.js, so they need the same hint as the rest.
// A bare specifier with no import-map entry is not hinted: the page cannot load it anyway, and
// naming it here would be a guess.
const mapped = [...bare].map(resolveSpec).filter((url) => typeof url === "string");
const localMapped = mapped.filter((url) => url.startsWith("./")).map((url) => url.replace(/^\.\//, ""));
for (const entry of localMapped) walk(entry);
for (const entry of entries) reachable.delete(entry);

const listed = [...reachable].sort();
const fix = process.argv.includes("--fix");

// Cross-origin modules and the origins they need: a second origin costs a DNS lookup and a TLS
// handshake before the download, so each wants a preconnect as well as a modulepreload.
const remoteUrls = mapped.filter((url) => /^https?:\/\//.test(url));
const remoteOrigins = [...new Set(remoteUrls.map((url) => new URL(url).origin))].sort();

const missing = listed.filter((f) => !hinted.includes(f));
// A hinted URL the graph no longer imports, excluding the remote ones counted separately above.
const stale = hinted.filter((f) => !listed.includes(f) && !remoteUrls.includes(f));
const missingRemote = remoteUrls.filter((u) => !hinted.includes(u));
const missingOrigins = remoteOrigins.filter((o) => !preconnected.includes(o));
// A preconnect to an origin nothing is fetched from any more: it opens a socket the page then wastes.
const staleOrigins = preconnected.filter((o) => !remoteOrigins.includes(o));

const clean =
  missing.length === 0 && stale.length === 0 && missingRemote.length === 0 &&
  missingOrigins.length === 0 && staleOrigins.length === 0;
if (clean) {
  console.log(
    `modulepreload ok entries=${entries.length} hinted=${hinted.length}` +
      ` remote=${remoteUrls.length} origins=${remoteOrigins.length}`,
  );
  process.exit(0);
}

const show = (label, files) => `${label}:\n${files.map((f) => `  ${f}`).join("\n")}`;
console.error(`modulepreload drift (${missing.length} missing, ${stale.length} stale, ${missingRemote.length} remote, ${missingOrigins.length} origins, ${staleOrigins.length} stale origins)`);
if (missing.length) console.error(show("imported but not hinted", missing));
if (stale.length) console.error(show("hinted but not imported", stale));
if (missingRemote.length) console.error(show("import-map module not preloaded", missingRemote));
if (missingOrigins.length) console.error(show("origin not preconnected", missingOrigins));
if (staleOrigins.length) console.error(show("preconnected but nothing is fetched from it", staleOrigins));

if (!fix) {
  console.error("\nrun with --fix to rewrite js/client/play.html");
  process.exit(1);
}

// Remote hints go first: the origin handshake is the longest lead time on the page, so it has to start
// before the local graph is written into the buffer.
const remoteLines = [
  ...remoteOrigins.map((o) => `  <link rel="preconnect" href="${o}" crossorigin>`),
  ...remoteUrls.map((u) => `  <link rel="modulepreload" href="${u}">`),
];
const lines = [...remoteLines, ...listed.map((f) => `  <link rel="modulepreload" href="./${f}">`)].join("\n");
// Replace the contiguous run of preconnect and modulepreload links with the current graph, or insert one.
const block = new RegExp(
  `(?:^[ \\t]*<link\\b[^>]*\\brel\\s*=\\s*"(?:modulepreload|preconnect)"[^>]*>\\n)+`,
  "m",
);
const next = block.test(html)
  ? html.replace(block, `${lines}\n`)
  : html.replace(/([ \t]*)<link rel="preload" href="\.\/pkg\/web_bg\.wasm"/, `${lines}\n$1<link rel="preload" href="./pkg/web_bg.wasm"`);
fs.writeFileSync(pagePath, next);
console.log(`fixed: ${listed.length} local and ${remoteUrls.length} remote hints written to js/client/play.html`);
