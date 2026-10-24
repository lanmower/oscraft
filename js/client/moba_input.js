// MOBA input capture. The pack decides what a key or a ground click means (scenarios/moba/controls.lua); this module only
// draws the drag box while the left button is held and mirrors the selection the pack reports. Its selection and skill
// calls are sent to the pack. The drag box is shown from the press that starts on the ground.
import { cooldownTicks } from "./moba_select.js";

function isGround(target) {
  return target instanceof HTMLCanvasElement && !target.closest("[data-ui]");
}

// send(name, args) hands an action to the pack (play.js sendInput). game reads the sim's ability cooldowns.
export function createMobaInput({ game, toWorld, toScreen, send, dragSlopPx }) {
  let selection = [];
  let press = null;
  let box = null;
  const overlay = document.createElement("div");
  overlay.dataset.ui = "box";
  overlay.style.cssText = "position:fixed;display:none;border:1px solid #7cff7c;background:rgba(124,255,124,.12);pointer-events:none";
  document.body.appendChild(overlay);

  // The pack's feedback names the selection of this seat; the HUD and panels read it from here.
  function onFeedback(e) {
    if (e.detail && e.detail.kind === "selection") selection = [...e.detail.ids];
  }

  // The ground rectangle the window shows, in tiles (the corners hud.js maps for the minimap).
  function viewRect() {
    const corners = [[0, 0], [window.innerWidth, 0], [0, window.innerHeight], [window.innerWidth, window.innerHeight]]
      .map(([x, y]) => toWorld(x, y));
    return {
      x0: Math.min(...corners.map((c) => c.x)),
      z0: Math.min(...corners.map((c) => c.z)),
      x1: Math.max(...corners.map((c) => c.x)),
      z1: Math.max(...corners.map((c) => c.z)),
    };
  }

  // The pack resolves a click on release, so a rectangle sent on every ground press is current when a double-click resolves.
  function onDown(e) {
    box = null;
    if (e.button !== 0 || !isGround(e.target)) return;
    press = { x: e.clientX, y: e.clientY, moved: false };
    send("viewport", viewRect());
  }

  function onMove(e) {
    if (!press) return;
    if (!press.moved && Math.hypot(e.clientX - press.x, e.clientY - press.y) > dragSlopPx) press.moved = true;
    if (press.moved) box = { x0: press.x, y0: press.y, x1: e.clientX, y1: e.clientY };
  }

  function onUp(e) {
    if (!press || e.button !== 0) return;
    press = null;
    box = null;
  }

  // The browser cancels a press it takes as a pan (a touch drag) and sends no pointerup, so the press and box end here.
  function onCancel() {
    press = null;
    box = null;
    draw();
  }

  window.addEventListener("moba:feedback", onFeedback);
  window.addEventListener("pointerdown", onDown);
  window.addEventListener("pointermove", onMove);
  window.addEventListener("pointerup", onUp);
  window.addEventListener("pointercancel", onCancel);

  function draw() {
    if (!box) {
      overlay.style.display = "none";
      return;
    }
    overlay.style.display = "block";
    overlay.style.left = `${Math.min(box.x0, box.x1)}px`;
    overlay.style.top = `${Math.min(box.y0, box.y1)}px`;
    overlay.style.width = `${Math.abs(box.x1 - box.x0)}px`;
    overlay.style.height = `${Math.abs(box.y1 - box.y0)}px`;
  }

  // Cooldown left (ticks) from the sim; the total from the ability's data at its held rank.
  function cooldownLeft(id, slot) {
    return typeof game.ability_cooldown === "function" ? Math.max(0, game.ability_cooldown(id, slot)) : 0;
  }

  function cooldownTotal(id, slot) {
    const ability = JSON.parse(game.abilities_json(id)).find((a) => a.slot === slot);
    if (!ability) return 0;
    return cooldownTicks(ability, typeof game.skill_rank === "function" ? game.skill_rank(id, slot) : 0);
  }

  function destroy() {
    window.removeEventListener("moba:feedback", onFeedback);
    window.removeEventListener("pointerdown", onDown);
    window.removeEventListener("pointermove", onMove);
    window.removeEventListener("pointerup", onUp);
    window.removeEventListener("pointercancel", onCancel);
    overlay.remove();
  }

  return {
    selection: () => selection,
    setSelection(ids) {
      selection = [...new Set(ids)];
      send("select", { ids: selection });
    },
    skillUp(slot) {
      send("skill_up", { slot });
    },
    draw,
    destroy,
    cooldownLeft,
    cooldownTotal,
  };
}
