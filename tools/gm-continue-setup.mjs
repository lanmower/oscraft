import { readdirSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const rowsDir = join(repoRoot, "work", "rows");
const ACTIONABLE_STATUS = "open";
const SETTLED_STATUSES = new Set(["resolved", "parked"]);

function parseRow(text) {
  const row = {};
  for (const line of text.split(/\r?\n/)) {
    const match = /^([A-Za-z_][A-Za-z0-9_]*):\s*(.*)$/.exec(line);
    if (!match) continue;
    const [, key, raw] = match;
    row[key] = unquote(raw.trim());
  }
  return row;
}

function unquote(raw) {
  if (raw.length >= 2 && raw.startsWith('"') && raw.endsWith('"')) {
    try {
      return JSON.parse(raw);
    } catch {
      return raw.slice(1, -1);
    }
  }
  return raw;
}

function loadRows() {
  return readdirSync(rowsDir)
    .filter((name) => name.endsWith(".yml"))
    .sort()
    .map((name) => {
      const row = parseRow(readFileSync(join(rowsDir, name), "utf8"));
      return { file: name, ...row };
    });
}

function classify(rows) {
  const plan = rows.filter((row) => row.status === ACTIONABLE_STATUS);
  const parked = rows.filter((row) => row.status === "parked");
  const defects = rows.filter((row) => !SETTLED_STATUSES.has(row.status) && row.status !== ACTIONABLE_STATUS);
  return { plan, parked, defects };
}

function planLine(row) {
  return `PLAN ${row.id ?? row.file} [${row.kind ?? "?"}] ${row.title ?? ""}`.trimEnd();
}

function parkedLine(row) {
  return `PARKED ${row.id ?? row.file} ${row.reason ?? ""}`.trimEnd();
}

function defectLine(row) {
  return `DEFECT ${row.id ?? row.file} status=${JSON.stringify(row.status ?? "")} (neither open, parked nor resolved)`;
}

function main() {
  const rows = loadRows();
  const { plan, parked, defects } = classify(rows);
  const lines = [
    `gm-continue setup: ${rows.length} rows, ${plan.length} open, ${parked.length} parked, ${defects.length} defects`,
    ...plan.map(planLine),
    ...parked.map(parkedLine),
    ...defects.map(defectLine),
  ];
  process.stdout.write(lines.join("\n") + "\n");
  process.exitCode = 0;
}

main();
