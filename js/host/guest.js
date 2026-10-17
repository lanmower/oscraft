// Native per-tick guest sim: decodes one ORDER frame per simulated peer each tick and applies it to unit state.
// Usage: node js/host/guest.js [--ticks=N] [--reps=N] [--peers=1,8,32] [--inject-superlinear]
// Prints one line per peer count, then RESULT: PASS|FAIL. Exit 0 on PASS, 1 on FAIL, 3 on a codec error.
import { encodeFrame, decodeFrame, FRAME } from '../wire/frame.js';

const CEILING = 1.5;
const args = Object.fromEntries(process.argv.slice(2).map((a) => {
  const [k, v] = a.replace(/^--/, '').split('=');
  return [k, v === undefined ? true : v];
}));
const TICKS = Number(args.ticks ?? 5000);
const REPS = Number(args.reps ?? 5);
const PEERS = String(args.peers ?? '1,8,32').split(',').map(Number);
const INJECT = Boolean(args['inject-superlinear']);

function peerFrames(n) {
  return Array.from({ length: n }, (_, i) => encodeFrame({ type: FRAME.ORDER, unitId: i + 1, x: i, y: -i }));
}

function runTicks(frames) {
  const n = frames.length;
  let sink = 0;
  const t0 = performance.now();
  for (let tick = 0; tick < TICKS; tick++) {
    const state = new Map();
    for (let i = 0; i < n; i++) {
      const f = decodeFrame(frames[i]);
      state.set(f.unitId, { x: f.x, y: f.y });
    }
    if (INJECT) for (let k = 0; k < n * n * 64; k++) sink += k;
    sink += state.size;
  }
  if (sink < 0) console.log(sink);
  return (performance.now() - t0) / TICKS;
}

function median(xs) {
  const s = [...xs].sort((a, b) => a - b);
  return s[Math.floor(s.length / 2)];
}

const perPeer = {};
try {
  for (const n of PEERS) {
    const frames = peerFrames(n);
    const samples = [];
    for (let r = 0; r < REPS; r++) samples.push(runTicks(frames));
    const perTick = median(samples);
    perPeer[n] = perTick / n;
    console.log(`peers=${n} ticks=${TICKS} per_tick_ms=${perTick.toFixed(5)} per_peer_ms=${perPeer[n].toFixed(7)} decodes_per_tick=${n}`);
  }
} catch (e) {
  console.log(`RESULT: REFUSED codec error: ${e.message}`);
  process.exit(3);
}

const base = PEERS[0];
const top = PEERS[PEERS.length - 1];
const ratio = perPeer[top] / perPeer[base];
console.log(`ratio=${ratio.toFixed(3)} base_peers=${base} top_peers=${top} ceiling=${CEILING}`);
if (ratio <= CEILING) {
  console.log(`RESULT: PASS per-peer ratio ${ratio.toFixed(3)} <= ${CEILING}`);
} else {
  console.log(`RESULT: FAIL per-peer ratio ${ratio.toFixed(3)} > ${CEILING}`);
  process.exit(1);
}
