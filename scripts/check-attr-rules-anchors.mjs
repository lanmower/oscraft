import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const option = (name, fallback) => process.argv.slice(2).find((arg) => arg.startsWith(`--${name}=`))?.slice(name.length + 3) ?? fallback;
const scenariosRoot = option("root", root);
const configPath = option("config", path.join(root, "scripts", "mod-host", "splice-anchors.json"));
const config = JSON.parse(fs.readFileSync(configPath, "utf8"));
const errors = [];

for (const generator of config.generators) {
  const text = fs.readFileSync(path.join(scenariosRoot, generator), "utf8");
  if (!text.includes(`"${config.begin}"`)) errors.push(`${generator} does not write the BEGIN anchor ${config.begin}`);
  if (!text.includes(`"${config.end}"`)) errors.push(`${generator} does not write the END anchor ${config.end}`);
}

const listed = new Set([...config.packs, ...config.unmanaged]);
const onDisk = fs.readdirSync(path.join(scenariosRoot, "scenarios")).filter((name) => fs.existsSync(path.join(scenariosRoot, "scenarios", name, "attr_rules.txt")));
for (const pack of onDisk) {
  if (!listed.has(pack)) errors.push(`${pack} has attr_rules.txt but is not listed in splice-anchors.json`);
}

for (const pack of [...config.packs, ...config.unmanaged]) {
  const file = path.join(scenariosRoot, "scenarios", pack, "attr_rules.txt");
  if (!fs.existsSync(file)) {
    errors.push(`${pack} has no attr_rules.txt`);
    continue;
  }
  const lines = fs.readFileSync(file, "utf8").split("\n");
  const begins = lines.map((line, index) => (line.startsWith(config.begin) ? index : -1)).filter((index) => index >= 0);
  const ends = lines.map((line, index) => (line.startsWith(config.end) ? index : -1)).filter((index) => index >= 0);
  if (!config.packs.includes(pack)) {
    if (begins.length + ends.length > 0) errors.push(`${pack}: stray generator anchor, this pack is not generated`);
    continue;
  }
  if (begins.length !== 1) errors.push(`${pack}: ${begins.length} BEGIN anchors, expected 1 (${config.begin})`);
  if (ends.length !== 1) errors.push(`${pack}: ${ends.length} END anchors, expected 1 (${config.end})`);
  if (begins.length === 1 && ends.length === 1 && ends[0] < begins[0]) errors.push(`${pack}: END before BEGIN`);
}

if (errors.length) {
  for (const error of errors) console.error(`FAIL ${error}`);
  process.exit(1);
}
console.log(`attr_rules anchors ok: ${config.packs.length} generated packs with one BEGIN and one END each, ${config.unmanaged.length} unmanaged pack with none, ${config.generators.length} generators write the anchors`);
