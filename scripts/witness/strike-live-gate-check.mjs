import { runWitness } from "./strike-while-walking.mjs";
import { setupLiveMatch } from "./strike-live-setup.mjs";

const expect = (cond, what) => { if (!cond) throw new Error(what); };

const fakePage = (phase, calls) => ({
  evaluate: async (fn) => {
    if (String(fn).includes("dataset.phase")) return phase;
    calls.push("evaluate");
    return true;
  },
  addScriptTag: async () => { calls.push("addScriptTag"); },
  locator: () => ({ getAttribute: async () => "false", click: async () => { calls.push("click"); } }),
  waitForTimeout: async () => {},
  waitForFunction: async () => { calls.push("waitForFunction"); },
  goto: async (url) => { calls.push("goto " + url); },
});

for (const phase of ["lobby", "loading", ""]) {
  const calls = [];
  let refused = null;
  try { await runWitness(fakePage(phase, calls)); } catch (err) { refused = err.message; }
  const shown = phase === "" ? "phase \"\"" : phase;
  expect(refused !== null && refused.includes("refused") && refused.includes(shown), "runWitness must refuse in phase " + JSON.stringify(phase) + ": " + refused);
  expect(!calls.includes("addScriptTag") && !calls.includes("click"), "a refused witness must not load scripts or click in phase " + JSON.stringify(phase));
}

{
  const calls = [];
  let failed = null;
  try { await setupLiveMatch(fakePage("lobby", calls), "gate-check"); } catch (err) { failed = err.message; }
  expect(failed !== null && failed.includes("did not reach a live match") && failed.includes("lobby"), "setup must fail when the page never reaches live: " + failed);
  expect(calls.some((c) => c.startsWith("goto ") && c.includes("#match-gate-check")), "setup must enter a named room (a hash), or matchmaking reloads the page at 15 s: " + calls.join(", "));
}

console.log("strike live gate ok (4 cases)");
