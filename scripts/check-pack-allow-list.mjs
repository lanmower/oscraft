import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), "..");
const packerSource = fs.readFileSync(path.join(root, "scripts", "pack-package.mjs"), "utf8");
const loaderSource = fs.readFileSync(path.join(root, "js", "client", "pack_load.js"), "utf8");

const extensionsIn = (source, pattern, name) => {
  const match = source.match(pattern);
  if (!match) throw new Error(`${name} not found`);
  return [...match[1].matchAll(/"(\.[a-z0-9]+)"/gi)].map((m) => m[1].toLowerCase());
};

const packerAllowed = extensionsIn(packerSource, /const ALLOWED = new Set\(\[([\s\S]*?)\]\)/, "pack-package.mjs ALLOWED");
const loaderText = extensionsIn(loaderSource, /export const TEXT_EXTENSIONS = Object\.freeze\(\[([^\]]*)\]\)/, "pack_load.js TEXT_EXTENSIONS");
const loaderAsset = extensionsIn(loaderSource, /export const ASSET_EXTENSIONS = Object\.freeze\(\[([^\]]*)\]\)/, "pack_load.js ASSET_EXTENSIONS");

const loaderAccepts = new Set([...loaderText, ...loaderAsset]);
const packerSet = new Set(packerAllowed);
const problems = [
  ...[...loaderAccepts].filter((ext) => !packerSet.has(ext)).map((ext) => `pack-package.mjs refuses ${ext}, which pack_load.js accepts`),
  ...[...packerSet].filter((ext) => !loaderAccepts.has(ext)).map((ext) => `pack-package.mjs allows ${ext}, which pack_load.js refuses`),
];
if (problems.length > 0) {
  console.error(problems.join("\n"));
  process.exit(1);
}
console.log(`check-pack-allow-list: ${packerSet.size} extensions; pack-package.mjs ALLOWED equals pack_load.js TEXT_EXTENSIONS + ASSET_EXTENSIONS`);
