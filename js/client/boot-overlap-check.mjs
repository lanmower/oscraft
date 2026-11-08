import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const playSource = readFileSync(resolve(root, "js/client/play.js"), "utf8");
const fail = (message) => {
  throw new Error(message);
};

const LOAD_SIM = "async function loadSim() {";
const IMAGE_LINE = "  const imageFetch = fetch(mapUrl).then((res) => res.arrayBuffer());\n";
const MARKS = ["const imageFetch = fetch(mapUrl)", "await import(pkgUrl)", "await mod.default()", "await imageFetch"];

function loadSimBody(source) {
  const start = source.indexOf(LOAD_SIM);
  if (start < 0) fail("js/client/play.js has no loadSim");
  const end = source.indexOf("\n}\n", start);
  if (end < 0) fail("loadSim has no closing brace");
  return source.slice(start, end);
}

function marks(body) {
  return MARKS.map((mark) => body.indexOf(mark));
}

function overlaps(body) {
  const [start, wasmImport, wasmInit, imageAwait] = marks(body);
  return start >= 0 && start < wasmImport && wasmImport < wasmInit && wasmInit < imageAwait;
}

const body = loadSimBody(playSource);
if (!overlaps(body)) fail("loadSim does not overlap the image fetch with the wasm import: marks " + marks(body).join(" "));

if (!body.includes(IMAGE_LINE)) fail("negative control cannot be built: the image fetch line changed");
const serial = body.replace(IMAGE_LINE, "").replace("  wasmModule = mod;\n", "  wasmModule = mod;\n" + IMAGE_LINE);
if (overlaps(serial)) fail("negative control: a serial image fetch passes the overlap check");

console.log("boot overlap ok: image fetch starts at offset " + marks(body)[0] + ", wasm import at " + marks(body)[1] + ", wasm init at " + marks(body)[2] + ", image awaited at " + marks(body)[3] + "; serial control rejected");
