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
const BODY_GAP = 0.16;
const BUNDLE_GAP = 0.3;
const ARC_DEPTH = 0.12;

export function sceneBodyRadius(model) {
  const milli = BODY_RADIUS_MILLI[model] ?? DEFAULT_BODY_RADIUS_MILLI;
  return milli / CELL_MILLI / REFERENCE_HERO_HEIGHT_CELLS;
}

function rowTeam(radii) {
  const xs = [];
  let x = 0;
  radii.forEach((r, i) => {
    if (i > 0) x += radii[i - 1] + r + BODY_GAP;
    xs.push(x);
  });
  const minX = Math.min(...xs.map((v, i) => v - radii[i]));
  const maxX = Math.max(...xs.map((v, i) => v + radii[i]));
  const cx = (minX + maxX) / 2;
  const half = (maxX - minX) / 2;
  const bodies = xs.map((v, i) => {
    const u = half > 0 ? (v - cx) / half : 0;
    return { x: v - cx, z: -ARC_DEPTH * (1 - u * u), r: radii[i] };
  });
  return { bodies, half };
}

export function sceneBundleLayout(radii) {
  const teams = [radii.slice(0, SCENE_TEAM_SIZE), radii.slice(SCENE_TEAM_SIZE)].map(rowTeam);
  const bundleHalf = Math.max(...teams.map((t) => t.half));
  const centerOffset = bundleHalf + BUNDLE_GAP / 2;
  const centers = [-centerOffset, centerOffset];
  const seats = [];
  teams.forEach((team, t) => {
    team.bodies.forEach((body, place) => {
      seats.push({ team: t, place, x: centers[t] + body.x, z: body.z, r: body.r, yaw: 0 });
    });
  });
  return { seats, centers, halfWidth: centerOffset + bundleHalf };
}
