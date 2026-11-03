const DEG = Math.PI / 180;

export function packCameraRules(ui) {
  const c = ui && ui.camera;
  if (!c || !(c.tileUnits > 0)) return null;
  const tiles = (units) => units / c.tileUnits;
  return {
    distance: tiles(c.distance),
    min: tiles(c.minDistance),
    max: tiles(c.maxDistance),
    step: tiles(c.zoomStep),
    pitch: (360 - c.angleOfAttack) * DEG,
    fovY: c.fovDegrees * DEG,
    far: Math.max(2 * tiles(c.maxDistance), c.farTiles || 0),
    rotationLock: c.rotationLock === true,
    zoomKeys: { increase: c.zoomKeys?.increase ?? [], decrease: c.zoomKeys?.decrease ?? [] },
    rotationLockKeys: c.rotationLockKeys ?? [],
  };
}
