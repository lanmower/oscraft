// The pure rules of a data swap, shared by play.js and js/wire/test.js (no DOM, no session).
//
// The host announces a swap at tick T (SWAP_LEAD_TICKS ahead). Each peer fetches and verifies the image, then sends
// SWAP_READY. The host gates the swap at T - SWAP_GATE_TICKS: it commits only when every connected peer has reported
// ready; otherwise it re-announces at T + SWAP_DEFER_TICKS, at most SWAP_MAX_DEFERS times, and then cancels.

export const SWAP_LEAD_TICKS = 32;
export const SWAP_GATE_TICKS = 8;
export const SWAP_DEFER_TICKS = 32;
export const SWAP_MAX_DEFERS = 3;

// A SWAP may apply only if its tick is after the last tick this peer applied (null before any swap). A repeated or older
// SWAP is ignored, so no peer applies one swap twice.
export function swapIsNew(lastAppliedTick, tick) {
  return lastAppliedTick === null || tick > lastAppliedTick;
}

// The host's decision for a pending swap at sim tick `ticks`:
//   "wait"   before the gate tick, or nothing to decide yet;
//   "commit" every connected peer has reported ready for this swap;
//   "defer"  a peer is missing and the swap may still move (re-announce at tick + SWAP_DEFER_TICKS);
//   "cancel" a peer is still missing after SWAP_MAX_DEFERS deferrals.
// `pending` is { tick, deferrals, readyPeers: Set }, `peers` is the set of connected peer keys.
export function swapGate(ticks, pending, peers) {
  if (ticks < pending.tick - SWAP_GATE_TICKS) return "wait";
  for (const key of peers) if (!pending.readyPeers.has(key)) return pending.deferrals < SWAP_MAX_DEFERS ? "defer" : "cancel";
  return "commit";
}

// What a SWAP names, by its file: "lua" (the pack's Lua mod set reloads), "data" (the map image swaps: the manifest, the
// map image named by `image`, and the pack's data files), or null (a hot-reloaded page module, or an asset, which applies
// at once and never swaps).
export function packSwapKind(file, image = null) {
  if (image !== null && file === image) return "data";
  if (/\.lua$/i.test(file)) return "lua";
  if (file === "manifest.json" || /\.(osd|json|tsv|txt)$/i.test(file)) return "data";
  return null;
}
