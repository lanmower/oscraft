// Hero select lobby for 5v5: ten player slots, team A (slots 0-4) and team B (slots 5-9).
// reduceLobby holds every rule and is a pure function of (state, action). createLobby renders the
// state and exchanges it with the rest of the room: each accepted change is announced on window
// event "oscraft:lobby" (detail = {state, action}); remote changes arrive on "oscraft:lobby-msg"
// (detail = action, carrying its own playerId). The transport is whoever dispatches those events.
export const TEAM_SIZE = 5;
export const SLOT_COUNT = 2 * TEAM_SIZE;
// Hero picks come from the rust/data roster: unit("hero", ...) in rust/data/moba/units.star.
export const HERO_ROSTER = Object.freeze(["hero"]);

const EMPTY_SLOT = Object.freeze({ owner: null, hero: null, ready: false });

export function emptyLobby() {
  return { host: null, started: false, slots: Array.from({ length: SLOT_COUNT }, () => ({ ...EMPTY_SLOT })) };
}

export const teamOf = (slot) => (slot < TEAM_SIZE ? 0 : 1);

const refuse = (state, error) => ({ state, error });

export function allReady(state) {
  return state.slots.every((s) => s.owner && s.hero && s.ready);
}

export function reduceLobby(state, action, roster = HERO_ROSTER) {
  if (state.started) return refuse(state, "match-started");
  const { type, playerId, slot } = action;
  if (!playerId) return refuse(state, "no-player");
  if (type === "start") {
    if (state.host !== playerId) return refuse(state, "not-host");
    if (!allReady(state)) return refuse(state, "not-ready");
    return { state: { ...state, started: true }, error: null };
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

export function createLobby({ root, playerId = "local", roster = HERO_ROSTER, onStart = () => {} } = {}) {
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
    label.textContent = `Slot ${index + 1}: ${seat.owner ? (seat.owner === playerId ? "you" : seat.owner.slice(0, 8)) : "open"}`;
    item.append(label);
    if (!seat.owner) {
      item.append(seatButton("Claim", () => act("claim", { slot: index }), state.slots.some((s) => s.owner === playerId)));
    } else if (seat.owner === playerId) {
      const select = document.createElement("select");
      const blank = document.createElement("option");
      blank.value = "";
      blank.textContent = "pick a hero";
      select.append(blank, ...roster.map((hero) => new Option(hero, hero, false, hero === seat.hero)));
      select.value = seat.hero || "";
      select.disabled = seat.ready;
      select.addEventListener("change", () => act("pick", { slot: index, hero: select.value }));
      item.append(select);
      item.append(seatButton(seat.ready ? "Unready" : "Ready", () => act("ready", { slot: index, ready: !seat.ready })));
      item.append(seatButton("Leave", () => act("release", { slot: index })));
    } else {
      const info = document.createElement("span");
      info.textContent = ` ${seat.hero || "no hero"}${seat.ready ? " (ready)" : ""}`;
      item.append(info);
    }
    return item;
  }

  function render() {
    const teams = [0, 1].map((team) => {
      const column = document.createElement("ol");
      column.dataset.team = team === 0 ? "A" : "B";
      const heading = document.createElement("h3");
      heading.textContent = `Team ${team === 0 ? "A" : "B"}`;
      const wrap = document.createElement("div");
      wrap.append(heading, column);
      column.append(...Array.from({ length: TEAM_SIZE }, (_, i) => renderSlot(team * TEAM_SIZE + i)));
      return wrap;
    });
    const ready = state.slots.filter((s) => s.owner && s.ready).length;
    const startButton = seatButton("Start match", () => act("start"), !(state.host === playerId && allReady(state)));
    const status = document.createElement("p");
    status.textContent = `${ready}/${SLOT_COUNT} ready${lastError ? ` (refused: ${lastError})` : ""}`;
    host.replaceChildren(...teams, status, startButton);
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
