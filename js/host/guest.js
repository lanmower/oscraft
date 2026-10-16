// Runs the oscraft wasip1 guest under node:wasi. The guest reads one run from stdin
// (u32 LE image length, image bytes, then text commands until EOF) and writes replies to stdout.
// Each run is a fresh instance, so the host keeps a command journal and replays it.
import { readFileSync, openSync, closeSync, writeFileSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { WASI } from 'node:wasi';

export async function loadGuest(wasmPath) {
  return WebAssembly.compile(readFileSync(wasmPath));
}

export function runGuest(module, image, commands) {
  const dir = mkdtempSync(join(tmpdir(), 'oscraft-guest-'));
  const inPath = join(dir, 'stdin');
  const outPath = join(dir, 'stdout');
  try {
    const text = Buffer.from(commands.map((c) => c + '\n').join(''), 'utf8');
    const len = Buffer.alloc(4);
    len.writeUInt32LE(image.length, 0);
    writeFileSync(inPath, Buffer.concat([len, image, text]));
    const inFd = openSync(inPath, 'r');
    const outFd = openSync(outPath, 'w');
    try {
      const wasi = new WASI({ version: 'preview1', args: ['oscraft_wasip1'], env: {}, stdin: inFd, stdout: outFd, stderr: outFd });
      const instance = new WebAssembly.Instance(module, { wasi_snapshot_preview1: wasi.wasiImport });
      wasi.start(instance);
    } finally {
      closeSync(inFd);
      closeSync(outFd);
    }
    return readFileSync(outPath, 'utf8').split('\n').filter(Boolean);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

export function lastReply(lines, prefix) {
  for (let i = lines.length - 1; i >= 0; i--) {
    if (lines[i].startsWith(prefix)) return lines[i].slice(prefix.length).trim();
  }
  return null;
}
