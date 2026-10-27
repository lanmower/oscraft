#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const flag = process.argv.indexOf("--root");
const root = flag > 0 ? resolve(process.argv[flag + 1]) : join(dirname(fileURLToPath(import.meta.url)), "..");
const scenarios = join(root, "scenarios");
const posix = (p) => p.split(sep).join("/");
const show = (p) => posix(relative(root, p));

function walk(dir) {
  const out = [];
  for (const name of readdirSync(dir)) {
    const full = join(dir, name);
    if (statSync(full).isDirectory()) out.push(...walk(full));
    else out.push(full);
  }
  return out;
}

function trackedOsd() {
  try {
    const out = execFileSync("git", ["ls-files", "-z", "--", "scenarios"], {
      cwd: root,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    });
    return out
      .split("\0")
      .filter((p) => p.endsWith(".osd"))
      .map((p) => posix(join(root, p)));
  } catch {
    return walk(scenarios)
      .filter((p) => p.endsWith(".osd"))
      .map(posix);
  }
}

const scenarioDirs = readdirSync(scenarios)
  .map((name) => join(scenarios, name))
  .filter((dir) => statSync(dir).isDirectory() && readdirSync(dir).includes("manifest.json"));

const osdFiles = trackedOsd();
const problems = [];
const claimed = new Set();
const totals = [];

for (const dir of scenarioDirs) {
  const name = posix(relative(scenarios, dir));
  let manifest;
  try {
    manifest = JSON.parse(readFileSync(join(dir, "manifest.json"), "utf8"));
  } catch (err) {
    problems.push(`scenarios/${name}/manifest.json: unreadable (${err.message})`);
    continue;
  }
  const listed = new Set(
    [...(Array.isArray(manifest.data) ? manifest.data : []), manifest.map].filter(
      (entry) => typeof entry === "string" && entry.endsWith(".osd"),
    ),
  );
  const owned = osdFiles.filter((file) => file.startsWith(posix(dir) + "/"));
  for (const file of owned) {
    claimed.add(file);
    const rel = posix(relative(dir, file));
    if (!listed.has(rel)) {
      problems.push(`${show(file)}: tracked .osd is not listed in scenarios/${name}/manifest.json data`);
    }
  }
  for (const entry of listed) {
    if (!owned.some((file) => posix(relative(dir, file)) === entry)) {
      problems.push(`scenarios/${name}/manifest.json: lists "${entry}", which is no tracked .osd of the scenario`);
    }
  }
  totals.push(`scenarios/${name}: ${owned.length} tracked .osd, ${listed.size} listed`);
}

for (const file of osdFiles) {
  if (!claimed.has(file)) problems.push(`${show(file)}: tracked .osd sits under no scenario that has a manifest.json`);
}

if (problems.length > 0) {
  console.log(problems.slice().sort().join("\n"));
  console.log(`RESULT: FAIL (${problems.length} unlisted or missing .osd over ${scenarioDirs.length} scenarios)`);
  process.exit(1);
}

console.log(totals.slice().sort().join("\n"));
console.log(`RESULT: PASS (${osdFiles.length} tracked .osd covered by ${scenarioDirs.length} scenario manifests)`);
