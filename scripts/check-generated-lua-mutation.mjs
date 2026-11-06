import { execFileSync, spawnSync } from "node:child_process";
import { copyFileSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync, mkdirSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const moba = resolve(root, "scenarios/moba");
const tmp = mkdtempSync(join(tmpdir(), "generated-lua-mutation-"));
try {
  mkdirSync(join(tmp, "scripts"), { recursive: true });
  mkdirSync(join(tmp, "scenarios/moba"), { recursive: true });
  for (const f of readdirSync(resolve(root, "scripts"))) {
    if (f.endsWith(".mjs")) copyFileSync(resolve(root, "scripts", f), join(tmp, "scripts", f));
  }
  for (const f of readdirSync(moba)) {
    const src = join(moba, f);
    if (statSync(src).isFile() && /[.](lua|tsv|osd|json)$/.test(f)) copyFileSync(src, join(tmp, "scenarios/moba", f));
    if (statSync(src).isDirectory() && ["core", "shared", "tools"].includes(f)) execFileSync("cp", ["-a", src, join(tmp, "scenarios/moba", f)]);
  }
  const target = join(tmp, "scenarios/moba/ability_fx.lua");
  writeFileSync(target, readFileSync(target, "utf8").replace("dur = ", "dur = 999999 "));
  const r = spawnSync(process.execPath, ["scripts/check-generated-lua.mjs"], { cwd: tmp, encoding: "utf8", timeout: 120000 });
  if (r.status === 0) throw new Error("a mutated committed ability_fx.lua still passes the generated Lua gate");
  console.log("generated lua mutation ok (mutated output refused, exit " + r.status + ")");
} finally {
  rmSync(tmp, { recursive: true, force: true });
}
