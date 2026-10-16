#!/usr/bin/env node
// Oscraft net host: runs the wasip1 guest and bridges it to peers over wireweave DataSession.
// Heavy deps load only after argument parsing, so --help works with no node_modules installed.
import { readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';

const USAGE = 'usage: node js/host/server.js --wasm <oscraft_wasip1.wasm> --data <image file or dir> --room <name> [--every <ticks>] [--tick-ms <ms>]';

function parseArgs(argv) {
  const out = { every: 1, tickMs: 100, room: process.env.OSCRAFT_ROOM || 'lobby', help: false };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--help' || a === '-h') out.help = true;
    else if (a === '--wasm') out.wasm = argv[++i];
    else if (a === '--data') out.data = argv[++i];
    else if (a === '--room') out.room = argv[++i];
    else if (a === '--every') out.every = Number(argv[++i]);
    else if (a === '--tick-ms') out.tickMs = Number(argv[++i]);
    else throw new Error(`unknown argument ${a}`);
  }
  return out;
}

function loadImage(dataPath) {
  const file = statSync(dataPath).isDirectory() ? join(dataPath, 'image.osdt') : dataPath;
  return readFileSync(file);
}

function emit(ev, fields = {}) {
  const tail = Object.entries(fields).map(([k, v]) => `${k}=${v}`).join(' ');
  console.log(tail ? `ev=${ev} ${tail}` : `ev=${ev}`);
}

async function main(opts) {
  const wire = await import('wireweave');
  const NostrTools = await import('nostr-tools');
  const xstate = await import('xstate');
  const ndc = await import('node-datachannel');
  const { RTCPeerConnection: PolyfillRTCPeerConnection } = await import('node-datachannel/polyfill');
  const { encodeState, decodeFrame } = await import('./frame.js');
  const { loadGuest, runGuest, lastReply } = await import('./guest.js');

  const image = loadImage(opts.data);
  const module = await loadGuest(opts.wasm);
  const journal = [];

  const storage = new Map();
  const storageAdapter = {
    getItem: (k) => (storage.has(k) ? storage.get(k) : null),
    setItem: (k, v) => storage.set(k, String(v)),
    removeItem: (k) => storage.delete(k),
  };
  const fsm = wire.createFSM(xstate);
  const auth = new wire.NostrAuth({ nostrTools: NostrTools, storage: storageAdapter });
  auth.loadFromStorage() || auth.generateKey();
  const pool = new wire.RelayPool({ verifyEvent: NostrTools.verifyEvent });
  pool.connect();

  const session = wire.createDataSession({
    fsm, xstate, relayPool: pool, auth, namespace: 'oscraft',
    createPeerConnection: (config) => {
      const nativePc = new ndc.PeerConnection('peer', {
        iceServers: config.iceServers.map((s) => s.urls),
        enableIceUdpMux: true,
      });
      return new PolyfillRTCPeerConnection({ peerConnection: nativePc });
    },
  });
  session.addEventListener('peer-open', (e) => emit('peer_open', { peer: e.detail.peerPubkey }));
  session.addEventListener('peer-close', (e) => emit('peer_close', { peer: e.detail.peerPubkey }));
  session.addEventListener('data', (e) => {
    const f = decodeFrame(e.detail.data);
    if (f && f.kind === 'order') journal.push(`order ${f.unit} ${f.x} ${f.y}`);
  });

  let tick = 0;
  const timer = setInterval(() => {
    tick += 1;
    journal.push('tick 1');
    const lines = runGuest(module, image, [...journal, 'unit-count', 'state-hash']);
    const units = Number(lastReply(lines, 'ok unit-count ') ?? 0);
    const hash = lastReply(lines, 'ok state-hash ') ?? '';
    if (tick % opts.every === 0) {
      session.broadcast(encodeState({ tick, units, hash }));
    }
    emit('tick', { n: tick, units });
  }, opts.tickMs);

  await session.connect(opts.room);
  emit('host_started', { room: opts.room, wasm: opts.wasm, image_bytes: image.length });

  const stop = () => {
    clearInterval(timer);
    try { session.disconnect?.(); } catch { /* best effort on shutdown */ }
    process.exit(0);
  };
  process.once('SIGINT', stop);
  process.once('SIGTERM', stop);
}

const opts = parseArgs(process.argv.slice(2));
if (opts.help) {
  console.log(USAGE);
  process.exit(0);
}
if (!opts.wasm || !opts.data) {
  console.error(USAGE);
  process.exit(2);
}
main(opts).catch((err) => {
  console.error(err);
  process.exit(1);
});
