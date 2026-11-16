import { relaysUiEvent, mirrorsHostLobby } from "./lobby_link.js";

const expect = (cond, what) => { if (!cond) throw new Error(what); };
const guest = { hosting: false, relayed: true, kind: "lobby", started: false };

expect(mirrorsHostLobby(guest) === true, "a guest mirrors the host's relayed pre-start lobby");
expect(mirrorsHostLobby({ ...guest, relayed: false }) === false, "a guest's own lobby events are not mirrored back into its sim");
expect(mirrorsHostLobby({ ...guest, hosting: true }) === false, "a host never mirrors its own lobby");
expect(mirrorsHostLobby({ ...guest, started: true }) === false, "a started lobby is not mirrored");
expect(mirrorsHostLobby({ ...guest, kind: "widget" }) === false, "only lobby state events are mirrored");

expect(relaysUiEvent({ kind: "widget", id: "lobby" }) === false, "the host does not relay its perspective-bound lobby widget");
expect(relaysUiEvent({ kind: "widget", id: "hud" }) === true, "other widgets still reach peers");
expect(relaysUiEvent({ kind: "lobby" }) === true, "the lobby state event still reaches peers");
expect(relaysUiEvent({ kind: "widget", id: "scoreboard" }) === true, "live panels still reach peers");

console.log("lobby mirror ok (9 cases)");
