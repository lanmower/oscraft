#!/usr/bin/env node
// Read-only probe of a deployed Pages site. For every artifact the play page uses: URL, HTTP status, bytes,
// sha256 and the sha/version the artifact claims. Exits 1 on any non-200, or when version.json's sha differs
// from --expect-sha.
// Usage: node scripts/observe/live.mjs <base-url> [--expect-sha <sha>]
import { fileURLToPath } from "node:url";
import { claimOf, discover, liveGet, normBase, sha256, table } from "./lib.mjs";
import { mapStats } from "./map-stats.mjs";

const VERSION_FILES = ["version.json", "js/client/version.json"];

export async function probe(base, expectSha) {
  const get = liveGet(base);
  const cache = new Map();
  const fetchOnce = async (rel) => {
    if (!cache.has(rel)) cache.set(rel, await get(rel));
    return cache.get(rel);
  };
  const paths = await discover(async (rel) => {
    const r = await fetchOnce(rel);
    return r.status === 200 ? r.buf.toString("utf8") : null;
  });
  await Promise.all(paths.map(fetchOnce));
  const rows = [];
  let failed = false;
  for (const rel of paths) {
    const r = await fetchOnce(rel);
    const ok = r.status === 200;
    let result = ok ? "OK" : `FAIL ${r.status || r.error}`;
    if (!ok) failed = true;
    let claim = ok ? claimOf(rel, r.buf, mapStats) : "-";
    if (ok && VERSION_FILES.includes(rel) && expectSha) {
      const claimed = (() => {
        try {
          return JSON.parse(r.buf.toString("utf8")).sha;
        } catch {
          return undefined;
        }
      })();
      if (claimed !== expectSha) {
        result = `MISMATCH want=${expectSha.slice(0, 12)}`;
        failed = true;
      }
    }
    rows.push([base + rel, r.status || "ERR", ok ? r.buf.length : "-", ok ? sha256(r.buf).slice(0, 16) : "-", claim, result]);
  }
  return { rows, failed };
}

async function main() {
  const args = process.argv.slice(2);
  const i = args.indexOf("--expect-sha");
  const expectSha = i >= 0 ? args[i + 1] : undefined;
  const base = args.find((a, j) => !a.startsWith("--") && (i < 0 || j !== i + 1));
  if (!base) {
    console.error("usage: live.mjs <base-url> [--expect-sha <sha>]");
    process.exit(2);
  }
  const { rows, failed } = await probe(normBase(base), expectSha);
  console.log(table(["URL", "STATUS", "BYTES", "SHA256", "CLAIM", "RESULT"], rows));
  process.exit(failed ? 1 : 0);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) await main();
