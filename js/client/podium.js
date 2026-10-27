// Podium: the pre-match 3D showcase. The two teams' picked heroes stand in
// two facing rows on plinths near map centre, drawn by the same renderer3d
// unit path as in-game units, until the game goes live (play.js builds the
// draw list from these entries and ends the podium on goLive, restoring
// the gameplay camera).

import { multiply, translation, rotationY, scaling } from "./render3d.js";

const PODIUM_HALF_SPAN_X = 4.5;
const ROW_SPAN_Z = 2.5;
export const PLINTH = { w: 2, d: 2, h: 0.3 };

export function makeBoxMesh(w, d, h) {
  const x = w / 2, z = d / 2, y0 = 0, y1 = h;
  const quad = (a, b, c, d2) => ({ a, b, c, d: d2 });
  const sides = [
    quad([-x, y0, -z], [x, y0, -z], [x, y1, -z], [-x, y1, -z]),
    quad([-x, y0, z], [x, y0, z], [x, y1, z], [-x, y1, z]),
    quad([-x, y0, -z], [-x, y0, z], [-x, y1, z], [-x, y1, -z]),
    quad([x, y0, -z], [x, y0, z], [x, y1, z], [x, y1, -z]),
  ];
  const positions = [], normals = [], indices = [];
  for (const side of sides) {
    const base = positions.length / 3;
    const corners = [side.a, side.b, side.c, side.d];
    const n = side.a[0] === side.b[0]
      ? (side.a[0] < 0 ? [-1, 0, 0] : [1, 0, 0])
      : (side.a[2] < 0 ? [0, 0, -1] : [0, 0, 1]);
    for (let i = 0; i < 4; i += 1) normals.push(...n);
    for (const corner of corners) positions.push(...corner);
    indices.push(base, base + 1, base + 2, base, base + 2, base + 3);
  }
  const top = [[-x, y1, -z], [x, y1, -z], [x, y1, z], [-x, y1, z]];
  const base = positions.length / 3;
  for (const corner of top) { positions.push(...corner); normals.push(0, 1, 0); }
  indices.push(base, base + 1, base + 2, base, base + 2, base + 3);
  return { positions: new Float32Array(positions), normals: new Float32Array(normals), indices: new Uint16Array(indices) };
}

export function createPodium() {
  return {
    active: false,
    plinth: null,
    plinthHeight: 0,
    entries: [],
    yaw: 0.6,
    cameraTarget: null,
    cameraDistance: 11,
  };
}

// teams: [{ team, type }] in team order. Every entry carries its team
// colour and slot; play.js fills e.mesh from the loaded unit meshes and
// e.joints from the rest pose when it draws.
export function podiumScene(podium, teams, opts = {}) {
  const centre = opts.centre ?? [0, 0, 0];
  const rows = new Map();
  teams.forEach((entry) => {
    const row = rows.get(entry.team) ?? [];
    row.push(entry);
    rows.set(entry.team, row);
  });
  const entries = [];
  for (const [team, members] of rows) {
    const span = ((members.length - 1) * ROW_SPAN_Z) / 2;
    members.forEach((m, i) => {
      const x = centre[0] + (team === 0 ? -PODIUM_HALF_SPAN_X : PODIUM_HALF_SPAN_X);
      const z = centre[2] + span - i * ROW_SPAN_Z;
      const y = opts.groundY ? opts.groundY(x, z) : 0;
      entries.push({ ...m, x, z, y, facing: team === 0 ? Math.PI / 2 : -Math.PI / 2 });
    });
  }
  podium.entries = entries;
  podium.plinth = makeBoxMesh(PLINTH.w, PLINTH.d, PLINTH.h);
  podium.plinthHeight = PLINTH.h;
  podium.cameraTarget = [centre[0], (centre[1] ?? 0) + 1.2, centre[2]];
  podium.active = entries.length > 0;
  return podium;
}

// Draw the already-assembled entries (mesh, joints, scale set by play.js).
export function podiumDraw(renderer3d, podium) {
  let drawn = 0;
  for (const e of podium.entries) {
    if (podium.plinth) {
      renderer3d.drawMesh(e.plinthMesh ?? podium.plinth, new Float32Array(translation(e.x, e.y, e.z)));
    }
    const standY = e.y + podium.plinthHeight;
    const stand = multiply(multiply(translation(e.x, standY, e.z), rotationY(e.facing)), scaling(e.scale ?? 1, e.scale ?? 1, e.scale ?? 1));
    const transform = new Float32Array(stand);
    if (e.joints) renderer3d.drawSkinned(e.mesh, transform, e.joints);
    else renderer3d.drawMesh(e.mesh, transform);
    drawn += 1;
  }
  return drawn;
}

export function podiumCameraStep(podium, dtMs) {
  if (!podium.cameraTarget) return null;
  podium.yaw += (dtMs / 1000) * 0.08;
  const yaw = Math.sin(podium.yaw) * 0.45;
  const step = { target: podium.cameraTarget, distance: podium.cameraDistance, yaw, pitch: 0.72 };
  return step;
}

// Stands placed upright on the ground under each hero, one draw per entry.
export function podiumPlinthTransforms(podium) {
  return podium.entries.map((e) => new Float32Array(translation(e.x, e.y, e.z)));
}

export default { createPodium, podiumScene, podiumDraw, podiumCameraStep };
