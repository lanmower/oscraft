import { LOBBY_LINK_WAIT_MS, lobbyInputRoute, heldCallsAfterSettle } from "./lobby_link.js";

const expect = (cond, what) => { if (!cond) throw new Error(what); };
const solo = { netState: null, hosting: false, peers: 0, settled: false, name: "ui", call: "mode", phase: "lobby" };

const queued = lobbyInputRoute(solo);
expect(queued.apply === true, "a solo lobby click must be applied without waiting for the network");
expect(queued.queue === true, "a solo lobby click is kept for host replay");
expect(queued.broadcast === false, "a solo lobby click is not broadcast");

expect(lobbyInputRoute({ ...solo, call: "world_ready" }).queue === false, "world_ready is never queued");
expect(lobbyInputRoute({ ...solo, settled: true }).queue === false, "settle stops queuing");
expect(lobbyInputRoute({ ...solo, phase: "live" }).queue === false, "live clicks are never queued");
expect(lobbyInputRoute({ ...solo, name: "pointer" }).queue === false, "non-ui input is never queued");
expect(lobbyInputRoute({ ...solo, netState: { tick: 1 } }).broadcast === true, "a net guest broadcasts");
expect(lobbyInputRoute({ ...solo, peers: 2 }).broadcast === true, "a peer page broadcasts");
expect(lobbyInputRoute({ ...solo, hosting: true }).queue === false, "a host does not queue");

expect(heldCallsAfterSettle({ peers: 0 }) === "clear", "a solo page discards its replay queue");
expect(heldCallsAfterSettle({ peers: 2 }) === "keep", "a peer page keeps the queue for ROSTER");
expect(Number.isFinite(LOBBY_LINK_WAIT_MS) && LOBBY_LINK_WAIT_MS >= 0, "the settle deadline is bounded");

console.log("lobby link ok (12 cases)");
