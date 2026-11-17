export const LOBBY_LINK_WAIT_MS = 8000;

export function lobbyInputRoute({ netState, hosting, peers, settled, name, call, phase }) {
  if (netState || (!hosting && peers > 0)) return { apply: false, broadcast: true, queue: false };
  const queue = !hosting && !settled && name === "ui" && call !== "world_ready" && phase === "lobby";
  return { apply: true, broadcast: false, queue };
}

export function heldCallsAfterSettle({ peers }) {
  return peers > 0 ? "keep" : "clear";
}

export function replayHeldCalls(held, { hasHost }) {
  if (!hasHost) return [];
  return held.splice(0);
}

export function worldReadyRoute({ hosting, peer, built, phase, confirmedFor, game, lobbyHost, me }) {
  const send = !peer && built && !!game && phase === "lobby" && confirmedFor !== game && (hosting || lobbyHost === me);
  return { send, registerHost: send && hosting && lobbyHost !== me };
}

export function relaysUiEvent(ev) {
  return !(ev.kind === "widget" && ev.data?.id === "lobby");
}

export function mirrorsHostLobby({ hosting, relayed, kind, started }) {
  return !hosting && relayed === true && kind === "lobby" && !started;
}
