// Target size of every doodad kind, from the measured bounds of its GLB. This is the one place for the scale rule:
// gen-dota-map.mjs writes the result to js/client/doodad_sizes.json, and play.js fits each model to its footprint.
// The footprint is the larger X or Z side of the model's first mesh, in cells (one cell is one world unit, one 64-unit GridNav cell).
import fs from "node:fs";
import { roleOf } from "./doodad-roles.mjs";

const UNIT_HALF = 0.9; // play.js UNIT_HALF: a unit model is fitted to 2 * UNIT_HALF cells across its larger side.
const HERO_STEM = "character-male-a"; // human-scale reference: the height play.js draws for it, from the same first-mesh fit.
// Target height per role, in hero heights. Trees tower over a hero, buildings over a tower base, props and items stay near waist height.
// Upright models keep their own proportions: the scale comes from the target height, so crown width follows the model.
export const ROLE_HEIGHT_IN_HEROES = { tree: 5, building: 2.5, prop: 0.75, nature: 0.4, rock: 0.23, item: 0.25, decoration: 0.5, character: 1 };
export const SCATTER_ROLES = new Set(["tree", "nature", "rock", "prop", "item", "decoration"]);
const FLAT_RATIO = 0.3; // a model under 0.3 of its footprint in height is flat (ground patch, log, fountain basin).
const FLAT_FOOTPRINT_PER_TARGET = 2; // flat models take twice the role's target height as their footprint.
const FOOTPRINT_MIN = 0.05;
const FOOTPRINT_MAX = 4;
const MAX_NATIVE = 100; // native extents above this are not scale-able assets (for example a skinned unit with corrupt bounds).
const UNIT_ONLY = /^(roshan|abaddon)-/; // unit models, drawn by the sim as units, never as scenery: never placed as doodads.
// Unit models drawn at their own size: target height in hero heights. Every other unit is fitted to a hero footprint (play.js UNIT_HALF).
export const UNIT_HEIGHT_IN_HEROES = { "roshan-skinned": 2.5 };

// Bounds of the first mesh's POSITION accessor, the same vertices play.js fits (glb.js parseGlb reads only that primitive).
export function firstPrimitiveBounds(file) {
  const buf = fs.readFileSync(file);
  if (buf.readUInt32LE(0) !== 0x46546c67) throw new Error("not a glb: " + file);
  const gltf = JSON.parse(buf.subarray(20, 20 + buf.readUInt32LE(12)).toString("utf8"));
  const accessor = gltf.accessors?.[gltf.meshes?.[0]?.primitives?.[0]?.attributes?.POSITION];
  if (!accessor?.min || !accessor?.max) return null;
  const x = accessor.max[0] - accessor.min[0];
  const h = accessor.max[1] - accessor.min[1];
  const z = accessor.max[2] - accessor.min[2];
  return { x, h, z, foot: Math.max(x, z) };
}

// Returns { sizes: model stem -> footprint in cells, heroHeight, bounds: stem -> measured bounds, units: unit stem -> footprint }.
// Models with no sane bounds are left out. A unit footprint keeps the model's own proportions at its target height, unclamped.
export function doodadSizes(modelsDir) {
  const stems = fs.readdirSync(modelsDir).filter((f) => f.endsWith(".glb")).map((f) => f.slice(0, -4)).sort();
  const bounds = {};
  for (const stem of stems) {
    const b = firstPrimitiveBounds(new URL(stem + ".glb", modelsDir));
    if (b && b.foot > 0 && b.foot <= MAX_NATIVE && b.h <= MAX_NATIVE && !UNIT_ONLY.test(stem)) bounds[stem] = b;
  }
  const hero = bounds[HERO_STEM];
  if (!hero || hero.h <= 0) throw new Error("hero reference model missing or flat: " + HERO_STEM);
  const heroHeight = 2 * UNIT_HALF * (hero.h / hero.foot);
  const sizes = {};
  for (const [stem, b] of Object.entries(bounds)) {
    const role = roleOf(stem);
    const targetHeight = (ROLE_HEIGHT_IN_HEROES[role] ?? ROLE_HEIGHT_IN_HEROES.decoration) * heroHeight;
    const flat = b.h / b.foot < FLAT_RATIO;
    const footprint = flat ? FLAT_FOOTPRINT_PER_TARGET * targetHeight : targetHeight * b.foot / b.h;
    sizes[stem] = Math.round(Math.min(FOOTPRINT_MAX, Math.max(FOOTPRINT_MIN, footprint)) * 1000) / 1000;
  }
  const units = {};
  for (const [stem, heroes] of Object.entries(UNIT_HEIGHT_IN_HEROES)) {
    const b = firstPrimitiveBounds(new URL(stem + ".glb", modelsDir));
    if (!b || b.h <= 0 || b.foot <= 0) throw new Error("unit model missing or flat: " + stem);
    units[stem] = Math.round(heroes * heroHeight * b.foot / b.h * 1000) / 1000;
  }
  return { sizes, heroHeight, bounds, units };
}
