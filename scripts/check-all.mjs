import { spawnSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const args = process.argv.slice(2);
const strict = args.includes("--strict");
const listOnly = args.includes("--list");
const asJson = args.includes("--json");

const tools = (name) => ["scenarios/moba/tools/" + name, "--check"];

const GATE_CHECKS = [
  "scripts/check-ability-effect-mapping.mjs",
  "scripts/check-doc-claims.mjs",
  "js/client/lobby-link-check.mjs",
  "js/client/lobby-replay-check.mjs",
  "scripts/check-pack-archive.mjs",
  "scripts/check-site-bundle.mjs",
  "scripts/check-wasip3-parity.mjs",
  "scripts/check-relay-tie-order.mjs",
  "scripts/check-release-layout.mjs",
  "js/client/frame-chain-check.mjs",
  "js/client/safe-storage-check.mjs",
  "js/client/hud-hot-check.mjs",
  "js/client/key-names-check.mjs",
  "scripts/check-wire-codec.mjs",
  "scripts/check-hook-order.mjs",
  "scripts/check-generated-lua.mjs",
  "scripts/check-lua-bundle.mjs",
  "scripts/check-example-mods.mjs",
  "scripts/check-effect-mapping-depth.mjs",
  "scripts/check-perf-probes.mjs",
  "scripts/check-browser-witnesses.mjs",
  "scripts/check-naming-doc.mjs",
  "scripts/check-acronyms.mjs",
  "scripts/check-modulepreload.mjs",
  "scripts/check-pack-allow-list.mjs",
  "scripts/check-staged-secrets.mjs",
  "scripts/check-hero-models.mjs",
  "scenarios/moba/tools/check-camp-placement.mjs",
  "scenarios/moba/tools/check-mode-hud.mjs",
  "js/client/camera-mouse-check.mjs",
  "js/client/draft-clock-check.mjs",
  "js/client/hero-run-check.mjs",
  "js/client/hero-view-lock-check.mjs",
  "js/client/hud-canvas-check.mjs",
  "js/client/hud-canvas-keys-check.mjs",
  "js/client/lobby-seats-check.mjs",
  "js/client/minimap-check.mjs",
  "js/client/mode-badge-check.mjs",
  "js/client/model-order-check.mjs",
  "js/client/net-readout-check.mjs",
  "js/client/net-recovery-check.mjs",
  "js/client/notice-toast-check.mjs",
  "js/client/ping-wheel-check.mjs",
  "js/client/swap-admit-check.mjs",
  "js/client/zoom-check.mjs",
];

const GATE_GENERATORS = [
  "gen-abilities-688g-missing.mjs",
  "gen-abilities.mjs",
  "gen-buff-fx.mjs",
  "gen-hero-sides.mjs",
  "gen-shops-688g.mjs",
  "gen-tower-bodies.mjs",
  "gen-trees-688g.mjs",
  "retire-688g-abilities.mjs",
  "apply-items-688g.mjs",
  "apply-pathing-688g.mjs",
  "gen-ability-fx.mjs",
  "gen-lane-routes-688g.mjs",
];

const DRIFT = [
  { id: "gen-turn-rates.mjs", output: "turn_rates.lua", reason: "reads the machine-local hy3d 6.88g unit.ini; a --check failure means the committed rates diverged from the generator" },
];

const EXTERNAL = [
  { id: "js/client/editor-check.mjs", reason: "needs the built rust/target/debug/osdt-data binary" },
  { id: "gen-ability-keys.mjs", reason: "needs the hy3d g688-lni directory" },
  { id: "gen-ability-text-688g.mjs", reason: "needs the hy3d 6.88g ability.ini; HEAD text is not reproducible (445 of 450 entries)" },
  { id: "gen-neutral-688g.mjs", reason: "needs --src with the 6.88g war3map.j script" },
  { id: "gen-heroes.mjs", reason: "needs --src ref-heroes-src; without it --check attempts a SteamDatabase fetch" },
];

const nodeEntry = (kind, id, state, argv, extra) => Object.assign({ kind: kind, id: id, state: state, argv: argv }, extra || {});
const entries = [
  { kind: "check", id: "git diff --check", state: "gate", exe: "git", argv: ["diff", "--check"] },
  ...GATE_CHECKS.map((id) => nodeEntry("check", id, "gate", [id])),
  ...GATE_GENERATORS.map((id) => nodeEntry("gen", id, "gate", tools(id))),
  ...DRIFT.map((d) => nodeEntry("gen", d.id, "drift", tools(d.id), { output: d.output, reason: d.reason })),
  ...EXTERNAL.map((e) => ({ kind: e.id.startsWith("js/") ? "check" : "gen", id: e.id, state: "external", reason: e.reason })),
];

if (listOnly) {
  for (const e of entries) console.log(e.state.padEnd(8) + " " + e.id + (e.reason ? "  " + e.reason : ""));
  console.log("LIST gates=" + entries.filter((e) => e.state === "gate").length + " drift=" + DRIFT.length + " external=" + EXTERNAL.length);
  process.exit(0);
}

const rows = [];
for (const e of entries) {
  if (e.state === "external") { rows.push(Object.assign({}, e, { verdict: "SKIP", ms: 0, text: e.reason })); continue; }
  const begun = process.hrtime.bigint();
  const r = spawnSync(e.exe || process.execPath, e.argv, { cwd: root, encoding: "utf8", timeout: 30000, maxBuffer: 4 * 1024 * 1024 });
  const ms = Number((process.hrtime.bigint() - begun) / 1000000n);
  const text = (String(r.stdout || "") + String(r.stderr || "")).trim();
  const code = r.status === null ? 124 : r.status;
  const last = text.split("\n").map((s) => s.trim()).filter(Boolean).at(-1) || "";
  let verdict = e.state === "gate" ? (code === 0 ? "ok" : "FAIL") : (code === 0 ? "ok" : "DRIFT");
  if (e.state === "drift" && /ENOENT|usage:|required|--src/.test(text)) verdict = "SKIP";
  rows.push(Object.assign({}, e, { verdict: verdict, code: code, ms: ms, text: last.slice(0, 150) }));
}

const gates = rows.filter((r) => r.state === "gate");
const failed = rows.filter((r) => r.verdict === "FAIL");
const drift = rows.filter((r) => r.verdict === "DRIFT");
const skipped = rows.filter((r) => r.verdict === "SKIP");
const ok = gates.filter((r) => r.verdict === "ok").length;
for (const r of rows) console.log(r.kind.padEnd(5) + " " + r.id.padEnd(48) + " " + r.verdict.padEnd(6) + " " + String(r.ms).padStart(6) + "ms  " + r.text);
for (const r of drift) console.log("DRIFT " + r.output + ": " + r.reason);
for (const r of skipped) if (r.state !== "external") console.log("SKIP " + r.id + ": " + r.text);
console.log("RESULT gates=" + gates.length + " ok=" + ok + " failed=" + failed.length + " drift=" + drift.length + " skipped=" + skipped.length + " strict=" + strict);
if (asJson) console.log(JSON.stringify({ gates: gates.length, ok: ok, failed: failed.length, drift: drift.length, skipped: skipped.length, rows: rows }, null, 2));
process.exit(failed.length || (strict && drift.length) ? 1 : 0);
