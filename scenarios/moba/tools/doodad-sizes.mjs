import fs from "node:fs";
import { roleOf } from "./doodad-roles.mjs";

const UNIT_HALF = 0.9;
const HERO_STEM = "character-male-a";
export const ROLE_HEIGHT_IN_HEROES = { tree: 5, building: 3, prop: 0.9, nature: 0.5, rock: 0.35, item: 0.3, decoration: 0.7, character: 1 };
export const ROLE_FOOTPRINT_MIN = { tree: 1.2, building: 1.5, prop: 0.5, nature: 0.4, rock: 0.25, item: 0.2, decoration: 0.35, character: 0.6 };
export const SCATTER_ROLES = new Set(["tree", "nature", "rock", "prop", "item", "decoration"]);
const FLAT_RATIO = 0.3;
const FLAT_FOOTPRINT_PER_TARGET = 2;
const FOUNTAIN_FOOTPRINT_SCALE = 1 / 3;
const FOOTPRINT_MIN = 0.05;
const FOOTPRINT_MAX = 4;
const MAX_NATIVE = 100;
const UNIT_ONLY = /-(skinned|lowpoly)$/;
export const UNIT_HEIGHT_IN_HEROES = { "roshan-skinned": 2.5 };

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

export function doodadSizes(modelsDir, doodadStems = null) {
  const stems = fs.readdirSync(modelsDir).filter((f) => f.endsWith(".glb")).map((f) => f.slice(0, -4)).sort();
  const bounds = {};
  for (const stem of stems) {
    const skinnedButNotADoodad = UNIT_ONLY.test(stem) && !doodadStems?.has(stem);
    const b = firstPrimitiveBounds(new URL(stem + ".glb", modelsDir));
    if (b && b.foot > 0 && b.foot <= MAX_NATIVE && b.h <= MAX_NATIVE && !skinnedButNotADoodad) bounds[stem] = b;
  }
  const hero = bounds[HERO_STEM];
  if (!hero || hero.h <= 0) throw new Error("hero reference model missing or flat: " + HERO_STEM);
  const heroHeight = 2 * UNIT_HALF * (hero.h / hero.foot);
  const sizes = {};
  for (const [stem, b] of Object.entries(bounds)) {
    const role = roleOf(stem);
    const targetHeight = (ROLE_HEIGHT_IN_HEROES[role] ?? ROLE_HEIGHT_IN_HEROES.decoration) * heroHeight;
    const flat = b.h / b.foot < FLAT_RATIO;
    const footprint = (flat ? FLAT_FOOTPRINT_PER_TARGET * targetHeight : targetHeight * b.foot / b.h) * (stem.startsWith("fountain-") ? FOUNTAIN_FOOTPRINT_SCALE : 1);
    sizes[stem] = Math.round(Math.min(FOOTPRINT_MAX, Math.max(FOOTPRINT_MIN, footprint, ROLE_FOOTPRINT_MIN[role] ?? 0)) * 1000) / 1000;
  }
  const units = {};
  for (const [stem, heroes] of Object.entries(UNIT_HEIGHT_IN_HEROES)) {
    const b = firstPrimitiveBounds(new URL(stem + ".glb", modelsDir));
    if (!b || b.h <= 0 || b.foot <= 0) throw new Error("unit model missing or flat: " + stem);
    units[stem] = Math.round(heroes * heroHeight * b.foot / b.h * 1000) / 1000;
  }
  return { sizes, heroHeight, bounds, units };
}
