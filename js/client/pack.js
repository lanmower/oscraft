// A pack: the files of a game package (Lua, data and assets) held in memory as a map of path -> bytes.
//
// The page never hard-codes a package file name: the manifest (manifest.json at the pack root) lists what the pack
// holds. A pack holds no code the page runs: a .js, .mjs, .html, .wasm or .ts path is refused on load, on edit and on
// transfer. Text files (.lua, .osd, .json, .txt, .tsv) are kept as strings; other files as Uint8Array.
//
// Reading (directory, tar.gz, zip, folder list) and the path rules live in pack_load.js, the one reader. This module holds
// the editor's side: the Pack class, the content hash, and the zip writer.

import { crc32, isText, normalizePath, refusedPath } from "./pack_load.js";

// 32-bit FNV-1a of the bytes: the content hash of a file (the same hash the SWAP frame uses for an image).
export function contentHash(bytes) {
  let h = 0x811c9dc5;
  for (let i = 0; i < bytes.length; i++) h = Math.imul(h ^ bytes[i], 0x01000193);
  return h >>> 0;
}

const encoder = new TextEncoder();
const decoder = new TextDecoder("utf-8", { fatal: true });

export function toBytes(value) {
  return typeof value === "string" ? encoder.encode(value) : value;
}

// Holds the files of one pack. set/delete keep the refusal rule; changed() lists what differs from the last save.
export class Pack {
  constructor(files = new Map(), manifest = null) {
    this.files = new Map();
    this.saved = new Map(); // path -> hash at the last save (what peers were sent)
    for (const [path, value] of files) this.set(path, value);
    this.manifest = manifest;
    for (const [path, value] of this.files) this.saved.set(path, contentHash(toBytes(value)));
  }
  set(path, value) {
    const p = normalizePath(path);
    const refused = refusedPath(p);
    if (refused) throw new Error(refused);
    this.files.set(p, value);
    return p;
  }
  delete(path) { return this.files.delete(path); }
  get(path) { return this.files.get(path); }
  paths() { return [...this.files.keys()].sort(); }
  // Files whose content differs from the last save (new or edited), as { path, bytes, hash }; deleted paths as { path, deleted: true }.
  changed() {
    const out = [];
    for (const path of this.paths()) {
      const bytes = toBytes(this.files.get(path));
      const hash = contentHash(bytes);
      if (this.saved.get(path) !== hash) out.push({ path, bytes, hash });
    }
    for (const path of this.saved.keys()) if (!this.files.has(path)) out.push({ path, deleted: true });
    return out;
  }
  // Records the current content as saved (after the changes went out to the peers).
  markSaved() {
    this.saved = new Map(this.paths().map((p) => [p, contentHash(toBytes(this.files.get(p)))]));
  }
  // The manifest is manifest.json at the root; its parsed value is kept for the page to read.
  readManifest() {
    const raw = this.files.get("manifest.json");
    if (raw === undefined) return null;
    this.manifest = JSON.parse(typeof raw === "string" ? raw : decoder.decode(raw));
    return this.manifest;
  }
}

// ---- zip writing (stored entries, so no compressor is needed) ----

// Writes a store-only zip of the pack's files, manifest included, as a Uint8Array.
export function writeZip(files) {
  const entries = [];
  let offset = 0;
  const chunks = [];
  const names = [...files.keys()].sort();
  for (const name of names) {
    const nameBytes = encoder.encode(name);
    const data = toBytes(files.get(name));
    const crc = crc32(data);
    const local = new Uint8Array(30 + nameBytes.length);
    const lv = new DataView(local.buffer);
    lv.setUint32(0, 0x04034b50, true);
    lv.setUint16(4, 20, true);
    lv.setUint16(8, 0, true); // stored
    lv.setUint32(14, crc, true);
    lv.setUint32(18, data.length, true);
    lv.setUint32(22, data.length, true);
    lv.setUint16(26, nameBytes.length, true);
    local.set(nameBytes, 30);
    chunks.push(local, data);
    entries.push({ nameBytes, crc, size: data.length, offset });
    offset += local.length + data.length;
  }
  const centralStart = offset;
  for (const e of entries) {
    const central = new Uint8Array(46 + e.nameBytes.length);
    const cv = new DataView(central.buffer);
    cv.setUint32(0, 0x02014b50, true);
    cv.setUint16(4, 20, true);
    cv.setUint16(6, 20, true);
    cv.setUint32(16, e.crc, true);
    cv.setUint32(20, e.size, true);
    cv.setUint32(24, e.size, true);
    cv.setUint16(28, e.nameBytes.length, true);
    cv.setUint32(42, e.offset, true);
    central.set(e.nameBytes, 46);
    chunks.push(central);
    offset += central.length;
  }
  const end = new Uint8Array(22);
  const ev = new DataView(end.buffer);
  ev.setUint32(0, 0x06054b50, true);
  ev.setUint16(8, entries.length, true);
  ev.setUint16(10, entries.length, true);
  ev.setUint32(12, offset - centralStart, true);
  ev.setUint32(16, centralStart, true);
  chunks.push(end);
  const out = new Uint8Array(offset + end.length);
  let at = 0;
  for (const c of chunks) { out.set(c, at); at += c.length; }
  return out;
}
