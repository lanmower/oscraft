import { readdirSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const fail = (what) => { throw new Error(what); };
const SOURCES = readdirSync(here).filter((n) => /\.(js|mjs)$/.test(n) && !/-check\.mjs$/.test(n));
const load = (name) => readFileSync(resolve(here, name), "utf8");

const BOOT_WRITE = "renderer3d.setCamera({ target: orbitOrigin";
const FIRST_HUD = "hud = createHud({";
const ENGAGE_HANDLER = "heroLockButton.onclick = () => {\n      heroViewLock.toggle();";

export function audit(sources) {
  const toggles = [];
  for (const name of Object.keys(sources)) {
    sources[name].split("\n").forEach((line, i) => {
      if (/heroViewLock\.toggle\(/.test(line)) toggles.push(`${name}:${i + 1}`);
    });
  }
  const play = sources["play.js"];
  const hud = sources["hud.js"];
  const boot = play.indexOf(BOOT_WRITE);
  const install = play.indexOf(FIRST_HUD);
  return {
    toggles,
    engageInHandler: hud.includes(ENGAGE_HANDLER),
    bootIndex: boot,
    installIndex: install,
    earlyInstalls: [...play.matchAll(/createHud\(/g)].map((m) => m.index).filter((at) => boot >= 0 && at < boot),
  };
}

export function violations(report) {
  const out = [];
  if (report.toggles.length !== 1 || report.toggles[0] !== "hud.js:182") out.push(`heroViewLock.toggle must have exactly one call site at hud.js:182, found ${report.toggles.join(", ") || "none"}`);
  if (!report.engageInHandler) out.push("the hero-lock toggle is not inside heroLockButton.onclick");
  if (report.bootIndex < 0) out.push("boot write not found in play.js");
  if (report.installIndex < 0) out.push("first createHud install not found in play.js");
  if (report.installIndex >= 0 && report.bootIndex >= 0 && report.installIndex < report.bootIndex) out.push("the hero-lock handler is installed before the boot write");
  if (report.earlyInstalls.length > 0) out.push(`createHud runs before the boot write at ${report.earlyInstalls.length} place(s)`);
  return out;
}

const sources = Object.fromEntries(SOURCES.map((name) => [name, load(name)]));
const positive = violations(audit(sources));
if (positive.length > 0) fail(positive.join("; "));

const earlier = sources["play.js"].replace('  bootAt("world");', '  createHud({});\n  bootAt("world");');
if (earlier === sources["play.js"]) fail("negative control: the bootAt(\"world\") anchor was not found");
const moved = violations(audit({ ...sources, "play.js": earlier }));
if (moved.length === 0) fail("negative control: an engage path installed before the boot write did not fail the gate");

const report = audit(sources);
console.log(`lock engage order ok: single engage site ${report.toggles[0]} inside the hero-lock onclick; createHud installs it at play.js:${sources["play.js"].slice(0, report.installIndex).split("\n").length} after the boot write at play.js:${sources["play.js"].slice(0, report.bootIndex).split("\n").length}`);
console.log(`negative control: an earlier createHud install fails with "${moved[0]}"`);
