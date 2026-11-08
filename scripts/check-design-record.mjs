import { existsSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const fail = (what) => { throw new Error(what); };
const RECORD = /^- Current tree figures: gates=(\d+), gate path refs=(\d+) with (\d+) missing, camera writes=(\d+) with (\d+) unclassified\.$/m;

export function parseRecord(text) {
  const m = RECORD.exec(text);
  if (!m) return null;
  return { gates: Number(m[1]), refs: Number(m[2]), refsMissing: Number(m[3]), writes: Number(m[4]), unclassified: Number(m[5]) };
}

export function actualFigures() {
  const list = spawnSync(process.execPath, ["scripts/check-all.mjs", "--list"], { cwd: root, encoding: "utf8", env: { ...process.env, G688_LNI: "" } });
  const gates = Number(/LIST gates=(\d+)/.exec(list.stdout)?.[1]);
  const checkAll = readFileSync(resolve(root, "scripts/check-all.mjs"), "utf8");
  const ids = [...checkAll.matchAll(/"((?:js|scripts|scenarios)\/[^"]+\.mjs)"/g)].map((m) => m[1]);
  const refsMissing = ids.filter((f) => !existsSync(resolve(root, f))).length;
  const audit = spawnSync(process.execPath, ["js/client/camera-target-audit-check.mjs"], { cwd: root, encoding: "utf8" });
  const writes = Number(/camera target audit ok: (\d+) writes/.exec(audit.stdout)?.[1]);
  const unclassified = audit.status === 0 ? 0 : Number.NaN;
  return { gates, refs: ids.length, refsMissing, writes, unclassified };
}

export function drift(record, actual) {
  const out = [];
  for (const key of ["gates", "refs", "refsMissing", "writes", "unclassified"]) {
    if (record[key] !== actual[key]) out.push(`${key} record=${record[key]} tree=${actual[key]}`);
  }
  return out;
}

const design = readFileSync(resolve(root, "DESIGN.md"), "utf8");
const record = parseRecord(design);
if (!record) fail("DESIGN.md has no \"Current tree figures\" line in the final four-repo record");
const actual = actualFigures();
const found = drift(record, actual);
if (found.length > 0) fail(`DESIGN.md four-repo figures drifted from the tree: ${found.join("; ")}`);

const control = parseRecord(design.replace(/gates=(\d+), gate path refs/, (_, n) => `gates=${Number(n) + 1}, gate path refs`));
if (!control || drift(control, actual).length === 0) fail("negative control: a drifted gate count did not fail the record check");

console.log(`design record ok: gates=${actual.gates}, gate path refs=${actual.refs} with ${actual.refsMissing} missing, camera writes=${actual.writes} with ${actual.unclassified} unclassified (matches DESIGN.md)`);
console.log(`negative control: a gate count one higher fails with "${drift(control, actual)[0]}"`);
