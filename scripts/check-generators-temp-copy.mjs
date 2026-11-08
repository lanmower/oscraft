import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const helper = fileURLToPath(new URL("./lib/check-by-temp-copy.mjs", import.meta.url));
const TREE = "scenarios/moba";
const UNITS_EXTRA = "scenarios/moba/units_extra.osd";
const UNITS_HEROES = "scenarios/moba/units_heroes.osd";
const UNITS_LANE = "scenarios/moba/units_lane.osd";
const UNITS_COURIER_SKINS = "scenarios/moba/units_courier_skins.osd";
const TOOLS = "scenarios/moba/tools";

const SAME = [
  ["hero melee then weapon feel", [
    "--script", `${TOOLS}/apply-hero-melee.mjs`, "--script", `${TOOLS}/apply-weapon-feel.mjs`,
    "--out", UNITS_EXTRA, "--out", UNITS_HEROES, "--out", UNITS_LANE,
  ]],
  ["courier skins", ["--script", `${TOOLS}/gen-courier-skins.mjs`, "--out", UNITS_COURIER_SKINS]],
  ["sentinel creeps", ["--script", `${TOOLS}/gen-sentinel-creeps.mjs`, "--out", UNITS_LANE]],
];
const TAMPER = ["courier skins tampered", [
  "--script", `${TOOLS}/gen-courier-skins.mjs`, "--out", UNITS_COURIER_SKINS, "--tamper", UNITS_COURIER_SKINS,
]];

function runHelper(args) {
  return spawnSync(process.execPath, [helper, "--tree", TREE, ...args], { cwd: root, encoding: "utf8" });
}

let failures = 0;
for (const [label, args] of SAME) {
  const result = runHelper(args);
  if (result.status !== 0) {
    failures += 1;
    console.log(`FAIL ${label}: exit ${result.status}\n${result.stdout}${result.stderr}`);
  }
}
const [tamperLabel, tamperArgs] = TAMPER;
const tampered = runHelper(tamperArgs);
if (tampered.status !== 1) {
  failures += 1;
  console.log(`FAIL ${tamperLabel}: expected exit 1, got ${tampered.status}`);
}

if (failures) {
  console.log(`generators temp-copy: ${failures} failed`);
  process.exit(1);
}
console.log(`generators temp-copy ok (${SAME.length} regenerations identical; tamper control caught)`);
