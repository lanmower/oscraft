import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const moba = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (f) => readFileSync(join(moba, f), "utf8");
const table = (f) => {
  const [head, ...rows] = read(f).trim().split("\n").map((l) => l.split("\t"));
  return rows.map((r) => Object.fromEntries(head.map((k, i) => [k, r[i] ?? ""])));
};
const problems = [];

const osd = read("items.osd");
const declared = new Set([...osd.matchAll(/^item\("([^"]+)"/gm)].map((m) => m[1]));
const idOfCode = new Map(table("items_688g.tsv").filter((r) => r.code688g && r.status !== "not_in_688g" && r.method !== "added_state").map((r) => [r.code688g, r.form_of || r.id]));
for (const [code, id] of idOfCode) if (!declared.has(id)) problems.push(`items_688g.tsv: ${code} maps to ${id}, which items.osd does not declare`);

const unadded = new Map(table("shops_unadded_688g.tsv").map((r) => [r.code688g, r]));
const listedCodes = new Set();
for (const shop of table("shops_688g.tsv")) {
  for (const code of shop.items_688g.split(",").filter(Boolean)) {
    listedCodes.add(code);
    const id = idOfCode.get(code);
    if (id === undefined && !unadded.has(code)) problems.push(`shops_688g.tsv: ${shop.shop_code} lists ${code}, which maps to no oscraft item and is not in shops_unadded_688g.tsv`);
    if (id !== undefined && unadded.has(code)) problems.push(`shops_unadded_688g.tsv: ${code} now maps to ${id}`);
  }
}
for (const [code, row] of unadded) {
  if (!listedCodes.has(code)) problems.push(`shops_unadded_688g.tsv: ${code} is listed by no shop`);
  if (row.kind !== "recipe" && row.kind !== "unadded") problems.push(`shops_unadded_688g.tsv: ${code} kind "${row.kind}" is neither recipe nor unadded`);
}


try {
  execFileSync(process.execPath, [join(moba, "tools", "gen-shops-688g.mjs"), "--check"], { stdio: "pipe" });
} catch (e) {
  problems.push(String(e.stderr || e.message).trim());
}

const needs = [
  ["shops.lua", /require\("shops_688g"\)/, "does not require shops_688g"],
  ["shops.lua", /data\.shops/, "does not read data.shops"],
  ["store.lua", /require\("shops"\)/, "does not require shops"],
  ["store.lua", /not shops\.listed\(def\.name\)/, "does not sell only what shops.listed names"],
  ["panels.lua", /shops\.listed\(def\.name\)/, "does not list only what shops.listed names"],
];
for (const [file, re, why] of [["store.lua", /def\.tags|unverified/, "still applies a depot-tag or unverified selling rule"], ["shops.lua", /unverified/, "still reads an unverified list"], ["panels.lua", /unverified\(/, "still gates on an unverified list"]]) if (re.test(read(file))) problems.push(`${file}: ${why}`);
for (const [file, re, why] of needs) if (!re.test(read(file))) problems.push(`${file}: ${why}`);

const manifest = JSON.parse(read("manifest.json")).modules ?? {};
if (!Object.values(manifest).includes("shops_688g.lua")) problems.push("manifest.json: shops_688g.lua is not a module");

if (problems.length > 0) {
  console.error(problems.join("\n"));
  console.error(`check-shops-688g: ${problems.length} problem(s)`);
  process.exit(1);
}
console.log(`check-shops-688g: ok (${listedCodes.size} codes, ${unadded.size} unadded, ${table("shops_extra_688g.tsv").length} extra)`);
