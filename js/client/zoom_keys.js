const keyName = (e) => (e.key === " " ? "space" : e.key.toLowerCase());

const has = (list, name) => Array.isArray(list) && list.includes(name);

export function stepDistance(distance, direction, rules) {
  const next = distance + direction * rules.step;
  return Math.min(rules.max, Math.max(rules.min, next));
}

export function createZoomKeys({ renderer, rules }) {
  function onKeyDown(e) {
    const r = renderer();
    if (!rules || !r || e.ctrlKey || e.metaKey || e.altKey) return false;
    const name = keyName(e);
    if (has(rules.zoomKeys.increase, name)) {
      r.setCamera({ distance: stepDistance(r.camera.distance, 1, rules) });
      return true;
    }
    if (has(rules.zoomKeys.decrease, name)) {
      r.setCamera({ distance: stepDistance(r.camera.distance, -1, rules) });
      return true;
    }
    if (has(rules.rotationLockKeys, name)) {
      if (!e.repeat) r.setCamera({ yawLocked: r.camera.yawLocked !== true });
      return true;
    }
    return false;
  }
  return { onKeyDown };
}
