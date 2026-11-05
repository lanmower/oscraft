export const LOBBY_LINK_WAIT_MS = 8000;

export function lobbyInputRoute({ netState, hosting, peers, settled, name, call, phase }) {
  if (netState || (!hosting && peers > 0)) return { apply: false, broadcast: true, queue: false };
  const queue = !hosting && !settled && name === "ui" && call !== "world_ready" && phase === "lobby";
  return { apply: true, broadcast: false, queue };
}

export function heldCallsAfterSettle({ peers }) {
  return peers > 0 ? "keep" : "clear";
}
