// Serves the staged site (scripts/dev/stage.sh output) on localhost at the same paths as GitHub Pages.
// Usage: node scripts/dev/serve.mjs [site-dir]   (default: .dev-site at the repo root)
import { createServer } from "node:http";
import { readFile, stat } from "node:fs/promises";
import { extname, join, normalize, resolve, sep, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const PORT = 8790;
const HOST = "127.0.0.1";
const root = resolve(process.argv[2] ?? join(dirname(fileURLToPath(import.meta.url)), "..", "..", ".dev-site"));

const TYPES = {
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".wasm": "application/wasm",
  ".glb": "model/gltf-binary",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".webp": "image/webp",
  ".ktx2": "image/ktx2",
  ".json": "application/json; charset=utf-8",
  ".html": "text/html; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".txt": "text/plain; charset=utf-8",
  ".bin": "application/octet-stream",
};

const NO_CACHE = { "cache-control": "no-store, must-revalidate", pragma: "no-cache", expires: "0" };

async function resolveFile(urlPath) {
  const rel = normalize(decodeURIComponent(urlPath.split("?")[0])).replace(/^([/\\])+/, "");
  const file = resolve(root, rel);
  if (file !== root && !file.startsWith(root + sep)) return null;
  const info = await stat(file).catch(() => null);
  if (!info) return null;
  return info.isDirectory() ? resolve(file, "index.html") : file;
}

const server = createServer(async (req, res) => {
  if (req.method !== "GET" && req.method !== "HEAD") {
    res.writeHead(405, { allow: "GET, HEAD", ...NO_CACHE }).end();
    return;
  }
  try {
    const file = await resolveFile(req.url ?? "/");
    const body = file ? await readFile(file).catch(() => null) : null;
    if (!body) {
      res.writeHead(404, { "content-type": "text/plain; charset=utf-8", ...NO_CACHE }).end("not found\n");
      return;
    }
    res.writeHead(200, {
      "content-type": TYPES[extname(file).toLowerCase()] ?? "application/octet-stream",
      "content-length": body.length,
      ...NO_CACHE,
    });
    res.end(req.method === "HEAD" ? undefined : body);
  } catch (err) {
    res.writeHead(500, { "content-type": "text/plain; charset=utf-8", ...NO_CACHE }).end(String(err?.message ?? err));
  }
});

server.listen(PORT, HOST, async () => {
  const exists = await stat(root).then(() => true, () => false);
  if (!exists) console.error(`site dir missing: ${root} (run scripts/dev/stage.sh first)`);
  console.log(`serving ${root}`);
  console.log(`http://localhost:${PORT}/js/client/play.html#moba`);
});
