import { execFileSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const rust = path.join(root, "rust");
const exe = path.join(rust, "target", "debug", process.platform === "win32" ? "mapexport.exe" : "mapexport");
const packs = ["moba", "moba-acronyms", "bota"];

const sha = (bytes) => crypto.createHash("sha256").update(bytes).digest("hex");
const same = (a, b) => sha(a) === sha(b);

execFileSync("cargo", ["build", "-p", "mapexport"], { cwd: rust, stdio: "inherit" });
const dir = fs.mkdtempSync(path.join(os.tmpdir(), "pack-image-"));
try {
  const holders = new Map();
  let failures = 0;
  for (const [index, name] of packs.entries()) {
    const packDir = path.join(root, "scenarios", name);
    const image = JSON.parse(fs.readFileSync(path.join(packDir, "manifest.json"), "utf8")).image;
    const committed = path.join(packDir, image);
    const out = path.join(dir, name + ".osdt");
    execFileSync(exe, [packDir, out], { stdio: "inherit" });
    const rebuilt = fs.readFileSync(out);
    const stored = fs.readFileSync(committed);
    if (index === 0) {
      const tampered = Buffer.from(rebuilt);
      tampered[tampered.length >> 1] ^= 1;
      if (same(tampered, rebuilt)) throw new Error("tamper control: a changed byte was not detected");
    }
    const holder = holders.get(sha(stored));
    if (holder !== undefined) {
      console.error(name + " image is byte-identical to " + holder + ": an image copied from another pack");
      failures += 1;
    }
    holders.set(sha(stored), name);
    if (!same(rebuilt, stored)) {
      console.error(name + " image stale: committed " + stored.length + " bytes sha256 " + sha(stored) + ", rebuilt " + rebuilt.length + " bytes sha256 " + sha(rebuilt));
      console.error("refresh: cd rust && cargo run -p mapexport -- ../scenarios/" + name + " ../scenarios/" + name + "/" + image);
      failures += 1;
    } else {
      console.log(name + " image ok: " + rebuilt.length + " bytes sha256 " + sha(rebuilt));
    }
  }
  if (failures > 0) process.exit(1);
  console.log("pack images ok: " + packs.length + " packs; tamper control caught");
} finally {
  fs.rmSync(dir, { recursive: true, force: true });
}
