export function chainFrame(step, schedule, onError) {
  try { step(); } catch (err) { onError(err); } finally { schedule(); }
}
