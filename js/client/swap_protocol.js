
export const SWAP_LEAD_TICKS = 32;
export const SWAP_GATE_TICKS = 8;
export const SWAP_DEFER_TICKS = 32;
export const SWAP_MAX_DEFERS = 3;

export function swapIsNew(lastAppliedTick, tick) {
  return lastAppliedTick === null || tick > lastAppliedTick;
}

export function swapImageState(swap) {
  if (!swap) return "unknown";
  if (swap.image) return "ready";
  if (swap.error) return "failed";
  return "in-flight";
}

export function swapGate(ticks, pending, peers) {
  if (ticks < pending.tick - SWAP_GATE_TICKS) return "wait";
  const state = swapImageState(pending);
  if (state === "failed") return "cancel";
  const unready = state === "in-flight" || [...peers].some((key) => !pending.readyPeers.has(key));
  if (!unready) return "commit";
  return pending.deferrals < SWAP_MAX_DEFERS ? "defer" : "cancel";
}

export function hostSwapStep(pending, ticks, peers) {
  const action = swapGate(ticks, pending, peers);
  if (action === "wait") return { action, pending };
  if (action === "commit") return { action, pending: { ...pending, decided: true } };
  if (action === "defer") {
    return {
      action,
      pending: { ...pending, tick: pending.tick + SWAP_DEFER_TICKS, deferrals: pending.deferrals + 1, error: null, readyPeers: new Set(), decided: false },
    };
  }
  return { action, pending: null };
}

export function swapAdmitPeer(pending, key) {
  const readyPeers = new Set(pending.readyPeers);
  readyPeers.delete(key);
  return { ...pending, readyPeers, decided: false };
}

export function packSwapKind(file, image = null) {
  if (image !== null && file === image) return "data";
  if (/\.lua$/i.test(file)) return "lua";
  if (/(^|\/)attr_rules\.txt$/i.test(file)) return "rules";
  if (file === "manifest.json" || /\.(osd|json|tsv|txt)$/i.test(file)) return "data";
  return null;
}
