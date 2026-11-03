import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const mobaDir = path.join(root, "scenarios", "moba");
const campDataText = fs.readFileSync(path.join(mobaDir, "camps_688g.lua"), "utf8");
const runtimeText = fs.readFileSync(path.join(mobaDir, "camps.lua"), "utf8");
const placementText = fs.readFileSync(path.join(mobaDir, "neutral.osd"), "utf8");

const UNITS_PER_MILLI = 64 / 1000;
const failures = [];
const reports = [];

const annulus = runtimeText.match(/OFFSET_MIN, OFFSET_MAX = (\d+), (\d+)/);
if (!annulus) failures.push("camps.lua: OFFSET_MIN, OFFSET_MAX not found");
const annulusMin = annulus ? Number(annulus[1]) : NaN;
const annulusMax = annulus ? Number(annulus[2]) : NaN;

const campRows = campDataText.split("\n");
const camps = [];
campRows.forEach((line, index) => {
  const head = line.match(/\{ id = (\d+), size = "(\w+)", check = \{([^}]*)\}, centre = \{([^}]*)\}, facing = (-?\d+),/);
  if (!head) return;
  const options = [...(campRows[index + 1] ?? "").matchAll(/\{([^{}]*)\}/g)].map((m) =>
    m[1].split(",").map((name) => name.trim().replace(/"/g, "")),
  );
  const [cx, cy] = head[4].split(",").map(Number);
  camps.push({ id: Number(head[1]), size: head[2], check: head[3].split(",").map(Number), cx, cy, options });
});

const placed = [...placementText.matchAll(/^spawn\("(\w+)", -1, (-?\d+), (-?\d+)\)$/gm)].map((m) => ({
  kind: m[1],
  x: Number(m[2]),
  y: Number(m[3]),
}));

let cursor = 0;
for (const camp of camps) {
  camp.pick = ((camp.id - 1) % camp.options.length) + 1;
  camp.members = [];
  for (const kind of camp.options[camp.pick - 1]) {
    const spawn = placed[cursor];
    cursor += 1;
    if (spawn === undefined || spawn.kind !== kind) {
      failures.push(`camp ${camp.id}: neutral.osd spawn ${cursor} is ${spawn ? spawn.kind : "missing"}, the stand-in option wants ${kind}`);
      continue;
    }
    camp.members.push(spawn);
  }
}

const nearestCamp = (x, y) =>
  camps.reduce((best, camp) => ((x - camp.cx) ** 2 + (y - camp.cy) ** 2 < (x - best.cx) ** 2 + (y - best.cy) ** 2 ? camp : best));
const distanceUnits = (member, camp) => Math.hypot(member.x - camp.cx, member.y - camp.cy) * UNITS_PER_MILLI;
const inCheckRect = (member, camp) =>
  member.x >= camp.check[0] && member.x <= camp.check[2] && member.y >= camp.check[1] && member.y <= camp.check[3];

const rectMisses = [];
let placedTotal = 0;
for (const camp of camps) {
  placedTotal += camp.members.length;
  const outside = camp.members.filter((m) => !(distanceUnits(m, camp) >= annulusMin && distanceUnits(m, camp) <= annulusMax));
  const byCentre = camp.members.filter((m) => nearestCamp(m.x, m.y) === camp);
  const byRect = camp.members.filter((m) => inCheckRect(m, camp));
  reports.push(
    `camp ${String(camp.id).padStart(2)} ${camp.size.padEnd(7)} pick ${camp.pick}/${camp.options.length} ` +
      `placed ${camp.members.length} byCentre ${byCentre.length} byRect ${byRect.length} ` +
      `annulus ${outside.length === 0 ? "ok" : "FAIL"}`,
  );
  if (outside.length > 0) failures.push(`camp ${camp.id}: ${outside.length} member(s) outside the ${annulusMin}..${annulusMax} annulus`);
  if (byCentre.length !== camp.members.length) {
    failures.push(`camp ${camp.id}: nearest-centre membership counts ${byCentre.length} of ${camp.members.length} placed members`);
  }
  if (byRect.length < camp.members.length) rectMisses.push(`camp ${camp.id} ${byRect.length} of ${camp.members.length} in the check rect`);
}

if (!runtimeText.includes('require("camps_688g")')) failures.push('camps.lua does not require "camps_688g"');
if (!runtimeText.includes("r.camp = camp_of(x, y)")) failures.push("camps.lua does not register members by nearest centre (camp_of)");
const blockedStart = runtimeText.indexOf("local function camp_blocked(c)");
const blockedEnd = runtimeText.indexOf("local function roll_option(c)");
if (blockedStart < 0 || blockedEnd < blockedStart) {
  failures.push("camps.lua: camp_blocked is not delimited by roll_option");
} else {
  const rectRead = /\bc\.[xy][12]\b/g;
  const reads = (runtimeText.match(rectRead) || []).length;
  const blockedReads = (runtimeText.slice(blockedStart, blockedEnd).match(rectRead) || []).length;
  if (reads !== blockedReads) failures.push(`camps.lua reads the check rect in ${reads - blockedReads} place(s) outside camp_blocked`);
}

console.log(reports.join("\n"));
console.log(
  `RECT READING MISS (the old gate counted by rect; membership now counts by centre): ${rectMisses.length === 0 ? "none" : rectMisses.join("; ")}`,
);
console.log(`camps ${camps.length} placed ${placedTotal} non-camp spawns ${placed.length - cursor}`);
if (failures.length === 0) {
  console.log(
    `CAMP PLACEMENT AGREES: ${camps.length} camps, ${placedTotal} members in the ${annulusMin}..${annulusMax} annulus, nearest-centre membership exact`,
  );
} else {
  for (const failure of failures) console.log(`FAIL ${failure}`);
  console.log(`CAMP PLACEMENT DISAGREES: ${failures.length} failure(s)`);
}
process.exitCode = failures.length === 0 ? 0 : 1;
