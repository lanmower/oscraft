export function keyNameOf(e) {
  const pad = /^Numpad([0-9])$/.exec(e.code || "");
  if (pad) return `num${pad[1]}`;
  return e.key === " " ? "space" : e.key.toLowerCase();
}

export function zoomKeyName(e) {
  return e.key === " " ? "space" : e.key.toLowerCase();
}
