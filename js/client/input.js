// Input capture. The pack decides what a key or a ground click means (the pack's controls.lua); this module only draws the
// drag box while the left button is held and mirrors the selection the pack reports. Its selection and skill calls are
// sent to the pack. The drag box is shown from the press that starts on the ground.
import { UI } from "./pack_load.js";

function isGround(target) {
  return target instanceof HTMLCanvasElement && !target.closest("[data-ui]");
}

// send(name, args) hands an action to the pack (play.js sendInput).
export function createInput({ send, dragSlopPx }) {
  let selection = [];
  let press = null;
  let box = null;
  const overlay = document.createElement("div");
  overlay.dataset.ui = "box";
  overlay.style.cssText = `position:fixed;display:none;border:1px solid ${UI.colours.selectBox};background:${UI.colours.selectFill};pointer-events:none`;
  document.body.appendChild(overlay);

  // The pack's feedback names the selection of this seat; the HUD and panels read it from here.
  function onFeedback(e) {
    if (e.detail && e.detail.kind === "selection") selection = [...e.detail.ids];
  }

  function onDown(e) {
    if (e.button !== 0 || !isGround(e.target)) return;
    press = { x: e.clientX, y: e.clientY, moved: false };
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

  window.addEventListener("pack:feedback", onFeedback);
  window.addEventListener("pointerdown", onDown);
  window.addEventListener("pointermove", onMove);
  window.addEventListener("pointerup", onUp);

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

  function destroy() {
    window.removeEventListener("pack:feedback", onFeedback);
    window.removeEventListener("pointerdown", onDown);
    window.removeEventListener("pointermove", onMove);
    window.removeEventListener("pointerup", onUp);
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
  };
}
