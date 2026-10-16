// Runs the oscraft wasip1 guest once per host under node:wasi, as one long-lived instance.
// The guest reads the image (u32 LE length, bytes) and then one command per line from stdin,
// and writes one reply line per command to stdout. Stdin and stdout are POSIX FIFOs; the guest
// runs on a worker thread because WASI start blocks on its stdin read. Tick cost does not grow
// with past commands: the instance keeps its state between commands.
import { openSync, closeSync, writeSync, read, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { execFileSync } from 'node:child_process';
import { Worker } from 'node:worker_threads';

const GUEST_WORKER = `
const { workerData, parentPort } = require('node:worker_threads');
const fs = require('node:fs');
const { WASI } = require('node:wasi');
const { wasmPath, cmdPath, repPath } = workerData;
const guestModule = new WebAssembly.Module(fs.readFileSync(wasmPath));
const stdin = fs.openSync(cmdPath, 'r+');
const stdout = fs.openSync(repPath, 'r+');
const wasi = new WASI({ version: 'preview1', args: ['oscraft_wasip1'], env: {}, stdin, stdout, stderr: 2 });
const instance = new WebAssembly.Instance(guestModule, { wasi_snapshot_preview1: wasi.wasiImport });
try {
  wasi.start(instance);
  parentPort.postMessage({ exit: 0 });
} catch (err) {
  parentPort.postMessage({ exit: 1, error: String((err && err.stack) || err) });
}
`;

function writeAll(fd, bytes) {
  let at = 0;
  while (at < bytes.length) at += writeSync(fd, bytes, at, bytes.length - at);
}

// Opens one guest instance. command(line) resolves with that command's single reply line,
// in the order commands were sent. close() stops the worker and removes the FIFOs.
export async function openGuest(wasmPath, image) {
  if (process.platform === 'win32') {
    throw new Error('the persistent guest transport uses POSIX FIFOs; win32 is not implemented yet');
  }
  const dir = mkdtempSync(join(tmpdir(), 'oscraft-guest-'));
  const cmdPath = join(dir, 'cmd');
  const repPath = join(dir, 'rep');
  execFileSync('mkfifo', [cmdPath, repPath]);
  const worker = new Worker(GUEST_WORKER, { eval: true, workerData: { wasmPath, cmdPath, repPath } });
  const cmdFd = openSync(cmdPath, 'r+');
  const repFd = openSync(repPath, 'r+');
  const waiting = [];
  let closed = false;
  const fail = (err) => {
    closed = true;
    for (const w of waiting.splice(0)) w.reject(err);
  };

  const len = Buffer.alloc(4);
  len.writeUInt32LE(image.length, 0);
  writeAll(cmdFd, Buffer.concat([len, image]));

  const buf = Buffer.alloc(65536);
  let carry = '';
  const pump = () => read(repFd, buf, 0, buf.length, null, (err, n) => {
    if (err) return fail(err);
    if (n === 0) return fail(new Error('guest reply stream closed'));
    carry += buf.toString('utf8', 0, n);
    let at;
    while ((at = carry.indexOf('\n')) >= 0) {
      const line = carry.slice(0, at);
      carry = carry.slice(at + 1);
      const w = waiting.shift();
      if (w) w.resolve(line);
    }
    if (!closed) pump();
  });
  pump();

  worker.on('message', (m) => {
    if (m.exit !== undefined) fail(new Error(m.error ? `guest failed: ${m.error}` : 'guest exited'));
  });
  worker.on('error', fail);
  worker.on('exit', () => fail(new Error('guest worker exited')));

  return {
    command(line) {
      return new Promise((resolve, reject) => {
        if (closed) return reject(new Error('guest is closed'));
        waiting.push({ resolve, reject });
        writeAll(cmdFd, Buffer.from(`${line}\n`, 'utf8'));
      });
    },
    async close() {
      closed = true;
      await worker.terminate();
      closeSync(cmdFd);
      closeSync(repFd);
      rmSync(dir, { recursive: true, force: true });
    },
  };
}

export function replyValue(line, prefix) {
  if (!line.startsWith(prefix)) throw new Error(`guest reply "${line}" does not start with "${prefix}"`);
  return line.slice(prefix.length).trim();
}

