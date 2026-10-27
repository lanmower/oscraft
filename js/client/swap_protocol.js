
export const SWAP_LEAD_TICKS = 32;
export const SWAP_GATE_TICKS = 8;
export const SWAP_DEFER_TICKS = 32;
export const SWAP_MAX_DEFERS = 3;

export function swapIsNew(lastAppliedTick, tick) {
  return lastAppliedTick === null || tick > lastAppliedTick;
}

export function swapGate(ticks, pending, peers) {
  if (ticks < pending.tick - SWAP_GATE_TICKS) return "wait";
  for (const key of peers) if (!pending.readyPeers.has(key)) return pending.deferrals < SWAP_MAX_DEFERS ? "defer" : "cancel";
  return "commit";
}

export function packSwapKind(file, image = null) {
  if (image !== null && file === image) return "data";
  if (/\.lua$/i.test(file)) return "lua";
  if (/(^|\/)attr_rules\.txt$/i.test(file)) return "rules";
  if (file === "manifest.json" || /\.(osd|json|tsv|txt)$/i.test(file)) return "data";
  return null;
}
