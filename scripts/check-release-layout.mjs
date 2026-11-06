import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const build = readFileSync(join(root, "scripts/package/build.sh"), "utf8");
const stage = readFileSync(join(root, "scripts/dev/stage.sh"), "utf8");
const layout = readFileSync(join(root, "scripts/layout-site.sh"), "utf8");
if (!build.includes("scripts/layout-site.sh")) throw new Error("the release archive does not use the shared site layout");
if (build.includes("cp -R js/client")) throw new Error("the release archive still duplicates the site copy");
if (!build.includes("wasm-pack build")) throw new Error("the release archive no longer builds wasm");
if (!stage.includes("scripts/layout-site.sh")) throw new Error("the dev stage does not use the shared site layout");
if (!layout.includes("version.json")) throw new Error("the shared layout lost version.json");
execFileSync("bash", ["-n", join(root, "scripts/package/build.sh")]);
execFileSync("bash", ["-n", join(root, "scripts/dev/stage.sh")]);
console.log("release layout ok (one layout path for Pages, dev stage and archive)");
