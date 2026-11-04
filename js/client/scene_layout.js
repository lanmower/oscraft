export const SCENE_TEAM_SIZE = 5;
const CELL_MILLI = 1000;
const REFERENCE_HERO_HEIGHT_CELLS = 0.864;
const DEFAULT_BODY_RADIUS_MILLI = 282;
const BODY_RADIUS_MILLI = Object.freeze({
  "phantom-lancer-skinned": 93,
  "rattletrap-skinned": 93,
  "naga-siren-skinned": 93,
  "slark-skinned": 93,
});
const BODY_GAP = 0.04;
const BUNDLE_GAP = 0.3;
const RING_STEP = 0.01;
const ANGLE_STEPS = 72;
const MAX_RING = 2;

export function sceneBodyRadius(model) {
  const milli = BODY_RADIUS_MILLI[model] ?? DEFAULT_BODY_RADIUS_MILLI;
  return milli / CELL_MILLI / REFERENCE_HERO_HEIGHT_CELLS;
}

function compactSpot(placed, r) {
  const fits = (x, z) => placed.every((p) => Math.hypot(x - p.x, z - p.z) >= r + p.r + BODY_GAP);
  let best = null;
  let bestReach = Infinity;
  for (let ring = 0; ring <= MAX_RING; ring += RING_STEP) {
    const steps = ring === 0 ? 1 : ANGLE_STEPS;
    for (let k = 0; k < steps; k += 1) {
      const angle = (2 * Math.PI * k) / steps;
      const x = ring * Math.cos(angle);
      const z = ring * Math.sin(angle);
      if (!fits(x, z)) continue;
      const reach = Math.max(Math.hypot(x, z) + r, ...placed.map((p) => Math.hypot(p.x, p.z) + p.r));
      if (reach < bestReach - 1e-9) {
        bestReach = reach;
        best = { x, z };
      }
    }
    if (best && ring > bestReach) break;
  }
  return best;
}

function packTeam(radii) {
  const placed = [];
  for (const r of radii) placed.push({ ...compactSpot(placed, r), r });
  let minX = Infinity;
  let maxX = -Infinity;
  let minZ = Infinity;
  let maxZ = -Infinity;
  for (const p of placed) {
    minX = Math.min(minX, p.x - p.r);
    maxX = Math.max(maxX, p.x + p.r);
    minZ = Math.min(minZ, p.z - p.r);
    maxZ = Math.max(maxZ, p.z + p.r);
  }
  const cx = (minX + maxX) / 2;
  const cz = (minZ + maxZ) / 2;
  return { bodies: placed.map((p) => ({ x: p.x - cx, z: p.z - cz, r: p.r })), half: (maxX - minX) / 2 };
}

export function sceneBundleLayout(radii) {
  const teams = [radii.slice(0, SCENE_TEAM_SIZE), radii.slice(SCENE_TEAM_SIZE)].map(packTeam);
  const bundleHalf = Math.max(...teams.map((t) => t.half));
  const centerOffset = bundleHalf + BUNDLE_GAP / 2;
  const sides = [-1, 1];
  const centers = sides.map((side) => side * centerOffset);
  const seats = [];
  teams.forEach((team, t) => {
    team.bodies.forEach((body, place) => {
      seats.push({ team: t, place, x: centers[t] + body.x, z: body.z, r: body.r, yaw: 0 });
    });
  });
  return { seats, centers, halfWidth: centerOffset + bundleHalf };
}
