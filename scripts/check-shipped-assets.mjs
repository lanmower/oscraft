import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, readdirSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, join, posix, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const fail = (what) => { throw new Error(what); };
const mb = (bytes) => `${(bytes / 1048576).toFixed(1)} MB`;

const PAGE = "js/client/play.html";
const PAGE_ROOTS = ["index.html", "version.json", "js/client/version.json", PAGE, "js/client/pkg/web.js", "js/client/pkg/web_bg.wasm"];
const MANIFEST = "scenarios/moba/manifest.json";
const SCENARIO = "scenarios/moba";
const ASSET_ROOT = "scenarios/moba/assets";
const LOADER_ASSET_KEYS = ["props", "propSizes", "ktx2", "anim"];
const CONTROL_TARGETS = ["js/client/pack_load.js", "scenarios/moba/attr_rules.txt"];
const TEXT = /\.(m?js|html|json|lua|osd|tsv|txt)$/i;
const JS_SPECIFIER = [
  /\bfrom\s*["']([^"'\n]+)["']/g,
  /\bimport\s*["']([^"'\n]+)["']/g,
  /\bimport\(\s*["']([^"'\n]+)["']\s*\)/g,
  /new URL\(\s*["']([^"'\n]+)["']\s*,\s*import\.meta\.url\s*\)/g,
];
const REQUIRE = /require\("([^"]+)"\)/g;
const LITERAL = /"([^"\n]{1,260})"|'([^'\n]{1,260})'/g;
const HTML_ATTR = /\b(?:src|href)="([^"]+)"/g;

function walk(base, rel = "") {
  return readdirSync(join(base, rel), { withFileTypes: true }).flatMap((entry) => {
    const next = rel ? `${rel}/${entry.name}` : entry.name;
    return entry.isDirectory() ? walk(base, next) : [next];
  });
}

function openSite(base) {
  const sizes = new Map(walk(base).sort().map((rel) => [rel, statSync(join(base, rel)).size]));
  return {
    files: [...sizes.keys()],
    has: (rel) => sizes.has(rel),
    size: (rel) => sizes.get(rel),
    read: (rel) => readFileSync(join(base, rel), "utf8"),
    bytes: (rel) => readFileSync(join(base, rel)),
  };
}

function hiding(site, rel) {
  return {
    files: site.files.filter((f) => f !== rel),
    has: (f) => f !== rel && site.has(f),
    size: site.size,
    read: site.read,
    bytes: site.bytes,
  };
}

function importMapOf(html) {
  const block = /<script type="importmap">([\s\S]*?)<\/script>/.exec(html);
  return block ? JSON.parse(block[1]).imports ?? {} : {};
}

function resolveSpecifier(spec, from, importMap) {
  if (spec.startsWith(".") || spec.startsWith("/")) return posix.normalize(posix.join(posix.dirname(from), spec)).replace(/^\//, "");
  for (const [key, target] of Object.entries(importMap)) {
    if (key === spec) return posix.normalize(posix.join("js/client", target));
    if (key.endsWith("/") && spec.startsWith(key)) return posix.normalize(posix.join("js/client", target, spec.slice(key.length)));
  }
  return null;
}

function literalBases(from) {
  return [posix.dirname(from), "js/client", SCENARIO, ASSET_ROOT, ""];
}

export function analyse(site) {
  const referenced = new Map();
  const missing = new Map();
  const queue = [];
  const directories = new Set();
  for (const rel of site.files) {
    for (let at = rel.indexOf("/"); at >= 0; at = rel.indexOf("/", at + 1)) directories.add(rel.slice(0, at));
  }
  const want = (raw, why, strict) => {
    const rel = posix.normalize(raw).replace(/\/+$/, "");
    if (!rel || rel === "." || referenced.has(rel)) return;
    if (site.has(rel)) {
      referenced.set(rel, why);
      queue.push(rel);
      return;
    }
    if (directories.has(rel)) {
      if (!strict) return;
      for (const file of site.files) if (file.startsWith(`${rel}/`)) want(file, why, false);
      return;
    }
    if (strict) missing.set(rel, why);
  };

  for (const rel of PAGE_ROOTS) want(rel, "page root", true);
  const html = site.has(PAGE) ? site.read(PAGE) : "";
  const importMap = importMapOf(html);
  for (const [, value] of html.matchAll(HTML_ATTR)) {
    if (value.includes(":") || value.startsWith("#")) continue;
    want(posix.normalize(posix.join(posix.dirname(PAGE), value)), `${PAGE} src or href`, true);
  }
  for (const target of Object.values(importMap)) want(posix.normalize(posix.join("js/client", target)), "importmap target", true);

  const manifest = site.has(MANIFEST) ? JSON.parse(site.read(MANIFEST)) : null;
  if (!manifest) fail(`site has no ${MANIFEST}`);
  want(MANIFEST, "manifest", true);
  for (const key of ["rules", "ui", "image"]) if (typeof manifest[key] === "string") want(`${SCENARIO}/${manifest[key]}`, `manifest.${key}`, true);

  const modules = manifest.modules ?? {};
  const visited = new Set();
  const visitModule = (file) => {
    if (visited.has(file)) return;
    visited.add(file);
    want(file, "manifest mods closure", true);
    if (!site.has(file)) return;
    for (const [, stem] of site.read(file).matchAll(REQUIRE)) if (modules[stem] !== undefined) visitModule(`${SCENARIO}/${modules[stem]}`);
  };
  for (const mod of manifest.mods ?? []) visitModule(`${SCENARIO}/${mod}`);

  const assets = manifest.assets ?? {};
  for (const key of LOADER_ASSET_KEYS) if (typeof assets[key] === "string") want(`${ASSET_ROOT}/${assets[key]}`, `manifest.assets.${key}`, true);
  for (const value of Object.values(assets.textures ?? {})) want(`${ASSET_ROOT}/${value}`, "manifest.assets.textures", true);
  const animMaps = `${ASSET_ROOT}/${assets.animMaps ?? "anim/maps"}/`;
  for (const rel of site.files) if (rel.startsWith(animMaps)) want(rel, "manifest.assets.animMaps", false);

  const image = site.has(`${SCENARIO}/${manifest.image}`) ? site.bytes(`${SCENARIO}/${manifest.image}`) : Buffer.alloc(0);
  const modelPrefix = `${ASSET_ROOT}/${assets.models ?? "models"}/`;
  for (const rel of site.files) {
    if (rel.startsWith(modelPrefix) && rel.endsWith(".glb") && image.indexOf(basename(rel, ".glb")) >= 0) want(rel, "model stem named in moba image", false);
  }

  let unresolvedBare = 0;
  while (queue.length > 0) {
    const from = queue.shift();
    if (from === MANIFEST || !TEXT.test(from)) continue;
    const text = site.read(from);
    if (/\.(m?js)$/i.test(from)) {
      for (const pattern of JS_SPECIFIER) {
        for (const [, spec] of text.matchAll(pattern)) {
          const target = resolveSpecifier(spec, from, importMap);
          if (target) want(target, `import in ${from}`, true);
          else if (!spec.startsWith(".") && !spec.startsWith("/")) unresolvedBare += 1;
        }
      }
    }
    for (const match of text.matchAll(LITERAL)) {
      const lit = match[1] ?? match[2];
      if (!lit || /\s/.test(lit) || lit.includes("://") || lit.startsWith("data:")) continue;
      for (const base of literalBases(from)) want(posix.normalize(posix.join(base, lit)), `literal in ${from}`, false);
    }
  }

  return { referenced, missing: [...missing].map(([rel, why]) => `${rel} (${why})`), unresolvedBare };
}

function report(site, result) {
  const unreferenced = site.files.filter((rel) => !result.referenced.has(rel));
  const sum = (list) => list.reduce((n, rel) => n + site.size(rel), 0);
  const refBytes = sum([...result.referenced.keys()]);
  const unrefBytes = sum(unreferenced);
  const totalBytes = refBytes + unrefBytes;
  const groups = new Map();
  for (const rel of unreferenced) {
    const key = rel.split("/").slice(0, 2).join("/");
    groups.set(key, (groups.get(key) ?? 0) + site.size(rel));
  }
  const top = [...groups].sort((a, b) => b[1] - a[1]).slice(0, 8).map(([k, v]) => `${k} ${mb(v)}`);
  console.log(`shipped assets: staged=${site.files.length} files ${mb(totalBytes)}; static first-draw reference set (upper bound)=${result.referenced.size} files ${mb(refBytes)}; not statically referenced=${unreferenced.length} files ${mb(unrefBytes)} (${((100 * unrefBytes) / Math.max(totalBytes, 1)).toFixed(1)}%); unresolved bare imports=${result.unresolvedBare}; missing=${result.missing.length}`);
  console.log(`unreferenced by top directory: ${top.join("; ")}`);
  return { unreferenced, unrefBytes };
}

function stage() {
  const dir = mkdtempSync(join(tmpdir(), "shipped-assets-"));
  const out = join(dir, "site");
  const run = spawnSync("bash", [join(root, "scripts/layout-site.sh"), out, "check", "check"], { cwd: root, encoding: "utf8", env: { ...process.env, SKIP_PACKS: "1" }, maxBuffer: 1 << 26 });
  if (run.status !== 0) {
    rmSync(dir, { recursive: true, force: true });
    fail(`layout-site.sh failed: ${run.stderr || run.stdout}`);
  }
  return { dir, out };
}

function main() {
  const given = process.argv.slice(2).find((arg) => !arg.startsWith("--"));
  const staged = given ? null : stage();
  try {
    const site = openSite(given ?? staged.out);
    const result = analyse(site);
    report(site, result);
    if (result.missing.length > 0) fail(`referenced assets missing from the staged site: ${result.missing.join("; ")}`);
    for (const target of CONTROL_TARGETS) {
      if (!result.referenced.has(target)) fail(`negative control target ${target} is not in the first-draw set`);
      const control = analyse(hiding(site, target));
      if (!control.missing.some((entry) => entry.startsWith(`${target} (`))) fail(`negative control: hiding ${target} did not report it missing`);
      console.log(`negative control: hiding ${target} reports missing (${control.missing.length} missing)`);
    }
    console.log("shipped assets ok (report only; no staged file is removed)");
  } finally {
    if (staged) rmSync(staged.dir, { recursive: true, force: true });
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) main();
