import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import * as current from "./swap_protocol.js";

const here = path.dirname(fileURLToPath(import.meta.url));
const OLD_REV = "9eb8ddc5^";
const OLD_PATH = "js/client/swap_protocol.js";
const PEER = "p1";
const JOINER = "p2";
const SWAP_TICK = 100;
const FIRST_TICK = 90;
const LAST_TICK = 140;
const SCHEDULE = {
  90: [["ready", PEER]],
  94: [["join", JOINER]],
  95: [["ready", PEER]],
  110: [["ready", JOINER]],
};

function oldStep(mod, state, ticks) {
  const pending = state.pending;
  if (pending.decided) return;
  const action = mod.swapGate(ticks, pending, state.peers);
  if (action === "wait") return;
  if (action === "commit") {
    pending.decided = true;
    return;
  }
  if (action === "defer") {
    state.pending = {
      ...pending,
      tick: pending.tick + mod.SWAP_DEFER_TICKS,
      deferrals: pending.deferrals + 1,
      error: null,
      readyPeers: new Set(),
      decided: false,
    };
    return;
  }
  state.pending = null;
}

function newStep(mod, state, ticks) {
  state.pending = mod.hostSwapStep(state.pending, ticks, state.peers).pending;
}

const VARIANTS = {
  old: { step: oldStep, admit: (mod, pending) => pending },
  new: { step: newStep, admit: (mod, pending, key) => mod.swapAdmitPeer(pending, key) },
};

function runScenario(name, mod) {
  const variant = VARIANTS[name];
  const state = {
    peers: new Set([PEER]),
    pending: {
      tick: SWAP_TICK,
      hash: "h1",
      file: "data.osd",
      image: { host: true },
      error: null,
      deferrals: 0,
      readyPeers: new Set(),
      decided: false,
    },
    committedAt: null,
    joinedAt: null,
    applied: null,
  };
  for (let ticks = FIRST_TICK; ticks <= LAST_TICK && state.applied === null && state.pending !== null; ticks += 1) {
    for (const [kind, key] of SCHEDULE[ticks] ?? []) {
      if (kind === "ready") state.pending.readyPeers.add(key);
      if (kind === "join") {
        state.joinedAt = ticks;
        state.peers.add(key);
        state.pending = variant.admit(mod, state.pending, key);
      }
    }
    if (state.pending === null) break;
    variant.step(mod, state, ticks);
    if (state.pending === null) break;
    if (state.pending.decided && state.committedAt === null) state.committedAt = ticks;
    if (state.pending.tick === ticks && state.pending.decided) {
      const unready = [...state.peers].filter((key) => !state.pending.readyPeers.has(key)).length;
      state.applied = { tick: ticks, unready };
    }
  }
  if (state.committedAt === null || state.committedAt >= state.joinedAt) {
    throw new Error(`${name}: the joiner did not join after the commit`);
  }
  return state;
}

function outcome(state) {
  if (state.applied === null) return { verdict: "FAIL", text: "not applied" };
  const verdict = state.applied.unready === 0 ? "PASS" : "FAIL";
  return { verdict, text: `applied at tick ${state.applied.tick} with ${state.applied.unready} peer(s) unready` };
}

const scratch = fs.mkdtempSync(path.join(os.tmpdir(), "swap-admit-"));
try {
  const oldSource = process.argv[2]
    ? fs.readFileSync(path.resolve(process.argv[2]), "utf8")
    : execFileSync("git", ["show", `${OLD_REV}:${OLD_PATH}`], { cwd: here, encoding: "utf8" });
  const oldFile = path.join(scratch, "swap_protocol.old.mjs");
  fs.writeFileSync(oldFile, oldSource);
  const old = await import(pathToFileURL(oldFile).href);
  const o = outcome(runScenario("old", old));
  const n = outcome(runScenario("new", current));
  console.log(`joiner after commit: old ${o.verdict} (${o.text}); new ${n.verdict} (${n.text})`);
  process.exitCode = o.verdict === "FAIL" && n.verdict === "PASS" ? 0 : 1;
} finally {
  fs.rmSync(scratch, { recursive: true, force: true });
}
