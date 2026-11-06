import { replayHeldCalls, heldCallsAfterSettle } from "./lobby_link.js";

const expect = (cond, what) => { if (!cond) throw new Error(what); };
const held = [{ name: "ui", args: { call: "mode" } }, { name: "ui", args: { call: "ready" } }, { name: "ui", args: { call: "claim" } }];
const sent = replayHeldCalls(held, { hasHost: true });
expect(sent.length === 3, "ROSTER sends every held call");
expect(sent.map((c) => c.args.call).join() === "mode,ready,claim", "held calls keep their order");
expect(held.length === 0, "the queue drains on ROSTER");
expect(replayHeldCalls(held, { hasHost: true }).length === 0, "a second ROSTER sends nothing");
const kept = [{ name: "ui", args: { call: "pick" } }];
expect(replayHeldCalls(kept, { hasHost: false }).length === 0, "no host keeps the calls queued");
expect(kept.length === 1, "calls survive until a host appears");
expect(heldCallsAfterSettle({ peers: 0 }) === "clear", "a solo page clears its queue");
expect(heldCallsAfterSettle({ peers: 2 }) === "keep", "a peer page keeps its queue for ROSTER");

console.log("lobby replay ok (8 cases)");
