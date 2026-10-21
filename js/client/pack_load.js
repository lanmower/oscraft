// The one reader of a game pack. A pack is a directory, or a .tar.gz archive, with a manifest.json at its root (the schema is
// in scenarios/moba/manifest.json). The page names it with ?package=<dir or archive>; the default is scenarios/moba, resolved
// from the site root. Every file the client needs (the UI json, the map image, the rules, the asset catalogue) is read through
// this module by its manifest path. Top-level await: the modules that import UI wait until the pack has loaded.

const SITE_ROOT = new URL("../../", import.meta.url);
const PACK_PARAM = new URLSearchParams(location.search).get("package") || "scenarios/moba";
const ARCHIVE = /\.(tar\.gz|tgz)$/i;

// Path normalisation inside an archive: "." segments drop, ".." pops a segment (a manifest path may climb out of its pack).
function normalize(path) {
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
  const dec = new TextDecoder();
  const field = (start, end) => dec.decode(bytes.subarray(start, end)).replace(/\0.*$/s, "");
  let off = 0;
  while (off + 512 <= bytes.length) {
    if (bytes.subarray(off, off + 512).every((b) => b === 0)) break;
    const name = field(off, off + 100);
    const size = parseInt(field(off + 124, off + 136).trim() || "0", 8);
    const type = bytes[off + 156];
    const prefix = field(off + 345, off + 500);
    const full = prefix ? `${prefix}/${name}` : name;
    off += 512;
    if (type === 0 || type === 48) files.set(normalize(full), bytes.slice(off, off + size));
    off += Math.ceil(size / 512) * 512;
  }
  return files;
}

const MIME = { json: "application/json", txt: "text/plain", osd: "text/plain", lua: "text/plain", glb: "model/gltf-binary", jpg: "image/jpeg", png: "image/png", ktx2: "image/ktx2", osdt: "application/octet-stream", bin: "application/octet-stream" };

async function openArchive(url) {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`pack ${url} HTTP ${res.status}`);
  const gunzipped = new Response(res.body.pipeThrough(new DecompressionStream("gzip")));
  const files = readTar(new Uint8Array(await gunzipped.arrayBuffer()));
  const blobs = new Map();
  const blobUrl = (rel) => {
    let u = blobs.get(rel);
    if (!u) {
      const bytes = files.get(rel);
      if (!bytes) throw new Error(`pack has no ${rel}`);
      const type = MIME[rel.split(".").pop()] ?? "application/octet-stream";
      u = URL.createObjectURL(new Blob([bytes], { type }));
      blobs.set(rel, u);
    }
    return u;
  };
  return {
    kind: "archive",
    has: (rel) => files.has(normalize(rel)),
    url: (rel) => blobUrl(normalize(rel)),
    bytes: async (rel) => {
      const bytes = files.get(normalize(rel));
      if (!bytes) throw new Error(`pack has no ${rel}`);
      return bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength);
    },
  };
}

function openDirectory(url) {
  return {
    kind: "directory",
    has: null,
    url: (rel) => new URL(rel, url).href,
    bytes: async (rel) => {
      const res = await fetch(new URL(rel, url));
      if (!res.ok) throw new Error(`pack file ${rel} HTTP ${res.status}`);
      return res.arrayBuffer();
    },
  };
}

const source = ARCHIVE.test(PACK_PARAM)
  ? await openArchive(new URL(PACK_PARAM, SITE_ROOT))
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
