
const TAU = 2 * Math.PI;

function pushQuad(idx, a, b, c, d) {
  idx.push(a, b, c, a, c, d, a, c, b, a, d, c);
}

export function barGeometry() {
  const positions = [0, -1, 0, 1, -1, 0, 1, 1, 0, 0, 1, 0, 0, 0, -1, 1, 0, -1, 1, 0, 1, 0, 0, 1];
  const indices = [];
  pushQuad(indices, 0, 1, 2, 3);
  pushQuad(indices, 4, 5, 6, 7);
  return { positions, indices };
}

export function cylinderGeometry(segments = 24) {
  const positions = [];
  const indices = [];
  for (let i = 0; i < segments; i++) {
    const a = (i / segments) * TAU;
    positions.push(Math.cos(a), 0, Math.sin(a), Math.cos(a), 1, Math.sin(a));
    const j = (i + 1) % segments;
    pushQuad(indices, 2 * i, 2 * j, 2 * j + 1, 2 * i + 1);
  }
  return { positions, indices };
}

export function coneGeometry(segments = 20) {
  const positions = [0, 0, 0];
  const indices = [];
  for (let i = 0; i < segments; i++) {
    const a = (i / segments) * TAU;
    positions.push(1, Math.cos(a), Math.sin(a));
    const j = (i + 1) % segments;
    indices.push(0, 1 + i, 1 + j, 0, 1 + j, 1 + i);
  }
  return { positions, indices };
}

export function discGeometry(segments = 32) {
  const positions = [0, 0, 0];
  const indices = [];
  for (let i = 0; i < segments; i++) {
    const a = (i / segments) * TAU;
    positions.push(Math.cos(a), 0, Math.sin(a));
    const j = (i + 1) % segments;
    indices.push(0, 1 + i, 1 + j, 0, 1 + j, 1 + i);
  }
  return { positions, indices };
}
