// TSL renderer: the render3d.js renderer contract (orbit camera, mesh and instance batch shapes, draw counters)
// drawn through three.js WebGPURenderer with TSL node materials. Shading matches render3d's fragment shader: a
// fixed directional light over a 0.3 ambient floor, additive glow for fx meshes, raw (unencoded) output.
// Skinned meshes run joint skinning in the vertex stage from a per-draw joint uniform array, and the ground
// view box culls unit and doodad draws with the same visibleGroundBounds and meshReach used by render3d.js.
import * as THREE from "three/webgpu";
import {
  attribute, dot, float, int, max, modelNormalMatrix, normalize, normalLocal, normalWorld, positionLocal,
  texture, uniform, uniformArray, varying, vec3, vec4,
} from "three/tsl";
import { createCamera, eyeOf, meshReach, MAX_JOINTS, orbitCamera, panCamera, visibleGroundBounds, zoomCamera } from "./render3d.js";

const LIGHT = normalize(vec3(0.4, 1.0, 0.6));
const AMBIENT = 0.3;
const DIFFUSE = 0.7;
const IDENTITY = new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]);
const PIPELINE_ERROR = /RenderPipeline|CommandBuffer/;
const ERROR_WINDOW_MS = 1000;
const ERROR_BURST = 3;

function shadeNode(base, glow, normal) {
  if (glow) return base;
  return base.mul(float(AMBIENT).add(float(DIFFUSE).mul(max(dot(normal, LIGHT), 0.0))));
}

// Vertex-stage skinning: world-space position and normal from the weighted joint palette, as render3d's VERT does.
function skinNodes(jointArray) {
  const weights = attribute("skinWeights", "vec4");
  const joints = attribute("skinJoints", "vec4");
  const jointAt = (lane) => jointArray.element(int(lane));
  const skin = jointAt(joints.x).mul(weights.x)
    .add(jointAt(joints.y).mul(weights.y))
    .add(jointAt(joints.z).mul(weights.z))
    .add(jointAt(joints.w).mul(weights.w));
  const position = skin.mul(vec4(positionLocal, 1.0)).xyz;
  const normal = varying(normalize(modelNormalMatrix.mul(skin.mul(vec4(normalLocal, 0.0)).xyz)));
  return { position, normal };
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
  const skinnedMaterials = new WeakMap();
  const skinnedUsed = new Map();
  const materialSet = new Set();
  const instanced = new WeakMap();
  const visibleBatches = new WeakMap();
  const pool = [];
  const recentErrors = [];
  const groundBox = [0, 0, 0, 0];
  let cullBounds = null;
  let poolUsed = 0;
  let frameStart = 0;
  let aspect = null;
  let lastWidth = -1;
  let lastHeight = -1;
  let pendingCompile = false;
  let clearColor = new THREE.Color(0.05, 0.06, 0.08);
  const device = backend === "webgpu" ? renderer.backend.device : null;

  // Installed before the first compile (compileFresh runs only from endFrame, after this block).
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
      .catch((error) => { if (debug) console.error("[tsl] compile", error); })
      .then(() => device.popErrorScope())
      .then((error) => { if (error && debug) console.error("[tsl] pipeline error", error.message); })
      .catch((error) => { if (debug) console.error("[tsl] scope", error); });
  }

  function geometryFor(mesh) {
    const cached = geometries.get(mesh);
    if (cached && !mesh.dirty) return cached;
    const geometry = new THREE.BufferGeometry();
    const vertexCount = mesh.positions.length / 3;
    geometry.setAttribute("position", new THREE.BufferAttribute(mesh.positions, 3));
    geometry.setAttribute("normal", new THREE.BufferAttribute(mesh.normals, 3));
    geometry.setAttribute("uv", new THREE.BufferAttribute(mesh.uvs || new Float32Array(vertexCount * 2), 2));
    if (mesh.joints && mesh.weights) {
      geometry.setAttribute("skinJoints", new THREE.BufferAttribute(mesh.joints, 4));
      geometry.setAttribute("skinWeights", new THREE.BufferAttribute(mesh.weights, 4));
    }
    if (mesh.indices) geometry.setIndex(new THREE.BufferAttribute(mesh.indices, 1));
    cached?.dispose();
    geometries.set(mesh, geometry);
    mesh.dirty = false;
    return geometry;
  }

  function baseMaterial(mesh, normal, extra) {
    const glow = mesh.blend === "add";
    const tint = uniform(new THREE.Color(...(mesh.color || [1, 1, 1])));
    const base = mesh.texture ? texture(mesh.texture).rgb : tint;
    const material = new THREE.MeshBasicNodeMaterial();
    material.colorNode = shadeNode(base, glow, normal);
    material.side = THREE.FrontSide;
    if (glow) {
      material.transparent = true;
      material.blending = THREE.AdditiveBlending;
      material.depthWrite = false;
      material.opacityNode = float(mesh.alpha ?? 1);
    }
    if (extra) extra(material);
    materialSet.add(material);
    pendingCompile = true;
    return material;
  }

  function materialFor(mesh) {
    const cached = materials.get(mesh);
    if (cached) return cached;
    const material = baseMaterial(mesh, normalWorld);
    materials.set(mesh, material);
    return material;
  }

  // One skinned material per draw of a given mesh within a frame: each owns a joint uniform array, so units that
  // share a mesh object keep their own poses. Slots persist across frames and are reused in submission order.
  function skinnedFor(mesh) {
    const k = skinnedUsed.get(mesh) ?? 0;
    skinnedUsed.set(mesh, k + 1);
    let slots = skinnedMaterials.get(mesh);
    if (!slots) {
      slots = [];
      skinnedMaterials.set(mesh, slots);
    }
    if (!slots[k]) {
      const jointArray = uniformArray(Array.from({ length: MAX_JOINTS }, () => new THREE.Matrix4()), "mat4");
      const { position, normal } = skinNodes(jointArray);
      const material = baseMaterial(mesh, normal, (m) => { m.positionNode = position; });
      slots[k] = { material, jointArray };
    }
    return slots[k];
  }

  function submit(mesh, transform, material, geometry) {
    const slot = poolUsed++;
    if (!pool[slot]) pool.push(new THREE.Mesh());
    const object = pool[slot];
    object.geometry = geometry;
    object.material = material;
    object.matrixAutoUpdate = false;
    object.matrix.fromArray(transform || IDENTITY);
    object.matrixWorld.copy(object.matrix);
    object.frustumCulled = false;
    scene.add(object);
    stats.drawCalls += 1;
    stats.instances += 1;
    return object;
  }

  function drawMesh(mesh, transform) {
    submit(mesh, transform, materialFor(mesh), geometryFor(mesh));
  }

  function drawSkinned(mesh, transform, joints) {
    if (!joints || !mesh.joints || !mesh.weights) {
      drawMesh(mesh, transform);
      return;
    }
    const { material, jointArray } = skinnedFor(mesh);
    const count = Math.min(MAX_JOINTS, joints.length / 16);
    for (let j = 0; j < count; j++) jointArray.array[j].fromArray(joints, 16 * j);
    submit(mesh, transform, material, geometryFor(mesh));
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

  function inCullBox(x, z, reach) {
    return x + reach >= cullBounds[0] && x - reach <= cullBounds[2]
      && z + reach >= cullBounds[1] && z - reach <= cullBounds[3];
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
      skinnedUsed.clear();
      stats.drawCalls = 0;
      stats.instances = 0;
      stats.culled = 0;
      const a = aspect ?? canvas.width / Math.max(1, canvas.height);
      syncCamera(a);
      cullBounds = visibleGroundBounds(camera, a, groundBox);
    },

    drawMesh(mesh, transform = null) {
      drawMesh(mesh, transform);
    },

    // False when a ground point with the given reach (meshReach) lies wholly outside the view box. Each false answer
    // counts one culled item in stats.culled. Always true when the view has no ground box (horizon on screen).
    isVisible(x, z, reach) {
      if (!cullBounds) return true;
      const inside = inCullBox(x, z, reach);
      if (!inside) stats.culled += 1;
      return inside;
    },

    drawInstanced(mesh, batch) {
      if (batch.count > 0) drawInstances(mesh, batch);
    },

    // Draws the instances that can be on screen and returns how many. A batch that is wholly visible draws as it is;
    // otherwise the visible matrices are compacted into a per-batch scratch batch re-uploaded each frame.
    drawInstancedVisible(mesh, batch) {
      if (!cullBounds) {
        if (batch.count > 0) drawInstances(mesh, batch);
        return batch.count;
      }
      const reach = meshReach(mesh);
      const src = batch.matrices;
      let n = 0;
      for (let i = 0; i < batch.count; i++) if (inCullBox(src[i * 16 + 12], src[i * 16 + 14], reach)) n += 1;
      stats.culled += batch.count - n;
      if (n === 0) return 0;
      if (n === batch.count) {
        drawInstances(mesh, batch);
        return n;
      }
      let vis = visibleBatches.get(batch);
      if (!vis) {
        vis = { matrices: new Float32Array(batch.count * 16), count: 0, dirty: true };
        visibleBatches.set(batch, vis);
      }
      const dst = vis.matrices;
      let w = 0;
      for (let i = 0; i < batch.count; i++) {
        const o = i * 16;
        if (!inCullBox(src[o + 12], src[o + 14], reach)) continue;
        for (let k = 0; k < 16; k++) dst[w * 16 + k] = src[o + k];
        w += 1;
      }
      vis.count = n;
      vis.dirty = true;
      drawInstances(mesh, vis);
      return n;
    },

    drawSkinned(mesh, transform, joints) {
      drawSkinned(mesh, transform, joints);
    },

    endFrame() {
      compileFresh();
      renderer.render(scene, threeCamera);
      stats.frameCpuMs = performance.now() - frameStart;
      return stats.frameCpuMs;
    },
  };
}
