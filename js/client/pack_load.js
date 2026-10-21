// The one reader of a game pack. A pack is a directory, a .tar.gz (or .tgz) archive, or a .zip, with a manifest.json at its root
// (the schema is in scenarios/moba/manifest.json). The page names it with ?package=<dir or archive>; the default is
// scenarios/moba, resolved from the site root. Every file the client needs (the UI json, the map image, the rules, the asset
// catalogue) is read through this module by its manifest path. Top-level await: the modules that import UI wait until the
// pack has loaded. pack.js (the editor's Pack) imports the readers and path rules from here and holds no parser of its own.

const SITE_ROOT = new URL("../../", import.meta.url);
const PACK_PARAM = new URLSearchParams(location.search).get("package") || "scenarios/moba";
const TAR_GZ = /\.(tar\.gz|tgz)$/i;
const ZIP = /\.zip$/i;

// ---- pack paths and file types ----

export const TEXT_EXTENSIONS = Object.freeze([".lua", ".osd", ".json", ".txt", ".tsv"]);
export const ASSET_EXTENSIONS = Object.freeze([".png", ".jpg", ".jpeg", ".webp", ".glb", ".gltf", ".bin", ".ogg", ".wav", ".ktx2", ".osdt"]);
const REFUSED = /\.(m?js|html?|wasm|ts|cjs|map)$/i;

// A pack path is relative, slash-separated, without "..", a leading slash, a backslash or an empty segment.
export function normalizePath(path) {
  const p = String(path).replace(/^\.\//, "");
  if (!p || p.startsWith("/") || p.includes("\\") || p.split("/").some((s) => s === "" || s === "." || s === "..")) {
    throw new Error(`pack path ${JSON.stringify(path)} is not a plain relative path`);
  }
  return p;
}

// Why a file may not be part of a pack, or null when it may. A pack holds no code the page runs.
export function refusedPath(path) {
  if (REFUSED.test(path)) return `${path}: code files are not part of a pack`;
  const dot = path.lastIndexOf(".");
  const ext = dot < 0 ? "" : path.slice(dot).toLowerCase();
  if (!TEXT_EXTENSIONS.includes(ext) && !ASSET_EXTENSIONS.includes(ext)) return `${path}: ${ext || "no extension"} is not a pack file type`;
  return null;
}

export const isText = (path) => TEXT_EXTENSIONS.includes(path.slice(path.lastIndexOf(".")).toLowerCase());

// ---- readers: each gives a Map of pack path -> bytes (text files as strings) ----

const utf8 = new TextDecoder("utf-8", { fatal: true });
const lenient = new TextDecoder();

// Path normalisation inside an archive: "." segments drop, ".." pops a segment (a manifest path may climb out of its pack).
function normalizeEntry(path) {
  const out = [];
  for (const seg of path.split("/")) {
    if (seg === "" || seg === ".") continue;
    if (seg === "..") out.pop();
    else out.push(seg);
  }
  return out.join("/");
}

// Reads a ustar archive (the format scripts/pack-package.mjs writes) into a map of normalized path to bytes.
function readTar(bytes) {
  const files = new Map();
  const field = (start, end) => lenient.decode(bytes.subarray(start, end)).replace(/\0.*$/s, "");
  let off = 0;
  while (off + 512 <= bytes.length) {
    if (bytes.subarray(off, off + 512).every((b) => b === 0)) break;
    const name = field(off, off + 100);
    const size = parseInt(field(off + 124, off + 136).trim() || "0", 8);
    const type = bytes[off + 156];
    const prefix = field(off + 345, off + 500);
    const full = prefix ? `${prefix}/${name}` : name;
    off += 512;
    if (type === 0 || type === 48) files.set(normalizeEntry(full), bytes.slice(off, off + size));
    off += Math.ceil(size / 512) * 512;
  }
  return files;
}

export function crc32(bytes) {
  let c = 0xffffffff;
  for (let i = 0; i < bytes.length; i++) c = CRC_TABLE[(c ^ bytes[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}
const CRC_TABLE = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();

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
    const name = utf8.decode(data.subarray(at + 46, at + 46 + nameLen));
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
    files.set(path, isText(path) ? utf8.decode(bytes) : bytes);
  }
  return { files, skipped };
}

// Reads a pack file the user picked (.zip, .tar.gz or .tgz) into { files, skipped }, as the readers above do.
export async function readPackFile(file) {
  if (!TAR_GZ.test(file.name) && !ZIP.test(file.name)) throw new Error(`${file.name}: a pack is a .zip, .tar.gz or .tgz`);
  if (!TAR_GZ.test(file.name)) return readZip(await file.arrayBuffer());
  const gunzipped = new Response(file.stream().pipeThrough(new DecompressionStream("gzip")));
  return { files: readTar(new Uint8Array(await gunzipped.arrayBuffer())), skipped: [] };
}

// The manifest of a pack read into `files` (a Map): it must be schema 1. Used by the lobby before it starts a game.
export function manifestOf(files) {
  const raw = files.get("manifest.json");
  if (raw === undefined) throw new Error("pack has no manifest.json");
  const parsed = JSON.parse(typeof raw === "string" ? raw : lenient.decode(raw));
  if (parsed.schema !== 1) throw new Error(`manifest schema ${parsed.schema} is not 1`);
  return parsed;
}

// ---- stash: the pack the lobby loaded, kept in IndexedDB for the page it starts (?stashed=1). One pack at a time. ----

const STASH_DB = "oscraft-packs";
const STASH_STORE = "packs";
const STASH_KEY = "pending";
const STASHED = new URLSearchParams(location.search).has("stashed");

function openStash() {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(STASH_DB, 1);
    req.onupgradeneeded = () => req.result.createObjectStore(STASH_STORE);
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

// Keeps `files` (a Map of pack path -> string or bytes) for the next page load.
export async function stashPack(files) {
  const db = await openStash();
  try {
    await new Promise((resolve, reject) => {
      const tx = db.transaction(STASH_STORE, "readwrite");
      tx.objectStore(STASH_STORE).put(files, STASH_KEY);
      tx.oncomplete = () => resolve();
      tx.onerror = () => reject(tx.error);
      tx.onabort = () => reject(tx.error || new Error("pack stash aborted"));
    });
  } finally {
    db.close();
  }
}

async function takeStash() {
  const db = await openStash();
  try {
    const files = await new Promise((resolve, reject) => {
      const req = db.transaction(STASH_STORE, "readonly").objectStore(STASH_STORE).get(STASH_KEY);
      req.onsuccess = () => resolve(req.result);
      req.onerror = () => reject(req.error);
    });
    if (!(files instanceof Map)) throw new Error("no loaded pack is stashed");
    return files;
  } finally {
    db.close();
  }
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
    files.set(clean, isText(clean) ? utf8.decode(bytes) : bytes);
  }
  return { files, skipped };
}

// ---- page sources: each answers has/url/bytes for a pack path ----

const MIME = { json: "application/json", txt: "text/plain", osd: "text/plain", lua: "text/plain", glb: "model/gltf-binary", jpg: "image/jpeg", png: "image/png", ktx2: "image/ktx2", osdt: "application/octet-stream", bin: "application/octet-stream" };
const encoder = new TextEncoder();

// A source over an in-memory map of files (a tar.gz or a zip): urls are blob URLs made on first use.
function fromFiles(files) {
  const blobs = new Map();
  const bytesOf = (rel) => {
    const v = files.get(normalizeEntry(rel));
    if (v === undefined) throw new Error(`pack has no ${rel}`);
    return typeof v === "string" ? encoder.encode(v) : v;
  };
  return {
    has: (rel) => files.has(normalizeEntry(rel)),
    url: (rel) => {
      const key = normalizeEntry(rel);
      let u = blobs.get(key);
      if (!u) {
        const type = MIME[key.split(".").pop()] ?? "application/octet-stream";
        u = URL.createObjectURL(new Blob([bytesOf(key)], { type }));
        blobs.set(key, u);
      }
      return u;
    },
    bytes: async (rel) => {
      const bytes = bytesOf(rel);
      return bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength);
    },
  };
}

async function openFetched(url) {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`pack ${url} HTTP ${res.status}`);
  return res;
}

async function openTarGz(url) {
  const res = await openFetched(url);
  const gunzipped = new Response(res.body.pipeThrough(new DecompressionStream("gzip")));
  return fromFiles(readTar(new Uint8Array(await gunzipped.arrayBuffer())));
}

async function openZip(url) {
  const res = await openFetched(url);
  return fromFiles((await readZip(await res.arrayBuffer())).files);
}

function openDirectory(url) {
  return {
    has: null,
    url: (rel) => new URL(rel, url).href,
    bytes: async (rel) => {
      const res = await fetch(new URL(rel, url));
      if (!res.ok) throw new Error(`pack file ${rel} HTTP ${res.status}`);
      return res.arrayBuffer();
    },
  };
}

const archiveUrl = new URL(PACK_PARAM, SITE_ROOT);
const source = STASHED ? fromFiles(await takeStash())
  : TAR_GZ.test(PACK_PARAM) ? await openTarGz(archiveUrl)
  : ZIP.test(PACK_PARAM) ? await openZip(archiveUrl)
  : openDirectory(new URL(PACK_PARAM.replace(/\/?$/, "/"), SITE_ROOT));

const manifestBytes = await source.bytes("manifest.json");
export const manifest = JSON.parse(new TextDecoder().decode(manifestBytes));
if (manifest.schema !== 1) throw new Error(`pack ${PACK_PARAM}: manifest schema ${manifest.schema} is not 1`);

// The asset root is a path inside the pack (archives rewrite it to "assets"). A key names an entry of manifest.assets:
// assetRel("textures.ground") gives its path, assetRel("models", "x.glb") a file under a directory entry.
export function assetRel(key, file = "") {
  let value = manifest.assets;
  for (const part of key.split(".")) value = value?.[part];
  if (typeof value !== "string") throw new Error(`pack ${manifest.name}: no asset ${key}`);
  return `${manifest.assets.root ?? "."}/${value}${file ? "/" + file : ""}`;
}

// URL of a pack file or asset: a fetchable URL in a directory pack, a blob URL in an archive.
export const packUrl = (rel) => source.url(rel);
export const assetUrl = (key, file = "") => source.url(assetRel(key, file));
export const packBytes = (rel) => source.bytes(rel);
export const packText = async (rel) => new TextDecoder().decode(await packBytes(rel));
export const packJson = async (rel) => JSON.parse(await packText(rel));
export const assetJson = async (key) => JSON.parse(await packText(assetRel(key)));

// The UI text and key bindings of the pack (moba_ui.json, or the pack's ui entry).
export const UI = await packJson(manifest.ui);
export const packName = manifest.name;
