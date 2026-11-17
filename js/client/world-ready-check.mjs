import { worldReadyRoute } from "./lobby_link.js";

const expect = (cond, what) => { if (!cond) throw new Error(what); };
const ME = "host-key";
const game = { id: "game-a" };
const host = { hosting: true, peer: false, built: true, phase: "lobby", confirmedFor: null, game, lobbyHost: ME, me: ME };

const first = worldReadyRoute(host);
expect(first.send === true, "a built host world is announced while the lobby has not confirmed it");
expect(first.registerHost === false, "a host the lobby already shows is not registered again");

const unregistered = worldReadyRoute({ ...host, lobbyHost: "" });
expect(unregistered.send === true && unregistered.registerHost === true, "a host the lobby does not show yet is registered before the announcement");

const otherHost = worldReadyRoute({ ...host, lobbyHost: "someone-else" });
expect(otherHost.send === true && otherHost.registerHost === true, "a lobby showing another host is corrected to this host");

expect(worldReadyRoute({ ...host, confirmedFor: game }).send === false, "a confirmed game is not announced again");
expect(worldReadyRoute({ ...host, confirmedFor: { id: "game-b" } }).send === true, "a new game object is announced again, so a rematch is not stranded");
expect(worldReadyRoute({ ...host, built: false }).send === false, "an unbuilt world is not announced");
expect(worldReadyRoute({ ...host, peer: true }).send === false, "a peer page does not announce a world");
expect(worldReadyRoute({ ...host, phase: "loading" }).send === false, "the announcement is only sent in the lobby phase");
expect(worldReadyRoute({ ...host, game: null }).send === false, "no game means no announcement");

const soloHost = worldReadyRoute({ ...host, hosting: false });
expect(soloHost.send === true && soloHost.registerHost === false, "a solo page that the lobby names as host announces without registering");
expect(worldReadyRoute({ ...host, hosting: false, lobbyHost: "someone-else" }).send === false, "a page that is not the lobby host does not announce");

const oneShot = (route) => ({ ...route, registerHost: false });
let caught = false;
try {
  expect(oneShot(unregistered).registerHost === true, "negative control: a host that is not registered must be registered first");
} catch {
  caught = true;
}
expect(caught, "the negative control must fail the registration check");

console.log("world ready route ok (12 cases, negative control rejected)");
