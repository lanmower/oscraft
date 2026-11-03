
import { translation } from "./render3d.js";

export const WARM_CHUNK = 24;
export const WARM_MAX_FRAMES = 240;
export const WARM_BUDGET_MS = 6000;
const SETTLE_FRAMES = 2;
const BLACK = [0, 0, 0];

const nextFrame = () => new Promise((resolve) => requestAnimationFrame(resolve));

export async function runWarmup(scene, { renderer, background, cancelled, onProgress = () => {}, now = () => performance.now() }) {
  const gl = renderer.gl;
  const pixel = new Uint8Array(4);
  const [x, y, z] = renderer.camera.target;
  const at = translation(x, y, z);
  const one = { matrices: at, count: 1, dirty: true };
  const terrainCount = scene.terrain ? 1 : 0;
  const unitsFrom = terrainCount + scene.instanced.length;
  const fxAt = unitsFrom + scene.units.length;
  const total = fxAt + (scene.fx ? 1 : 0);

  function drawItem(i) {
    if (i < terrainCount) scene.terrain();
    else if (i < unitsFrom) renderer.drawInstanced(scene.instanced[i - terrainCount], one);
    else if (i < fxAt) {
      const { mesh, joints } = scene.units[i - unitsFrom];
      if (joints) renderer.drawSkinned(mesh, at, joints);
      else renderer.drawInstanced(mesh, one);
    } else scene.fx(at);
  }

  function finishFrame() {
    renderer.endFrame();
    gl.readPixels(0, 0, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, pixel);
  }

  const t0 = now();
  let done = 0;
  let frames = 0;
  while (done < total && frames < WARM_MAX_FRAMES && now() - t0 < WARM_BUDGET_MS) {
    await nextFrame();
    if (cancelled()) return { total, done, frames, ms: now() - t0, truncated: true, cancelled: true };
    renderer.beginFrame(BLACK);
    const end = Math.min(total, done + WARM_CHUNK);
    for (; done < end; done++) drawItem(done);
    finishFrame();
    frames += 1;
    onProgress(done, total);
  }
  for (let s = 0; s < SETTLE_FRAMES && total > 0; s++) {
    await nextFrame();
    renderer.beginFrame(BLACK);
    drawItem(0);
    if (scene.units.length > 0) drawItem(unitsFrom);
    finishFrame();
    frames += 1;
  }
  renderer.beginFrame(background);
  finishFrame();
  return { total, done, frames, ms: now() - t0, truncated: done < total, cancelled: false };
}
