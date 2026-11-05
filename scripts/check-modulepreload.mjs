import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const page = join(root, "js", "client", "play.html");
const html = readFileSync(page, "utf8");
const links = [...html.matchAll(/<link\b[^>]*>/gi)].map((match) => match[0]);
const preloads = links.filter((tag) => /\brel="preload"/.test(tag));
const wasm = preloads.filter((tag) => /web_bg\.wasm|as="fetch"/.test(tag));
const modulepreloads = links.filter((tag) => /\brel="modulepreload"/.test(tag));
const missing = modulepreloads
  .map((tag) => (tag.match(/href="([^"]+)"/) || [])[1])
  .filter(Boolean)
  .filter((href) => !existsSync(resolve(dirname(page), href)));
if (wasm.length || missing.length) {
  console.log(`check-modulepreload: FAIL wasm=${wasm.length} missing=${missing.join(",")}`);
  process.exitCode = 1;
} else {
  console.log(`check-modulepreload: ${modulepreloads.length} modulepreload hints, 0 wasm preloads, all targets exist`);
}
