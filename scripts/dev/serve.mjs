import { createServer } from "node:http";
import { watch } from "node:fs";
import { readFile, stat } from "node:fs/promises";
import { join, normalize, resolve, sep, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { contentType } from "./mime.mjs";

const HOST = "127.0.0.1";
const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const flags = process.argv.slice(2).filter((a) => a.startsWith("--"));
const positional = process.argv.slice(2).filter((a) => !a.startsWith("--"));
const PORT = Number(flags.find((a) => a.startsWith("--port="))?.slice(7) ?? process.env.PORT ?? 0);
const stagedOnly = flags.includes("--staged-only");
const root = resolve(positional[0] ?? join(repoRoot, ".dev-site"));

const LIVE = ["js/client/", "scenarios/", "mods/"];
const STAGED_ONLY = ["js/client/pkg/", "js/client/version.json"];

const NO_CACHE = { "cache-control": "no-store, must-revalidate", pragma: "no-cache", expires: "0" };

const HMR_PATH = "/__hmr";
const DEBOUNCE_MS = 150;
const KEEPALIVE_MS = 25000;
const WATCHED = ["js/client", "scenarios/moba"];
const watchBase = (dir) => (stagedOnly ? join(root, dir) : join(repoRoot, dir));
const clients = new Set();
const debounce = new Map();

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

function onFileEvent(rel) {
  const kind = hmrKind(rel);
  if (kind === null) return;
  clearTimeout(debounce.get(rel));
  debounce.set(rel, setTimeout(async () => {
    debounce.delete(rel);
    const file = await resolveFile(rel);
    if (!file || !(await stat(file).then((s) => s.isFile(), () => false))) return;
    sendChange(rel, kind);
  }, DEBOUNCE_MS));
}

function startWatching() {
  const targets = WATCHED.map((dir) => [dir, watchBase(dir)]);
  targets.push(["js/client/pkg", join(root, "js/client/pkg")]);
  for (const [dir, base] of targets) {
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

async function resolveFile(urlPath) {
  const rel = normalize(decodeURIComponent(urlPath.split("?")[0])).replace(/^([/\\])+/, "").split(sep).join("/");
  const relPosix = rel === "." ? "" : rel;
  const candidates = [];
  const live = !stagedOnly && LIVE.some((p) => relPosix.startsWith(p)) && !STAGED_ONLY.some((p) => relPosix.startsWith(p));
  if (live) candidates.push([repoRoot, resolve(repoRoot, relPosix)]);
  candidates.push([root, resolve(root, relPosix)]);
  for (const [base, file] of candidates) {
    if (file !== base && !file.startsWith(base + sep)) continue;
    const info = await stat(file).catch(() => null);
    if (!info) continue;
    return info.isDirectory() ? resolve(file, "index.html") : file;
  }
  return null;
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
      "content-type": contentType(file),
      "content-length": body.length,
      ...NO_CACHE,
    });
    res.end(req.method === "HEAD" ? undefined : body);
  } catch (err) {
    res.writeHead(500, { "content-type": "text/plain; charset=utf-8", ...NO_CACHE }).end(String(err?.message ?? err));
  }
});

server.listen(PORT, HOST, async () => {
  const port = server.address().port;
  const exists = await stat(root).then(() => true, () => false);
  if (!exists) console.error(`site dir missing: ${root} (run scripts/dev/stage.sh first)`);
  startWatching();
  console.log(`serving ${root}${stagedOnly ? " (staged only)" : ` with js/client, scenarios and mods live from ${repoRoot}`}`);
  console.log(`live reload: http://localhost:${port}${HMR_PATH} (pushed on change; no polling)`);
  console.log(`http://localhost:${port}/js/client/play.html#moba`);
});
