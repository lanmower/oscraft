// Live debug API on window.oscraft, for the browser console. Installed on every page load; it does no work per frame
// and reads the sim only when a call is made. Mutating calls take the same paths the mouse, the keys and the page's
// own switches take (commands.order / commands.attack, the input layer's selection, the renderer's camera limits,
// the ?renderer / ?terrain / ?shadows / fog switches), so they never change combat, timing or the sim's rules.
// Camera positions are ground coordinates: x and y are world x and z, the same convention as the mod UI camera move.

const HELP = [
  "oscraft.help()                          print this list",
  "oscraft.getCamera()                     {x, y, distance, yaw, pitch, minDistance, maxDistance, bounds} or null (flat view)",
  "oscraft.setCamera({x, y, distance, yaw, pitch})  set fields, then clamp with the mouse's limits; returns getCamera()",
  "oscraft.centreOn(unitId)                move the camera target onto a unit (distance and angles kept)",
  "oscraft.getState()                      tick, localPlayer, controlled, units, selected, renderer, frame counters, terrain mode, fog, shadows",
  "oscraft.selectUnit(id)                  select one of the visible units through the input layer",
  "oscraft.issueOrder(id, kind, x, y)      kind 'move' to (x, y); 'stop' in place; 'attack' an enemy unit picked at (x, y). Returns the sim's accept",
  "oscraft.getOverrides(unitId)            {classOverride: 0..3 | null, holdOverride: true | false | null}: the mod-set target class and attack-move hold; null without a local sim",
  "oscraft.setClassOverride(unitId, cls)   cls 0..3 sets the unit's target class, null clears it; a guest sends it to the host. Returns the accept",
  "oscraft.setAttackHold(unitId, hold)     true always holds on attack-move, false never holds, null restores the Rust rule. Returns the accept",
  "oscraft.setShadows(bool)                sun shadow pass on or off (renderer setShadows)",
  "oscraft.setFog(bool)                    fog-of-war overlay and fog unit filter on or off (render side only; the sim's fog rule is unchanged)",
  "oscraft.setTerrain('splat' | 'tiles')   terrain mode; reloads the page with ?terrain= (boot-time switch)",
  "oscraft.setRenderer('webgl')    renderer (the only one); reloads the page",
  "oscraft.debug.actions()         the pack's controls.lua actions (asked through debug_actions); null without a local sim",
  "oscraft.debug.effects()         the last 64 display effects the page drained from the sim (read-only copy)",
  "oscraft.debug.modErrors()       the mod error log of this page, newest last: [{tick, text}] (read-only copy)",
  "oscraft.debug.swap()            {applied, pending, rejections}: the last data swap, the pending one and refused announcements",
  "oscraft.debug.run(action, args) dispatch a pack action as the page does (a guest sends it to the host); false without a sim",
];

// hooks are closures over play.js state, so every read sees the current value.
export function createDebugApi(hooks) {
  const renderer = () => hooks.renderer();
  const unitById = (id) => hooks.units().find((u) => u.id === id) || null;

  function cameraView(r) {
    const c = r.camera;
    return {
      x: c.target[0], y: c.target[2], distance: c.distance, yaw: c.yaw, pitch: c.pitch,
      minDistance: c.minDistance, maxDistance: c.maxDistance, bounds: c.bounds ? [...c.bounds] : null,
    };
  }

  // Clamps the camera through the renderer's own limits: orbit clamps pitch, zoom clamps distance, pan clamps the target to bounds.
  function clampCamera(r) {
    r.orbit(0, 0);
    r.zoom(0);
    r.pan(0, 0);
  }

  // Read-only views of the page's pack state, plus run (a dispatch through the same path as the input layer).
  const debug = Object.freeze({
    actions: () => hooks.debugActions(),
    effects: () => hooks.effects(),
    modErrors: () => hooks.modErrors(),
    swap: () => hooks.swap(),
    run: (action, args = null) => (typeof action === "string" && action !== "" ? hooks.run(action, args) : false),
  });

  const api = {
    debug,

    help() {
      console.log(HELP.join("\n"));
      return HELP;
    },

    getCamera() {
      const r = renderer();
      return r ? cameraView(r) : null;
    },

    setCamera(opts = {}) {
      const r = renderer();
      if (!r) return null;
      const cur = r.camera.target;
      const patch = { target: [opts.x ?? cur[0], 0, opts.y ?? cur[2]] };
      for (const key of ["distance", "yaw", "pitch"]) if (opts[key] !== undefined) patch[key] = opts[key];
      r.setCamera(patch);
      clampCamera(r);
      return cameraView(r);
    },

    centreOn(unitId) {
      const r = renderer();
      const u = unitById(unitId);
      if (!r || !u) return false;
      r.setCamera({ target: [u.x, 0, u.z] });
      clampCamera(r);
      return true;
    },

    getState() {
      const game = hooks.game();
      const me = hooks.localPlayer();
      const units = hooks.units();
      const controlledUnit = units.find((u) => u.player === me && hooks.isControlled(u.id)) || null;
      const controlled = controlledUnit && game ? {
        id: controlledUnit.id,
        x: controlledUnit.x,
        z: controlledUnit.z,
        hp: controlledUnit.hp ?? null,
        mana: typeof game.max_mana === "function" ? game.max_mana(controlledUnit.id) : null,
        level: typeof game.character_level === "function" ? game.character_level(controlledUnit.id) : null,
      } : null;
      const r = renderer();
      const selection = hooks.selected();
      return {
        tick: game ? game.ticks() : hooks.netTick(),
        localPlayer: me,
        controlled,
        units: units.map((u) => ({ id: u.id, type: u.type, owner: u.player, x: u.x, z: u.z, hp: u.hp ?? null, controlled: hooks.isControlled(u.id), attributes: hooks.isControlled(u.id) ? hooks.attributes?.(u.id) ?? null : null })),
        selected: selection.length ? selection[0] : -1,
        selection,
        renderer: r ? hooks.rendererKind() : "flat",
        frame: r ? { drawCalls: r.stats.drawCalls, instances: r.stats.instances, cpuMs: r.stats.frameCpuMs, ...hooks.drawCounts() } : null,
        terrainMode: hooks.terrainMode(),
        fog: hooks.fogOn(),
        shadows: Boolean(r && hooks.shadowsOn()),
        lastSwap: hooks.lastSwap ? hooks.lastSwap() : null,
      };
    },

    selectUnit(id) {
      if (!unitById(id)) return false;
      hooks.select([id]);
      return true;
    },

    issueOrder(id, kind, x, y) {
      const u = unitById(id);
      if (!u) return false;
      if (kind === "move") return hooks.order(id, x, y);
      if (kind === "stop") return hooks.order(id, u.x, u.z);
      if (kind === "attack") {
        const target = hooks.pick({ x, z: y });
        if (!target || target.player === hooks.localPlayer()) return false;
        return hooks.attack(id, target.id);
      }
      return false;
    },

    getOverrides(unitId) {
      const o = hooks.overrides(unitId);
      if (!o) return null;
      return {
        classOverride: o.classOverride < 0 ? null : o.classOverride,
        holdOverride: o.holdOverride < 0 ? null : o.holdOverride === 1,
      };
    },

    setClassOverride(unitId, cls) {
      if (!unitById(unitId)) return false;
      if (cls !== null && !(Number.isInteger(cls) && cls >= 0 && cls <= 3)) return false;
      return Boolean(hooks.setClassOverride(unitId, cls === null ? -1 : cls));
    },

    setAttackHold(unitId, hold) {
      if (!unitById(unitId)) return false;
      if (hold !== null && typeof hold !== "boolean") return false;
      return Boolean(hooks.setAttackHold(unitId, hold === null ? -1 : hold ? 1 : 0));
    },

    setShadows(on) {
      return hooks.setShadows(Boolean(on));
    },

    setFog(on) {
      return hooks.setFog(Boolean(on));
    },

    setTerrain(mode) {
      if (mode !== "splat" && mode !== "tiles") return false;
      reloadWith("terrain", mode === "tiles" ? "tiles" : null);
      return true;
    },

    setRenderer(kind) {
      if (kind !== "webgl") return false;
      reloadWith("renderer", null);
      return true;
    },
  };

  // The boot-time switches (?renderer, ?terrain) are read once at load, so changing them is a reload with the new query.
  function reloadWith(name, value) {
    const url = new URL(location.href);
    if (value === null) url.searchParams.delete(name);
    else url.searchParams.set(name, value);
    location.assign(url.href);
  }

  return api;
}

export function installDebugApi(hooks) {
  window.oscraft = createDebugApi(hooks);
  return window.oscraft;
}
