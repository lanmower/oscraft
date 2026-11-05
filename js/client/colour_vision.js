const SETTING_KEY = "oscraft.colourVision";

function storedName(storage) {
  try {
    return storage?.getItem(SETTING_KEY) ?? null;
  } catch {
    return null;
  }
}

function defaultStorage() {
  try {
    return globalThis.localStorage ?? null;
  } catch {
    return null;
  }
}

export function requestedColourVision(search = globalThis.location?.search ?? "", storage = defaultStorage()) {
  return new URLSearchParams(search).get("cv") ?? storedName(storage);
}

export function activePalette(ui, search = globalThis.location?.search ?? "", storage = defaultStorage()) {
  const vision = ui?.colourVision ?? {};
  const palettes = vision.palettes ?? {};
  const requested = requestedColourVision(search, storage);
  const name = requested !== null && palettes[requested] ? requested : vision.order?.[0];
  return { name, ...(palettes[name] ?? {}) };
}

export function rgbOfHex(hex) {
  const value = parseInt(hex.slice(1), 16);
  return [((value >> 16) & 255) / 255, ((value >> 8) & 255) / 255, (value & 255) / 255];
}

export function seatRampHex(palette, player) {
  return palette.teams?.[player % 2]?.[player >> 1] ?? null;
}
