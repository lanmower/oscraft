import { chooseRoom, matchmakingTarget, settleMatch, MATCH_ROOM_PREFIX, DEFAULT_ROOM } from "./room_match.js";

const expect = (cond, what) => { if (!cond) throw new Error(what); };

expect(matchmakingTarget("", [], 10, "abc") === "match-abc", "a page with no room starts a match room when none is open");
expect(matchmakingTarget("", [{ name: "x", players: 3 }], 10, "n") === "x", "a page with no room joins an open match");
expect(matchmakingTarget("", [{ name: "x", players: 3 }], 10, "n") !== null, "play alone is not the default for a page with no room");
expect(matchmakingTarget("chosen", [{ name: "x", players: 3 }], 10, "n") === null, "a page that named its room is never redirected");
expect(matchmakingTarget("", [{ name: DEFAULT_ROOM, players: 1 }], 10, "n") === MATCH_ROOM_PREFIX + "n", "a page never matches into its own default room");
expect(chooseRoom([{ name: "full", players: 10 }], 10, "n") === MATCH_ROOM_PREFIX + "n", "a full room is never joined");
expect(chooseRoom([{ name: "empty", players: 0 }], 10, "n") === MATCH_ROOM_PREFIX + "n", "an empty room is never joined");
expect(chooseRoom([{ name: "a", players: 1 }, { name: "b", players: 3 }], 10, "n") === "b", "the fullest open room is joined");
expect(chooseRoom([{ name: "b", players: 2 }, { name: "a", players: 2 }], 10, "n") === "a", "ties go to the lower name");

const order = [];
await settleMatch("", [], 10, "abc", async (target) => { await null; order.push("announce " + target); }, (target) => order.push("go " + target));
expect(order.join("|") === "announce match-abc|go match-abc", "a page announces its match room before it moves to it");
const shared = [];
const first = [];
await settleMatch("", [...shared], 10, "a", async (target) => { shared.push({ name: target, players: 1 }); }, (target) => first.push(target));
const second = [];
await settleMatch("", [...shared], 10, "b", async () => {}, (target) => second.push(target));
expect(first.length === 1 && second.length === 1 && first[0] === second[0], "a page one second behind joins the room its predecessor announced");
console.log("room match ok (11 cases)");
