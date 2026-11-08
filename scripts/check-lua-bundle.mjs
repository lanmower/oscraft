import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const moba = resolve(root, "scenarios/moba");
const manifest = JSON.parse(readFileSync(join(moba, "manifest.json"), "utf8"));
const rels = [...new Set([...Object.values(manifest.modules ?? {}), ...(manifest.mods ?? [])])];
const readRel = (rel) => readFileSync(join(moba, rel), "utf8");

const dir = mkdtempSync(join(tmpdir(), "lua-bundle-check-"));
try {
  const out = join(dir, "moba.modules.json");
  execFileSync(process.execPath, [resolve(root, "scripts/bundle-lua.mjs"), moba, out], { encoding: "utf8" });
  const bundle = JSON.parse(readFileSync(out, "utf8"));
  const differ = rels.filter((rel) => bundle[rel] !== readRel(rel));
  if (differ.length) throw new Error("lua bundle bodies differ from their files: " + differ.slice(0, 6).join(", "));

  const { LUA_BUNDLE, moduleClosure, moduleTextReader } = await import("../js/client/lua_closure.js");
  const pages = readFileSync(join(root, ".github/workflows/pages.yml"), "utf8");
  if (!pages.includes("_site/scenarios/moba/" + LUA_BUNDLE)) throw new Error("pages.yml does not write the bundle the client reads (" + LUA_BUNDLE + ")");

  let perFile = 0;
  const fileReader = async (rel) => { perFile += 1; return readRel(rel); };
  const expected = await moduleClosure(manifest, fileReader);
  let fallbacks = 0;
  const bundleReader = moduleTextReader(bundle, async (rel) => { fallbacks += 1; return readRel(rel); });
  const closure = await moduleClosure(manifest, bundleReader);
  if (JSON.stringify(closure) !== JSON.stringify(expected)) throw new Error("the bundle closure differs from the per-file closure");
  if (fallbacks !== 0) throw new Error("the bundle path read " + fallbacks + " modules outside the bundle");

  const stub = { ...bundle, [expected[0].name]: "return {}" };
  const stubbed = await moduleClosure(manifest, moduleTextReader(stub, fileReader));
  if (JSON.stringify(stubbed) === JSON.stringify(expected)) throw new Error("the mutation case did not change the closure");

  console.log("lua bundle ok: " + rels.length + " modules in the bundle; boot closure " + expected.length + " modules, " + perFile + " per-file reads when unbundled, 0 when bundled; mutation case caught");
} finally {
  rmSync(dir, { recursive: true, force: true });
}
