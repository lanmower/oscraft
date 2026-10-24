// Serves the staged site (scripts/dev/stage.sh output) on localhost at the same paths as GitHub Pages.
// Usage: node scripts/dev/serve.mjs [site-dir]   (default: .dev-site at the repo root)
//
// Hot reload is pushed, not polled: the server watches the served source files and sends one message per changed file on
// GET /__hmr (server-sent events). The page only listens (js/client/hmr.js). A message is {kind, path}, path relative to the
// site root; the kinds are the ones hmr.js handles (shader, module, asset, lua, map, model, wasm). A page that is not
// served by this server gets a 404 there and never retries.
import { createServer } from "node:http";
import { watch } from "node:fs";
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

// ---- live reload channel ----

const HMR_PATH = "/__hmr";
const DEBOUNCE_MS = 150; // one editor save or one copy writes several events; they become one message
const KEEPALIVE_MS = 25000;
const WATCHED = ["js/client", "scenarios/moba"]; // the trees the page loads its code, data, mods and models from
const clients = new Set();
const debounce = new Map(); // site path -> timer

// The reload kind of a site-relative path, or null when no page reacts to that file. Mirrors what hmr.js applies.
function hmrKind(rel) {
  if (rel.startsWith("js/client/vendor/")) return null;
  if (rel.startsWith("js/client/pkg/")) return "wasm";
  if (rel.startsWith("js/client/shaders/") && rel.endsWith(".glsl")) return "shader";
  if (rel.startsWith("js/client/") && rel.endsWith(".js") && !rel.endsWith("/hmr.js")) return "module";
  if (rel.startsWith("scenarios/moba/assets/models/") && rel.endsWith(".glb")) return "model";
  if (rel === "scenarios/moba/map.osd" || rel === "scenarios/moba/moba.osdt") return "map";
  if (rel.startsWith("scenarios/moba/") && rel.endsWith(".lua")) return "lua";
  if (/^scenarios\/moba\/(assets\/)?[^/]+\.json$/.test(rel) && rel !== "scenarios/moba/manifest.json") return "asset";
  return null;
}

function sendChange(rel, kind) {
  const message = `event: change\ndata: ${JSON.stringify({ kind, path: rel })}\n\n`;
  for (const res of clients) res.write(message);
}

// A changed file is announced once it has been quiet for DEBOUNCE_MS and still exists (a delete announces nothing).
function onFileEvent(rel) {
  const kind = hmrKind(rel);
  if (kind === null) return;
  clearTimeout(debounce.get(rel));
  debounce.set(rel, setTimeout(async () => {
    debounce.delete(rel);
    if (!(await stat(join(root, rel)).then((s) => s.isFile(), () => false))) return;
    sendChange(rel, kind);
  }, DEBOUNCE_MS));
}

function startWatching() {
  for (const dir of WATCHED) {
    const base = join(root, dir);
    try {
      watch(base, { recursive: true }, (_event, filename) => {
        if (!filename) return;
        onFileEvent(normalize(join(dir, filename)).split(sep).join("/"));
      });
    } catch (err) {
      console.error(`not watching ${base}: ${err.message}`);
    }
  }
}

function openChannel(req, res) {
  res.writeHead(200, { "content-type": "text/event-stream", "cache-control": "no-store", connection: "keep-alive" });
  res.write("retry: 1000\n\n");
  clients.add(res);
  req.on("close", () => clients.delete(res));
}

setInterval(() => {
  for (const res of clients) res.write(": keepalive\n\n");
}, KEEPALIVE_MS).unref();

// ---- static files ----

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
  if ((req.url ?? "").split("?")[0] === HMR_PATH) {
    openChannel(req, res);
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
  startWatching();
  console.log(`serving ${root}`);
  console.log(`live reload: http://localhost:${PORT}${HMR_PATH} (pushed on change; no polling)`);
  console.log(`http://localhost:${PORT}/js/client/play.html#moba`);
});
