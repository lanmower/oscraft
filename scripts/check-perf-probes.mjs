import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const perf = resolve(root, "scripts/perf");
const draw = readFileSync(resolve(perf, "first-draw.mjs"), "utf8");
for (const metric of ["median-ms=", "transferred-bytes=", "requests="]) {
  if (!draw.includes(metric)) throw new Error("first-draw.mjs lost metric " + metric);
}
const server = resolve(perf, "asset-packs-server.mjs");
const serverSource = existsSync(server) ? readFileSync(server, "utf8") : "";
if (serverSource && !serverSource.includes("createServer")) throw new Error("the perf server does not start a server");
const named = readdirSync(perf).filter((f) => f.includes("check") && f.endsWith(".mjs"));
if (named.length) throw new Error("a perf file is still named as a check: " + named.join(", "));
if (readFileSync(resolve(root, "scripts/check-all.mjs"), "utf8").includes("scripts/perf/")) throw new Error("a perf probe is wired as a gate");
console.log("perf probes ok (3 metrics, server renamed, no perf file is a gate)");
