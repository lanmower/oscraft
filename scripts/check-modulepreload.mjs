#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const clientDir = path.join(root, "js", "client");
const pagePath = path.join(clientDir, "play.html");

const html = fs.readFileSync(pagePath, "utf8");

const entries = [...html.matchAll(/<script\b[^>]*\btype\s*=\s*"module"[^>]*\bsrc\s*=\s*"([^"]+)"/g)]
  .map((m) => m[1])
  .filter((src) => src.startsWith("./") || !src.startsWith("/"))
  .map((src) => src.replace(/^\.\//, ""));

const hinted = [...html.matchAll(/<link\b[^>]*\brel\s*=\s*"modulepreload"[^>]*\bhref\s*=\s*"([^"]+)"/g)]
  .map((m) => m[1].replace(/^\.\//, ""));

const importMap = JSON.parse(
  html.match(/<script\b[^>]*\btype\s*=\s*"importmap"[^>]*>([\s\S]*?)<\/script>/)?.[1] ?? '{"imports":{}}',
).imports ?? {};

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
    return;
  }
  for (const re of [FROM, BARE_IMPORT]) {
    re.lastIndex = 0;
    let m;
    while ((m = re.exec(src))) {
      const spec = m[1];
      if (!spec.startsWith(".")) { bare.add(spec); continue; }
      const rel = path.normalize(path.join(path.dirname(file), spec)).split(path.sep).join("/");
      if (rel.startsWith("..")) continue;
      walk(rel);
    }
  }
};
for (const entry of entries) walk(entry);
for (const entry of entries) reachable.delete(entry);

const mapped = [...bare].map(resolveSpec).filter((url) => typeof url === "string");
const localMapped = mapped.filter((url) => url.startsWith("./")).map((url) => url.replace(/^\.\//, ""));
for (const entry of localMapped) walk(entry);
for (const entry of entries) reachable.delete(entry);

const listed = [...reachable].sort();
const fix = process.argv.includes("--fix");

const remoteUrls = mapped.filter((url) => /^https?:\/\//.test(url));
const remoteOrigins = [...new Set(remoteUrls.map((url) => new URL(url).origin))].sort();

const missing = listed.filter((f) => !hinted.includes(f));
const stale = hinted.filter((f) => !listed.includes(f) && !remoteUrls.includes(f));
const missingRemote = remoteUrls.filter((u) => !hinted.includes(u));
const missingOrigins = remoteOrigins.filter((o) => !preconnected.includes(o));
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

const remoteLines = [
  ...remoteOrigins.map((o) => `  <link rel="preconnect" href="${o}" crossorigin>`),
  ...remoteUrls.map((u) => `  <link rel="modulepreload" href="${u}">`),
];
const lines = [...remoteLines, ...listed.map((f) => `  <link rel="modulepreload" href="./${f}">`)].join("\n");
const block = new RegExp(
  `(?:^[ \\t]*<link\\b[^>]*\\brel\\s*=\\s*"(?:modulepreload|preconnect)"[^>]*>\\n)+`,
  "m",
);
const next = block.test(html)
  ? html.replace(block, `${lines}\n`)
  : html.replace(/([ \t]*)<link rel="preload" href="\.\/pkg\/web_bg\.wasm"/, `${lines}\n$1<link rel="preload" href="./pkg/web_bg.wasm"`);
fs.writeFileSync(pagePath, next);
console.log(`fixed: ${listed.length} local and ${remoteUrls.length} remote hints written to js/client/play.html`);
