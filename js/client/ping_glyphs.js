export const PING_GLYPHS = Object.freeze({
  ping: Object.freeze({ name: "Ping", circle: 5 }),
  danger: Object.freeze({ name: "Danger", points: Object.freeze([[8, 1], [15, 14.5], [1, 14.5]]) }),
  on_my_way: Object.freeze({ name: "On my way", points: Object.freeze([[1, 5.5], [9, 5.5], [9, 1.5], [15, 8], [9, 14.5], [9, 10.5], [1, 10.5]]) }),
  item: Object.freeze({ name: "Item ready", points: Object.freeze([[8, 1], [15, 8], [8, 15], [1, 8]]) }),
  attack: Object.freeze({ name: "Attack here", points: Object.freeze([[8, 1], [14, 8], [10.5, 8], [10.5, 15], [5.5, 15], [5.5, 8], [2, 8]]) }),
  help: Object.freeze({ name: "Need help", points: Object.freeze([[6, 1], [10, 1], [10, 6], [15, 6], [15, 10], [10, 10], [10, 15], [6, 15], [6, 10], [1, 10], [1, 6], [6, 6]]) }),
  retreat: Object.freeze({ name: "Retreat", points: Object.freeze([[8, 15], [1, 8], [4, 5], [8, 9], [12, 5], [15, 8]]) }),
  vision: Object.freeze({ name: "Watch here", points: Object.freeze([[2, 2], [14, 2], [14, 14], [2, 14]]) }),
  missing: Object.freeze({ name: "Missing", points: Object.freeze([[8, 1], [15, 6], [12, 15], [4, 15], [1, 6]]) }),
});

export const WHEEL_KINDS = Object.freeze(["ping", "danger", "on_my_way", "attack", "help", "retreat", "vision", "missing"]);

export function hasPingGlyph(kind) {
  return Object.hasOwn(PING_GLYPHS, kind);
}

export function pingGlyph(kind) {
  return hasPingGlyph(kind) ? PING_GLYPHS[kind] : PING_GLYPHS.ping;
}

export function pingGlyphSvg(kind) {
  const glyph = pingGlyph(kind);
  const shape = glyph.points
    ? `<polygon points="${glyph.points.map(([x, y]) => `${x},${y}`).join(" ")}" fill="currentColor"/>`
    : `<circle cx="8" cy="8" r="${glyph.circle}" fill="currentColor"/>`;
  return `<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">${shape}</svg>`;
}
