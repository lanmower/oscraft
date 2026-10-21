// MOBA input layer. Left-drag draws a box and selects own units (heroes first), click selects one unit, shift adds.
// Right-click moves or attacks the whole selection. Keys come from moba_ui.json and can be rebound by a mod.
// Left pointer events are taken in the window's capture phase, so the orbit camera's left-drag pan does not run.
// A targeted cast or item arms on its key; the next left click casts at that point. Right-click, Escape or the same key cancels.
// Panels learn of actions through "moba:action" events on window; Lua binds arrive as "moba:bind" events.
// Targeting and refusals reach the HUD as "moba:feedback" events: targetArm, targetEnd and refusal.
import { boxSelect, clickSelect, ownIds, ControlGroups, resolveKey, cooldownTicks, formationPoints, mergeConfig, targetSpec, refusalText } from "./moba_select.js";

// Camera glide to a ground point through the mod UI's camera move (its renderer is the 3D view; a flat view ignores it).
// seconds is the ease length: the move eases in and out over it instead of jumping.
export const CENTRE_SECONDS = 0.4;
export function centreCamera(x, z) {
  if (window.__modUi) window.__modUi.push({ kind: "camera", x, y: z, zoom: 1, seconds: CENTRE_SECONDS });
}

function feedback(detail) {
  window.dispatchEvent(new CustomEvent("moba:feedback", { detail }));
}

// Offsets (world cells) tried after a move point the sim refuses: the cell's neighbours, then the diagonal ring.
const MOVE_SPOTS = [[1, 0], [-1, 0], [0, 1], [0, -1], [1.5, 1.5], [-1.5, 1.5], [1.5, -1.5], [-1.5, -1.5]];

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
  let armed = null;
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

  // A ground order is a move, or an attack-move when `attackMove` is set (A then click): the units walk to the
  // point and strike enemies they meet on the way. Clicking an enemy attacks it either way.
  function orderSelected(world, enemy, attackMove = false) {
    const ids = me();
    if (ids.length === 0) return false;
    const points = formationPoints(ids.length, world.x, world.z);
    if (enemy) feedback({ kind: "attack", x: world.x, z: world.z, targetId: enemy.id });
    else feedback({ kind: attackMove ? "attackMove" : "move", x: world.x, z: world.z, ids });
    let ok = true;
    ids.forEach((id, i) => {
      if (enemy) {
        const struck = commands.attack(id, enemy.id);
        // A right click on a foe this unit cannot strike is a move to that spot, as in reference; an attack-move onto it stays an attack.
        if (struck || attackMove) {
          ok = struck && ok;
          return;
        }
      }
      const send = attackMove ? commands.attackMove : commands.order;
      // The formation spot first, then the cell and its neighbours, so a move onto a blocked cell lands on a walkable one next to it.
      const spots = [[points[i].x, points[i].z], ...MOVE_SPOTS.map(([dx, dz]) => [world.x + dx, world.z + dz])];
      const sent = spots.some(([x, z]) => send(id, x, z));
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

  // Hold: each own selected unit stops and attacks only what is in reach, until a move or attack replaces the hold.
  function holdSelected() {
    let ok = true;
    for (const id of me()) ok = commands.hold(id) && ok;
    return ok;
  }

  function cooldownLeft(id, slot) {
    const c = cooldowns.get(`${id}:${slot}`);
    return c ? Math.max(0, c.until - game.ticks()) : 0;
  }

  // A cast or item dropped before the sim is never counted or refused by the sim, so it names its reason on the page for the CI witness.
  function dropCast(reason) {
    document.body.dataset.lastRefusal = `cast_dropped ${reason}`;
    feedback({ kind: "refusal", text: refusalText(reason) });
    return false;
  }

  // A command the sim refused: its refusal name is already on the page (noteLocalRefusal), so the HUD reads that.
  function refusedBySim() {
    feedback({ kind: "refusal", text: refusalText(document.body.dataset.lastRefusal) });
    return false;
  }

  function abilityIn(hero, slot) {
    return JSON.parse(game.abilities_json(hero)).find((a) => a.slot === slot) ?? null;
  }

  function castAbility(hero, ability, slot, world, target) {
    const left = cooldownLeft(hero, slot);
    if (left > 0) return dropCast(`cooldown hero=${hero} slot=${slot} ticks=${left}`);
    const ok = commands.cast(hero, slot, target ? target.id : -1, world.x, world.z);
    if (!ok) return refusedBySim();
    // The sim charges the cooldown only when the cast fires: a cast it queued (out of range) or refused has none,
    // so the client cooldown follows the sim's own ability_cooldown, never the accepted flag alone.
    if (typeof game.ability_cooldown === "function") {
      const simLeft = game.ability_cooldown(hero, slot);
      if (simLeft > 0) {
        const total = cooldownTicks(ability, typeof game.skill_rank === "function" ? game.skill_rank(hero, slot) : 0);
        cooldowns.set(`${hero}:${slot}`, { until: game.ticks() + simLeft, total });
      }
    }
    return ok;
  }

  // The key of a targeted cast or item arms it; a plain one casts at once at the pointer.
  function castKey(slot, world) {
    const hero = heroInSelection();
    if (hero < 0) return dropCast(`no_hero_selected slot=${slot}`);
    const ability = abilityIn(hero, slot);
    if (!ability) return dropCast(`no_ability hero=${hero} slot=${slot}`);
    const spec = targetSpec(ability);
    if (spec.targetKind === "none") return castAbility(hero, ability, slot, world, pick(world));
    if (cooldownLeft(hero, slot) > 0) return dropCast(`cooldown hero=${hero} slot=${slot}`);
    arm({ item: false, slot, targetKind: spec.targetKind, radius: spec.radius });
    return true;
  }

  function useItem(hero, slot, target, world) {
    const p = byId().get(hero);
    const at = world ?? { x: p ? p.x : 0, z: p ? p.z : 0 };
    const ok = commands.useItem(hero, slot, target ? target.id : -1, at.x, at.z);
    return ok ? true : refusedBySim();
  }

  // The item in an inventory slot, read from the unit's inventory and the item catalog; null when the slot is empty.
  function itemKey(slot) {
    const hero = heroInSelection();
    if (hero < 0) return dropCast(`no_hero_selected item=${slot}`);
    const cell = JSON.parse(game.inventory_json(hero)).slots[slot];
    if (!cell || cell[0] === 0) return dropCast(`no_item hero=${hero} slot=${slot}`);
    const entry = JSON.parse(game.item_catalog_json())[cell[0] - 1];
    const spec = targetSpec(entry);
    if (spec.targetKind === "none") return useItem(hero, slot, null, null);
    arm({ item: true, slot, targetKind: spec.targetKind, radius: spec.radius });
    return true;
  }

  function arm(state) {
    armed = state;
    feedback({ kind: "targetArm", slot: state.slot, item: state.item, radius: state.radius, targetKind: state.targetKind });
  }

  function disarm() {
    if (!armed) return false;
    armed = null;
    feedback({ kind: "targetEnd" });
    return true;
  }

  // A point cast or item beyond its cast range is refused here: the sim parks it and raises no refusal. The range is
  // the export's radius (whole world units, the same cast range the sim reads).
  function beyondCastRange(hero, range, world) {
    const p = byId().get(hero);
    const dist = p ? Math.hypot(world.x - p.x, world.z - p.z) : -1;
    document.body.dataset.rangeProbe = `hero=${hero} found=${Number(!!p)} radius=${range} dist=${dist.toFixed(2)}`;
    if (!p || !(range > 0)) return false;
    return dist > range;
  }

  // The left click that completes an armed cast. A unit-targeted spell keeps waiting while the click finds no unit.
  function resolveArmed(world) {
    const hero = heroInSelection();
    if (hero < 0) {
      disarm();
      return dropCast("no_hero_selected armed");
    }
    const target = pick(world);
    if (armed.targetKind === "unit" && !target) return dropCast(`no_target slot=${armed.slot}`);
    const a = armed;
    document.body.dataset.armProbe = `entered kind=${a.targetKind} item=${Number(a.item)} radius=${a.radius} slot=${a.slot}`;
    disarm();
    if (a.targetKind === "point" && beyondCastRange(hero, a.radius, world)) return dropCast(`out_of_range hero=${hero} slot=${a.slot}`);
    if (a.item) return useItem(hero, a.slot, target, world);
    const ability = abilityIn(hero, a.slot);
    if (!ability) return dropCast(`no_ability hero=${hero} slot=${a.slot}`);
    return castAbility(hero, ability, a.slot, world, target);
  }

  function skillUp(slot) {
    const hero = heroInSelection();
    return hero >= 0 && game.skill_up(hero, slot);
  }

  function centreOn(id) {
    const u = byId().get(id);
    if (u) centreCamera(u.x, u.z);
  }

  // Space: the camera glides to the hero, or to the first selected unit when no hero is selected.
  function centreSelected() {
    const hero = heroInSelection();
    const id = hero >= 0 ? hero : me()[0];
    if (id !== undefined) centreOn(id);
  }

  function onKeyDown(e) {
    if (isTyping(e.target)) return;
    if (armed && e.key === "Escape") {
      e.preventDefault();
      disarm();
      return;
    }
    const act = resolveKey(cfg.keys, e.key, { ctrl: e.ctrlKey || e.metaKey, shift: e.shiftKey, alt: e.altKey });
    document.body.dataset.keyProbe = `${e.key}>${act ? `${act.action}:${act.slot ?? ""}` : "null"} armed=${armed ? `${armed.item ? "item" : "cast"}:${armed.slot}` : "none"}`;
    if (!act) return;
    e.preventDefault();
    const world = toWorld(pointer.x, pointer.y);
    const armedSame = armed && (act.action === "cast" || act.action === "item") && act.slot === armed.slot && (act.action === "item") === armed.item;
    if (armedSame) {
      disarm();
      return;
    }
    switch (act.action) {
      case "attackMove":
        armedAttackMove = true;
        break;
      case "stop":
        stopSelected();
        break;
      case "hold":
        holdSelected();
        break;
      case "centre":
        centreSelected();
        break;
      case "cast":
        castKey(act.slot, world);
        break;
      case "item":
        itemKey(act.slot);
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
    if (armed) {
      resolveArmed(toWorld(e.clientX, e.clientY));
      return;
    }
    const hit = pick(toWorld(e.clientX, e.clientY));
    if (armedAttackMove) {
      armedAttackMove = false;
      const enemy = hit && hit.player !== localPlayer() ? hit : null;
      const point = toWorld(e.clientX, e.clientY);
      const sent = orderSelected(point, enemy, true);
      document.body.dataset.attackPick = `hit=${hit ? `${hit.id}:p${hit.player}` : "none"} local=${localPlayer()} sel=${me().join("|") || "none"} enemy=${enemy ? enemy.id : "none"} sent=${sent}`;
      return;
    }
    setSelection(clickSelect(hit ? hit.id : -1, { current: selection, additive: p.shift }));
  }

  // Right-click: cancels an armed cast or item; otherwise attacks the enemy under the pointer, else moves the selection there.
  function commandAt(e) {
    if (armed) {
      disarm();
      return false;
    }
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
