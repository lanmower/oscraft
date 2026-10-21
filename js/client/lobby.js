// Hero select lobby for 5v5: ten player slots, team A (slots 0-4) and team B (slots 5-9).
// reduceLobby holds every rule and is a pure function of (state, action). createLobby renders the
// state and exchanges it with the rest of the room: each accepted change is announced on window
// event "oscraft:lobby" (detail = {state, action}); remote changes arrive on "oscraft:lobby-msg"
// (detail = action, carrying its own playerId). The transport is whoever dispatches those events.
import { UI } from "./pack_load.js";
import { fill } from "./moba_select.js";

export const TEAM_SIZE = 5;
export const SLOT_COUNT = 2 * TEAM_SIZE;
// The hero roster is passed in by the page: play.js reads it from the live sim (unit_type_names, filtered to the
// `hero <unit>` lines of scenarios/moba/hero_rules.txt), so a pick is offered only when the sim can spawn that type.
// An empty roster offers no picks and no bot fill.
export const heroNamesFromRules = (text) =>
  text.split("\n").filter((line) => line.startsWith("hero ")).map((line) => line.split(/\s+/)[1]);

// The picker's roster as name -> type index. A name is kept only when the sim's catalog holds it as a hero kind
// (heroFlags[index] === 1, the player-controlled flag) and hero_rules.txt declares it. Structures, creeps and every
// other kind are dropped, and the index is the one spawn_attributed takes.
export function heroKindsFor(typeNames, heroFlags, ruleNames) {
  const kinds = new Map();
  for (const name of ruleNames) {
    const kind = typeNames.indexOf(name);
    if (kind >= 0 && heroFlags[kind] === 1) kinds.set(name, kind);
  }
  return kinds;
}

const EMPTY_SLOT = Object.freeze({ owner: null, hero: null, ready: false });

export function emptyLobby() {
  return { host: null, started: false, slots: Array.from({ length: SLOT_COUNT }, () => ({ ...EMPTY_SLOT })) };
}

export const teamOf = (slot) => (slot < TEAM_SIZE ? 0 : 1);

// A computer seat is an owner string "bot:<slot>", ready from the start. Bot rule: each computer takes the first
// roster hero no seat holds yet, so bots differ while the roster allows; once it is exhausted they take roster[0].
export const BOT_OWNER_PREFIX = "bot:";
export const isBot = (owner) => typeof owner === "string" && owner.startsWith(BOT_OWNER_PREFIX);

const refuse = (state, error) => ({ state, error });

export function allReady(state) {
  return state.slots.every((s) => s.owner && s.hero && s.ready);
}

export function reduceLobby(state, action, roster = []) {
  if (state.started) return refuse(state, "match-started");
  const { type, playerId, slot } = action;
  if (!playerId) return refuse(state, "no-player");
  if (type === "start") {
    if (state.host !== playerId) return refuse(state, "not-host");
    if (!allReady(state)) return refuse(state, "not-ready");
    return { state: { ...state, started: true }, error: null };
  }
  if (type === "fill") {
    if (state.host !== playerId) return refuse(state, "not-host");
    if (state.slots.every((s) => s.owner)) return refuse(state, "nothing-to-fill");
    if (roster.length === 0) return refuse(state, "no-roster");
    const next = structuredClone(state);
    next.slots.forEach((s, i) => {
      if (s.owner) return;
      const held = new Set(next.slots.map((seat) => seat.hero));
      const hero = roster.find((h) => !held.has(h)) ?? roster[0];
      next.slots[i] = { owner: `${BOT_OWNER_PREFIX}${i}`, hero, ready: true };
    });
    return { state: next, error: null };
  }
  if (!Number.isInteger(slot) || slot < 0 || slot >= SLOT_COUNT) return refuse(state, "bad-slot");
  const seat = state.slots[slot];
  const next = structuredClone(state);
  const mine = seat.owner === playerId;
  switch (type) {
    case "claim":
      if (seat.owner) return refuse(state, "slot-taken");
      if (state.slots.some((s) => s.owner === playerId)) return refuse(state, "already-seated");
      next.slots[slot] = { owner: playerId, hero: null, ready: false };
      next.host = next.host || playerId;
      break;
    case "release":
      if (!mine) return refuse(state, "not-owner");
      next.slots[slot] = { ...EMPTY_SLOT };
      if (next.host === playerId) {
        const heir = next.slots.find((s) => s.owner);
        next.host = heir ? heir.owner : null;
      }
      break;
    case "pick":
      if (!mine) return refuse(state, "not-owner");
      if (seat.ready) return refuse(state, "ready-locked");
      if (!roster.includes(action.hero)) return refuse(state, "unknown-hero");
      next.slots[slot].hero = action.hero;
      break;
    case "ready":
      if (!mine) return refuse(state, "not-owner");
      if (action.ready && !seat.hero) return refuse(state, "no-hero");
      next.slots[slot].ready = !!action.ready;
      break;
    default:
      return refuse(state, "unknown-action");
  }
  return { state: next, error: null };
}

export function createLobby({ root, playerId = "local", roster = [], onStart = () => {} } = {}) {
  const host = root || document.body.appendChild(document.createElement("section"));
  host.id = host.id || "lobby";
  let state = emptyLobby();
  let lastError = "";

  function apply(action, announce) {
    const result = reduceLobby(state, action, roster);
    lastError = result.error || "";
    if (result.error) return render();
    const wasStarted = state.started;
    state = result.state;
    if (announce) window.dispatchEvent(new CustomEvent("oscraft:lobby", { detail: { state, action } }));
    render();
    if (!wasStarted && state.started) {
      host.hidden = true;
      window.dispatchEvent(new Event("oscraft:horn"));
      onStart(state);
    }
  }

  function act(type, extra = {}) {
    apply({ type, playerId, ...extra }, true);
  }

  function seatButton(label, onClick, disabled = false) {
    const button = document.createElement("button");
    button.type = "button";
    button.textContent = label;
    button.disabled = disabled;
    button.addEventListener("click", onClick);
    return button;
  }

  function renderSlot(index) {
    const seat = state.slots[index];
    const item = document.createElement("li");
    item.dataset.slot = String(index);
    item.dataset.team = teamOf(index) === 0 ? "A" : "B";
    const label = document.createElement("span");
    const L = UI.lobby;
    const ownerText = seat.owner ? (seat.owner === playerId ? L.owners.you : isBot(seat.owner) ? L.owners.computer : seat.owner.slice(0, L.ownerIdLength)) : L.owners.open;
    label.textContent = fill(L.slotLabel, { n: index + 1, owner: ownerText });
    item.append(label);
    if (!seat.owner) {
      item.append(seatButton(L.claim, () => act("claim", { slot: index }), state.slots.some((s) => s.owner === playerId)));
    } else if (seat.owner === playerId) {
      const select = document.createElement("select");
      const blank = document.createElement("option");
      blank.value = "";
      blank.textContent = UI.lobby.pickHero;
      select.append(blank, ...roster.map((hero) => new Option(hero, hero, false, hero === seat.hero)));
      select.value = seat.hero || "";
      select.disabled = seat.ready;
      select.addEventListener("change", () => act("pick", { slot: index, hero: select.value }));
      item.append(select);
      item.append(seatButton(seat.ready ? UI.lobby.unready : UI.lobby.ready, () => act("ready", { slot: index, ready: !seat.ready })));
      item.append(seatButton(UI.lobby.leave, () => act("release", { slot: index })));
    } else {
      const info = document.createElement("span");
      info.textContent = ` ${seat.hero || UI.lobby.noHero}${seat.ready ? UI.lobby.readySuffix : ""}`;
      item.append(info);
    }
    return item;
  }

  function render() {
    const teams = [0, 1].map((team) => {
      const column = document.createElement("ol");
      column.dataset.team = team === 0 ? "A" : "B";
      const heading = document.createElement("h3");
      heading.textContent = UI.lobby.teamNames[team];
      const wrap = document.createElement("div");
      wrap.append(heading, column);
      column.append(...Array.from({ length: TEAM_SIZE }, (_, i) => renderSlot(team * TEAM_SIZE + i)));
      return wrap;
    });
    const ready = state.slots.filter((s) => s.owner && s.ready).length;
    const fillButton = seatButton(UI.lobby.fillBots, () => act("fill"), !(state.host === playerId && state.slots.some((s) => !s.owner)));
    const startButton = seatButton(UI.lobby.startMatch, () => act("start"), !(state.host === playerId && allReady(state)));
    const status = document.createElement("p");
    status.textContent = fill(UI.lobby.status, { ready, total: SLOT_COUNT }) + (lastError ? fill(UI.lobby.refused, { error: lastError }) : "");
    host.replaceChildren(...teams, status, fillButton, startButton);
  }

  function onRemote(event) {
    apply(event.detail, false);
  }

  window.addEventListener("oscraft:lobby-msg", onRemote);
  render();
  return {
    snapshot: () => structuredClone(state),
    dispatch: (action) => apply(action, true),
    destroy: () => window.removeEventListener("oscraft:lobby-msg", onRemote),
  };
}
