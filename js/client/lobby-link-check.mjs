import { expect } from "./check-expect.mjs";
import { LOBBY_LINK_WAIT_MS, lobbyInputRoute, heldCallsAfterSettle, localApplyArgs } from "./lobby_link.js";

const solo = { netState: null, hosting: false, peers: 0, settled: false, name: "ui", call: "mode", phase: "lobby" };

const queued = lobbyInputRoute(solo);
expect(queued.apply === true, "a solo lobby click must be applied without waiting for the network");
expect(queued.queue === true, "a solo lobby click is kept for host replay");
expect(queued.broadcast === false, "a solo lobby click is not broadcast");

expect(lobbyInputRoute({ ...solo, call: "world_ready" }).queue === false, "world_ready is never queued");
expect(lobbyInputRoute({ ...solo, settled: true }).queue === false, "settle stops queuing");
expect(lobbyInputRoute({ ...solo, phase: "live" }).queue === false, "live clicks are never queued");

const alone = { netState: null, hosting: false, peers: 2, settled: false, name: "ui", call: "alone", phase: "lobby" };
expect(lobbyInputRoute(alone).apply === true, "play alone must be applied on the page");
expect(lobbyInputRoute(alone).broadcast === false, "play alone must not be broadcast");
expect(lobbyInputRoute(alone).queue === false, "play alone must not wait for host replay");
expect(lobbyInputRoute({ ...solo, name: "pointer" }).queue === false, "non-ui input is never queued");
expect(lobbyInputRoute({ ...solo, netState: { tick: 1 } }).broadcast === true, "a net guest broadcasts");
expect(lobbyInputRoute({ ...solo, peers: 2 }).broadcast === true, "a peer page broadcasts");
expect(lobbyInputRoute({ ...solo, hosting: true }).queue === false, "a host does not queue");

expect(heldCallsAfterSettle({ peers: 0 }) === "clear", "a solo page discards its replay queue");
expect(heldCallsAfterSettle({ peers: 2 }) === "keep", "a peer page keeps the queue for ROSTER");
expect(Number.isFinite(LOBBY_LINK_WAIT_MS) && LOBBY_LINK_WAIT_MS >= 0, "the settle deadline is bounded");

const claim = { call: "claim", args: { slot: 5 } };
const tagged = localApplyArgs({ name: "ui", args: claim, hosting: false });
expect(tagged.args.preview === true, "a joiner's local claim is applied as a preview");
expect(tagged.args.slot === 5 && tagged.call === "claim", "the preview keeps the claim's call and seat");
expect(claim.args.preview === undefined, "the sent copy is never tagged");
expect(localApplyArgs({ name: "ui", args: claim, hosting: true }) === claim, "a host's claim is applied as sent");
expect(localApplyArgs({ name: "ui", args: { call: "mode", args: { code: "ap" } }, hosting: false }).args.preview === undefined, "only claims are previews");
expect(localApplyArgs({ name: "pointer", args: claim, hosting: false }) === claim, "non-ui input is never a preview");

console.log("lobby link ok (22 cases)");
