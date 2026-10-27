const TICKS_PER_SECOND = 16;
const SEND_EVERY_MS = 1000;

export function createDraftClock({ send, allowed, schedule = setInterval, cancel = clearInterval }) {
  let active = false;
  let handle = null;

  function beat() {
    if (active && allowed()) send(TICKS_PER_SECOND);
  }

  function setActive(on) {
    if (on === active) return;
    active = on;
    if (on) {
      handle = schedule(beat, SEND_EVERY_MS);
    } else if (handle !== null) {
      cancel(handle);
      handle = null;
    }
  }

  return { setActive, beat, isActive: () => active };
}
