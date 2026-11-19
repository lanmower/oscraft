import { chooseRoom, matchmakingTarget, MATCH_ROOM_PREFIX } from "./room_match.js";

const expect = (cond, what) => { if (!cond) throw new Error(what); };

expect(matchmakingTarget("", [], 10, "abc") === "match-abc", "a page with no room starts a match room when none is open");
expect(matchmakingTarget("", [{ name: "x", players: 3 }], 10, "n") === "x", "a page with no room joins an open match");
expect(matchmakingTarget("", [{ name: "x", players: 3 }], 10, "n") !== null, "play alone is not the default for a page with no room");
expect(matchmakingTarget("chosen", [{ name: "x", players: 3 }], 10, "n") === null, "a page that named its room is never redirected");
expect(chooseRoom([{ name: "full", players: 10 }], 10, "n") === MATCH_ROOM_PREFIX + "n", "a full room is never joined");
expect(chooseRoom([{ name: "empty", players: 0 }], 10, "n") === MATCH_ROOM_PREFIX + "n", "an empty room is never joined");
expect(chooseRoom([{ name: "a", players: 1 }, { name: "b", players: 3 }], 10, "n") === "b", "the fullest open room is joined");
expect(chooseRoom([{ name: "b", players: 2 }, { name: "a", players: 2 }], 10, "n") === "a", "ties go to the lower name");

console.log("room match ok (8 cases)");
