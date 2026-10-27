import { extname } from "node:path";

export const TYPES = {
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".wasm": "application/wasm",
  ".glb": "model/gltf-binary",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".webp": "image/webp",
  ".svg": "image/svg+xml",
  ".ktx2": "image/ktx2",
  ".ogg": "audio/ogg",
  ".json": "application/json; charset=utf-8",
  ".html": "text/html; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".txt": "text/plain; charset=utf-8",
  ".lua": "text/plain; charset=utf-8",
  ".tsv": "text/tab-separated-values; charset=utf-8",
  ".osd": "text/plain; charset=utf-8",
  ".glsl": "text/plain; charset=utf-8",
  ".md": "text/markdown; charset=utf-8",
  ".osdt": "application/octet-stream",
  ".tar": "application/x-tar",
  ".gz": "application/gzip",
  ".bin": "application/octet-stream",
};

const COMPRESSIBLE = /^(text\/|application\/(json|wasm|x-tar)|image\/svg)/;

export function contentType(file) {
  return TYPES[extname(file).toLowerCase()] ?? "application/octet-stream";
}

export function isCompressible(type) {
  return COMPRESSIBLE.test(type);
}
