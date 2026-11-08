import { brotliCompressSync, constants, gzipSync } from "node:zlib";
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const site = resolve(process.argv[2] ?? join(root, "_site"));
const moba = join(site, "scenarios/moba");
const { moduleClosure } = await import(pathToFileURL(join(site, "js/client/lua_closure.js")).href);
const manifest = JSON.parse(readFileSync(join(moba, "manifest.json"), "utf8"));
const closure = await moduleClosure(manifest, async (rel) => readFileSync(join(moba, rel), "utf8"));
const gzip = (bytes) => gzipSync(bytes, { level: 9 }).length;
const brotli = (bytes) => brotliCompressSync(bytes, { params: { [constants.BROTLI_PARAM_QUALITY]: 11 } }).length;
const size = (buffers) => buffers.reduce((total, bytes) => ({ requests: total.requests + 1, raw: total.raw + bytes.length, gzip: total.gzip + gzip(bytes), brotli: total.brotli + brotli(bytes) }), { requests: 0, raw: 0, gzip: 0, brotli: 0 });
const perFile = size(closure.map((entry) => readFileSync(join(moba, entry.name))));
const bundle = size([readFileSync(join(moba, "bundle/moba.modules.json"))]);
const asset = (rel) => { const bytes = readFileSync(join(site, rel)); return { raw: bytes.length, gzip: gzip(bytes), brotli: brotli(bytes) }; };
console.log(JSON.stringify({ closureModules: closure.length, luaPerFileRequests: perFile, luaBundle: bundle, wasm: asset("js/client/pkg/web_bg.wasm"), image: asset("scenarios/moba/moba.osdt") }, null, 2));
