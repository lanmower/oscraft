import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const moba = resolve(root, "scenarios/moba");
const modules = JSON.parse(readFileSync(join(moba, "manifest.json"), "utf8")).modules ?? {};

export function bundleProblems(text) {
  const problems = [];
  for (const [stem, rel] of Object.entries(modules)) {
    if (!text.includes("package.preload[" + JSON.stringify(stem) + "]")) problems.push("missing stem " + stem);
    const body = readFileSync(join(moba, rel), "utf8").trim();
    if (!text.includes(body)) problems.push("missing body " + stem);
  }
  return problems;
}

const dir = mkdtempSync(join(tmpdir(), "lua-bundle-check-"));
try {
  const out = join(dir, "moba.modules.lua");
  execFileSync(process.execPath, [resolve(root, "scripts/bundle-lua.mjs"), moba, out], { encoding: "utf8" });
  const text = readFileSync(out, "utf8");
  const problems = bundleProblems(text);
  if (problems.length) throw new Error("lua bundle: " + problems.slice(0, 6).join(", "));
  const stub = Object.entries(modules).map(([stem]) => "package.preload[" + JSON.stringify(stem) + "] = function(...) return {}\nend").join("\n");
  if (!bundleProblems(stub).length) throw new Error("the mutation case did not detect stub bodies");
  console.log("lua bundle ok " + Object.keys(modules).length + " stems, bodies verified, mutation case caught");
} finally {
  rmSync(dir, { recursive: true, force: true });
}
