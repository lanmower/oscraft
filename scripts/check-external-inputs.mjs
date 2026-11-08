import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const pins = readFileSync(join(root, "scenarios/moba/tools/external-688g.tsv"), "utf8")
  .split(/\r?\n/)
  .filter((line) => line.length > 0)
  .slice(1)
  .map((line) => {
    const [name, env, relative, sha256] = line.split("\t");
    return { name, env, relative, sha256 };
  });

if (pins.length === 0 || pins.some((pin) => !/^[0-9a-f]{64}$/.test(pin.sha256))) {
  console.log("external 6.88g pins malformed: every row needs a 64-hex sha256");
  process.exit(1);
}

const verdicts = pins.map((pin) => {
  const named = process.env[pin.env] || "";
  if (!named) return { pin, state: "skipped" };
  const file = pin.relative ? join(named, pin.relative) : named;
  let digest;
  try {
    digest = createHash("sha256").update(readFileSync(file)).digest("hex");
  } catch (err) {
    return { pin, state: "failed", detail: "cannot read " + file + " (" + err.code + ")" };
  }
  if (digest !== pin.sha256) {
    return { pin, state: "failed", detail: "sha256 " + digest + " differs from the pin " + pin.sha256 + " (" + file + ")" };
  }
  return { pin, state: "verified" };
});

for (const verdict of verdicts) {
  if (verdict.state === "failed") console.log("FAIL " + verdict.pin.name + ": " + verdict.detail);
  if (verdict.state === "skipped") console.log("skipped " + verdict.pin.name + ": " + verdict.pin.env + " unset");
}

const failed = verdicts.filter((verdict) => verdict.state === "failed").length;
const verified = verdicts.filter((verdict) => verdict.state === "verified").length;
if (failed) {
  console.log("external 6.88g inputs: " + failed + " differ from their pins");
  process.exit(1);
}
console.log("external 6.88g inputs ok: " + verified + " verified, " + (verdicts.length - verified) + " skipped");
