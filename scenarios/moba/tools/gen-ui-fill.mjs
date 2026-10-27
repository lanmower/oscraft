import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const moba = path.join(path.dirname(fileURLToPath(import.meta.url)), "..");
const rows = fs
  .readFileSync(path.join(moba, "ui-fill.tsv"), "utf8")
  .split("\n")
  .slice(1)
  .filter((line) => line !== "")
  .map((line) => line.split("\t"));
const ROLE_TAGS = new Set(JSON.parse(fs.readFileSync(path.join(moba, "moba_ui.json"), "utf8")).roles);

const rowsWith = (prefix) =>
  rows.filter(([key]) => key.startsWith(prefix)).map(([key, value]) => [key.slice(prefix.length), value]);
const luaKey = (name) => `[${JSON.stringify(name)}]`;

const roles = rowsWith("lobby.role:").map(([id, value]) => {
  for (const tag of value.split("/")) {
    if (!ROLE_TAGS.has(tag)) throw new Error(`lobby.role row names an unknown role tag: ${id} = ${tag}`);
  }
  return `    ${luaKey(id)} = ${JSON.stringify(value)},`;
});

const BASE = /^(.+)\|str (\d+)\+(\d+)\|agi (\d+)\+(\d+)\|int (\d+)\+(\d+)/;
const base = rowsWith("lobby.base:").map(([id, value]) => {
  const m = BASE.exec(value);
  if (m === null) throw new Error(`lobby.base row does not parse: ${id}`);
  const fields = [JSON.stringify(m[1]), ...m.slice(2).map(Number)];
  return `    ${luaKey(id)} = { ${fields.join(", ")} },`;
});

const SHOP_KINDS = new Map([
  ["Home Base", "home"],
  ["Side Shop", "side"],
  ["Secret Shop", "secret"],
]);
const shopKind = rowsWith("shop.kind:").flatMap(([building, value]) => {
  if (SHOP_KINDS.has(value)) return [`    ${luaKey(building)} = ${JSON.stringify(SHOP_KINDS.get(value))},`];
  if (value === building) return [];
  throw new Error(`shop.kind row names no kind: ${building} = ${value}`);
});

fs.writeFileSync(
  path.join(moba, "ui_fill.lua"),
  [
    "return {",
    "  roles = {",
    ...roles,
    "  },",
    "  base = {",
    ...base,
    "  },",
    "  shop_kind = {",
    ...shopKind,
    "  },",
    "}",
    "",
  ].join("\n"),
);
