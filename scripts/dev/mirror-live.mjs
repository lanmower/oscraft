import http from "node:http";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const LIVE = process.env.OSCRAFT_LIVE ?? "https://lanmower.github.io/oscraft";
const PORT = Number(process.env.PORT ?? 8765);
const LOCAL = ["js/client/"];
const TYPES = { ".html": "text/html", ".js": "text/javascript", ".mjs": "text/javascript", ".css": "text/css", ".json": "application/json", ".wasm": "application/wasm", ".glb": "model/gltf-binary", ".png": "image/png", ".jpg": "image/jpeg", ".ktx2": "image/ktx2", ".lua": "text/plain", ".osdt": "application/octet-stream", ".bin": "application/octet-stream" };
const cache = new Map();
async function fromLive(rel) {
  if (cache.has(rel)) return cache.get(rel);
  const res = await fetch(LIVE + "/" + rel);
  const body = res.ok ? Buffer.from(await res.arrayBuffer()) : null;
  const entry = { status: res.status, body, type: res.headers.get("content-type") };
  cache.set(rel, entry);
  return entry;
}
http.createServer(async (req, res) => {
  const rel = decodeURIComponent(new URL(req.url, "http://x").pathname).replace(/^\/+/, "") || "js/client/play.html";
  const local = LOCAL.some((p) => rel.startsWith(p)) && !rel.startsWith("js/client/pkg/") && !rel.startsWith("js/client/vendor/") ? path.join(ROOT, rel) : null;
  if (local && fs.existsSync(local) && fs.statSync(local).isFile()) {
    res.writeHead(200, { "content-type": TYPES[path.extname(local)] ?? "application/octet-stream", "cache-control": "no-store" });
    fs.createReadStream(local).pipe(res);
    return;
  }
  try {
    const entry = await fromLive(rel);
    if (!entry.body) { res.writeHead(entry.status); res.end(); return; }
    res.writeHead(200, { "content-type": entry.type ?? TYPES[path.extname(rel)] ?? "application/octet-stream", "cache-control": "no-store" });
    res.end(entry.body);
  } catch (err) {
    res.writeHead(502); res.end(String(err));
  }
}).listen(PORT, "127.0.0.1", () => console.log("mirror on http://127.0.0.1:" + PORT + " -> " + LIVE + " (local: " + LOCAL.join(", ") + ")"));
