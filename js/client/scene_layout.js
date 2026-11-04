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
const SEARCH_STEPS = 48;
export const SCENE_ROW_DEPTH = 2;
export const SCENE_VIEW_PITCH = 0.6;

export function sceneBodyRadius(model) {
  const milli = BODY_RADIUS_MILLI[model] ?? DEFAULT_BODY_RADIUS_MILLI;
  return milli / CELL_MILLI / REFERENCE_HERO_HEIGHT_CELLS;
}

function lineXs(radii) {
  const xs = [];
  let x = 0;
  radii.forEach((r, i) => {
    if (i > 0) x += radii[i - 1] + r + BODY_GAP;
    xs.push(x);
  });
  return xs;
}

function bundleTeam(radii) {
  const backCount = Math.ceil(radii.length / 2);
  const backRadii = radii.slice(0, backCount);
  const frontRadii = radii.slice(backCount);
  const backXs = lineXs(backRadii);
  const frontXs = frontRadii.map((_, i) => (backXs[i] + backXs[i + 1]) / 2);
  const bodies = [
    ...backXs.map((x, i) => ({ x, z: -SCENE_ROW_DEPTH / 2, r: backRadii[i] })),
    ...frontXs.map((x, i) => ({ x, z: SCENE_ROW_DEPTH / 2, r: frontRadii[i] })),
  ];
  const minX = Math.min(...bodies.map((b) => b.x - b.r));
  const maxX = Math.max(...bodies.map((b) => b.x + b.r));
  const cx = (minX + maxX) / 2;
  return { bodies: bodies.map((b) => ({ ...b, x: b.x - cx })), half: (maxX - minX) / 2 };
}

export function sceneBundleLayout(radii) {
  const teams = [radii.slice(0, SCENE_TEAM_SIZE), radii.slice(SCENE_TEAM_SIZE)].map(bundleTeam);
  const bundleHalf = Math.max(...teams.map((t) => t.half));
  const centerOffset = bundleHalf + BUNDLE_GAP / 2;
  const centers = [-centerOffset, centerOffset];
  const seats = [];
  teams.forEach((team, t) => {
    team.bodies.forEach((body, place) => {
      seats.push({ team: t, place, x: centers[t] + body.x, z: body.z, r: body.r, yaw: 0 });
    });
  });
  return seats;
}

function projectTo(point, camera) {
  const dy = point[1] - camera.eyeY;
  const dz = point[2] - camera.eyeZ;
  const depth = dy * camera.forwardY + dz * camera.forwardZ;
  const up = dy * camera.upY + dz * camera.upZ;
  return [point[0] / (depth * camera.tanHalfX), up / (depth * camera.tanHalfY)];
}

function cameraAt(distance, targetY, pitch, aspect, fovY) {
  const eyeY = targetY + distance * Math.sin(pitch);
  const eyeZ = distance * Math.cos(pitch);
  const fy = targetY - eyeY;
  const fz = -eyeZ;
  const len = Math.hypot(fy, fz);
  const forwardY = fy / len;
  const forwardZ = fz / len;
  const tanHalfY = Math.tan(fovY / 2);
  return { eyeY, eyeZ, forwardY, forwardZ, upY: -forwardZ, upZ: forwardY, tanHalfX: tanHalfY * aspect, tanHalfY };
}

export function sceneFrame(seats, { aspect, pitch, fovY, heights, fill }) {
  const points = seats.flatMap((s) => heights.flatMap((y) => [s.x - s.r, s.x + s.r].map((x) => [x, y, s.z])));
  const verticalSpan = (distance, targetY) => {
    const camera = cameraAt(distance, targetY, pitch, aspect, fovY);
    const ys = points.map((p) => projectTo(p, camera)[1]);
    return [Math.min(...ys), Math.max(...ys)];
  };
  const centeredTarget = (distance) => {
    let lo = -2;
    let hi = 3;
    for (let i = 0; i < SEARCH_STEPS; i += 1) {
      const mid = (lo + hi) / 2;
      const [low, high] = verticalSpan(distance, mid);
      if (low + high > 0) lo = mid;
      else hi = mid;
    }
    return (lo + hi) / 2;
  };
  const fitsAt = (distance) => {
    const targetY = centeredTarget(distance);
    const camera = cameraAt(distance, targetY, pitch, aspect, fovY);
    return points.every((p) => {
      const [x, y] = projectTo(p, camera);
      return Math.abs(x) <= fill && Math.abs(y) <= fill;
    });
  };
  let near = 0.5;
  let far = 40;
  for (let i = 0; i < SEARCH_STEPS; i += 1) {
    const mid = (near + far) / 2;
    if (fitsAt(mid)) far = mid;
    else near = mid;
  }
  return { distance: far, targetY: centeredTarget(far) };
}
