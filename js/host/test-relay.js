// Minimal Nostr relay for the net e2e job: NIP-01 EVENT, REQ and CLOSE over RFC 6455 text frames.
// Events live in memory. Replaceable (10000-19999) and parameterized replaceable (30000-39999)
// kinds keep only the newest event per key; ephemeral kinds (20000-29999) are forwarded, not stored.
// Usage: node js/host/test-relay.js [port]   (default port 7447, or RELAY_PORT)
import { createServer } from 'node:http';
import { createHash } from 'node:crypto';
import { pathToFileURL } from 'node:url';

const GUID = '258EAFA5-E914-47DA-95CA-C5AB0DC85B11';

function textFrame(text) {
  const payload = Buffer.from(text, 'utf8');
  let header;
  if (payload.length < 126) {
    header = Buffer.from([0x81, payload.length]);
  } else if (payload.length < 65536) {
    header = Buffer.alloc(4);
    header[0] = 0x81; header[1] = 126; header.writeUInt16BE(payload.length, 2);
  } else {
    header = Buffer.alloc(10);
    header[0] = 0x81; header[1] = 127; header.writeBigUInt64BE(BigInt(payload.length), 2);
  }
  return Buffer.concat([header, payload]);
}

function controlFrame(opcode, payload = Buffer.alloc(0)) {
  return Buffer.concat([Buffer.from([0x80 | opcode, payload.length]), payload]);
}

// Consumes complete client frames (always masked) from buf; returns the unconsumed tail.
function readFrames(buf, onFrame) {
  let at = 0;
  while (buf.length - at >= 2) {
    const opcode = buf[at] & 0x0f;
    const masked = (buf[at + 1] & 0x80) !== 0;
    let len = buf[at + 1] & 0x7f;
    let off = at + 2;
    if (len === 126) {
      if (buf.length < off + 2) break;
      len = buf.readUInt16BE(off); off += 2;
    } else if (len === 127) {
      if (buf.length < off + 8) break;
      len = Number(buf.readBigUInt64BE(off)); off += 8;
    }
    const maskLen = masked ? 4 : 0;
    if (buf.length < off + maskLen + len) break;
    const mask = masked ? buf.subarray(off, off + 4) : null;
    off += maskLen;
    const payload = Buffer.from(buf.subarray(off, off + len));
    if (mask) for (let i = 0; i < len; i++) payload[i] ^= mask[i & 3];
    onFrame(opcode, payload);
    at = off + len;
  }
  return buf.subarray(at);
}

const replaceKey = (ev) => {
  if (ev.kind === 0 || ev.kind === 3 || (ev.kind >= 10000 && ev.kind < 20000)) return `${ev.pubkey}:${ev.kind}`;
  if (ev.kind >= 30000 && ev.kind < 40000) {
    const d = ev.tags.find((t) => t[0] === 'd');
    return `${ev.pubkey}:${ev.kind}:${d ? d[1] : ''}`;
  }
  return null;
};

const matches = (ev, f) => {
  if (f.ids && !f.ids.some((id) => ev.id.startsWith(id))) return false;
  if (f.authors && !f.authors.some((a) => ev.pubkey.startsWith(a))) return false;
  if (f.kinds && !f.kinds.includes(ev.kind)) return false;
  if (f.since !== undefined && ev.created_at < f.since) return false;
  if (f.until !== undefined && ev.created_at > f.until) return false;
  for (const key of Object.keys(f)) {
    if (!key.startsWith('#')) continue;
    const name = key.slice(1);
    if (!ev.tags.some((t) => t[0] === name && f[key].includes(t[1]))) return false;
  }
  return true;
};

export function startRelay({ port = 0, host = '127.0.0.1' } = {}) {
  const events = new Map();
  const byKey = new Map();
  const subs = new Set();

  const send = (sock, msg) => { if (!sock.destroyed) sock.write(textFrame(JSON.stringify(msg))); };

  const deliver = (ev) => {
    for (const sub of subs) {
      if (sub.filters.some((f) => matches(ev, f))) send(sub.sock, ['EVENT', sub.id, ev]);
    }
  };

  const onEvent = (sock, ev) => {
    if (!ev || typeof ev.id !== 'string' || typeof ev.kind !== 'number') {
      return send(sock, ['OK', ev && ev.id ? ev.id : '', false, 'invalid: malformed event']);
    }
    if (events.has(ev.id)) return send(sock, ['OK', ev.id, true, 'duplicate: already have this event']);
    if (ev.kind >= 20000 && ev.kind < 30000) {
      send(sock, ['OK', ev.id, true, '']);
      return deliver(ev);
    }
    const key = replaceKey(ev);
    if (key) {
      const old = byKey.get(key);
      if (old && (old.created_at > ev.created_at || (old.created_at === ev.created_at && old.id < ev.id))) {
        return send(sock, ['OK', ev.id, true, 'duplicate: superseded by a newer event']);
      }
      if (old) events.delete(old.id);
      byKey.set(key, ev);
    }
    events.set(ev.id, ev);
    send(sock, ['OK', ev.id, true, '']);
    deliver(ev);
  };

  const onReq = (sock, subId, filters) => {
    const sub = { sock, id: subId, filters };
    subs.add(sub);
    for (const f of filters) {
      const hits = [...events.values()].filter((ev) => matches(ev, f)).sort((a, b) => b.created_at - a.created_at);
      const limited = f.limit !== undefined ? hits.slice(0, f.limit) : hits;
      for (const ev of limited) send(sock, ['EVENT', subId, ev]);
    }
    send(sock, ['EOSE', subId]);
  };

  const onMessage = (sock, text) => {
    let msg;
    try { msg = JSON.parse(text); } catch { return; }
    if (!Array.isArray(msg)) return;
    if (msg[0] === 'EVENT') onEvent(sock, msg[1]);
    else if (msg[0] === 'REQ') onReq(sock, String(msg[1]), msg.slice(2));
    else if (msg[0] === 'CLOSE') {
      for (const sub of subs) if (sub.sock === sock && sub.id === String(msg[1])) subs.delete(sub);
    }
  };

  const server = createServer((req, res) => {
    res.writeHead(200, { 'content-type': 'text/plain' });
    res.end('oscraft test relay\n');
  });

  server.on('upgrade', (req, sock) => {
    const accept = createHash('sha1').update(req.headers['sec-websocket-key'] + GUID).digest('base64');
    sock.write('HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n' +
      `Sec-WebSocket-Accept: ${accept}\r\n\r\n`);
    let pending = Buffer.alloc(0);
    sock.on('data', (chunk) => {
      pending = readFrames(Buffer.concat([pending, chunk]), (opcode, payload) => {
        if (opcode === 1) onMessage(sock, payload.toString('utf8'));
        else if (opcode === 9) sock.write(controlFrame(10, payload));
        else if (opcode === 8) sock.end(controlFrame(8));
      });
    });
    sock.on('close', () => { for (const sub of subs) if (sub.sock === sock) subs.delete(sub); });
    sock.on('error', () => {});
  });

  return new Promise((resolve) => {
    server.listen(port, host, () => resolve({ server, port: server.address().port, close: () => server.close() }));
  });
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const port = Number(process.argv[2] || process.env.RELAY_PORT || 7447);
  const relay = await startRelay({ port });
  console.log(`ev=relay_listening url=ws://127.0.0.1:${relay.port}`);
}
