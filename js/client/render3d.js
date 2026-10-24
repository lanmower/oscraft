// 3D view for the WebGL2 client: orbit camera math, column-major mat4 helpers and a
// depth-tested mesh draw path. Import-safe under node: nothing touches a DOM or a GL
// context until createRenderer(gl) is called.

const PITCH_LIMIT = Math.PI / 2 - 0.01;
// Half of fovY (PI/3): a lower pitch puts the top screen edge at or above the horizon, so it no longer meets the ground.
const PITCH_MIN = Math.PI / 6 + 0.01;
const WORLD_UP = [0, 1, 0];
const IDENTITY_16 = [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1];
// Every helper below takes an optional `out` and writes into it; callers on a hot path pass their own scratch.
const productScratch = new Float32Array(16);
const projectionScratch = new Float32Array(16);
const viewScratch = new Float32Array(16);

export function identity(out = new Float32Array(16)) {
  out.set(IDENTITY_16);
  return out;
}

// out may alias a or b: the product is formed in a shared scratch before it is copied out.
export function multiply(a, b, out = new Float32Array(16)) {
  for (let c = 0; c < 4; c++) {
    for (let r = 0; r < 4; r++) {
      let s = 0;
      for (let k = 0; k < 4; k++) s += a[k * 4 + r] * b[c * 4 + k];
      productScratch[c * 4 + r] = s;
    }
  }
  out.set(productScratch);
  return out;
}

export function translation(x, y, z, out = new Float32Array(16)) {
  identity(out);
  out[12] = x; out[13] = y; out[14] = z;
  return out;
}

export function scaling(x, y, z, out = new Float32Array(16)) {
  identity(out);
  out[0] = x; out[5] = y; out[10] = z;
  return out;
}

export function rotationY(angle) {
  const c = Math.cos(angle), s = Math.sin(angle);
  return new Float32Array([c, 0, -s, 0, 0, 1, 0, 0, s, 0, c, 0, 0, 0, 0, 1]);
}

export function rotationX(angle) {
  const c = Math.cos(angle), s = Math.sin(angle);
  return new Float32Array([1, 0, 0, 0, 0, c, s, 0, 0, -s, c, 0, 0, 0, 0, 1]);
}

export function perspective(fovY, aspect, near, far, out = new Float32Array(16)) {
  const f = 1 / Math.tan(fovY / 2);
  const nf = 1 / (near - far);
  out.set([
    f / aspect, 0, 0, 0,
    0, f, 0, 0,
    0, 0, (far + near) * nf, -1,
    0, 0, 2 * far * near * nf, 0,
  ]);
  return out;
}

export function lookAt(eye, target, up, out = new Float32Array(16)) {
  const z = normalize(sub(eye, target));
  const x = normalize(cross(up, z));
  const y = cross(z, x);
  out.set([
    x[0], y[0], z[0], 0,
    x[1], y[1], z[1], 0,
    x[2], y[2], z[2], 0,
    -dot(x, eye), -dot(y, eye), -dot(z, eye), 1,
  ]);
  return out;
}

// Normal matrix: inverse transpose of the upper 3x3 of a model matrix, as column-major mat3.
export function normalMatrix(m, out = new Float32Array(9)) {
  const a00 = m[0], a01 = m[4], a02 = m[8];
  const a10 = m[1], a11 = m[5], a12 = m[9];
  const a20 = m[2], a21 = m[6], a22 = m[10];
  const c00 = a11 * a22 - a12 * a21;
  const c01 = a12 * a20 - a10 * a22;
  const c02 = a10 * a21 - a11 * a20;
  const det = a00 * c00 + a01 * c01 + a02 * c02;
  const inv = det === 0 ? 0 : 1 / det;
  out.set([
    c00 * inv, c01 * inv, c02 * inv,
    (a02 * a21 - a01 * a22) * inv, (a00 * a22 - a02 * a20) * inv, (a01 * a20 - a00 * a21) * inv,
    (a01 * a12 - a02 * a11) * inv, (a02 * a10 - a00 * a12) * inv, (a00 * a11 - a01 * a10) * inv,
  ]);
  return out;
}

function sub(a, b) { return [a[0] - b[0], a[1] - b[1], a[2] - b[2]]; }
function dot(a, b) { return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]; }
function cross(a, b) {
  return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
}
function normalize(v) {
  const len = Math.hypot(v[0], v[1], v[2]) || 1;
  return [v[0] / len, v[1] / len, v[2] / len];
}

export function createCamera(opts = {}) {
  return {
    target: opts.target ? [...opts.target] : [0, 0, 0],
    yaw: opts.yaw ?? 0,
    pitch: opts.pitch ?? 0.9,
    distance: opts.distance ?? 30,
    minDistance: opts.minDistance ?? 2,
    maxDistance: opts.maxDistance ?? 400,
    fovY: opts.fovY ?? Math.PI / 3,
    near: opts.near ?? 0.1,
    far: opts.far ?? 2000,
  };
}

// Eye position: the camera orbits the target at `distance`, yaw around world Y, pitch above the ground plane.
export function eyeOf(cam) {
  const cp = Math.cos(cam.pitch);
  return [
    cam.target[0] + cam.distance * cp * Math.sin(cam.yaw),
    cam.target[1] + cam.distance * Math.sin(cam.pitch),
    cam.target[2] + cam.distance * cp * Math.cos(cam.yaw),
  ];
}

export function viewMatrix(cam) {
  return lookAt(eyeOf(cam), cam.target, [0, 1, 0]);
}

// Near plane grows with the orbit distance so depth precision holds when zoomed out: at the far zoom the fixed near
// plane (0.1) left a far/near ratio near 2400, and coplanar floor layers flickered. Eye height is ~0.78 * distance,
// so the nearest floor is always beyond 0.02 * distance.
const NEAR_PER_DISTANCE = 0.02;
export function nearOf(cam) {
  return Math.max(cam.near, cam.distance * NEAR_PER_DISTANCE);
}

export function projectionMatrix(cam, aspect) {
  return perspective(cam.fovY, aspect, nearOf(cam), cam.far);
}

export function viewProjection(cam, aspect, out = new Float32Array(16)) {
  perspective(cam.fovY, aspect, nearOf(cam), cam.far, projectionScratch);
  lookAt(eyeOf(cam), cam.target, WORLD_UP, viewScratch);
  return multiply(projectionScratch, viewScratch, out);
}

// Ground-plane box [minX, minZ, maxX, maxZ] that the view frustum covers, from the four corner rays meeting y = 0.
// Null when a corner ray misses the ground (the horizon is on screen), so nothing may be culled.
export function visibleGroundBounds(cam, aspect, out = [0, 0, 0, 0]) {
  const eye = eyeOf(cam);
  const fwd = normalize(sub(cam.target, eye));
  const right = normalize(cross(fwd, WORLD_UP));
  const up = cross(right, fwd);
  const t = Math.tan(cam.fovY / 2);
  out[0] = Infinity; out[1] = Infinity; out[2] = -Infinity; out[3] = -Infinity;
  for (const sx of [-1, 1]) {
    for (const sy of [-1, 1]) {
      const dy = fwd[1] + right[1] * sx * t * aspect + up[1] * sy * t;
      if (dy >= 0) return null;
      const s = -eye[1] / dy;
      const x = eye[0] + (fwd[0] + right[0] * sx * t * aspect + up[0] * sy * t) * s;
      const z = eye[2] + (fwd[2] + right[2] * sx * t * aspect + up[2] * sy * t) * s;
      out[0] = Math.min(out[0], x); out[1] = Math.min(out[1], z);
      out[2] = Math.max(out[2], x); out[3] = Math.max(out[3], z);
    }
  }
  return out;
}

const reachCache = new WeakMap();
// Ground reach of a mesh about its origin: the largest horizontal distance from the origin plus its top height.
// Culling adds it as a margin, so a tall model whose top is on screen is kept. Cached per mesh object.
export function meshReach(mesh) {
  let reach = reachCache.get(mesh);
  if (reach === undefined) {
    const p = mesh.positions;
    let half = 0, top = 0;
    for (let v = 0; v < p.length; v += 3) {
      half = Math.max(half, Math.abs(p[v]), Math.abs(p[v + 2]));
      top = Math.max(top, p[v + 1]);
    }
    reach = half + top;
    reachCache.set(mesh, reach);
  }
  return reach;
}

// Screen-space deltas in pixels; yaw and pitch are radians per pixel.
export function orbitCamera(cam, dxPx, dyPx, radiansPerPx = 0.005) {
  cam.yaw -= dxPx * radiansPerPx;
  cam.pitch = Math.min(PITCH_LIMIT, Math.max(PITCH_MIN, cam.pitch + dyPx * radiansPerPx));
  return cam;
}

// Keeps the target inside cam.bounds ([minX, minZ, maxX, maxZ] on the ground plane) when the camera has one.
export function clampTarget(cam) {
  if (!cam.bounds) return cam;
  cam.target[0] = Math.min(cam.bounds[2], Math.max(cam.bounds[0], cam.target[0]));
  cam.target[2] = Math.min(cam.bounds[3], Math.max(cam.bounds[1], cam.target[2]));
  return cam;
}

// Slides the target across the ground plane: horizontal drag follows the camera's right vector,
// vertical drag follows its horizontal forward vector. Scale is world units per pixel at the target.
export function panCamera(cam, dxPx, dyPx, viewportHeightPx) {
  const k = (2 * cam.distance * Math.tan(cam.fovY / 2)) / Math.max(1, viewportHeightPx);
  const right = [Math.cos(cam.yaw), 0, -Math.sin(cam.yaw)];
  const forward = [-Math.sin(cam.yaw), 0, -Math.cos(cam.yaw)];
  for (let i = 0; i < 3; i++) {
    cam.target[i] += (-right[i] * dxPx + forward[i] * dyPx) * k;
  }
  return clampTarget(cam);
}

// Wheel delta in the usual browser units (about 100 per notch). Positive zooms out.
export function zoomCamera(cam, wheelDelta) {
  const d = cam.distance * Math.exp(wheelDelta * 0.001);
  cam.distance = Math.min(cam.maxDistance, Math.max(cam.minDistance, d));
  return cam;
}

// Joint palette size of the skinned path; a skin with more joints than this draws unskinned.
export const MAX_JOINTS = 64;
// Level of detail: an instance switches to its low model beyond LOD_FAR_PER_DISTANCE times the camera distance, and
// returns to the full model only inside LOD_HYSTERESIS times that switch distance, so it does not flicker at the edge.
export const LOD_FAR_PER_DISTANCE = 1.5;
export const LOD_HYSTERESIS = 0.75;

// Lighting for meshes without their own: ambient, diffuse and gain (1 = unchanged).
export const DEFAULT_SHADE = [0.3, 0.7, 1];
// Sun direction (the same vector the shaders light by), the shadow map's texture unit, and how far the sun's eye sits.
const SUN_DIR = (() => { const s = [0.4, 1, 0.6]; const n = Math.hypot(...s); return s.map((v) => v / n); })();
const SHADOW_UNIT = 9;
const RIPPLE_UNIT = 11;
// The grass variant weight map (one byte per cell) sits on the next free unit. A splat with no variant map binds a 1x1 zero map.
const VARIANT_UNIT = 12;
const NO_VARIANT = { width: 1, height: 1, data: new Uint8Array(1) };
const ZERO_TEXEL = new Uint8Array(1);
const SHADOW_DISTANCE = 400;
// GLSL sources live in js/client/shaders/*.glsl. They are fetched once when this module loads (top-level await), so
// createRenderer only compiles text that has arrived. A line `#include "name.glsl"` is replaced by that file, and
// __MAX_JOINTS__ by the joint palette size. Under node (no document) nothing is fetched and createRenderer refuses to run.
const SHADER_DIR = new URL("./shaders/", import.meta.url);
export const SHADER_FILES = ["mesh.vert.glsl", "mesh.frag.glsl", "splat.frag.glsl", "shadow.glsl"];
// Which shader files each program is linked from. The vertex shader is shared.
const PROGRAM_SHADERS = {
  mesh: { vertex: "mesh.vert.glsl", fragment: "mesh.frag.glsl" },
  splat: { vertex: "mesh.vert.glsl", fragment: "splat.frag.glsl" },
};
const SHADERS = typeof document === "undefined" ? null : await loadShaders("");

// The water ripple height field: a greyscale rock height map (512 px, repeats). If it does not load, the water keeps its
// analytic ripples (the shader falls back on uRippleOn).
const RIPPLE_BITMAP = typeof document === "undefined" ? null : await loadRippleBitmap();
async function loadRippleBitmap() {
  try {
    // The texture is named by the pack (manifest.assets.textures.rockDisp); pack_load reads the pack only in the browser.
    const { assetUrl } = await import("./pack_load.js");
    const res = await fetch(assetUrl("textures.rockDisp"));
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    return await createImageBitmap(await res.blob());
  } catch (err) {
    console.error(`render3d: ripple height texture failed to load, keeping the analytic ripples: ${err.message}`);
    return null;
  }
}

// query: "" for the page load; a cache-busting query when a hot reload re-fetches the files.
async function loadShaders(query) {
  const out = {};
  for (const name of SHADER_FILES) out[name] = await readShader(name, query);
  return out;
}
async function readShader(name, query) {
  const text = await expandIncludes(await fetchText(name, query), query);
  return text.replaceAll("__MAX_JOINTS__", String(MAX_JOINTS));
}
async function fetchText(name, query) {
  const res = await fetch(new URL(name, SHADER_DIR).href + query, { cache: query ? "no-store" : "default" });
  if (!res.ok) throw new Error(`${name}: HTTP ${res.status}`);
  return res.text();
}
async function expandIncludes(text, query) {
  const out = [];
  for (const line of text.split("\n")) {
    const m = /^#include "([^"]+)"\s*$/.exec(line);
    out.push(m ? await readShader(m[1], query) : line);
  }
  return out.join("\n");
}
function orthographic(l, r, b, t, n, f) {
  const out = new Float32Array(16);
  out[0] = 2 / (r - l);
  out[5] = 2 / (t - b);
  out[10] = -2 / (f - n);
  out[12] = -(r + l) / (r - l);
  out[13] = -(t + b) / (t - b);
  out[14] = -(f + n) / (f - n);
  out[15] = 1;
  return out;
}

function compile(gl, type, source) {
  const shader = gl.createShader(type);
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    const log = gl.getShaderInfoLog(shader);
    gl.deleteShader(shader);
    throw new Error(log);
  }
  return shader;
}

// Attribute slots are fixed for every program, so one mesh's buffers bind the same way in the main and splat programs.
const ATTRIBUTE_NAMES = ["aPos", "aNormal", "aUv", "aJoints", "aWeights", "aInst"];
// Compiles and links one program. On failure everything it created is deleted and the error is thrown, so a failed
// hot reload leaves no half-built program behind.
function linkProgram(gl, vertexSource, fragmentSource) {
  const program = gl.createProgram();
  const shaders = [];
  try {
    shaders.push(compile(gl, gl.VERTEX_SHADER, vertexSource), compile(gl, gl.FRAGMENT_SHADER, fragmentSource));
    shaders.forEach((shader) => gl.attachShader(program, shader));
    ATTRIBUTE_NAMES.forEach((name, slot) => gl.bindAttribLocation(program, slot, name));
    gl.linkProgram(program);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(program));
  } catch (err) {
    gl.deleteProgram(program);
    throw err;
  } finally {
    shaders.forEach((shader) => gl.deleteShader(shader));
  }
  return program;
}

// Uniform locations of each program, by the names the renderer sets. Re-fetched whenever a program is relinked.
function meshUniforms(gl, p) {
  return {
    model: gl.getUniformLocation(p, "uModel"),
    normal: gl.getUniformLocation(p, "uNormal"),
    viewProj: gl.getUniformLocation(p, "uViewProj"),
    color: gl.getUniformLocation(p, "uColor"),
    tint: gl.getUniformLocation(p, "uTint"),
    shade: gl.getUniformLocation(p, "uShade"),
    shadowOn: gl.getUniformLocation(p, "uShadowOn"),
    useTex: gl.getUniformLocation(p, "uUseTex"),
    tex: gl.getUniformLocation(p, "uTex"),
    skinned: gl.getUniformLocation(p, "uSkinned"),
    joints: gl.getUniformLocation(p, "uJoints[0]"),
    alpha: gl.getUniformLocation(p, "uAlpha"),
    glow: gl.getUniformLocation(p, "uGlow"),
    instanced: gl.getUniformLocation(p, "uInstanced"),
    lightVP: gl.getUniformLocation(p, "uLightVP"),
    shadow: gl.getUniformLocation(p, "uShadow"),
    shadowTexel: gl.getUniformLocation(p, "uShadowTexel"),
  };
}
function splatUniforms(gl, p) {
  return {
    control: gl.getUniformLocation(p, "uControl"),
    mapSize: gl.getUniformLocation(p, "uMapSize"),
    uvScale: gl.getUniformLocation(p, "uUvScale"),
    band: gl.getUniformLocation(p, "uBand"),
    shade: gl.getUniformLocation(p, "uShade"),
    layerTint: gl.getUniformLocation(p, "uLayerTint[0]"),
    waterMap: gl.getUniformLocation(p, "uWaterMap"),
    time: gl.getUniformLocation(p, "uTime"),
    eye: gl.getUniformLocation(p, "uEye"),
    waterTint: gl.getUniformLocation(p, "uWaterTint"),
    variant: gl.getUniformLocation(p, "uVariant"),
    variantOn: gl.getUniformLocation(p, "uVariantOn"),
    shadowOn: gl.getUniformLocation(p, "uShadowOn"),
    viewProj: gl.getUniformLocation(p, "uViewProj"),
    model: gl.getUniformLocation(p, "uModel"),
    normal: gl.getUniformLocation(p, "uNormal"),
    skinned: gl.getUniformLocation(p, "uSkinned"),
    instanced: gl.getUniformLocation(p, "uInstanced"),
    lightVP: gl.getUniformLocation(p, "uLightVP"),
    shadow: gl.getUniformLocation(p, "uShadow"),
    shadowTexel: gl.getUniformLocation(p, "uShadowTexel"),
    ripple: gl.getUniformLocation(p, "uRipple"),
    rippleOn: gl.getUniformLocation(p, "uRippleOn"),
    alb: [0, 1, 2, 3].map((k) => gl.getUniformLocation(p, "uAlb" + k)),
    disp: [0, 1, 2, 3].map((k) => gl.getUniformLocation(p, "uDisp" + k)),
  };
}

// Uploads a mesh once and keeps its buffers. Mesh shape:
//   { positions: Float32Array xyz, normals: Float32Array xyz, uvs?: Float32Array uv,
//     indices?: Uint16Array|Uint32Array, color?: [r,g,b] in 0..1, texture?: WebGLTexture,
//     dirty?: boolean }
// Set mesh.dirty = true after changing its arrays so the next draw re-uploads them.
// Every renderer made by createRenderer, so the hot reload (hmr.js) can reach the live ones.
export const liveRenderers = new Set();

export function createRenderer(gl) {
  if (!SHADERS) throw new Error("render3d: shader sources are not loaded (createRenderer needs a browser page)");
  // The programs are the live handles: a hot reload swaps them in place, so every draw below reads them at call time.
  let program = linkProgram(gl, SHADERS[PROGRAM_SHADERS.mesh.vertex], SHADERS[PROGRAM_SHADERS.mesh.fragment]);
  let splatProgram = linkProgram(gl, SHADERS[PROGRAM_SHADERS.splat.vertex], SHADERS[PROGRAM_SHADERS.splat.fragment]);
  gl.useProgram(program);

  const attr = {
    pos: gl.getAttribLocation(program, "aPos"),
    normal: gl.getAttribLocation(program, "aNormal"),
    uv: gl.getAttribLocation(program, "aUv"),
    joints: gl.getAttribLocation(program, "aJoints"),
    weights: gl.getAttribLocation(program, "aWeights"),
    inst: gl.getAttribLocation(program, "aInst"),
  };
  // Filled in place by recompile, so the uniform objects keep their identity.
  const uni = meshUniforms(gl, program);
  const splatUni = splatUniforms(gl, splatProgram);
  const rippleTex = RIPPLE_BITMAP ? makeRippleTexture(RIPPLE_BITMAP) : null;
  gl.uniform1i(uni.tex, 0);
  gl.useProgram(splatProgram);
  gl.uniform1i(splatUni.ripple, RIPPLE_UNIT);
  gl.uniform1f(splatUni.rippleOn, rippleTex ? 1 : 0);
  gl.useProgram(program);
  gl.enable(gl.DEPTH_TEST);
  gl.depthFunc(gl.LEQUAL);

  const camera = createCamera();
  const uploads = new WeakMap();
  const instanceBuffers = new WeakMap();
  const stats = { drawCalls: 0, instances: 0, culled: 0, lod: 0, frameCpuMs: 0 };
  const viewProj = identity();
  const unitModel = identity();
  // Sun shadow map: one depth target over the playfield, filled by re-issuing the caster draws with the light's matrix.
  // shadowFbo/shadowTex: the working map the lit pass samples. staticFbo/staticTex: the cached layer of static casters.
  let shadowFbo = null, shadowTex = null, staticFbo = null, staticTex = null, shadowSize = 0, shadowOn = false, passVP = viewProj;
  let casterGen = 0, shadowGen = -1, savedCullBounds = null;
  const lightVP = identity();
  const modelScratch = new Float32Array(16);
  const normalScratch = new Float32Array(9);
  const groundBox = [0, 0, 0, 0];
  const compactedBatches = new WeakMap();
  const lodTiers = new WeakMap();
  let cullBounds = null;
  let eye = [0, 0, 0];
  let frameStart = 0;
  let aspect = null;

  function upload(mesh) {
    let buf = uploads.get(mesh);
    if (!buf || mesh.dirty) {
      if (!buf) {
        buf = { pos: gl.createBuffer(), normal: gl.createBuffer(), uv: gl.createBuffer(), index: null, count: 0, indexType: 0, joints: null, weights: null };
        casterGen += 1;
      }
      const vertexCount = mesh.positions.length / 3;
      gl.bindBuffer(gl.ARRAY_BUFFER, buf.pos);
      gl.bufferData(gl.ARRAY_BUFFER, mesh.positions, gl.STATIC_DRAW);
      gl.bindBuffer(gl.ARRAY_BUFFER, buf.normal);
      gl.bufferData(gl.ARRAY_BUFFER, mesh.normals, gl.STATIC_DRAW);
      gl.bindBuffer(gl.ARRAY_BUFFER, buf.uv);
      gl.bufferData(gl.ARRAY_BUFFER, mesh.uvs || new Float32Array(vertexCount * 2), gl.STATIC_DRAW);
      if (mesh.joints && mesh.weights) {
        if (!buf.joints) { buf.joints = gl.createBuffer(); buf.weights = gl.createBuffer(); }
        gl.bindBuffer(gl.ARRAY_BUFFER, buf.joints);
        gl.bufferData(gl.ARRAY_BUFFER, mesh.joints, gl.STATIC_DRAW);
        gl.bindBuffer(gl.ARRAY_BUFFER, buf.weights);
        gl.bufferData(gl.ARRAY_BUFFER, mesh.weights, gl.STATIC_DRAW);
      }
      if (mesh.indices) {
        if (!buf.index) buf.index = gl.createBuffer();
        gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, buf.index);
        gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, mesh.indices, gl.STATIC_DRAW);
        buf.count = mesh.indices.length;
        buf.indexType = mesh.indices instanceof Uint32Array ? gl.UNSIGNED_INT : gl.UNSIGNED_SHORT;
      } else {
        buf.count = vertexCount;
        buf.indexType = 0;
      }
      uploads.set(mesh, buf);
      mesh.dirty = false;
    }
    return buf;
  }

  // Re-fetches the shader files and relinks the named programs ('mesh', 'splat'; default both) in place. The new
  // program replaces the old one only when it compiles and links; otherwise the old program stays, the GLSL error is
  // logged, and nothing else changes. Runs between frames (it awaits the fetch first), so no draw sees a half swap.
  // Resolves to the names that were swapped.
  async function recompile(names = Object.keys(PROGRAM_SHADERS)) {
    let fresh;
    try {
      fresh = await loadShaders(`?v=${Date.now()}`);
    } catch (err) {
      console.error(`render3d: shader fetch failed, keeping the current programs: ${err.message}`);
      return [];
    }
    const swapped = [];
    for (const name of [names].flat()) {
      const src = PROGRAM_SHADERS[name];
      if (!src) continue;
      let next;
      try {
        next = linkProgram(gl, fresh[src.vertex], fresh[src.fragment]);
      } catch (err) {
        console.error(`render3d: ${name} program not recompiled, keeping the old one:\n${err.message}`);
        continue;
      }
      const old = name === "mesh" ? program : splatProgram;
      if (name === "mesh") program = next;
      else splatProgram = next;
      Object.assign(name === "mesh" ? uni : splatUni, name === "mesh" ? meshUniforms(gl, next) : splatUniforms(gl, next));
      gl.deleteProgram(old);
      restoreProgramState(name);
      swapped.push(name);
    }
    gl.useProgram(program);
    return swapped;
  }

  // Sets the per-program uniforms that are not re-sent every draw, on a freshly linked program.
  function restoreProgramState(name) {
    const p = name === "mesh" ? program : splatProgram;
    const locs = name === "mesh" ? uni : splatUni;
    gl.useProgram(p);
    if (name === "mesh") gl.uniform1i(locs.tex, 0);
    if (shadowTex) {
      gl.uniformMatrix4fv(locs.lightVP, false, lightVP);
      gl.uniform1i(locs.shadow, SHADOW_UNIT);
      gl.uniform1f(locs.shadowTexel, 1 / shadowSize);
    }
    gl.uniform1f(locs.shadowOn, shadowOn && shadowTex ? 1 : 0);
    if (name === "splat") {
      gl.uniform1i(locs.ripple, RIPPLE_UNIT);
      gl.uniform1f(locs.rippleOn, rippleTex ? 1 : 0);
    }
  }

  const renderer = {
    gl,
    camera,
    stats,
    recompile,

    // Merges the given fields into the orbit camera. Accepts the createCamera fields plus `aspect`.
    setCamera(opts = {}) {
      if (opts.aspect !== undefined) aspect = opts.aspect;
      for (const key of ["target", "yaw", "pitch", "distance", "minDistance", "maxDistance", "fovY", "near", "far", "bounds"]) {
        if (opts[key] !== undefined) camera[key] = key === "target" ? [...opts.target] : opts[key];
      }
      return camera;
    },

    orbit(dxPx, dyPx) { return orbitCamera(camera, dxPx, dyPx); },
    pan(dxPx, dyPx) { return panCamera(camera, dxPx, dyPx, gl.drawingBufferHeight); },
    zoom(wheelDelta) { return zoomCamera(camera, wheelDelta); },

    // Clears colour and depth, latches the view-projection and the visible ground box for this frame, and starts
    // the CPU timer that endFrame closes.
    beginFrame(clearRgb = [0.05, 0.06, 0.08]) {
      frameStart = performance.now();
      gl.viewport(0, 0, gl.drawingBufferWidth, gl.drawingBufferHeight);
      const a = aspect ?? gl.drawingBufferWidth / Math.max(1, gl.drawingBufferHeight);
      viewProjection(camera, a, viewProj);
      cullBounds = visibleGroundBounds(camera, a, groundBox);
      eye = eyeOf(camera);
      gl.clearColor(clearRgb[0], clearRgb[1], clearRgb[2], 1);
      gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
      stats.drawCalls = 0;
      stats.instances = 0;
      stats.culled = 0;
      stats.lod = 0;
      gl.useProgram(program);
      gl.uniformMatrix4fv(uni.viewProj, false, viewProj);
    },

    // Closes the frame's CPU timer: stats.frameCpuMs is the milliseconds since beginFrame (draw submission only, not GPU time).
    endFrame() {
      stats.frameCpuMs = performance.now() - frameStart;
      return stats.frameCpuMs;
    },

    // False when a ground point with the given reach (meshReach) lies wholly outside the view box. Each false answer
    // counts one culled item in stats.culled. Always true when the view has no ground box (horizon on screen).
    isVisible(x, z, reach) {
      if (!cullBounds) return true;
      const inside = inCullBox(x, z, reach);
      if (!inside) stats.culled += 1;
      return inside;
    },

    // Draws the batch's instances that can be on screen, returning how many were drawn. A batch whose instances are
    // all visible draws as it is, keeping its static upload; otherwise the visible matrices are compacted into a
    // per-batch scratch batch that re-uploads each frame.
    drawInstancedVisible(mesh, batch) {
      return drawVisible(mesh, batch);
    },

    // Like drawInstancedVisible, but instances beyond the LOD distance draw with lowMesh (same matrices). Without a
    // lowMesh it is drawInstancedVisible. Tiers are kept per instance with LOD_HYSTERESIS between the two switches.
    drawInstancedLod(mesh, batch, lowMesh = null) {
      return lowMesh ? drawLod(mesh, batch, lowMesh) : drawVisible(mesh, batch);
    },

    // transform: column-major 16-element model matrix (identity when omitted).
    drawMesh(mesh, transform = null) {
      draw(mesh, transform, null);
    },

    // One draw call for many copies of an unskinned mesh. batch: { matrices: Float32Array of count * 16
    // column-major model matrices, count, dirty?: boolean }. The matrices are re-uploaded when batch.dirty
    // is set or the batch has not been uploaded yet; a static batch is uploaded once. Normals use the
    // upper 3x3 of each matrix, so scales must be uniform.
    drawInstanced(mesh, batch) {
      if (batch.count > 0) drawInstances(mesh, batch);
    },

    // joints: packed column-major mat4 per joint (at most MAX_JOINTS), from anim.js jointMatrices. Drawn with
    // the skinned path when the mesh carries joints and weights; otherwise it draws as drawMesh does.
    drawSkinned(mesh, transform, joints) {
      draw(mesh, transform, joints);
    },

    drawSplat,
    updateSplatCell,
    setShadows,
    beginShadowPass,
    beginShadowUnits,
    endShadowPass,
  };
  liveRenderers.add(renderer);
  return renderer;

  function bindMaterial(mesh) {
    gl.uniform3fv(uni.color, mesh.color || [1, 1, 1]);
    gl.uniform3fv(uni.tint, mesh.tint || [1, 1, 1]);
    gl.uniform3fv(uni.shade, mesh.shade || DEFAULT_SHADE);
    const glow = mesh.blend === "add";
    gl.uniform1i(uni.glow, glow ? 1 : 0);
    gl.uniform1f(uni.alpha, mesh.alpha ?? 1);
    if (glow) {
      gl.enable(gl.BLEND);
      gl.blendFunc(gl.SRC_ALPHA, gl.ONE);
      gl.depthMask(false);
    }
    gl.uniform1i(uni.useTex, mesh.texture ? 1 : 0);
    if (mesh.texture) {
      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, mesh.texture);
    }
    return glow;
  }

  function bindGeometry(buf) {
    gl.bindBuffer(gl.ARRAY_BUFFER, buf.pos);
    gl.enableVertexAttribArray(attr.pos);
    gl.vertexAttribPointer(attr.pos, 3, gl.FLOAT, false, 0, 0);
    gl.bindBuffer(gl.ARRAY_BUFFER, buf.normal);
    gl.enableVertexAttribArray(attr.normal);
    gl.vertexAttribPointer(attr.normal, 3, gl.FLOAT, false, 0, 0);
    gl.bindBuffer(gl.ARRAY_BUFFER, buf.uv);
    gl.enableVertexAttribArray(attr.uv);
    gl.vertexAttribPointer(attr.uv, 2, gl.FLOAT, false, 0, 0);
  }

  function drawInstances(mesh, batch) {
    if (isTransparent(mesh)) {
      drawEachInstance(mesh, batch);
      return;
    }
    const buf = upload(mesh);
    bindGeometry(buf);
    gl.disableVertexAttribArray(attr.joints);
    gl.disableVertexAttribArray(attr.weights);
    let inst = instanceBuffers.get(batch);
    if (!inst) {
      inst = gl.createBuffer();
      instanceBuffers.set(batch, inst);
      batch.dirty = true;
      casterGen += 1;
    }
    gl.bindBuffer(gl.ARRAY_BUFFER, inst);
    if (batch.dirty) {
      gl.bufferData(gl.ARRAY_BUFFER, batch.matrices.subarray(0, batch.count * 16), gl.DYNAMIC_DRAW);
      batch.dirty = false;
    }
    for (let c = 0; c < 4; c++) {
      gl.enableVertexAttribArray(attr.inst + c);
      gl.vertexAttribPointer(attr.inst + c, 4, gl.FLOAT, false, 64, c * 16);
      gl.vertexAttribDivisor(attr.inst + c, 1);
    }
    gl.uniform1i(uni.skinned, 0);
    gl.uniform1i(uni.instanced, 1);
    const glow = bindMaterial(mesh);
    if (buf.index) {
      gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, buf.index);
      gl.drawElementsInstanced(gl.TRIANGLES, buf.count, buf.indexType, 0, batch.count);
    } else {
      gl.drawArraysInstanced(gl.TRIANGLES, 0, buf.count, batch.count);
    }
    if (glow) {
      gl.disable(gl.BLEND);
      gl.depthMask(true);
    }
    for (let c = 0; c < 4; c++) {
      gl.vertexAttribDivisor(attr.inst + c, 0);
      gl.disableVertexAttribArray(attr.inst + c);
    }
    gl.uniform1i(uni.instanced, 0);
    stats.drawCalls += 1;
    stats.instances += batch.count;
  }

  function inCullBox(x, z, reach) {
    return x + reach >= cullBounds[0] && x - reach <= cullBounds[2]
      && z + reach >= cullBounds[1] && z - reach <= cullBounds[3];
  }

  // The compacted copy of a batch's visible instances, one per slot ('high' or 'low'). Its ids and spare arrays
  // swap each frame so the kept set can be compared with the last one without allocating.
  function compactedFor(batch, slot) {
    let slots = compactedBatches.get(batch);
    if (!slots) {
      slots = {};
      compactedBatches.set(batch, slots);
    }
    let vis = slots[slot];
    if (!vis) {
      vis = {
        matrices: new Float32Array(batch.count * 16),
        ids: new Uint32Array(batch.count),
        spare: new Uint32Array(batch.count),
        count: 0,
        dirty: true,
      };
      slots[slot] = vis;
    }
    return vis;
  }

  // Writes the source indices of instances whose ground box is on screen into ids; returns how many.
  function keepInside(src, count, reach, ids) {
    let n = 0;
    for (let i = 0; i < count; i++) {
      if (!cullBounds || inCullBox(src[i * 16 + 12], src[i * 16 + 14], reach)) ids[n++] = i;
    }
    return n;
  }

  // Copies the n kept instances (their ids are in vis.spare) into vis. vis.dirty is set only when the kept set or
  // any kept matrix differs from what was last uploaded, so a static scene re-uploads nothing.
  function compact(vis, src, n) {
    const ids = vis.spare;
    const prev = vis.ids;
    let changed = vis.count !== n;
    for (let k = 0; k < n && !changed; k++) if (prev[k] !== ids[k]) changed = true;
    vis.ids = ids;
    vis.spare = prev;
    vis.count = n;
    const m = vis.matrices;
    for (let k = 0; k < n; k++) {
      const s = ids[k] * 16;
      const d = k * 16;
      for (let c = 0; c < 16; c++) {
        const v = src[s + c];
        if (m[d + c] !== v) {
          m[d + c] = v;
          changed = true;
        }
      }
    }
    vis.dirty = changed;
  }

  function drawVisible(mesh, batch) {
    if (!cullBounds) {
      if (batch.count > 0) drawInstances(mesh, batch);
      return batch.count;
    }
    const vis = compactedFor(batch, 'high');
    const n = keepInside(batch.matrices, batch.count, meshReach(mesh), vis.spare);
    stats.culled += batch.count - n;
    if (n === 0) return 0;
    if (n === batch.count) {
      drawInstances(mesh, batch);
      return n;
    }
    compact(vis, batch.matrices, n);
    drawInstances(mesh, vis);
    return n;
  }

  function drawLod(mesh, batch, lowMesh) {
    const count = batch.count;
    const src = batch.matrices;
    const reach = meshReach(mesh);
    let tiers = lodTiers.get(batch);
    if (!tiers) {
      tiers = new Uint8Array(count);
      lodTiers.set(batch, tiers);
    }
    const far = camera.distance * LOD_FAR_PER_DISTANCE;
    const back = far * LOD_HYSTERESIS;
    const hi = compactedFor(batch, 'high');
    const lo = compactedFor(batch, 'low');
    let nh = 0;
    let nl = 0;
    for (let i = 0; i < count; i++) {
      const o = i * 16;
      if (cullBounds && !inCullBox(src[o + 12], src[o + 14], reach)) continue;
      const dx = src[o + 12] - eye[0];
      const dy = src[o + 13] - eye[1];
      const dz = src[o + 14] - eye[2];
      const d = Math.hypot(dx, dy, dz);
      let tier = tiers[i];
      if (tier === 0 && d > far) tier = 1;
      else if (tier === 1 && d < back) tier = 0;
      tiers[i] = tier;
      if (tier === 0) hi.spare[nh++] = i;
      else lo.spare[nl++] = i;
    }
    const kept = nh + nl;
    stats.culled += count - kept;
    stats.lod += nl;
    if (nh > 0) {
      compact(hi, src, nh);
      drawInstances(mesh, hi);
    }
    if (nl > 0) {
      compact(lo, src, nl);
      drawInstances(lowMesh, lo);
    }
    return kept;
  }

  // Transparent meshes (additive glow or alpha below one) draw one instance at a time, in batch order, instead of
  // through the instanced path, so they blend in the order they were placed.
  function isTransparent(mesh) {
    return mesh.blend === "add" || (mesh.alpha ?? 1) < 1;
  }

  function drawEachInstance(mesh, batch) {
    const m = batch.matrices;
    for (let i = 0; i < batch.count; i++) {
      for (let k = 0; k < 16; k++) modelScratch[k] = m[i * 16 + k];
      drawModel(mesh, null);
    }
  }

  // Splat ground: one draw of the terrain mesh. splat = { control: {data, width, height}, layers: [{albedo, height}] x4,
  // waterMap: {data, width, height}, variant: {data, width, height} (optional grass variant weight), waterTint, width, height
  // (map size in cells), uvScale, band, gpu }; the control, water and variant textures are created on first draw and kept.
  function controlTexture(control) {
    const tex = gl.createTexture();
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    // Linear: ground layers blend across cell borders. The rock (cliff) channel is read nearest in the shader, so cliffs stay hard.
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, control.width, control.height, 0, gl.RGBA, gl.UNSIGNED_BYTE, control.data);
    return tex;
  }
  // One byte per cell (R8): the water weight map and the grass variant weight map. Linear filtering blends each edge across
  // a cell; nearest filtering cut the water edge at cell boundaries, which showed as squares under the surface.
  function r8Texture(map) {
    const tex = gl.createTexture();
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.R8, map.width, map.height, 0, gl.RED, gl.UNSIGNED_BYTE, map.data);
    return tex;
  }
  function drawSplat(mesh, splat) {
    if (!splat.gpu) {
      const variant = splat.variant || NO_VARIANT;
      splat.gpu = {
        control: controlTexture(splat.control), waterMap: r8Texture(splat.waterMap),
        variant: r8Texture(variant), variantOn: !!splat.variant && splat.variant.data.some((v) => v > 0),
      };
      casterGen += 1;
    }
    const buf = upload(mesh);
    gl.useProgram(splatProgram);
    bindGeometry(buf);
    gl.disableVertexAttribArray(attr.joints);
    gl.disableVertexAttribArray(attr.weights);
    for (let c = 0; c < 4; c++) gl.disableVertexAttribArray(attr.inst + c);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, splat.gpu.control);
    gl.uniform1i(splatUni.control, 0);
    // Water: the weight map on unit 10 (unit 9 is the shadow map).
    gl.activeTexture(gl.TEXTURE10);
    gl.bindTexture(gl.TEXTURE_2D, splat.gpu.waterMap);
    gl.uniform1i(splatUni.waterMap, 10);
    gl.activeTexture(gl.TEXTURE0 + VARIANT_UNIT);
    gl.bindTexture(gl.TEXTURE_2D, splat.gpu.variant);
    gl.uniform1i(splatUni.variant, VARIANT_UNIT);
    gl.uniform1f(splatUni.variantOn, splat.gpu.variantOn ? 1 : 0);
    if (rippleTex) {
      gl.activeTexture(gl.TEXTURE0 + RIPPLE_UNIT);
      gl.bindTexture(gl.TEXTURE_2D, rippleTex);
      gl.activeTexture(gl.TEXTURE0);
    }
    gl.uniform3fv(splatUni.waterTint, splat.waterTint);
    gl.uniform1f(splatUni.time, performance.now() / 1000);
    gl.uniform3f(splatUni.eye, eye[0], eye[1], eye[2]);
    splat.layers.forEach((layer, k) => {
      gl.activeTexture(gl.TEXTURE1 + k);
      gl.bindTexture(gl.TEXTURE_2D, layer.albedo);
      gl.uniform1i(splatUni.alb[k], 1 + k);
      gl.activeTexture(gl.TEXTURE5 + k);
      gl.bindTexture(gl.TEXTURE_2D, layer.height);
      gl.uniform1i(splatUni.disp[k], 5 + k);
    });
    gl.uniform2f(splatUni.mapSize, splat.width, splat.height);
    gl.uniform1f(splatUni.uvScale, splat.uvScale);
    gl.uniform1f(splatUni.band, splat.band);
    gl.uniform3fv(splatUni.shade, splat.shade);
    gl.uniform3fv(splatUni.layerTint, splat.tints.flat());
    gl.uniformMatrix4fv(splatUni.viewProj, false, passVP);
    gl.uniformMatrix4fv(splatUni.model, false, unitModel);
    gl.uniformMatrix3fv(splatUni.normal, false, normalMatrix(unitModel, normalScratch));
    gl.uniform1i(splatUni.skinned, 0);
    gl.uniform1i(splatUni.instanced, 0);
    if (buf.index) {
      gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, buf.index);
      gl.drawElements(gl.TRIANGLES, buf.count, buf.indexType, 0);
    } else {
      gl.drawArrays(gl.TRIANGLES, 0, buf.count);
    }
    gl.useProgram(program);
    stats.drawCalls += 1;
    stats.instances += 1;
  }
  // Rewrites one cell's control texel in place: a sub-region upload, no rebuild of the terrain mesh.
  function updateSplatCell(splat, index, texel) {
    if (!splat.gpu) return;
    const w = splat.control.width;
    // Creep sets a plain ground kind, never a grass variant, so the cell's variant weight clears with its texel.
    if (splat.variant) {
      splat.variant.data[index] = 0;
      gl.activeTexture(gl.TEXTURE0 + VARIANT_UNIT);
      gl.bindTexture(gl.TEXTURE_2D, splat.gpu.variant);
      gl.texSubImage2D(gl.TEXTURE_2D, 0, index % w, Math.floor(index / w), 1, 1, gl.RED, gl.UNSIGNED_BYTE, ZERO_TEXEL);
    }
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, splat.gpu.control);
    gl.texSubImage2D(gl.TEXTURE_2D, 0, index % w, Math.floor(index / w), 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, texel);
    stats.texelUploads = (stats.texelUploads || 0) + 1;
  }

  // The ripple height field: 512 px, repeating, mipmapped. Bound on its own unit beside the shadow and water maps.
  function makeRippleTexture(bitmap) {
    const tex = gl.createTexture();
    gl.activeTexture(gl.TEXTURE0 + RIPPLE_UNIT);
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.REPEAT);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.REPEAT);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, gl.RGBA, gl.UNSIGNED_BYTE, bitmap);
    gl.generateMipmap(gl.TEXTURE_2D);
    gl.activeTexture(gl.TEXTURE0);
    return tex;
  }

  // A depth-only render target of `size` texels square. Returns null when the framebuffer is incomplete.
  function makeDepthTarget(size) {
    const tex = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.DEPTH_COMPONENT24, size, size, 0, gl.DEPTH_COMPONENT, gl.UNSIGNED_INT, null);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    const fbo = gl.createFramebuffer();
    gl.bindFramebuffer(gl.FRAMEBUFFER, fbo);
    gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.DEPTH_ATTACHMENT, gl.TEXTURE_2D, tex, 0);
    const complete = gl.checkFramebufferStatus(gl.FRAMEBUFFER) === gl.FRAMEBUFFER_COMPLETE;
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    return complete ? { tex, fbo } : null;
  }

  // Builds the sun's shadow maps once: an orthographic view over a square of `halfExtent` cells around `center`.
  function setShadows({ enabled, size, center, halfExtent }) {
    shadowOn = Boolean(enabled);
    if (!shadowOn) return false;
    if (!shadowTex) {
      shadowSize = size;
      const working = makeDepthTarget(size);
      const layer = working && makeDepthTarget(size);
      if (!working || !layer) { shadowOn = false; return false; }
      shadowTex = working.tex;
      shadowFbo = working.fbo;
      staticTex = layer.tex;
      staticFbo = layer.fbo;
    }
    shadowGen = -1;
    const reach = 2 * SHADOW_DISTANCE;
    const eye = [center[0] + SUN_DIR[0] * SHADOW_DISTANCE, center[1] + SUN_DIR[1] * SHADOW_DISTANCE, center[2] + SUN_DIR[2] * SHADOW_DISTANCE];
    const proj = orthographic(-halfExtent, halfExtent, -halfExtent, halfExtent, 1, reach);
    multiply(proj, lookAt(eye, center, [0, 1, 0]), lightVP);
    for (const [p, locs] of [[program, uni], [splatProgram, splatUni]]) {
      gl.useProgram(p);
      gl.uniformMatrix4fv(locs.lightVP, false, lightVP);
      gl.uniform1i(locs.shadow, SHADOW_UNIT);
      gl.uniform1f(locs.shadowTexel, 1 / size);
      gl.uniform1f(locs.shadowOn, 1);
    }
    gl.useProgram(program);
    gl.activeTexture(gl.TEXTURE0 + SHADOW_UNIT);
    gl.bindTexture(gl.TEXTURE_2D, shadowTex);
    gl.activeTexture(gl.TEXTURE0);
    return true;
  }

  // Shadow pass, in three steps. The static casters (terrain, doodads, buildings, towers) live in a cached layer that is
  // redrawn only when a caster GPU resource is created (casterGen moves). Each frame the working map is refilled from
  // that layer with one depth blit (no draw calls), and the dynamic casters (units) are drawn on top of it.
  //   beginShadowPass()   binds the static layer; returns { staticDirty }, or null when shadows are off. When staticDirty,
  //                       the caller draws the static casters into it.
  //   beginShadowUnits()  copies the static depth into the working map and binds it for the dynamic casters.
  //   endShadowPass()     restores the lit pass.
  function beginShadowPass() {
    if (!shadowOn || !shadowTex) return null;
    const staticDirty = shadowGen !== casterGen;
    savedCullBounds = cullBounds;
    cullBounds = null;
    gl.activeTexture(gl.TEXTURE0 + SHADOW_UNIT);
    gl.bindTexture(gl.TEXTURE_2D, null);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindFramebuffer(gl.FRAMEBUFFER, staticFbo);
    gl.viewport(0, 0, shadowSize, shadowSize);
    gl.colorMask(false, false, false, false);
    gl.enable(gl.POLYGON_OFFSET_FILL);
    gl.polygonOffset(2, 4);
    if (staticDirty) {
      gl.clear(gl.DEPTH_BUFFER_BIT);
      shadowGen = casterGen;
    }
    gl.useProgram(program);
    gl.uniform1f(uni.shadowOn, 0);
    gl.uniformMatrix4fv(uni.viewProj, false, lightVP);
    gl.useProgram(splatProgram);
    gl.uniform1f(splatUni.shadowOn, 0);
    gl.useProgram(program);
    passVP = lightVP;
    return { staticDirty };
  }

  function beginShadowUnits() {
    gl.bindFramebuffer(gl.READ_FRAMEBUFFER, staticFbo);
    gl.bindFramebuffer(gl.DRAW_FRAMEBUFFER, shadowFbo);
    gl.blitFramebuffer(0, 0, shadowSize, shadowSize, 0, 0, shadowSize, shadowSize, gl.DEPTH_BUFFER_BIT, gl.NEAREST);
    gl.bindFramebuffer(gl.FRAMEBUFFER, shadowFbo);
  }

  function endShadowPass() {
    cullBounds = savedCullBounds;
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    gl.viewport(0, 0, gl.drawingBufferWidth, gl.drawingBufferHeight);
    gl.colorMask(true, true, true, true);
    gl.disable(gl.POLYGON_OFFSET_FILL);
    passVP = viewProj;
    gl.useProgram(program);
    gl.uniform1f(uni.shadowOn, 1);
    gl.uniformMatrix4fv(uni.viewProj, false, viewProj);
    gl.useProgram(splatProgram);
    gl.uniform1f(splatUni.shadowOn, 1);
    gl.useProgram(program);
    gl.activeTexture(gl.TEXTURE0 + SHADOW_UNIT);
    gl.bindTexture(gl.TEXTURE_2D, shadowTex);
    gl.activeTexture(gl.TEXTURE0);
  }

  function draw(mesh, transform, joints) {
    if (transform) modelScratch.set(transform);
    else identity(modelScratch);
    drawModel(mesh, joints);
  }

  // Draws the mesh with the model matrix already in modelScratch.
  function drawModel(mesh, joints) {
    const model = modelScratch;
    const buf = upload(mesh);
    bindGeometry(buf);
    const skinned = Boolean(joints && buf.joints);
    if (skinned) {
      const palette = joints.length > MAX_JOINTS * 16 ? joints.subarray(0, MAX_JOINTS * 16) : joints;
      gl.bindBuffer(gl.ARRAY_BUFFER, buf.joints);
      gl.enableVertexAttribArray(attr.joints);
      gl.vertexAttribPointer(attr.joints, 4, gl.FLOAT, false, 0, 0);
      gl.bindBuffer(gl.ARRAY_BUFFER, buf.weights);
      gl.enableVertexAttribArray(attr.weights);
      gl.vertexAttribPointer(attr.weights, 4, gl.FLOAT, false, 0, 0);
      gl.uniformMatrix4fv(uni.joints, false, palette);
    } else {
      gl.disableVertexAttribArray(attr.joints);
      gl.disableVertexAttribArray(attr.weights);
    }
    gl.uniform1i(uni.skinned, skinned ? 1 : 0);
    gl.uniform1i(uni.instanced, 0);
    gl.uniformMatrix4fv(uni.model, false, model);
    gl.uniformMatrix3fv(uni.normal, false, normalMatrix(model, normalScratch));
    const glow = bindMaterial(mesh);
    if (buf.index) {
      gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, buf.index);
      gl.drawElements(gl.TRIANGLES, buf.count, buf.indexType, 0);
    } else {
      gl.drawArrays(gl.TRIANGLES, 0, buf.count);
    }
    if (glow) {
      gl.disable(gl.BLEND);
      gl.depthMask(true);
    }
    stats.drawCalls += 1;
    stats.instances += 1;
  }
}

export function setCamera(renderer, opts) {
  return renderer.setCamera(opts);
}

export function drawMesh(renderer, mesh, transform) {
  return renderer.drawMesh(mesh, transform);
}

export function drawInstanced(renderer, mesh, batch) {
  return renderer.drawInstanced(mesh, batch);
}
