const PING_GLYPHS = Object.freeze({
  ping: Object.freeze({ name: "Ping", circle: 5 }),
  danger: Object.freeze({ name: "Danger", points: Object.freeze([[8, 1], [15, 14.5], [1, 14.5]]) }),
  on_my_way: Object.freeze({ name: "On my way", points: Object.freeze([[1, 5.5], [9, 5.5], [9, 1.5], [15, 8], [9, 14.5], [9, 10.5], [1, 10.5]]) }),
  item: Object.freeze({ name: "Item ready", points: Object.freeze([[8, 1], [15, 8], [8, 15], [1, 8]]) }),
});

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
