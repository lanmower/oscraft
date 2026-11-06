import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const docs = resolve(root, "docs");
if (existsSync(join(docs, "NAMING.md"))) throw new Error("the retired rename spec is still present");
for (const f of readdirSync(docs).filter((name) => name.endsWith(".md"))) {
  const text = readFileSync(join(docs, f), "utf8");
  if (text.includes("](NAMING.md)") || text.includes("](./NAMING.md)") || text.includes("](docs/NAMING.md)")) throw new Error(f + " still links NAMING.md");
}
console.log("naming spec ok (retired, no doc links)");
