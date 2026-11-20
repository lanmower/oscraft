import assert from "node:assert/strict";
import { raceVerdict, startRelay } from "./matchmaking-race.mjs";

assert.equal(raceVerdict(["#match-a", "#match-a", "#match-a"]).ok, true, "three pages in one match room pass");
assert.equal(raceVerdict(["#match-a", "#match-b", "#match-a"]).ok, false, "a split room is red");
assert.match(raceVerdict(["#match-a", "#match-b"]).reasons[0], /split into 2 match rooms/);
assert.equal(raceVerdict(["#match-a", "#lobby", "#match-a"]).ok, false, "a page on the default room is red");
assert.equal(raceVerdict(["", "#match-a", "#match-a"]).ok, false, "a page with no room is red");

async function until(test) {
  for (let i = 0; i < 300 && !test(); i += 1) await new Promise((resolve) => setTimeout(resolve, 10));
  assert.ok(test(), "the relay answered in time");
}

const relay = await startRelay();
try {
  const messages = [];
  const reader = new WebSocket(relay.url);
  reader.addEventListener("message", (m) => messages.push(JSON.parse(m.data)));
  await new Promise((resolve) => reader.addEventListener("open", resolve, { once: true }));
  reader.send(JSON.stringify(["REQ", "sub", { kinds: [30078], "#t": ["oscraft-room"] }]));
  await until(() => messages.some((m) => m[0] === "EOSE"));
  const writer = new WebSocket(relay.url);
  await new Promise((resolve) => writer.addEventListener("open", resolve, { once: true }));
  writer.send(JSON.stringify(["EVENT", { id: "e1", kind: 30078, created_at: 1, pubkey: "p", tags: [["t", "oscraft-room"]], content: "" }]));
  await until(() => messages.some((m) => m[0] === "EVENT" && m[2].id === "e1"));
  reader.close();
  writer.close();
} finally {
  await relay.close();
}

console.log("matchmaking race checks ok (5 verdict cases, relay forwards a matching event)");
