import { createRequire as __gmCreateRequire } from 'node:module';
const require = __gmCreateRequire(process.cwd() + '/');
(async () => {
  const fs = await import("node:fs");
  const { pathToFileURL } = await import("node:url");
  const rel = "js/client/play.js";
  const src = fs.readFileSync(rel, "utf8");
  const check = (text) => {
    const L = text.split("\n");
    const A = [], B = [], C = [];
    const at = {};
    const swapAt = L.findIndex((l) => /frame\.type === FRAME\.SWAP\) \{/.test(l));
    const guardAt = L.findIndex((l, i) => i > swapAt && /^\s*if \(pendingSwap\) \{/.test(l));
    const setAt = L.findIndex((l, i) => i > swapAt && /^\s*pendingSwap = swap;/.test(l));
    const guardReturns = guardAt >= 0 && L.slice(guardAt, guardAt + 9).some((l) => /^\s*return;/.test(l));
    at.peerGuard = guardAt + 1;
    at.peerSet = setAt + 1;
    if (!(swapAt >= 0 && guardAt > swapAt && setAt > guardAt && guardReturns)) A.push("peer SWAP handler does not refuse while pendingSwap is set");
    const annDef = L.findIndex((l) => /^async function announceSwap\(/.test(l));
    const annGuard = L.findIndex((l, i) => i > annDef && /if \(pendingSwap \|\| swapFetching\)/.test(l));
    const annSet = L.findIndex((l, i) => i > annDef && /swapFetching = true;/.test(l));
    at.announceSwapGuard = annGuard + 1;
    if (!(annDef >= 0 && annGuard > annDef && annSet > annGuard)) B.push("announceSwap does not refuse before it sets swapFetching");
    const calls = [];
    L.forEach((l, i) => {
      if (!/\bannouncePackSwap\(/.test(l) || /function announcePackSwap\(/.test(l)) return;
      calls.push(i + 1);
      if (!/if \(pendingSwap \|\| swapFetching/.test(L.slice(Math.max(0, i - 8), i).join("\n"))) B.push("announcePackSwap call at line " + (i + 1) + " has no pendingSwap/swapFetching guard");
    });
    at.packCalls = calls;
    if (calls.length === 0) B.push("no announcePackSwap call found, so the check would pass vacuously");
    const allowed = [/^\s*pendingSwap = null;/, /^\s*pendingSwap = swap;/, /^\s*pendingSwap = \{ tick, hash, file, kind,/, /^\s*pendingSwap = \{ tick, hash, file, url,/, /^\s*pendingSwap = \{ \.\.\.p, tick,/];
    const writes = [];
    L.forEach((l, i) => {
      if (/let pendingSwap/.test(l) || !/(^|[^.\w])pendingSwap = /.test(l)) return;
      writes.push(i + 1);
      if (!allowed.some((re) => re.test(l))) C.push("unclassified pendingSwap write at line " + (i + 1));
    });
    at.writes = writes;
    if (writes.length === 0) C.push("no pendingSwap writes found");
    return { A, B, C, at };
  };
  const M1 = src.replace(/\n {4}if \(pendingSwap\) \{\n {6}refuseSwap\(/, "\n    if (false) {\n      refuseSwap(");
  const M2 = src.replace("if (pendingSwap || swapFetching) { refuseSwap(", "if (false) { refuseSwap(");
  const M3 = src.replace("function sendRoster(peerPubkey) {", "pendingSwap = { tick: 0, hash: 0, file: \"x\" };\nfunction sendRoster(peerPubkey) {");
  const real = check(src), m1 = check(M1), m2 = check(M2), m3 = check(M3);
  const P = await import(pathToFileURL(process.cwd() + "/js/client/swap_protocol.js").href);
  const peers = new Set(["peerA"]);
  const commitWithoutReady = [];
  for (let d = 0; d <= P.SWAP_MAX_DEFERS; d++) for (let t = 100 - P.SWAP_GATE_TICKS; t <= 100; t++) {
    if (P.swapGate(t, { tick: 100, deferrals: d, readyPeers: new Set() }, peers) === "commit") commitWithoutReady.push("tick " + t + " deferrals " + d);
  }
  const readyCommit = P.swapGate(100 - P.SWAP_GATE_TICKS, { tick: 100, deferrals: 0, readyPeers: new Set(["peerA"]) }, peers);
  const proto = { commitWithoutReady, readyCommit, ok: commitWithoutReady.length === 0 && readyCommit === "commit" };
  const applied = M1 !== src && M2 !== src && M3 !== src;
  const ok = real.A.length + real.B.length + real.C.length === 0 && m1.A.length > 0 && m2.B.length > 0 && m3.C.length > 0 && applied && proto.ok;
  console.log(JSON.stringify({ file: rel, lines: src.split("\n").length, real, mutations: { applied, M1_flags_A: m1.A.length, M2_flags_B: m2.B.length, M3_flags_C: m3.C.length }, protocol: proto }, null, 1));
  console.log("RESULT: " + (ok ? "PASS" : "FAIL"));
  process.exitCode = ok ? 0 : 1;
})().catch((e) => { console.error(e); process.exitCode = 1; });