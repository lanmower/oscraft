// Runs one persistent oscraft wasip1 guest per host. The guest runs in a child node process whose
// stdin and stdout are anonymous pipes from spawn, the same on linux, macos and windows. The child's
// fd 0 blocks in read until the parent writes the next command line, so the instance stays alive
// between commands and tick cost depends on the sim state, not on the number of past commands.
import { readFileSync } from 'node:fs';
import { spawn } from 'node:child_process';

const GUEST_CHILD = [
  "const fs = require('node:fs');",
  "const { WASI } = require('node:wasi');",
  'const guestModule = new WebAssembly.Module(fs.readFileSync(process.argv[1]));',
  "const wasi = new WASI({ version: 'preview1', args: ['oscraft_wasip1'], env: {}, stdin: 0, stdout: 1, stderr: 2 });",
  'const instance = new WebAssembly.Instance(guestModule, { wasi_snapshot_preview1: wasi.wasiImport });',
  'wasi.start(instance);',
  'process.exit(0);',
].join(' ');

// Opens one guest instance on the image. command(line) resolves with that command's single reply
// line; replies arrive in the order commands were written. close() ends stdin so the guest returns.
export async function openGuest(wasmPath, image) {
  const child = spawn(process.execPath, ['-e', GUEST_CHILD, wasmPath], { stdio: ['pipe', 'pipe', 'inherit'] });
  const waiting = [];
  let failure = null;
  let carry = '';
  const fail = (err) => {
    failure ??= err;
    for (const w of waiting.splice(0)) w.reject(failure);
  };
  child.stdout.setEncoding('utf8');
  child.stdout.on('data', (chunk) => {
    carry += chunk;
    let at;
    while ((at = carry.indexOf('\n')) >= 0) {
      const line = carry.slice(0, at);
      carry = carry.slice(at + 1);
      const w = waiting.shift();
      if (w) w.resolve(line);
    }
  });
  child.on('error', fail);
  child.on('exit', (code) => fail(new Error(`guest exited with code ${code}`)));
  child.stdin.on('error', fail);

  const len = Buffer.alloc(4);
  len.writeUInt32LE(image.length, 0);
  child.stdin.write(Buffer.concat([len, image]));

  return {
    command(line) {
      return new Promise((resolve, reject) => {
        if (failure) return reject(failure);
        waiting.push({ resolve, reject });
        child.stdin.write(`${line}\n`);
      });
    },
    async close() {
      if (failure || child.exitCode !== null || child.signalCode !== null) return;
      const exited = new Promise((resolve) => child.once('exit', resolve));
      const killer = setTimeout(() => child.kill(), 5000);
      child.stdin.end();
      await exited;
      clearTimeout(killer);
    },
  };
}

export function replyValue(line, prefix) {
  if (!line.startsWith(prefix)) throw new Error(`guest reply "${line}" does not start with "${prefix}"`);
  return line.slice(prefix.length).trim();
}

const fields = (line, prefix, n) => {
  const parts = replyValue(line, prefix).split(/\s+/).map(Number);
  if (parts.length !== n || parts.some((v) => !Number.isFinite(v))) throw new Error(`guest reply "${line}" is not ${n} numbers`);
  return parts;
};

// One tick: advance one tick, then read the state hash and every unit record in one pipelined batch.
// A unit record is {id, kind, player, x, y, hp}. The guest has no per-unit kind command, so kind is 0
// until the guest exposes one.
export async function readTick(guest) {
  const [tickLine, hashLine, countLine] = await Promise.all([
    guest.command('tick 1'),
    guest.command('state-hash'),
    guest.command('unit-count'),
  ]);
  replyValue(tickLine, 'ok tick ');
  const hash = replyValue(hashLine, 'ok state-hash ');
  const count = Number(replyValue(countLine, 'ok unit-count '));
  const replies = await Promise.all(Array.from({ length: count }, (_, id) => Promise.all([
    guest.command(`unit-position ${id}`),
    guest.command(`unit-player ${id}`),
    guest.command(`unit-hp ${id}`),
  ])));
  const units = replies.map(([pos, player, hp], id) => {
    const [x, y] = fields(pos, 'ok unit-position ', 2);
    return { id, kind: 0, player: fields(player, 'ok unit-player ', 1)[0], x, y, hp: fields(hp, 'ok unit-hp ', 1)[0] };
  });
  return { hash, count, units };
}
