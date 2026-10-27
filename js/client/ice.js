const DEFAULT_ICE_SERVERS = Object.freeze([
  Object.freeze({ urls: 'stun:stun.l.google.com:19302' }),
  Object.freeze({ urls: 'stun:stun1.l.google.com:19302' }),
]);

const TURN = /^turns?:/;

export function iceServersFrom(raw) {
  if (raw === null || raw === undefined || raw === '') {
    return { servers: DEFAULT_ICE_SERVERS.map((s) => ({ ...s })), default: true };
  }
  let list;
  try {
    list = JSON.parse(raw);
  } catch (err) {
    throw new Error(`ICE config is not JSON: ${err.message}`);
  }
  if (!Array.isArray(list)) throw new Error('ICE config must be a JSON list');
  for (const entry of list) {
    const urls = entry && entry.urls;
    const all = typeof urls === 'string' ? [urls] : urls;
    if (!Array.isArray(all) || all.length === 0 || !all.every((u) => typeof u === 'string')) {
      throw new Error('ICE entry needs urls: a string or a non-empty list of strings');
    }
    if (all.some((u) => TURN.test(u))) throw new Error('TURN is refused: connections are direct, use stun: urls only');
  }
  return { servers: list, default: false };
}
