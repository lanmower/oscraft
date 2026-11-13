import fs from "node:fs";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { roleOf } from "./prop-roles.mjs";

const require = createRequire(new URL("../../../scripts/assets/package.json", import.meta.url));
const { NodeIO } = require("@gltf-transform/core");
const { EXTMeshoptCompression, KHRMeshQuantization, KHRTextureBasisu } = require("@gltf-transform/extensions");
const { MeshoptDecoder } = require("meshoptimizer");

const io = new NodeIO()
  .registerExtensions([EXTMeshoptCompression, KHRMeshQuantization, KHRTextureBasisu])
  .registerDependencies({ "meshopt.decoder": MeshoptDecoder });
await MeshoptDecoder.ready;

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
export const UNIT_FOOTPRINT_TIMES_DEFAULT = { "tower-skinned": 2, "tower-t2-skinned": 2, "tower-t3-skinned": 2 };
const FOOTPRINT_OVERRIDES = Object.freeze({ "moon-well-lowpoly": 2.4255 });
const DIVISOR = { 5120: 127, 5121: 255, 5122: 32767, 5123: 65535 };
const IDENTITY = [[1, 0, 0], [0, 1, 0], [0, 0, 1]];

const round3 = (v) => Math.round(v * 1000) / 1000;

const decodeComponent = (value, accessor) => {
  if (!accessor.getNormalized()) return value;
  const divisor = DIVISOR[accessor.getComponentType()];
  return divisor ? Math.max(value / divisor, -1) : value;
};

const positionsOf = (prim) => {
  const acc = prim.getAttribute("POSITION");
  const arr = acc.getArray();
  const size = acc.getElementSize();
  const out = [];
  for (let i = 0; i < acc.getCount(); i++) out.push([0, 1, 2].map((c) => decodeComponent(arr[i * size + c], acc)));
  return out;
};

const newBox = () => ({ min: [Infinity, Infinity, Infinity], max: [-Infinity, -Infinity, -Infinity] });
const grow = (box, p) => {
  for (let c = 0; c < 3; c++) {
    box.min[c] = Math.min(box.min[c], p[c]);
    box.max[c] = Math.max(box.max[c], p[c]);
  }
};
const extentOf = (box) => ({ x: box.max[0] - box.min[0], h: box.max[1] - box.min[1], z: box.max[2] - box.min[2] });

const localOf = (node) => {
  const [x, y, z, w] = node.getRotation();
  const [sx, sy, sz] = node.getScale();
  const R = [
    [1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
    [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
    [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)],
  ];
  return { R: R.map((row) => [row[0] * sx, row[1] * sy, row[2] * sz]), t: node.getTranslation() };
};

const compose = (parent, local) => ({
  R: [0, 1, 2].map((r) => [0, 1, 2].map((c) => parent.R[r][0] * local.R[0][c] + parent.R[r][1] * local.R[1][c] + parent.R[r][2] * local.R[2][c])),
  t: [0, 1, 2].map((r) => parent.R[r][0] * local.t[0] + parent.R[r][1] * local.t[1] + parent.R[r][2] * local.t[2] + parent.t[r]),
});

const worldExtent = (root) => {
  const box = newBox();
  const visit = (node, parent) => {
    const world = compose(parent, localOf(node));
    const mesh = node.getMesh();
    if (mesh) {
      for (const prim of mesh.listPrimitives()) {
        for (const p of positionsOf(prim)) {
          grow(box, [0, 1, 2].map((r) => world.R[r][0] * p[0] + world.R[r][1] * p[1] + world.R[r][2] * p[2] + world.t[r]));
        }
      }
    }
    for (const child of node.listChildren()) visit(child, world);
  };
  for (const scene of root.listScenes()) for (const node of scene.listChildren()) visit(node, { R: IDENTITY, t: [0, 0, 0] });
  return extentOf(box);
};

async function modelBounds(file, withWorld) {
  const root = (await io.read(fileURLToPath(file))).getRoot();
  const first = root.listMeshes()[0]?.listPrimitives()[0];
  if (!first) return null;
  const box = newBox();
  for (const p of positionsOf(first)) grow(box, p);
  return { raw: extentOf(box), world: withWorld ? worldExtent(root) : null };
}

export async function propSizes(modelsDir, propStems = null, nativeStems = new Set()) {
  const stems = fs.readdirSync(modelsDir).filter((f) => f.endsWith(".glb")).map((f) => f.slice(0, -4)).sort();
  const bounds = {};
  for (const stem of stems) {
    if (UNIT_ONLY.test(stem) && !propStems?.has(stem)) continue;
    const m = await modelBounds(new URL(stem + ".glb", modelsDir), nativeStems.has(stem));
    if (!m) continue;
    const { x, h, z } = m.raw;
    const foot = Math.max(x, z);
    if (foot > 0 && foot <= MAX_NATIVE && h <= MAX_NATIVE) bounds[stem] = { x, h, z, foot, world: m.world };
  }
  const hero = bounds[HERO_STEM];
  if (!hero || hero.h <= 0) throw new Error("hero reference model missing or flat: " + HERO_STEM);
  const heroHeight = 2 * UNIT_HALF * (hero.h / hero.foot);
  const sizes = {};
  for (const [stem, b] of Object.entries(bounds)) {
    if (FOOTPRINT_OVERRIDES[stem] !== undefined) {
      sizes[stem] = FOOTPRINT_OVERRIDES[stem];
      continue;
    }
    if (nativeStems.has(stem)) {
      sizes[stem] = round3(Math.max(b.world.x, b.world.z));
      continue;
    }
    const role = roleOf(stem);
    const targetHeight = (ROLE_HEIGHT_IN_HEROES[role] ?? ROLE_HEIGHT_IN_HEROES.decoration) * heroHeight;
    const flat = b.h / b.foot < FLAT_RATIO;
    const footprint = (flat ? FLAT_FOOTPRINT_PER_TARGET * targetHeight : targetHeight * b.foot / b.h) * (stem.startsWith("fountain-") ? FOUNTAIN_FOOTPRINT_SCALE : 1);
    sizes[stem] = round3(Math.min(FOOTPRINT_MAX, Math.max(FOOTPRINT_MIN, footprint, ROLE_FOOTPRINT_MIN[role] ?? 0)));
  }
  const units = {};
  for (const [stem, times] of Object.entries(UNIT_FOOTPRINT_TIMES_DEFAULT)) units[stem] = round3(times * 2 * UNIT_HALF);
  for (const [stem, heroes] of Object.entries(UNIT_HEIGHT_IN_HEROES)) {
    const m = await modelBounds(new URL(stem + ".glb", modelsDir), false);
    if (!m || m.raw.h <= 0) throw new Error("unit model missing or flat: " + stem);
    const foot = Math.max(m.raw.x, m.raw.z);
    if (foot <= 0) throw new Error("unit model missing or flat: " + stem);
    units[stem] = round3(heroes * heroHeight * foot / m.raw.h);
  }
  return { sizes, heroHeight, bounds, units };
}
