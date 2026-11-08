import crypto from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const Q = String.fromCharCode(34);
const NL = String.fromCharCode(10);
const args = process.argv.slice(2);
const stamp = args.includes("--stamp");
const site = args.find((arg) => !arg.startsWith("--"));

const REQUIRED = [
  "version.json",
  "js/client/version.json",
  "js/client/editor.html",
  "js/client/play.html",
  "js/client/play.js",
  "js/client/pack_load.js",
  "js/client/play.bundle.js",
  "scenarios/moba/manifest.json",
  "scenarios/moba/moba.osdt",
  "js/client/pkg/web.js",
  "js/client/pkg/web_bg.wasm",
  "js/client/vendor/wireweave/src/index.js",
  "js/client/vendor/three/libs/draco/draco_wasm_wrapper.js",
  "scenarios/moba/bundle/moba.modules.json",
];
const NONEMPTY = ["js/client/play.bundle.js", "js/client/pkg/web_bg.wasm", "scenarios/moba/bundle/moba.modules.json"];
const FINGERPRINT = ["js/client/play.bundle.js", "js/client/pkg/web_bg.wasm", "scenarios/moba/moba.osdt", "scenarios/moba/bundle/moba.modules.json"];

const fail = (what) => { throw new Error(what); };
const at = (dir, ...parts) => path.join(dir, ...parts);

const hrefOf = (tag) => {
  const mark = "href=" + Q;
  const found = tag.indexOf(mark);
  if (found < 0) return null;
  const start = found + mark.length;
  const end = tag.indexOf(Q, start);
  return end < 0 ? null : tag.slice(start, end);
};

const preloadTargets = (html) => html.split("<link").slice(1).map((part) => part.split(">")[0]).filter((tag) => tag.includes("modulepreload")).map(hrefOf).filter(Boolean);
const localRel = (rel) => rel.startsWith("./") ? rel.slice(2) : rel;

function fingerprint(dir) {
  const hash = crypto.createHash("sha256");
  for (const rel of FINGERPRINT) {
    hash.update(rel);
    hash.update(fs.readFileSync(at(dir, rel)));
  }
  return hash.digest("hex");
}

function inspect(dir) {
  const missing = REQUIRED.filter((rel) => !fs.existsSync(at(dir, rel)));
  if (missing.length) fail("site is missing " + missing.join(", "));
  for (const rel of NONEMPTY) if (fs.statSync(at(dir, rel)).size === 0) fail(rel + " is empty");
  const chunks = fs.readdirSync(at(dir, "js/client")).filter((f) => f.startsWith("chunk-") && f.endsWith(".js"));
  if (!chunks.length) fail("esbuild produced no split chunks");
  const html = fs.readFileSync(at(dir, "js/client/play.html"), "utf8");
  if (!html.includes("src=" + Q + "play.bundle.js" + Q)) fail("play.html does not load play.bundle.js");
  const missingPreloads = preloadTargets(html).filter((rel) => !fs.existsSync(at(dir, "js/client", localRel(rel))));
  if (missingPreloads.length) fail("modulepreload targets missing: " + missingPreloads.join(", "));
  const version = JSON.parse(fs.readFileSync(at(dir, "version.json"), "utf8"));
  const clientVersion = JSON.parse(fs.readFileSync(at(dir, "js/client/version.json"), "utf8"));
  if (JSON.stringify(version) !== JSON.stringify(clientVersion)) fail("version.json copies differ");
  if (version.contentSha256 && version.contentSha256 !== fingerprint(dir)) fail("contentSha256 does not match the built artifacts");
  return { chunks: chunks.length, preloads: preloadTargets(html).length, contentSha256: version.contentSha256 ?? null, sha: version.sha };
}

function writeStamp(dir) {
  const version = JSON.parse(fs.readFileSync(at(dir, "version.json"), "utf8"));
  version.contentSha256 = fingerprint(dir);
  const text = JSON.stringify(version) + NL;
  fs.writeFileSync(at(dir, "version.json"), text);
  fs.writeFileSync(at(dir, "js/client/version.json"), text);
}

function selfTest() {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "site-bundle-check-"));
  try {
    const fixtureHtml = "<link rel=" + Q + "modulepreload" + Q + " href=" + Q + "./preload-fixture.js" + Q + "><script type=" + Q + "module" + Q + " src=" + Q + "play.bundle.js" + Q + "></script>";
    for (const rel of REQUIRED) {
      const abs = at(dir, rel);
      fs.mkdirSync(path.dirname(abs), { recursive: true });
      fs.writeFileSync(abs, rel === "js/client/play.html" ? fixtureHtml : "fixture");
    }
    fs.writeFileSync(at(dir, "version.json"), JSON.stringify({ sha: "fixture", ref: "local", run: "local" }) + NL);
    fs.copyFileSync(at(dir, "version.json"), at(dir, "js/client/version.json"));
    fs.writeFileSync(at(dir, "js/client/chunk-ABC.js"), "export const fixture = 1;");
    fs.writeFileSync(at(dir, "js/client/preload-fixture.js"), "export const preload = 1;");
    inspect(dir);
    writeStamp(dir);
    const stamped = inspect(dir);
    if (!stamped.contentSha256) fail("stamp did not add contentSha256");
    const chunk = at(dir, "js/client/chunk-ABC.js");
    fs.rmSync(chunk);
    let refused = false;
    try { inspect(dir); } catch { refused = true; }
    if (!refused) fail("a missing chunk did not fail the check");
    fs.writeFileSync(chunk, "export const fixture = 1;");
    if (stamped.preloads !== 1) fail("modulepreload detection found " + stamped.preloads + " targets");
    fs.rmSync(at(dir, "js/client/preload-fixture.js"));
    refused = false;
    try { inspect(dir); } catch { refused = true; }
    if (!refused) fail("a missing modulepreload target did not fail the check");
    console.log("site bundle ok (13 required files, chunk, preloads, html, version parity, fingerprint, 2 negative cases)");
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
}

if (!site) selfTest();
else {
  if (stamp) writeStamp(site);
  const result = inspect(site);
  console.log("site bundle ok " + JSON.stringify(result));
}
