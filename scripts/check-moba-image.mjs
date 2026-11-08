import { execFileSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const rust = path.join(root, "rust");
const pack = path.join(root, "scenarios", "moba");
const committed = path.join(pack, "moba.osdt");
const exe = path.join(rust, "target", "debug", process.platform === "win32" ? "mapexport.exe" : "mapexport");
const refresh = "cd rust && cargo run -p mapexport -- ../scenarios/moba ../scenarios/moba/moba.osdt";

const sha = (bytes) => crypto.createHash("sha256").update(bytes).digest("hex");
const same = (a, b) => sha(a) === sha(b);

execFileSync("cargo", ["build", "-p", "mapexport"], { cwd: rust, stdio: "inherit" });
const dir = fs.mkdtempSync(path.join(os.tmpdir(), "moba-image-"));
const out = path.join(dir, "moba.osdt");
try {
  execFileSync(exe, [pack, out], { stdio: "inherit" });
  const rebuilt = fs.readFileSync(out);
  const stored = fs.readFileSync(committed);

  const tampered = Buffer.from(rebuilt);
  tampered[tampered.length >> 1] ^= 1;
  if (same(tampered, rebuilt)) throw new Error("tamper control: a changed byte was not detected");

  if (!same(rebuilt, stored)) {
    console.error(`moba image stale: committed ${stored.length} bytes sha256 ${sha(stored)}, rebuilt ${rebuilt.length} bytes sha256 ${sha(rebuilt)}`);
    console.error(`refresh the committed image: ${refresh}`);
    process.exit(1);
  }
  console.log(`moba image ok: ${rebuilt.length} bytes sha256 ${sha(rebuilt)}; tamper control caught`);
} finally {
  fs.rmSync(dir, { recursive: true, force: true });
}
