import { createFrameTimer } from "./frame_timer.js";
import { createNetReadout, describeReadout, linkState, measureRoundTripMs, selectedRoundTripMs } from "./net_readout.js";

const failures = [];
const expect = (name, got, want) => {
  const ok = JSON.stringify(got) === JSON.stringify(want);
  if (!ok) failures.push(`${name}: got ${JSON.stringify(got)} want ${JSON.stringify(want)}`);
  console.log(`${ok ? "ok" : "FAIL"} ${name} ${JSON.stringify(got)}`);
};

const report = (entries) => new Map(entries.map((e) => [e.id, e]));
const pair = (id, rtt, extra = {}) => ({ id, type: "candidate-pair", state: "succeeded", nominated: true, currentRoundTripTime: rtt, ...extra });

expect("state good", linkState(60, 40), "good");
expect("state fair by fps", linkState(30, 40), "fair");
expect("state poor by rtt", linkState(60, 300), "poor");
expect("state unknown", linkState(0, null), "unknown");

expect("rtt picks nominated pair in ms", selectedRoundTripMs(report([pair("a", 0.05), { id: "l", type: "local-candidate" }])), 50);
expect("rtt ignores unselected pair", selectedRoundTripMs(report([pair("a", 0.05, { nominated: false })])), null);
expect("rtt ignores pair without value", selectedRoundTripMs(report([pair("a", undefined)])), null);

const pcs = [
  { getStats: async () => report([pair("a", 0.12)]) },
  { getStats: async () => report([pair("b", 0.08)]) },
  { getStats: async () => { throw new Error("closed"); } },
];
expect("measure takes lowest across peers", await measureRoundTripMs(pcs), 80);
expect("measure with no peers", await measureRoundTripMs([]), null);

const described = describeReadout(59.6, 42.4);
expect("describe text", described.text, "60 FPS  42 ms");
expect("describe label names state", described.label, "Connection good: 60 FPS  42 ms");
expect("describe glyph and colour", [described.glyph, described.colour], ["●", "#7bd88f"]);
expect("describe no data", describeReadout(0, null).text, "-- FPS  -- ms");

let now = 0;
const timer = createFrameTimer(() => now);
for (let i = 0; i < 10; i++) {
  timer.tick();
  now += 1000 / 30;
}
expect("timer-derived fps", describeReadout(Math.round(1000 / timer.summary().meanMs), null).state, "fair");

const listeners = [];
const doc = {
  body: { append(node) { doc.attached = node; } },
  defaultView: {
    setInterval: (fn) => { listeners.push(fn); return listeners.length; },
    clearInterval: (id) => { listeners[id - 1] = null; },
  },
  createElement: () => ({ style: {}, hidden: false, textContent: "", setAttribute(k, v) { this[k] = v; }, append() {} }),
};
const readout = createNetReadout({ doc, timer, peerConnections: pcs, refreshMs: 1000 });
expect("starts hidden", readout.isShown(), false);
expect("toggle shows", readout.toggle(), true);
await new Promise((resolve) => setTimeout(resolve, 0));
expect("shown label carries rtt", readout.element["aria-label"], "Connection fair: 30 FPS  80 ms");
expect("toggle hides", readout.toggle(), false);
expect("hidden element", readout.element.hidden, true);

if (failures.length) {
  console.log(failures.join("\n"));
  process.exit(1);
}
console.log("net-readout-check: all passed");
