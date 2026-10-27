export function createFrameTimer(now = () => performance.now(), capacity = 240) {
  const samples = [];
  let last = null;
  return {
    tick() {
      const t = now();
      if (last !== null) {
        samples.push(t - last);
        if (samples.length > capacity) samples.shift();
      }
      last = t;
    },
    summary() {
      if (samples.length === 0) return { frames: 0, meanMs: 0, p95Ms: 0 };
      const sorted = [...samples].sort((a, b) => a - b);
      const meanMs = sorted.reduce((s, v) => s + v, 0) / sorted.length;
      const p95Ms = sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * 0.95))];
      return { frames: sorted.length, meanMs, p95Ms };
    },
  };
}
