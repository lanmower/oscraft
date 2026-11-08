import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const ENTRIES = ["play.js", "rooms.js"];
const EXTERNALS = ["./pkg/*", "./vendor/*", "wireweave", "wireweave/*", "nostr-tools", "xstate", "node:*"];
const PRELOAD = /^\s*<link rel="modulepreload" href="([^"]+)">\s*$/;

const replaceOnce = (text, from, to) => {
  const count = text.split(from).length - 1;
  if (count !== 1) throw new Error(`expected one ${from} in play.html, found ${count}`);
  return text.replace(from, to);
};

const bundledInputs = (metaPath, client) =>
  Object.keys(JSON.parse(fs.readFileSync(metaPath, "utf8")).inputs)
    .map((key) => path.relative(client, path.resolve(root, key)))
    .filter((rel) => !rel.startsWith(".."))
    .sort();

const siteDir = process.argv[2];
if (!siteDir) {
  console.error("usage: node scripts/bundle-client.mjs <site-dir>");
  process.exitCode = 2;
} else {
  const client = path.resolve(siteDir, "js", "client");
  const scratch = fs.mkdtempSync(path.join(os.tmpdir(), "bundle-client-"));
  try {
    const metaPath = path.join(scratch, "meta.json");
    execFileSync(
      "npx",
      [
        "--yes",
        "esbuild@0.25.0",
        ...ENTRIES.map((entry) => path.join(client, entry)),
        "--bundle",
        "--format=esm",
        "--splitting",
        `--outdir=${client}`,
        "--entry-names=[name].bundle",
        "--chunk-names=chunk-[hash]",
        ...EXTERNALS.map((pattern) => `--external:${pattern}`),
        `--metafile=${metaPath}`,
      ],
      { cwd: root, stdio: "inherit" },
    );
    const inputs = bundledInputs(metaPath, client);
    const pagePath = path.join(client, "play.html");
    const kept = [];
    let removed = 0;
    for (const line of fs.readFileSync(pagePath, "utf8").split("\n")) {
      const preload = line.match(PRELOAD);
      if (preload && inputs.includes(preload[1].replace(/^\.\//, ""))) {
        removed += 1;
        continue;
      }
      kept.push(line);
    }
    let page = kept.join("\n");
    page = replaceOnce(page, '<script type="module" src="play.js"></script>', '<script type="module" src="play.bundle.js"></script>');
    page = replaceOnce(page, '<script type="module" src="rooms.js"></script>', '<script type="module" src="rooms.bundle.js"></script>');
    fs.writeFileSync(pagePath, page);
    fs.writeFileSync(path.join(client, "bundled-inputs.json"), JSON.stringify(inputs) + "\n");
    const remaining = page.split("\n").filter((line) => PRELOAD.test(line)).length;
    console.log(`bundle-client: ${inputs.length} inputs bundled, removed ${removed} stale modulepreloads, ${remaining} kept`);
  } finally {
    fs.rmSync(scratch, { recursive: true, force: true });
  }
}
