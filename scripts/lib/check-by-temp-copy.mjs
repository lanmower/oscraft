import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const SKIPPED_DIRS = new Set([".git", "node_modules", "target"]);

const inside = (dir, rel) => rel === dir || rel.startsWith(`${dir}/`);

export function committedText(rel) {
  return execFileSync("git", ["-C", repo, "show", `HEAD:${rel}`], { encoding: "utf8", maxBuffer: 1 << 30 });
}

export function checkByTempCopy({ tree, scripts, args = [], outputs, tamper }) {
  for (const script of scripts) if (!inside(tree, script)) throw new Error(`${script} is not inside ${tree}`);
  if (tamper !== undefined && !outputs.includes(tamper)) throw new Error(`${tamper} is not one of the outputs`);
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "check-by-temp-copy-"));
  try {
    fs.cpSync(path.join(repo, tree), path.join(tmp, tree), {
      recursive: true,
      filter: (src) => !SKIPPED_DIRS.has(path.basename(src)),
    });
    const committed = new Map(outputs.map((out) => [out, committedText(out)]));
    if (tamper !== undefined) committed.set(tamper, `${committed.get(tamper)}tampered-committed-copy\n`);
    for (const script of scripts) {
      execFileSync(process.execPath, [path.join(tmp, script), ...args], { cwd: tmp, stdio: ["ignore", "pipe", "pipe"] });
    }
    const differing = outputs.filter((out) => fs.readFileSync(path.join(tmp, out), "utf8") !== committed.get(out));
    return { scripts, outputs, differing };
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
}

function parseArgs(argv) {
  const options = { args: [], outputs: [], scripts: [] };
  for (let i = 0; i < argv.length; i += 2) {
    const value = argv[i + 1];
    if (value === undefined) throw new Error(`${argv[i]} needs a value`);
    if (argv[i] === "--tree") options.tree = value;
    else if (argv[i] === "--script") options.scripts.push(value);
    else if (argv[i] === "--arg") options.args.push(value);
    else if (argv[i] === "--out") options.outputs.push(value);
    else if (argv[i] === "--tamper") options.tamper = value;
    else throw new Error(`unknown argument ${argv[i]}`);
  }
  if (!options.tree || options.scripts.length === 0 || options.outputs.length === 0) {
    throw new Error("usage: check-by-temp-copy.mjs --tree <dir> --script <file>... --out <file>... [--arg <value>]... [--tamper <out>]");
  }
  return options;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  let result;
  try {
    result = checkByTempCopy(parseArgs(process.argv.slice(2)));
  } catch (error) {
    console.error(`check-by-temp-copy: ${error.message}`);
    if (error.stderr) console.error(String(error.stderr).trim());
    process.exit(2);
  }
  for (const out of result.outputs) {
    console.log(`${out}: ${result.differing.includes(out) ? "differs from committed" : "matches committed"}`);
  }
  if (result.differing.length > 0) {
    console.error(`check-by-temp-copy: ${result.scripts.join(" then ")} regenerates ${result.differing.length} output(s) differently`);
    process.exit(1);
  }
  console.log(`check-by-temp-copy: ${result.scripts.join(" then ")} regenerates ${result.outputs.length} output(s) identically`);
}
