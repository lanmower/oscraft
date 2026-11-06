import { execFileSync } from "node:child_process";
import { readFileSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const manifest = JSON.parse(readFileSync(resolve(root, "scenarios/moba/manifest.json"), "utf8"));
const stems = Object.keys(manifest.modules ?? {});
const dir = mkdtempSync(join(tmpdir(), "lua-bundle-check-"));
try {
  const out = join(dir, "moba.modules.lua");
  execFileSync(process.execPath, [resolve(root, "scripts/bundle-lua.mjs"), resolve(root, "scenarios/moba"), out], { encoding: "utf8" });
  const text = readFileSync(out, "utf8");
  const missing = stems.filter((stem) => !text.includes("package.preload[" + JSON.stringify(stem) + "]"));
  if (missing.length) throw new Error("Lua bundle omits " + missing.length + " modules: " + missing.slice(0, 8).join(", "));
  console.log("lua bundle ok " + stems.length + " module stems preloaded");
} finally {
  rmSync(dir, { recursive: true, force: true });
}
