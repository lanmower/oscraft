#!/usr/bin/env node
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { discover, liveGet, normBase, sha256, table } from "./lib.mjs";
import { flatten, mapStats } from "./map-stats.mjs";

const opt = (args, name, dflt) => {
  const i = args.indexOf(name);
  return i >= 0 ? args[i + 1] : dflt;
};

// Negative-check categories: `--probe <category>` tampers with one discovered
// path of that kind and requires the compare to report MISMATCH for it, which
// is how the gate is proven to fail when a shipped file differs.
const PROBE = {
  manifest: (r) => /manifest\.json$/.test(r),
  lua: (r) => r.endsWith(".lua"),
  "pack-index": (r) => /\/packs\/index\.json$/.test(r),
  atlas: (r) => /\/packs\/[^/]+\.(webp|ktx2)$/.test(r),
  pack: (r) => /\/packs\/character\/.*\.bin$/.test(r),
  latest: (r) => r === "packs/latest.json",
  shader: (r) => r.endsWith(".glsl"),
  asset: (r) => /\.(glb|jpg|ogg)$/.test(r),
  "scene-texture": (r) => /\.(ktx2|png)$/.test(r) && !r.includes("/packs/"),
  catalog: (r) => /(?:^|\/)assets\.json$/.test(r),
};

function readLocal(dir) {
  return async (rel) => {
    try {
      return readFileSync(join(dir, rel));
    } catch {
      return null;
    }
  };
}

function statsOf(buf) {
  if (!buf) return null;
  try {
    return flatten(mapStats(buf));
  } catch (e) {
    return { parse_error: e.message };
  }
}

async function pass(paths, local, base, expectSha) {
  const request = liveGet(base, Date.now());
  // Pages answers 503/429 when a full-catalogue pass bursts at it, and a
  // dropped connection surfaces as status 0: retry those on the spot rather
  // than fail (or re-run) an attempt over thousands of otherwise identical
  // paths. A real 404 or a mismatch is returned at once.
  const transient = (s) => s === 503 || s === 429 || s === 0;
  const get = async (rel) => {
    let r = await request(rel);
    for (let i = 0; i < 3 && transient(r.status); i++) {
      await new Promise((res) => setTimeout(res, 500 * (i + 1)));
      r = await request(rel);
    }
    return r;
  };
  const live = new Map();
  // The extended path set runs into the thousands (asset catalogue); a
  // bounded batch keeps the fetch fan-out sane against Pages.
  const BATCH = 32;
  for (let i = 0; i < paths.length; i += BATCH) {
    await Promise.all(paths.slice(i, i + BATCH).map(async (rel) => live.set(rel, await get(rel))));
  }
  const rows = [];
  const statRows = [];
  let bad = 0;
  for (const rel of paths) {
    const l = local.get(rel);
    const r = live.get(rel);
    const liveOk = r.status === 200;
    const liveSha = liveOk ? sha256(r.buf) : null;
    const localSha = l ? sha256(l) : null;
    let result;
    if (!l) result = "missing locally";
    else if (!liveOk) result = `HTTP ${r.status || r.error}`;
    else if (localSha !== liveSha) result = "MISMATCH";
    else result = "match";
    if (result !== "match") bad++;
    if (expectSha && rel.endsWith("version.json") && liveOk) {
      try {
        if (JSON.parse(r.buf.toString("utf8")).sha !== expectSha) {
          result = `MISMATCH want=${expectSha.slice(0, 12)}`;
          bad++;
        }
      } catch {
        result = "unparseable version.json";
        bad++;
      }
    }
    // A green run covers thousands of paths; print only the differences.
    if (result !== "match") rows.push([rel, localSha ? localSha.slice(0, 12) : "-", liveSha ? liveSha.slice(0, 12) : "-", result]);
    // map-stats applies to OSDT images only: discovery now also reaches the
    // character pack chunks, which are OSPK archives, not map images.
    if (rel.endsWith(".bin") && l && liveOk && !statsOf(l).parse_error) {
      const a = statsOf(l);
      const b = statsOf(r.buf);
      for (const key of new Set([...Object.keys(a), ...Object.keys(b)])) {
        const same = a[key] === b[key];
        if (!same) bad++;
        statRows.push([`${rel} ${key}`, a[key] ?? "-", b[key] ?? "-", same ? "same" : "DIFF"]);
      }
    }
  }
  return { rows, statRows, bad };
}

async function main() {
  const args = process.argv.slice(2);
  const valueSlots = new Set(["--expect-sha", "--attempts", "--backoff", "--probe"].map((f) => args.indexOf(f)).filter((i) => i >= 0).map((i) => i + 1));
  const [dir, base] = args.filter((a, i) => !a.startsWith("--") && !valueSlots.has(i));
  if (!dir || !base) {
    console.error("usage: compare.mjs <local-site-dir> <base-url> [--expect-sha sha] [--attempts n] [--backoff s] [--probe category]");
    process.exit(2);
  }
  const expectSha = opt(args, "--expect-sha");
  const attempts = Number(opt(args, "--attempts", 6));
  const backoff = Number(opt(args, "--backoff", 10));
  const local = readLocal(dir);
  const paths = await discover(async (rel) => {
    const b = await local(rel);
    return b ? b.toString("utf8") : null;
  });
  const localBuf = new Map();
  for (const rel of paths) localBuf.set(rel, await local(rel));
  const probeCat = opt(args, "--probe");
  if (probeCat) {
    const pred = PROBE[probeCat];
    if (!pred) {
      console.error(`--probe: unknown category "${probeCat}"; known: ${Object.keys(PROBE).join(", ")}`);
      process.exit(2);
    }
    const candidates = paths.filter((p) => pred(p) && localBuf.get(p)?.length);
    if (!candidates.length) {
      console.error(`--probe: no discovered path in category ${probeCat}`);
      process.exit(2);
    }
    // A negative check only proves detection when the untouched file matches
    // the live copy first, so walk candidates until one agrees with the site.
    const base0 = normBase(base);
    let rel = null;
    for (const cand of candidates.slice(0, 25)) {
      const before = await pass([cand], localBuf, base0, expectSha);
      if (before.rows.length === 0) { rel = cand; break; }
    }
    if (!rel) {
      console.error(`--probe: every ${probeCat} candidate already differs from the live site; cannot discriminate`);
      process.exit(2);
    }
    const original = localBuf.get(rel);
    localBuf.set(rel, Buffer.concat([original, Buffer.from("\nprobe\n")]));
    const out = await pass([rel], localBuf, base0, expectSha);
    localBuf.set(rel, original);
    const row = out.rows.find((r) => r[0] === rel);
    const detected = row !== undefined && row[3].startsWith("MISMATCH");
    console.log(`probe ${probeCat}: ${rel} matched live, then tampered -> ${row ? row[3] : "match"}`);
    console.log(detected ? `RESULT: probe detected (${probeCat})` : `RESULT: probe NOT detected (${probeCat})`);
    process.exit(detected ? 0 : 1);
  }
  let last;
  for (let n = 1; n <= attempts; n++) {
    last = await pass(paths, localBuf, normBase(base), expectSha);
    if (last.bad === 0) break;
    if (n < attempts) {
      const wait = backoff * n;
      console.error(`attempt ${n}/${attempts}: ${last.bad} difference(s); retrying in ${wait}s`);
      await new Promise((res) => setTimeout(res, wait * 1000));
    }
  }
  console.log(table(["ARTIFACT", "LOCAL", "LIVE", "RESULT"], last.rows));
  if (last.statRows.length) {
    console.log("\nmap-stats, local vs live:");
    console.log(table(["FIELD", "LOCAL", "LIVE", "DIFF"], last.statRows));
  }
  console.log(last.bad === 0 ? `\nRESULT: all artifacts match (${paths.length} paths)` : `\nRESULT: ${last.bad} difference(s) of ${paths.length} paths`);
  process.exit(last.bad === 0 ? 0 : 1);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) await main();
