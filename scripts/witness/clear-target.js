(() => {
  const distance = (a, b) => Math.hypot(a.x - b.x, a.z - b.z);
  const segmentDistance = (p, a, b) => {
    const dx = b.x - a.x;
    const dz = b.z - a.z;
    const lengthSq = dx * dx + dz * dz;
    const t = lengthSq === 0 ? 0 : Math.max(0, Math.min(1, ((p.x - a.x) * dx + (p.z - a.z) * dz) / lengthSq));
    return Math.hypot(p.x - (a.x + t * dx), p.z - (a.z + t * dz));
  };
  const stopPointOf = (from, c, reachCells) => {
    const dx = from.x - c.x;
    const dz = from.z - c.z;
    const length = Math.hypot(dx, dz) || 1;
    return { x: c.x + (dx / length) * reachCells, z: c.z + (dz / length) * reachCells };
  };
  const blockersOf = (rule, units, c, from, heroId) => {
    const stop = stopPointOf(from, c, rule.reachCells);
    return units.filter(
      (v) =>
        v.id !== heroId && v.id !== c.id && rule.alive(v) && rule.hostile(v) &&
        (segmentDistance(v, from, stop) <= rule.isolationCells || distance(v, stop) < distance(c, stop)),
    );
  };
  const nearestOf = (pool, from) => pool.reduce((best, u) => (best === null || distance(u, from) < distance(best, from) ? u : best), null);
  const describe = (rule, from, c, v) => {
    const stop = stopPointOf(from, c, rule.reachCells);
    const cause = segmentDistance(v, from, stop) <= rule.isolationCells ? "on the approach" : "nearer the stop point";
    return `${v.id} (player ${v.player}, ${v.type}, ${cause}, ${distance(v, stop).toFixed(1)} cells from the stop point)`;
  };
  const pick = (rule, units, from, heroId) => {
    const candidates = units.filter(
      (u) => u.id !== heroId && !rule.excluded.includes(u.id) && u.player === rule.targetOwner && rule.alive(u) && rule.hostile(u),
    );
    if (candidates.length === 0) return { creep: null, aborted: "no hostile creep to strike" };
    const clear = candidates.filter((c) => blockersOf(rule, units, c, from, heroId).length === 0);
    if (clear.length > 0) return { creep: nearestOf(clear, from), aborted: null };
    const first = nearestOf(candidates, from);
    const blockers = blockersOf(rule, units, first, from, heroId).map((v) => describe(rule, from, first, v));
    return {
      creep: null,
      aborted: `no clear creep: all ${candidates.length} candidates are blocked; nearest ${first.id} is blocked by ${blockers.join("; ")}`,
    };
  };
  const place = (rule, units, from, heroId) => {
    const hostiles = units.filter((v) => v.id !== heroId && rule.alive(v) && rule.hostile(v));
    const owned = units.filter((u) => u.player === rule.targetOwner && u.id !== heroId);
    if (owned.length === 0) return { point: null, kind: null, aborted: "no neutral creep kind to place" };
    const kind = owned[0].type;
    for (const radius of rule.placeRadii) {
      for (let deg = 0; deg < 360; deg += rule.placeStepDeg) {
        const angle = (deg * Math.PI) / 180;
        const p = { x: from.x + radius * Math.cos(angle), z: from.z + radius * Math.sin(angle) };
        const clear = hostiles.every(
          (v) =>
            distance(v, p) > rule.isolationCells &&
            segmentDistance(v, from, p) > rule.isolationCells &&
            (v.player === rule.targetOwner || distance(v, p) > rule.laneCells),
        );
        if (clear) return { point: p, kind, aborted: null };
      }
    }
    return { point: null, kind, aborted: "no clear point to place a creep within the placement radii" };
  };
  const choose = (rule, roster, from, heroId, spawn) => {
    const picked = pick(rule, roster(), from, heroId);
    if (picked.creep !== null || !rule.placeTarget) return picked;
    const placed = place(rule, roster(), from, heroId);
    if (placed.point === null) return { creep: null, aborted: placed.aborted };
    const id = spawn(placed.kind, placed.point.x, placed.point.z);
    if (id < 0) return { creep: null, aborted: `could not place a creep at ${placed.point.x.toFixed(1)}, ${placed.point.z.toFixed(1)}` };
    return pick(rule, roster(), from, heroId);
  };
  globalThis.clearTarget = { distance, segmentDistance, stopPointOf, blockersOf, pick, place, choose };
})();
