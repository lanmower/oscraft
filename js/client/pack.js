// A pack: the files of a game package (Lua, data and assets) held in memory as a map of path -> bytes.
//
// The page never hard-codes a package file name: the manifest (manifest.json at the pack root) lists what the pack
// holds. A pack holds no code the page runs: a .js, .mjs, .html, .wasm or .ts path is refused on load, on edit and on
// transfer. Text files (.lua, .osd, .json, .txt, .tsv) are kept as strings; other files as Uint8Array.

export const TEXT_EXTENSIONS = Object.freeze([".lua", ".osd", ".json", ".txt", ".tsv"]);
export const ASSET_EXTENSIONS = Object.freeze([".png", ".jpg", ".jpeg", ".webp", ".glb", ".gltf", ".bin", ".ogg", ".wav", ".ktx2"]);
const REFUSED = /\.(m?js|html?|wasm|ts|cjs|map)$/i;

// A pack path is relative, slash-separated, without "..", a leading slash, a backslash or an empty segment.
export function normalizePath(path) {
  const p = String(path).replace(/^\.\//, "");
  if (!p || p.startsWith("/") || p.includes("\\") || p.split("/").some((s) => s === "" || s === "." || s === "..")) {
    throw new Error(`pack path ${JSON.stringify(path)} is not a plain relative path`);
  }
  return p;
}

// Why a file may not be part of a pack, or null when it may.
export function refusedPath(path) {
  if (REFUSED.test(path)) return `${path}: code files are not part of a pack`;
  const dot = path.lastIndexOf(".");
  const ext = dot < 0 ? "" : path.slice(dot).toLowerCase();
  if (!TEXT_EXTENSIONS.includes(ext) && !ASSET_EXTENSIONS.includes(ext)) return `${path}: ${ext || "no extension"} is not a pack file type`;
  return null;
}

export const isText = (path) => TEXT_EXTENSIONS.includes(path.slice(path.lastIndexOf(".")).toLowerCase());

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

// ---- zip reading ----

const CRC_TABLE = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();

export function crc32(bytes) {
  let c = 0xffffffff;
  for (let i = 0; i < bytes.length; i++) c = CRC_TABLE[(c ^ bytes[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

async function inflateRaw(bytes) {
  if (typeof DecompressionStream !== "function") throw new Error("this browser cannot read deflated zip entries");
  const stream = new Blob([bytes]).stream().pipeThrough(new DecompressionStream("deflate-raw"));
  return new Uint8Array(await new Response(stream).arrayBuffer());
}

// Reads a zip archive (stored or deflated entries) into a Map of path -> bytes. Directories and refused paths are skipped
// and reported in `skipped`.
export async function readZip(buffer) {
  const data = new Uint8Array(buffer);
  const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
  let eocd = -1;
  for (let i = data.length - 22; i >= Math.max(0, data.length - 22 - 65535); i--) {
    if (view.getUint32(i, true) === 0x06054b50) { eocd = i; break; }
  }
  if (eocd < 0) throw new Error("not a zip archive");
  const count = view.getUint16(eocd + 10, true);
  let at = view.getUint32(eocd + 16, true);
  const files = new Map();
  const skipped = [];
  for (let n = 0; n < count; n++) {
    if (view.getUint32(at, true) !== 0x02014b50) throw new Error("zip central directory is damaged");
    const method = view.getUint16(at + 10, true);
    const crc = view.getUint32(at + 16, true);
    const csize = view.getUint32(at + 20, true);
    const nameLen = view.getUint16(at + 28, true);
    const extraLen = view.getUint16(at + 30, true);
    const commentLen = view.getUint16(at + 32, true);
    const localAt = view.getUint32(at + 42, true);
    const name = decoder.decode(data.subarray(at + 46, at + 46 + nameLen));
    at += 46 + nameLen + extraLen + commentLen;
    if (name.endsWith("/")) continue;
    if (view.getUint32(localAt, true) !== 0x04034b50) throw new Error(`zip entry ${name} has a damaged local header`);
    const start = localAt + 30 + view.getUint16(localAt + 26, true) + view.getUint16(localAt + 28, true);
    const raw = data.subarray(start, start + csize);
    let bytes;
    if (method === 0) bytes = raw.slice();
    else if (method === 8) bytes = await inflateRaw(raw);
    else throw new Error(`zip entry ${name} uses unsupported method ${method}`);
    if (crc32(bytes) !== crc) throw new Error(`zip entry ${name} fails its CRC`);
    let path;
    try { path = normalizePath(name); } catch { skipped.push(name); continue; }
    if (refusedPath(path)) { skipped.push(path); continue; }
    files.set(path, isText(path) ? decoder.decode(bytes) : bytes);
  }
  return { files, skipped };
}

// Reads a folder picked with <input webkitdirectory> (File objects with webkitRelativePath). The first path segment is the
// folder's own name and is dropped, so the manifest sits at the root of the pack.
export async function readFileList(list) {
  const files = new Map();
  const skipped = [];
  for (const file of list) {
    const rel = file.webkitRelativePath || file.name;
    const path = rel.includes("/") ? rel.slice(rel.indexOf("/") + 1) : rel;
    let clean;
    try { clean = normalizePath(path); } catch { skipped.push(path); continue; }
    if (refusedPath(clean)) { skipped.push(clean); continue; }
    const bytes = new Uint8Array(await file.arrayBuffer());
    files.set(clean, isText(clean) ? decoder.decode(bytes) : bytes);
  }
  return { files, skipped };
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
