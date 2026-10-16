#!/usr/bin/env node
// Oscraft net host: runs one persistent wasip1 guest and bridges it to peers over wireweave DataSession.
// Heavy deps load only after argument parsing, so --help works with no node_modules installed.
import { readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';

const USAGE = 'usage: node js/host/server.js --wasm <oscraft_wasip1.wasm> --data <image file or dir> --room <name> [--every <ticks>] [--tick-ms <ms>]';

function parseArgs(argv) {
  const out = { every: 1, tickMs: 100, room: process.env.OSCRAFT_ROOM || 'lobby', help: false, relays: null, namespace: process.env.OSCRAFT_NAMESPACE || 'oscraft' };
  if (process.env.OSCRAFT_RELAYS) out.relays = process.env.OSCRAFT_RELAYS.split(',').map((s) => s.trim()).filter(Boolean);
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

// A probe reply is the guest's line for one unit command; null when the guest reports the unit out of range.
function probeValue(line, prefix) {
  return line && line.startsWith(prefix) ? line.slice(prefix.length).trim() : null;
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
  const { encodeFrame, decodeFrame, FRAME } = await import('../wire/frame.js');
  const { openGuest, replyValue } = await import('./guest.js');

  const image = loadImage(opts.data);
  const guest = await openGuest(opts.wasm, image);

  const storage = new Map();
  const storageAdapter = {
    getItem: (k) => (storage.has(k) ? storage.get(k) : null),
    setItem: (k, v) => storage.set(k, String(v)),
    removeItem: (k) => storage.delete(k),
  };
  const fsm = wire.createFSM(xstate);
  const auth = new wire.NostrAuth({ nostrTools: NostrTools, storage: storageAdapter });
  auth.loadFromStorage() || auth.generateKey();
  const pool = new wire.RelayPool({
    verifyEvent: NostrTools.verifyEvent,
    ...(opts.relays ? { relays: opts.relays, fallbackRelays: [], autoRotate: false } : {}),
  });
  pool.connect();

  const session = wire.createDataSession({
    fsm, xstate, relayPool: pool, auth, namespace: opts.namespace,
    iceServers: opts.relays ? [] : null,
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
    if (f.type === FRAME.ORDER) {
      guest.command(`order ${f.unitId} ${f.x} ${f.y}`).catch((err) => emit('order_failed', { error: JSON.stringify(err.message) }));
    }
  });

  let tick = 0;
  let busy = false;
  let knownUnits = 0;
  const timer = setInterval(async () => {
    if (busy) return;
    busy = true;
    try {
      tick += 1;
      const probes = [];
      for (let i = 0; i < knownUnits; i++) {
        probes.push(guest.command(`unit-position ${i}`), guest.command(`unit-player ${i}`), guest.command(`unit-hp ${i}`));
      }
      const [, unitsLine, hashLine, ...probeLines] = await Promise.all([
        guest.command('tick 1'),
        guest.command('unit-count'),
        guest.command('state-hash'),
        ...probes,
      ]);
      const units = Number(replyValue(unitsLine, 'ok unit-count '));
      const hash = replyValue(hashLine, 'ok state-hash ');
      if (tick % opts.every === 0) {
        const wireUnits = [];
        for (let i = 0; i < knownUnits; i++) {
          const pos = probeValue(probeLines[3 * i], 'ok unit-position ');
          const player = probeValue(probeLines[3 * i + 1], 'ok unit-player ');
          const hp = probeValue(probeLines[3 * i + 2], 'ok unit-hp ');
          if (pos === null || player === null || hp === null) continue;
          const [x, y] = pos.split(/\s+/).map(Number);
          wireUnits.push({ id: i, kind: 0, player: Number(player), x: Math.round(x), y: Math.round(y), hp: Math.round(Number(hp)) });
        }
        session.broadcast(encodeFrame({ type: FRAME.STATE, tick, hash: Number.parseInt(hash.slice(-8), 16) >>> 0, units: wireUnits }));
      }
      knownUnits = units;
      emit('tick', { n: tick, units });
    } catch (err) {
      emit('tick_failed', { n: tick, error: JSON.stringify(err.message) });
    } finally {
      busy = false;
    }
  }, opts.tickMs);

  await session.connect(opts.room);
  emit('host_started', { room: opts.room, wasm: opts.wasm, image_bytes: image.length });

  const stop = async () => {
    clearInterval(timer);
    try { session.disconnect?.(); } catch { /* best effort on shutdown */ }
    await guest.close();
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
