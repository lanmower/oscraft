
const SITE_ROOT = new URL("../../", import.meta.url);
const PACK_PARAMS = new URLSearchParams(location.search);
const PACK_PARAM = PACK_PARAMS.get("pack") || PACK_PARAMS.get("package") || "scenarios/moba";
const TAR_GZ = /\.(tar\.gz|tgz)$/i;
const ZIP = /\.zip$/i;


export const TEXT_EXTENSIONS = Object.freeze([".lua", ".osd", ".json", ".txt", ".tsv"]);
export const ASSET_EXTENSIONS = Object.freeze([".png", ".jpg", ".jpeg", ".webp", ".glb", ".gltf", ".bin", ".ogg", ".wav", ".ktx2", ".osdt"]);
const REFUSED = /\.(m?js|html?|wasm|ts|cjs|map)$/i;

export function normalizePath(path) {
  const p = String(path).replace(/^\.\//, "");
  if (!p || p.startsWith("/") || p.includes("\\") || p.split("/").some((s) => s === "" || s === "." || s === "..")) {
    throw new Error(`pack path ${JSON.stringify(path)} is not a plain relative path`);
  }
  return p;
}

export function refusedPath(path) {
  if (REFUSED.test(path)) return `${path}: code files are not part of a pack`;
  const dot = path.lastIndexOf(".");
  const ext = dot < 0 ? "" : path.slice(dot).toLowerCase();
  if (!TEXT_EXTENSIONS.includes(ext) && !ASSET_EXTENSIONS.includes(ext)) return `${path}: ${ext || "no extension"} is not a pack file type`;
  return null;
}

export const isText = (path) => TEXT_EXTENSIONS.includes(path.slice(path.lastIndexOf(".")).toLowerCase());


const utf8 = new TextDecoder("utf-8", { fatal: true });
const lenient = new TextDecoder();

function normalizeEntry(path) {
  const out = [];
  for (const seg of path.split("/")) {
    if (seg === "" || seg === ".") continue;
    if (seg === "..") out.pop();
    else out.push(seg);
  }
  return out.join("/");
}

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

export async function readPackFile(file) {
  if (!TAR_GZ.test(file.name) && !ZIP.test(file.name)) throw new Error(`${file.name}: a pack is a .zip, .tar.gz or .tgz`);
  if (!TAR_GZ.test(file.name)) return readZip(await file.arrayBuffer());
  const gunzipped = new Response(file.stream().pipeThrough(new DecompressionStream("gzip")));
  return { files: readTar(new Uint8Array(await gunzipped.arrayBuffer())), skipped: [] };
}

export function manifestOf(files) {
  const raw = files.get("manifest.json");
  if (raw === undefined) throw new Error("pack has no manifest.json");
  const parsed = JSON.parse(typeof raw === "string" ? raw : lenient.decode(raw));
  if (parsed.schema !== 1) throw new Error(`manifest schema ${parsed.schema} is not 1`);
  return parsed;
}


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


const MIME = { json: "application/json", txt: "text/plain", osd: "text/plain", lua: "text/plain", glb: "model/gltf-binary", jpg: "image/jpeg", png: "image/png", ktx2: "image/ktx2", osdt: "application/octet-stream", bin: "application/octet-stream" };
const encoder = new TextEncoder();

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

async function openSplitArchive(url, name) {
  const indexUrl = `${url}.parts.json`;
  const res = await fetchRetrying(indexUrl);
  if (!res.ok) return null;
  const index = await res.json();
  if (index?.schema !== 1 || !Array.isArray(index.parts) || index.parts.length === 0) {
    throw new Error(`${indexUrl}: not a pack parts index`);
  }
  const pieces = await Promise.all(
    index.parts.map(async (part) => {
      const res = await fetchRetrying(new URL(part.file, indexUrl).href);
      if (!res.ok) throw new Error(`pack part ${part.file} HTTP ${res.status}`);
      const bytes = new Uint8Array(await res.arrayBuffer());
      if (bytes.length !== part.size) throw new Error(`pack part ${part.file}: ${bytes.length} bytes, index says ${part.size}`);
      return bytes;
    }),
  );
  const joined = new Uint8Array(pieces.reduce((n, piece) => n + piece.length, 0));
  let at = 0;
  for (const piece of pieces) {
    joined.set(piece, at);
    at += piece.length;
  }
  if (Number.isInteger(index.size) && joined.length !== index.size) {
    throw new Error(`${index.archive}: the parts join to ${joined.length} bytes, the index says ${index.size}`);
  }
  const { files } = await readPackFile(new File([joined], name));
  return fromFiles(files);
}

async function openArchive(url) {
  const name = decodeURIComponent(new URL(url).pathname.split("/").pop());
  const split = await openSplitArchive(url, name);
  if (split) return split;
  const res = await openFetched(url);
  const { files } = await readPackFile(new File([await res.blob()], name));
  return fromFiles(files);
}

const TRANSIENT_STATUS = new Set([502, 503, 504]);
const RETRY_PAUSES_MS = [400, 1000, 2000];
export async function fetchRetrying(url, init) {
  for (let attempt = 0; ; attempt++) {
    let res = null;
    try {
      res = await fetch(url, init);
    } catch (error) {
      if (attempt >= RETRY_PAUSES_MS.length) throw error;
    }
    if (res && !(TRANSIENT_STATUS.has(res.status) && attempt < RETRY_PAUSES_MS.length)) return res;
    await new Promise((resolve) => setTimeout(resolve, RETRY_PAUSES_MS[attempt]));
  }
}

function openDirectory(url) {
  return {
    has: null,
    url: (rel) => new URL(rel, url).href,
    bytes: async (rel) => {
      const res = await fetchRetrying(new URL(rel, url));
      if (!res.ok) throw new Error(`pack file ${rel} HTTP ${res.status}`);
      return res.arrayBuffer();
    },
  };
}

const archiveUrl = new URL(PACK_PARAM, SITE_ROOT);
const source = STASHED ? fromFiles(await takeStash())
  : TAR_GZ.test(PACK_PARAM) || ZIP.test(PACK_PARAM) ? await openArchive(archiveUrl)
  : openDirectory(new URL(PACK_PARAM.replace(/\/?$/, "/"), SITE_ROOT));

const manifestBytes = await source.bytes("manifest.json");
export const manifest = JSON.parse(new TextDecoder().decode(manifestBytes));
if (manifest.schema !== 1) throw new Error(`pack ${PACK_PARAM}: manifest schema ${manifest.schema} is not 1`);

export function assetRel(key, file = "") {
  let value = manifest.assets;
  for (const part of key.split(".")) value = value?.[part];
  if (typeof value !== "string") throw new Error(`pack ${manifest.name}: no asset ${key}`);
  return `${manifest.assets.root ?? "."}/${value}${file ? "/" + file : ""}`;
}

export const packUrl = (rel) => source.url(rel);
export const assetUrl = (key, file = "") => source.url(assetRel(key, file));
export const packBytes = (rel) => source.bytes(rel);
export const packText = async (rel) => new TextDecoder().decode(await packBytes(rel));
export const packJson = async (rel) => JSON.parse(await packText(rel));
const assetTextCache = new Map();

export const assetJson = async (key) => {
  const rel = assetRel(key);
  let text = assetTextCache.get(rel);
  if (text === undefined) {
    text = await packText(rel);
    assetTextCache.set(rel, text);
  }
  return JSON.parse(text);
};

export const UI = await packJson(manifest.ui);
export const packName = manifest.name;

export function modulePaths(manifest) {
  return Object.values(manifest.modules ?? {});
}

export async function moduleClosure(manifest, readText) {
  const byStem = manifest.modules ?? {};
  const out = [];
  const seen = new Set();
  const reads = new Map();
  const read = (file) => {
    if (!reads.has(file)) {
      reads.set(file, readText(file).then((text) => {
        for (const [, stem] of text.matchAll(/require\("([^"]+)"\)/g)) if (byStem[stem] !== undefined) read(byStem[stem]);
        return text;
      }));
    }
    return reads.get(file);
  };
  for (const root of manifest.mods ?? []) read(root);
  const visit = async (file) => {
    if (seen.has(file)) return;
    seen.add(file);
    const text = await read(file);
    out.push({ name: file, text });
    for (const [, stem] of text.matchAll(/require\("([^"]+)"\)/g)) {
      if (byStem[stem] !== undefined) await visit(byStem[stem]);
    }
  };
  for (const root of manifest.mods ?? []) await visit(root);
  return out;
}
