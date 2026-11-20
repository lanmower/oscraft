export const MATCH_ROOM_PREFIX = "match-";

export function chooseRoom(rooms, maxPlayers, nonce) {
  const open = rooms.filter((room) => room.players > 0 && room.players < maxPlayers);
  if (open.length === 0) return MATCH_ROOM_PREFIX + nonce;
  open.sort((a, b) => b.players - a.players || a.name.localeCompare(b.name));
  return open[0].name;
}

export const DEFAULT_ROOM = "lobby";

export function matchmakingTarget(hash, rooms, maxPlayers, nonce) {
  if (hash.length > 0) return null;
  return chooseRoom(rooms.filter((room) => room.name !== DEFAULT_ROOM), maxPlayers, nonce);
}

export async function settleMatch(hash, rooms, maxPlayers, nonce, announce, go) {
  const target = matchmakingTarget(hash, rooms, maxPlayers, nonce);
  if (target === null) return;
  await announce(target);
  go(target);
}
