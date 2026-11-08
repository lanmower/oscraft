import { readFileSync } from "node:fs";
import { dirname, extname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { TYPES as devTypes } from "./dev/mime.mjs";
import { cacheControlFor, contentTypeFor, median, reportPasses } from "./perf/first-draw.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const probeSource = readFileSync(resolve(here, "perf/asset-packs-server.mjs"), "utf8");
const fail = (message) => {
  throw new Error(message);
};

const probeTypes = [...probeSource.matchAll(/\[("\.[a-z0-9]+"),\s*("[^"]+")\]/g)].map((m) => [JSON.parse(m[1]), JSON.parse(m[2])]);
if (probeTypes.length < 10) fail(`pack-cache probe type table not found (${probeTypes.length} entries)`);
const mimeMismatch = (typeOf) => probeTypes.filter(([ext, type]) => typeOf(`x${ext}`) !== type).map(([ext]) => ext);

const wrongMime = mimeMismatch(contentTypeFor);
if (wrongMime.length) fail("first-draw MIME types differ from the pack-cache probe for: " + wrongMime.join(", "));
const devControl = mimeMismatch((rel) => devTypes[extname(rel)] ?? "application/octet-stream");
if (devControl.length === 0) fail("negative control: scripts/dev/mime.mjs matches the pack-cache probe, so the MIME fixture cannot reject");

const IMMUTABLE = "public, max-age=31536000, immutable";
const REVALIDATE = "public, max-age=0, must-revalidate";
for (const literal of [IMMUTABLE, REVALIDATE]) {
  if (!probeSource.includes(literal)) fail(`pack-cache probe no longer sends ${literal}`);
}
const cacheFixtures = [
  { rel: "scenarios/moba/assets/packs/character/hero.abc.bin", expected: IMMUTABLE },
  { rel: "scenarios/moba/assets/packs/index.json", expected: IMMUTABLE },
  { rel: "site/packs/index.json", expected: REVALIDATE },
  { rel: "js/client/play.js", expected: "no-store" },
];
const cacheRejected = (policy) => cacheFixtures.filter((f) => policy(f.rel) !== f.expected).map((f) => f.rel);
const wrongCache = cacheRejected(cacheControlFor);
if (wrongCache.length) fail("cache-control differs from the pack-cache probe for: " + wrongCache.join(", "));
const priorPolicy = () => "no-store";
const cacheControl = cacheRejected(priorPolicy);
if (cacheControl.length === 0) fail("negative control: a no-store policy for every path passes the cache fixtures");

const cold = [5000, 6000, 7000];
const warm = [1000, 1200, 1400];
const reportFixtures = [
  {
    name: "cold and warm on separate lines",
    passes: { cold, warm },
    expected: ["first-draw cold worldbuilt-median-ms=6000 loads=3", "first-draw warm worldbuilt-median-ms=1200 loads=3"],
  },
  {
    name: "cold only",
    passes: { cold: [7000], warm: [] },
    expected: ["first-draw cold worldbuilt-median-ms=7000 loads=1"],
  },
  {
    name: "warm only",
    passes: { cold: [], warm: [900] },
    expected: ["first-draw warm worldbuilt-median-ms=900 loads=1"],
  },
];
const pooledReport = (passes) => {
  const all = [...passes.cold, ...passes.warm];
  return [`first-draw worldbuilt-median-ms=${median(all)} loads=${all.length}`];
};
const reportRejected = (report) =>
  reportFixtures.filter((f) => JSON.stringify(report(f.passes)) !== JSON.stringify(f.expected)).map((f) => f.name);

const wrongReport = reportRejected(reportPasses);
if (wrongReport.length) fail("cold and warm results are not reported separately for: " + wrongReport.join(", "));
const pooledControl = reportRejected(pooledReport);
if (pooledControl.length === 0) fail("negative control: a pooled cold+warm median passes the report fixtures");

console.log(
  `first-draw passes ok (${probeTypes.length} MIME types match the pack-cache probe; ${cacheFixtures.length} cache-control fixtures; ` +
    `${reportFixtures.length} report fixtures; pooled control rejected on: ${pooledControl.join(", ")}; ` +
    `no-store control rejected on: ${cacheControl.join(", ")}; dev MIME control rejected on: ${devControl.length} types)`,
);
