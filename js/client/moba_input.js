// MOBA input layer. Left-drag draws a box and selects own units (heroes first), click selects one unit, shift adds.
// Right-click moves or attacks the whole selection. Keys come from moba_ui.json and can be rebound by a mod.
// Left pointer events are taken in the window's capture phase, so the orbit camera's left-drag pan does not run.
// Panels learn of actions through "moba:action" events on window; Lua binds arrive as "moba:bind" events.
import { boxSelect, clickSelect, ownIds, ControlGroups, resolveKey, cooldownTicks, formationPoints, mergeConfig } from "./moba_select.js";

// Camera glide to a ground point through the mod UI's camera move (its renderer is the 3D view; a flat view ignores it).
export function centreCamera(x, z) {
  if (window.__modUi) window.__modUi.push({ kind: "camera", x, y: z, zoom: 1, seconds: 0 });
}

function isGround(target) {
  return target instanceof HTMLCanvasElement && !target.closest("[data-ui]");
}

function isTyping(target) {
  return target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement || target instanceof HTMLSelectElement;
}

export function createMobaInput({ game, units, localPlayer, pick, toWorld, toScreen, commands, isHero, select, config }) {
  let cfg = config;
  let selection = [];
  let armedAttackMove = false;
  let press = null;
  let box = null;
  const pointer = { x: 0, y: 0 };
  const groups = new ControlGroups(cfg.doubleTapMs);
  const cooldowns = new Map();
  const overlay = document.createElement("div");
  overlay.dataset.ui = "box";
  overlay.style.cssText = "position:fixed;display:none;border:1px solid #7cff7c;background:rgba(124,255,124,.12);pointer-events:none";
  document.body.appendChild(overlay);

  const me = () => ownIds(selection, units(), localPlayer());
  const byId = () => new Map(units().map((u) => [u.id, u]));
  const projected = () => units().map((u) => {
    const p = toScreen(u.x, u.z, u.top || 0);
    return { ...u, sx: p.x, sy: p.y };
  });

  function setSelection(ids) {
    selection = [...new Set(ids)];
    select(selection[0] ?? -1);
  }

  function heroInSelection() {
    return me().find((id) => isHero(id)) ?? -1;
  }

  function orderSelected(world, enemy) {
    const ids = me();
    if (ids.length === 0) return false;
    const points = formationPoints(ids.length, world.x, world.z);
    let ok = true;
    ids.forEach((id, i) => {
      if (enemy) {
        ok = commands.attack(id, enemy.id) && ok;
        return;
      }
      const sent = commands.order(id, points[i].x, points[i].z) || commands.order(id, world.x, world.z);
      ok = sent && ok;
    });
    return ok;
  }

  function stopSelected() {
    const lookup = byId();
    let ok = true;
    for (const id of me()) {
      const u = lookup.get(id);
      if (u) ok = commands.order(id, u.x, u.z) && ok;
    }
    return ok;
  }

  function cooldownLeft(id, slot) {
    const c = cooldowns.get(`${id}:${slot}`);
    return c ? Math.max(0, c.until - game.ticks()) : 0;
  }

  // A cast dropped before the sim is never counted or refused by the sim, so it names its reason on the page for the CI witness.
  function dropCast(reason) {
    document.body.dataset.lastRefusal = `cast_dropped ${reason}`;
    return false;
  }

  function cast(slot, world) {
    const hero = heroInSelection();
    if (hero < 0) return dropCast(`no_hero_selected slot=${slot}`);
    const ability = JSON.parse(game.abilities_json(hero)).find((a) => a.slot === slot);
    if (!ability) return dropCast(`no_ability hero=${hero} slot=${slot}`);
    const left = cooldownLeft(hero, slot);
    if (left > 0) return dropCast(`cooldown hero=${hero} slot=${slot} ticks=${left}`);
    const target = pick(world);
    const ok = commands.cast(hero, slot, target ? target.id : -1, world.x, world.z);
    if (ok) {
      const total = cooldownTicks(ability, game.skill_rank(hero, slot));
      cooldowns.set(`${hero}:${slot}`, { until: game.ticks() + total, total });
    }
    return ok;
  }

  function skillUp(slot) {
    const hero = heroInSelection();
    return hero >= 0 && game.skill_up(hero, slot);
  }

  function centreOn(id) {
    const u = byId().get(id);
    if (u) centreCamera(u.x, u.z);
  }

  function onKeyDown(e) {
    if (isTyping(e.target)) return;
    const act = resolveKey(cfg.keys, e.key, { ctrl: e.ctrlKey || e.metaKey, shift: e.shiftKey, alt: e.altKey });
    if (!act) return;
    e.preventDefault();
    const world = toWorld(pointer.x, pointer.y);
    switch (act.action) {
      case "attackMove":
        armedAttackMove = true;
        break;
      case "stop":
      case "hold":
        stopSelected();
        break;
      case "cast":
        cast(act.slot, world);
        break;
      case "skillUp":
        skillUp(act.slot);
        break;
      case "setGroup":
        groups.set(act.slot, me());
        break;
      case "recallGroup": {
        const r = groups.recall(act.slot, performance.now());
        const alive = ownIds(r.ids, units(), localPlayer());
        setSelection(alive);
        if (r.centre && alive.length) centreOn(alive[0]);
        break;
      }
      case "custom":
        window.dispatchEvent(new CustomEvent("moba:action", { detail: { action: "custom", name: act.name } }));
        break;
      default:
        window.dispatchEvent(new CustomEvent("moba:action", { detail: { action: act.action } }));
    }
  }

  function onPointerDown(e) {
    if (e.button !== 0 || !isGround(e.target)) return;
    e.stopPropagation();
    press = { x: e.clientX, y: e.clientY, shift: e.shiftKey, moved: false };
  }

  function onPointerMove(e) {
    pointer.x = e.clientX;
    pointer.y = e.clientY;
    if (!press) return;
    if (!press.moved && Math.hypot(e.clientX - press.x, e.clientY - press.y) > cfg.dragSlopPx) press.moved = true;
    if (press.moved) box = { x0: press.x, y0: press.y, x1: e.clientX, y1: e.clientY };
  }

  function onPointerUp(e) {
    if (!press || e.button !== 0) return;
    e.stopPropagation();
    const p = press;
    press = null;
    box = null;
    if (p.moved) {
      setSelection(boxSelect(projected(), { x0: p.x, y0: p.y, x1: e.clientX, y1: e.clientY }, {
        localPlayer: localPlayer(), isHero, current: selection, additive: p.shift,
      }));
      return;
    }
    const hit = pick(toWorld(e.clientX, e.clientY));
    if (armedAttackMove) {
      armedAttackMove = false;
      const enemy = hit && hit.player !== localPlayer() ? hit : null;
      orderSelected(toWorld(e.clientX, e.clientY), enemy);
      return;
    }
    setSelection(clickSelect(hit ? hit.id : -1, { current: selection, additive: p.shift }));
  }

  // Right-click: attack the enemy under the pointer, else move the selection there.
  function commandAt(e) {
    const world = toWorld(e.clientX, e.clientY);
    armedAttackMove = false;
    const hit = pick(world);
    const enemy = hit && hit.player !== localPlayer() ? hit : null;
    document.body.dataset.rightCmd = `ids=${me().join("|") || "none"} sel=${selection.join("|") || "none"} hit=${hit ? `${hit.id}:p${hit.player}` : "none"} enemy=${enemy ? enemy.id : "none"}`;
    return orderSelected(world, enemy);
  }

  function onBind(e) {
    const { key, action, remove } = e.detail || {};
    if (typeof key !== "string") return;
    const custom = { ...cfg.keys.custom };
    if (remove) delete custom[key.toLowerCase()];
    else if (typeof action === "string") custom[key.toLowerCase()] = action;
    cfg = { ...cfg, keys: { ...cfg.keys, custom } };
  }

  function onConfig(e) {
    cfg = mergeConfig(cfg, e.detail || {});
    groups.windowMs = cfg.doubleTapMs;
  }

  window.addEventListener("pointerdown", onPointerDown, { capture: true });
  window.addEventListener("pointerup", onPointerUp, { capture: true });
  window.addEventListener("pointermove", onPointerMove);
  window.addEventListener("keydown", onKeyDown);
  window.addEventListener("moba:bind", onBind);
  window.addEventListener("moba:config", onConfig);

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
    window.removeEventListener("pointerdown", onPointerDown, { capture: true });
    window.removeEventListener("pointerup", onPointerUp, { capture: true });
    window.removeEventListener("pointermove", onPointerMove);
    window.removeEventListener("keydown", onKeyDown);
    window.removeEventListener("moba:bind", onBind);
    window.removeEventListener("moba:config", onConfig);
    overlay.remove();
  }

  return {
    selection: () => selection,
    setSelection,
    commandAt,
    skillUp,
    draw,
    destroy,
    cooldownLeft,
    cooldownTotal(id, slot) {
      return cooldowns.get(`${id}:${slot}`)?.total ?? 0;
    },
    config: () => cfg,
  };
}
