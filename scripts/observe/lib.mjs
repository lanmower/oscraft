// Shared helpers for the observe scripts: artifact discovery from the play page, hashing, table printing.
import { createHash } from "node:crypto";
import { setDefaultResultOrder } from "node:dns";

// Some hosts resolve IPv6 first and stall on the connect; fetch then times out although curl works.
setDefaultResultOrder?.("ipv4first");

export const FIXED = ["version.json", "js/client/version.json"];

export const sha256 = (buf) => createHash("sha256").update(buf).digest("hex");

// Site-relative path of a file reference, resolved against the file that contains it.
const resolveFrom = (fromRel, spec) => new URL(spec, `http://site/${fromRel}`).pathname.slice(1);

// Walks the play page: its scripts, data-map, data-pkg, relative static imports, and .wasm references.
// readText(rel) returns the file text or null when absent. Returns sorted site-relative paths.
export async function discover(readText, page = "js/client/play.html") {
  const found = new Set([...FIXED, page]);
  const queue = [];
  const html = await readText(page);
  if (html == null) return [...found].sort();
  const attr = [...html.matchAll(/\b(src|data-map|data-pkg)="([^"]+)"/g)];
  for (const [, , v] of attr) {
    if (/^(https?:|data:)/.test(v)) continue;
    queue.push(resolveFrom(page, v));
  }
  while (queue.length) {
    const rel = queue.shift();
    if (found.has(rel)) continue;
    found.add(rel);
    if (!/\.m?js$/.test(rel)) continue;
    const text = await readText(rel);
    if (text == null) continue;
    for (const m of text.matchAll(/(?:from\s+|import\s*\(\s*)["'](\.{1,2}\/[^"']+)["']/g)) queue.push(resolveFrom(rel, m[1]));
    for (const m of text.matchAll(/["'`]([^"'`\s]+\.wasm)["'`]/g)) queue.push(resolveFrom(rel, m[1]));
  }
  return [...found].sort();
}

// Fixed-width table: rows are arrays of cell strings, widths fit the widest cell per column.
export function table(headers, rows) {
  const all = [headers, ...rows].map((r) => r.map((c) => String(c ?? "")));
  const widths = headers.map((_, i) => Math.max(...all.map((r) => r[i].length)));
  const line = (r) => r.map((c, i) => c.padEnd(widths[i])).join("  ").trimEnd();
  return [line(all[0]), widths.map((w) => "-".repeat(w)).join("  "), ...all.slice(1).map(line)].join("\n");
}

export const normBase = (base) => (base.endsWith("/") ? base : `${base}/`);

// Reads one site-relative path from the live base. A query cache-buster keeps a Pages edge copy from hiding the origin.
export function liveGet(base, cacheBust = Date.now()) {
  return async (rel) => {
    try {
      const r = await fetch(`${base}${rel}?cb=${cacheBust}`, { cache: "no-store", redirect: "follow" });
      return { status: r.status, buf: Buffer.from(await r.arrayBuffer()) };
    } catch (e) {
      return { status: 0, buf: null, error: e.cause?.code || e.message };
    }
  };
}

// Describes what an artifact claims about itself: version.json's sha, or an OSDT image's version and grid.
export function claimOf(rel, buf, mapStats) {
  if (!buf) return "-";
  if (rel.endsWith("version.json")) {
    try {
      const j = JSON.parse(buf.toString("utf8"));
      return `sha=${j.sha ?? "?"} built=${j.built ?? "?"}`;
    } catch {
      return "unparseable";
    }
  }
  if (rel.endsWith(".bin")) {
    try {
      const s = mapStats(buf);
      return `OSDT v${s.format.version} ${s.grid.width}x${s.grid.height}`;
    } catch (e) {
      return `not OSDT: ${e.message}`;
    }
  }
  return "-";
}
