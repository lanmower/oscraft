// TSL renderer: the render3d.js renderer contract (orbit camera, mesh and instance batch shapes, draw counters)
// drawn through three.js WebGPURenderer with TSL node materials. Shading matches render3d's fragment shader: a
// fixed directional light over a 0.3 ambient floor, additive glow for fx meshes, raw (unencoded) output.
import * as THREE from "three/webgpu";
import { dot, float, max, normalize, normalWorld, texture, uniform, vec3 } from "three/tsl";
import { createCamera, eyeOf, orbitCamera, panCamera, zoomCamera } from "./render3d.js";

const LIGHT = normalize(vec3(0.4, 1.0, 0.6));
const AMBIENT = 0.3;
const DIFFUSE = 0.7;
const IDENTITY = new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]);
const PIPELINE_ERROR = /RenderPipeline|CommandBuffer/;
const ERROR_WINDOW_MS = 1000;
const ERROR_BURST = 3;

function shadeNode(base, glow) {
  if (glow) return base;
  return base.mul(float(AMBIENT).add(float(DIFFUSE).mul(max(dot(normalWorld, LIGHT), 0.0))));
}

async function initRenderer(canvas, forceWebGL) {
  const renderer = new THREE.WebGPURenderer({ canvas, antialias: true, forceWebGL });
  await renderer.init();
  return renderer;
}

export async function createTslRenderer(canvas, { debug = false } = {}) {
  let renderer = null;
  try {
    renderer = await initRenderer(canvas, false);
  } catch (error) {
    if (debug) console.warn("[tsl] WebGPU unavailable, retrying on the WebGL2 backend", error);
  }
  if (!renderer) renderer = await initRenderer(canvas, true);
  const backend = renderer.backend?.device ? "webgpu" : "webgl2";

  renderer.outputColorSpace = THREE.LinearSRGBColorSpace;
  renderer.toneMapping = THREE.NoToneMapping;
  renderer.sortObjects = false;
  renderer.setPixelRatio(1);

  const camera = createCamera();
  const threeCamera = new THREE.PerspectiveCamera();
  if (THREE.WebGPUCoordinateSystem !== undefined) threeCamera.coordinateSystem = THREE.WebGPUCoordinateSystem;
  const scene = new THREE.Scene();
  const stats = { drawCalls: 0, instances: 0, culled: 0, frameCpuMs: 0 };
  const geometries = new WeakMap();
  const materials = new WeakMap();
  const materialSet = new Set();
  const instanced = new WeakMap();
  const pool = [];
  const recentErrors = [];
  let poolUsed = 0;
  let frameStart = 0;
  let aspect = null;
  let lastWidth = -1;
  let lastHeight = -1;
  let pendingCompile = false;
  let clearColor = new THREE.Color(0.05, 0.06, 0.08);
  const device = backend === "webgpu" ? renderer.backend.device : null;

  if (device) {
    device.addEventListener("uncapturedError", (event) => {
      const message = event.error?.message ?? String(event.error);
      if (debug) console.error("[tsl] uncapturedError", message);
      if (!PIPELINE_ERROR.test(message)) return;
      const now = performance.now();
      recentErrors.push(now);
      while (recentErrors.length && now - recentErrors[0] > ERROR_WINDOW_MS) recentErrors.shift();
      if (recentErrors.length < ERROR_BURST) return;
      recentErrors.length = 0;
      for (const material of materialSet) material.needsUpdate = true;
    });
  }

  function compileFresh() {
    if (!pendingCompile) return;
    pendingCompile = false;
    if (!device) {
      renderer.compileAsync(scene, threeCamera).catch((error) => debug && console.error("[tsl] compile", error));
      return;
    }
    device.pushErrorScope("validation");
    renderer.compileAsync(scene, threeCamera)
      .then(() => device.popErrorScope())
      .then((error) => { if (error && debug) console.error("[tsl] pipeline error", error.message); })
      .catch((error) => { if (debug) console.error("[tsl] compile", error); });
  }

  function geometryFor(mesh) {
    const cached = geometries.get(mesh);
    if (cached && !mesh.dirty) return cached;
    const geometry = new THREE.BufferGeometry();
    const vertexCount = mesh.positions.length / 3;
    geometry.setAttribute("position", new THREE.BufferAttribute(mesh.positions, 3));
    geometry.setAttribute("normal", new THREE.BufferAttribute(mesh.normals, 3));
    geometry.setAttribute("uv", new THREE.BufferAttribute(mesh.uvs || new Float32Array(vertexCount * 2), 2));
    if (mesh.indices) geometry.setIndex(new THREE.BufferAttribute(mesh.indices, 1));
    cached?.dispose();
    geometries.set(mesh, geometry);
    mesh.dirty = false;
    return geometry;
  }

  function materialFor(mesh) {
    const cached = materials.get(mesh);
    if (cached) return cached;
    const glow = mesh.blend === "add";
    const tint = uniform(new THREE.Color(...(mesh.color || [1, 1, 1])));
    const base = mesh.texture ? texture(mesh.texture).rgb : tint;
    const material = new THREE.MeshBasicNodeMaterial();
    material.colorNode = shadeNode(base, glow);
    material.side = THREE.FrontSide;
    if (glow) {
      material.transparent = true;
      material.blending = THREE.AdditiveBlending;
      material.depthWrite = false;
      material.opacityNode = float(mesh.alpha ?? 1);
    }
    materials.set(mesh, material);
    materialSet.add(material);
    pendingCompile = true;
    return material;
  }

  function drawMesh(mesh, transform) {
    const slot = poolUsed++;
    if (!pool[slot]) pool.push(new THREE.Mesh());
    const object = pool[slot];
    object.geometry = geometryFor(mesh);
    object.material = materialFor(mesh);
    object.matrixAutoUpdate = false;
    object.matrix.fromArray(transform || IDENTITY);
    object.matrixWorld.copy(object.matrix);
    object.frustumCulled = false;
    scene.add(object);
    stats.drawCalls += 1;
    stats.instances += 1;
  }

  function drawInstances(mesh, batch) {
    let entry = instanced.get(batch);
    if (!entry) {
      entry = { object: new THREE.InstancedMesh(geometryFor(mesh), materialFor(mesh), 1), array: null };
      entry.object.frustumCulled = false;
      instanced.set(batch, entry);
    }
    const { object } = entry;
    object.geometry = geometryFor(mesh);
    object.material = materialFor(mesh);
    if (entry.array !== batch.matrices) {
      object.instanceMatrix = new THREE.InstancedBufferAttribute(batch.matrices, 16);
      entry.array = batch.matrices;
    }
    if (batch.dirty) {
      object.instanceMatrix.needsUpdate = true;
      batch.dirty = false;
    }
    object.count = batch.count;
    scene.add(object);
    stats.drawCalls += 1;
    stats.instances += batch.count;
  }

  function syncCamera(viewAspect) {
    threeCamera.fov = (camera.fovY * 180) / Math.PI;
    threeCamera.aspect = viewAspect;
    threeCamera.near = camera.near;
    threeCamera.far = camera.far;
    threeCamera.updateProjectionMatrix();
    const eye = eyeOf(camera);
    threeCamera.position.set(eye[0], eye[1], eye[2]);
    threeCamera.up.set(0, 1, 0);
    threeCamera.lookAt(camera.target[0], camera.target[1], camera.target[2]);
    threeCamera.updateMatrixWorld();
  }

  return {
    backend,
    camera,
    stats,

    setCamera(opts = {}) {
      if (opts.aspect !== undefined) aspect = opts.aspect;
      for (const key of ["target", "yaw", "pitch", "distance", "minDistance", "maxDistance", "fovY", "near", "far", "bounds"]) {
        if (opts[key] !== undefined) camera[key] = key === "target" ? [...opts.target] : opts[key];
      }
      return camera;
    },

    orbit(dxPx, dyPx) { return orbitCamera(camera, dxPx, dyPx); },
    pan(dxPx, dyPx) { return panCamera(camera, dxPx, dyPx, canvas.height); },
    zoom(wheelDelta) { return zoomCamera(camera, wheelDelta); },

    uploadTexture(image) {
      const map = new THREE.Texture(image.source);
      map.flipY = false;
      map.colorSpace = THREE.NoColorSpace;
      map.needsUpdate = true;
      return map;
    },

    beginFrame(clearRgb = [0.05, 0.06, 0.08]) {
      frameStart = performance.now();
      if (canvas.width !== lastWidth || canvas.height !== lastHeight) {
        lastWidth = canvas.width;
        lastHeight = canvas.height;
        renderer.setSize(canvas.width, canvas.height, false);
      }
      clearColor = new THREE.Color(clearRgb[0], clearRgb[1], clearRgb[2]);
      renderer.setClearColor(clearColor, 1);
      scene.clear();
      poolUsed = 0;
      stats.drawCalls = 0;
      stats.instances = 0;
      stats.culled = 0;
      syncCamera(aspect ?? canvas.width / Math.max(1, canvas.height));
    },

    drawMesh(mesh, transform = null) {
      drawMesh(mesh, transform);
    },

    isVisible() {
      return true;
    },

    drawInstanced(mesh, batch) {
      if (batch.count > 0) drawInstances(mesh, batch);
    },

    drawInstancedVisible(mesh, batch) {
      if (batch.count > 0) drawInstances(mesh, batch);
      return batch.count;
    },

    drawSkinned(mesh, transform) {
      drawMesh(mesh, transform);
    },

    endFrame() {
      compileFresh();
      renderer.render(scene, threeCamera);
      stats.frameCpuMs = performance.now() - frameStart;
      return stats.frameCpuMs;
    },
  };
}
