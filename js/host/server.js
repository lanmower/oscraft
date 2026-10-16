#!/usr/bin/env node
// Oscraft net host: runs one persistent wasip1 guest and bridges it to peers over wireweave DataSession.
// Heavy deps load only after argument parsing, so --help works with no node_modules installed.
import { readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { replyValue } from './guest.js';

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

function emit(ev, fields = {}) {
  const tail = Object.entries(fields).map(([k, v]) => `${k}=${v}`).join(' ');
  console.log(tail ? `ev=${ev} ${tail}` : `ev=${ev}`);
}

export const PLAYER_SLOTS = 256;

// Assigns the lowest player id no other peer holds. Returns null when every slot is taken.
export function assignPlayer(players, peerPubkey) {
  if (players.has(peerPubkey)) return players.get(peerPubkey);
  const taken = new Set(players.values());
  for (let p = 0; p < PLAYER_SLOTS; p++) {
    if (!taken.has(p)) {
      players.set(peerPubkey, p);
      return p;
    }
  }
  return null;
}

// Applies a peer's order only when the unit's owner (guest unit-player) is the peer's player.
// Resolves 'applied' or 'refused'; a refusal sends nothing to the guest.
export async function applyOrder({ guest, players, peerPubkey, unitId, x, y }) {
  const player = players.get(peerPubkey);
  const owner = player === undefined ? null : Number(replyValue(await guest.command(`unit-player ${unitId}`), 'ok unit-player '));
  if (player === undefined || owner !== player) {
    emit('order_refused', { reason: 'not_owner', peer: peerPubkey.slice(0, 8), unit: unitId });
    return 'refused';
  }
  await guest.command(`order ${unitId} ${x} ${y}`);
  return 'applied';
}

async function main(opts) {
  const wire = await import('wireweave');
  const NostrTools = await import('nostr-tools');
  const xstate = await import('xstate');
  const ndc = await import('node-datachannel');
  const { RTCPeerConnection: PolyfillRTCPeerConnection, RTCSessionDescription, RTCIceCandidate } = await import('node-datachannel/polyfill');
  // wireweave reads the WebRTC constructors as globals when it applies signals; Node has none.
  globalThis.RTCSessionDescription ??= RTCSessionDescription;
  globalThis.RTCIceCandidate ??= RTCIceCandidate;
  const { encodeFrame, decodeFrame, FRAME } = await import('../wire/frame.js');
  const { openGuest, readTick } = await import('./guest.js');

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
  const players = new Map();
  session.addEventListener('peer-open', (e) => emit('peer_open', { peer: e.detail.peerPubkey, player: assignPlayer(players, e.detail.peerPubkey) }));
  session.addEventListener('peer-close', (e) => {
    const peer = session.peers.get(e.detail.peerPubkey);
    const sdp = peer?.pc?.remoteDescription?.sdp ?? '';
    emit('peer_close', { peer: e.detail.peerPubkey, ice: peer?.pc?.iceConnectionState ?? 'gone', conn: peer?.pc?.connectionState ?? 'gone', remote_candidates: (sdp.match(/a=candidate/g) || []).length, remote_mdns: sdp.split('.local').length - 1 });
  });
  session.addEventListener('data', (e) => {
    const f = decodeFrame(e.detail.data);
    if (f.type === FRAME.ORDER) {
      applyOrder({ guest, players, peerPubkey: e.detail.peerPubkey, unitId: f.unitId, x: f.x, y: f.y })
        .catch((err) => emit('order_failed', { error: JSON.stringify(err.message) }));
    }
  });

  let tick = 0;
  let busy = false;
  const timer = setInterval(async () => {
    if (busy) return;
    busy = true;
    try {
      tick += 1;
      const { hash, count, units } = await readTick(guest);
      if (tick % opts.every === 0) {
        session.broadcast(encodeFrame({ type: FRAME.STATE, tick, hash: Number.parseInt(hash.slice(-8), 16) >>> 0, units }));
      }
      emit('tick', { n: tick, units: count });
      if (tick % 20 === 0) {
        for (const [pk, peer] of session.peers) {
          const pc = peer.pc;
          emit('pc', { peer: pk.slice(0, 8), ice: pc?.iceConnectionState, conn: pc?.connectionState, local: pc?.localDescription?.type ?? 'none', remote: pc?.remoteDescription?.type ?? 'none', dc: peer.dc?.readyState ?? 'none' });
        }
      }
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

// Runs the host only when this file is the entry point, so the CI job can import the gate.
const isEntry = process.argv[1] !== undefined && import.meta.url === pathToFileURL(process.argv[1]).href;
if (isEntry) {
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
}
