import fs from "node:fs";
import os from "node:os";
import path from "node:path";

export function isolateSnapshot(root) {
  if (process.env.CROSS_CONTRACT_SNAPSHOT) return process.env.CROSS_CONTRACT_SNAPSHOT;
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "oscraft-snapshot-"));
  const file = path.join(dir, "cross-contract-snapshot.json");
  fs.copyFileSync(path.join(root, "scenarios", "bota", "cross-contract-snapshot.json"), file);
  process.env.CROSS_CONTRACT_SNAPSHOT = file;
  process.on("exit", () => fs.rmSync(dir, { recursive: true, force: true }));
  return file;
}
