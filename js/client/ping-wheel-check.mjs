import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { PING_GLYPHS, WHEEL_KINDS, pingGlyphSvg } from "./ping_glyphs.js";
import { wheelKindAt, WHEEL_KEYS } from "./ping_wheel.js";

const fail = (what) => { throw new Error(what); };

if (WHEEL_KINDS.length !== 8) fail(`wheel has ${WHEEL_KINDS.length} kinds, want 8`);
if (new Set(WHEEL_KINDS).size !== 8) fail("wheel kinds repeat");
if (WHEEL_KINDS.includes("item")) fail("item is a status ping, not a wheel alert");
if (WHEEL_KEYS.join(",") !== "g,v") fail(`wheel keys ${WHEEL_KEYS.join(",")}, want g,v`);

const rings = [[0, -40], [40, -40], [40, 0], [40, 40], [0, 40], [-40, 40], [-40, 0], [-40, -40]];
const picked = rings.map(([dx, dy]) => wheelKindAt(dx, dy));
if (picked.join(",") !== WHEEL_KINDS.join(",")) fail(`wedges ${picked.join(",")} do not match ${WHEEL_KINDS.join(",")}`);
if (wheelKindAt(3, -2) !== null) fail("dead zone at the centre must pick nothing");

const luaPath = fileURLToPath(new URL("../../scenarios/moba/pings.lua", import.meta.url));
const luaKinds = new Set([...readFileSync(luaPath, "utf8").matchAll(/^\s{2}(\w+) = \{ colour/gm)].map((m) => m[1]));
const missing = WHEEL_KINDS.filter((kind) => !luaKinds.has(kind));
if (missing.length) fail(`sim pings.lua lacks wheel kinds ${missing.join(",")}`);

for (const kind of WHEEL_KINDS) {
  if (!PING_GLYPHS[kind]?.name) fail(`${kind} has no name for title or aria label`);
  if (!/<(polygon|circle) /.test(pingGlyphSvg(kind))) fail(`${kind} draws no glyph shape`);
}

console.log(`ping-wheel kinds=${WHEEL_KINDS.join(",")} wedges=${picked.join(",")} sim=${WHEEL_KINDS.length} names=${WHEEL_KINDS.map((k) => PING_GLYPHS[k].name).join("|")}`);
