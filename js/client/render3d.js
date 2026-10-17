// 3D view for the WebGL2 client: orbit camera math, column-major mat4 helpers and a
// depth-tested mesh draw path. Import-safe under node: nothing touches a DOM or a GL
// context until createRenderer(gl) is called.

const PITCH_LIMIT = Math.PI / 2 - 0.01;

export function identity() {
  return new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]);
}

export function multiply(a, b) {
  const out = new Float32Array(16);
  for (let c = 0; c < 4; c++) {
    for (let r = 0; r < 4; r++) {
      let s = 0;
      for (let k = 0; k < 4; k++) s += a[k * 4 + r] * b[c * 4 + k];
      out[c * 4 + r] = s;
    }
  }
  return out;
}

export function translation(x, y, z) {
  const m = identity();
  m[12] = x; m[13] = y; m[14] = z;
  return m;
}

export function scaling(x, y, z) {
  const m = identity();
  m[0] = x; m[5] = y; m[10] = z;
  return m;
}

export function rotationY(angle) {
  const c = Math.cos(angle), s = Math.sin(angle);
  return new Float32Array([c, 0, -s, 0, 0, 1, 0, 0, s, 0, c, 0, 0, 0, 0, 1]);
}

export function rotationX(angle) {
  const c = Math.cos(angle), s = Math.sin(angle);
  return new Float32Array([1, 0, 0, 0, 0, c, s, 0, 0, -s, c, 0, 0, 0, 0, 1]);
}

export function perspective(fovY, aspect, near, far) {
  const f = 1 / Math.tan(fovY / 2);
  const nf = 1 / (near - far);
  return new Float32Array([
    f / aspect, 0, 0, 0,
    0, f, 0, 0,
    0, 0, (far + near) * nf, -1,
    0, 0, 2 * far * near * nf, 0,
  ]);
}

export function lookAt(eye, target, up) {
  const z = normalize(sub(eye, target));
  const x = normalize(cross(up, z));
  const y = cross(z, x);
  return new Float32Array([
    x[0], y[0], z[0], 0,
    x[1], y[1], z[1], 0,
    x[2], y[2], z[2], 0,
    -dot(x, eye), -dot(y, eye), -dot(z, eye), 1,
  ]);
}

// Normal matrix: inverse transpose of the upper 3x3 of a model matrix, as column-major mat3.
export function normalMatrix(m) {
  const a00 = m[0], a01 = m[4], a02 = m[8];
  const a10 = m[1], a11 = m[5], a12 = m[9];
  const a20 = m[2], a21 = m[6], a22 = m[10];
  const c00 = a11 * a22 - a12 * a21;
  const c01 = a12 * a20 - a10 * a22;
  const c02 = a10 * a21 - a11 * a20;
  const det = a00 * c00 + a01 * c01 + a02 * c02;
  const inv = det === 0 ? 0 : 1 / det;
  return new Float32Array([
    c00 * inv, c01 * inv, c02 * inv,
    (a02 * a21 - a01 * a22) * inv, (a00 * a22 - a02 * a20) * inv, (a01 * a20 - a00 * a21) * inv,
    (a01 * a12 - a02 * a11) * inv, (a02 * a10 - a00 * a12) * inv, (a00 * a11 - a01 * a10) * inv,
  ]);
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

export function projectionMatrix(cam, aspect) {
  return perspective(cam.fovY, aspect, cam.near, cam.far);
}

export function viewProjection(cam, aspect) {
  return multiply(projectionMatrix(cam, aspect), viewMatrix(cam));
}

// Screen-space deltas in pixels; yaw and pitch are radians per pixel.
export function orbitCamera(cam, dxPx, dyPx, radiansPerPx = 0.005) {
  cam.yaw -= dxPx * radiansPerPx;
  cam.pitch = Math.min(PITCH_LIMIT, Math.max(-PITCH_LIMIT, cam.pitch + dyPx * radiansPerPx));
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
  return cam;
}

// Wheel delta in the usual browser units (about 100 per notch). Positive zooms out.
export function zoomCamera(cam, wheelDelta) {
  const d = cam.distance * Math.exp(wheelDelta * 0.001);
  cam.distance = Math.min(cam.maxDistance, Math.max(cam.minDistance, d));
  return cam;
}

const VERT = `#version 300 es
in vec3 aPos; in vec3 aNormal; in vec2 aUv;
uniform mat4 uModel; uniform mat3 uNormal; uniform mat4 uViewProj;
out vec3 vNormal; out vec2 vUv;
void main() {
  vec4 world = uModel * vec4(aPos, 1.0);
  vNormal = uNormal * aNormal; vUv = aUv;
  gl_Position = uViewProj * world;
}`;

const FRAG = `#version 300 es
precision mediump float;
in vec3 vNormal; in vec2 vUv;
uniform vec3 uColor; uniform bool uUseTex; uniform sampler2D uTex;
out vec4 outColor;
void main() {
  vec3 light = normalize(vec3(0.4, 1.0, 0.6));
  float diffuse = max(dot(normalize(vNormal), light), 0.0);
  vec3 base = uUseTex ? texture(uTex, vUv).rgb : uColor;
  outColor = vec4(base * (0.3 + 0.7 * diffuse), 1.0);
}`;

function compile(gl, type, source) {
  const shader = gl.createShader(type);
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(shader));
  return shader;
}

// Uploads a mesh once and keeps its buffers. Mesh shape:
//   { positions: Float32Array xyz, normals: Float32Array xyz, uvs?: Float32Array uv,
//     indices?: Uint16Array|Uint32Array, color?: [r,g,b] in 0..1, texture?: WebGLTexture,
//     dirty?: boolean }
// Set mesh.dirty = true after changing its arrays so the next draw re-uploads them.
export function createRenderer(gl) {
  const program = gl.createProgram();
  gl.attachShader(program, compile(gl, gl.VERTEX_SHADER, VERT));
  gl.attachShader(program, compile(gl, gl.FRAGMENT_SHADER, FRAG));
  gl.linkProgram(program);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(program));
  gl.useProgram(program);

  const attr = {
    pos: gl.getAttribLocation(program, "aPos"),
    normal: gl.getAttribLocation(program, "aNormal"),
    uv: gl.getAttribLocation(program, "aUv"),
  };
  const uni = {
    model: gl.getUniformLocation(program, "uModel"),
    normal: gl.getUniformLocation(program, "uNormal"),
    viewProj: gl.getUniformLocation(program, "uViewProj"),
    color: gl.getUniformLocation(program, "uColor"),
    useTex: gl.getUniformLocation(program, "uUseTex"),
    tex: gl.getUniformLocation(program, "uTex"),
  };
  gl.uniform1i(uni.tex, 0);
  gl.enable(gl.DEPTH_TEST);
  gl.depthFunc(gl.LEQUAL);

  const camera = createCamera();
  const uploads = new WeakMap();
  let viewProj = identity();
  let aspect = null;

  function upload(mesh) {
    let buf = uploads.get(mesh);
    if (!buf || mesh.dirty) {
      if (!buf) buf = { pos: gl.createBuffer(), normal: gl.createBuffer(), uv: gl.createBuffer(), index: null, count: 0, indexType: 0 };
      const vertexCount = mesh.positions.length / 3;
      gl.bindBuffer(gl.ARRAY_BUFFER, buf.pos);
      gl.bufferData(gl.ARRAY_BUFFER, mesh.positions, gl.STATIC_DRAW);
      gl.bindBuffer(gl.ARRAY_BUFFER, buf.normal);
      gl.bufferData(gl.ARRAY_BUFFER, mesh.normals, gl.STATIC_DRAW);
      gl.bindBuffer(gl.ARRAY_BUFFER, buf.uv);
      gl.bufferData(gl.ARRAY_BUFFER, mesh.uvs || new Float32Array(vertexCount * 2), gl.STATIC_DRAW);
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

  return {
    gl,
    camera,

    // Merges the given fields into the orbit camera. Accepts the createCamera fields plus `aspect`.
    setCamera(opts = {}) {
      if (opts.aspect !== undefined) aspect = opts.aspect;
      for (const key of ["target", "yaw", "pitch", "distance", "minDistance", "maxDistance", "fovY", "near", "far"]) {
        if (opts[key] !== undefined) camera[key] = key === "target" ? [...opts.target] : opts[key];
      }
      return camera;
    },

    orbit(dxPx, dyPx) { return orbitCamera(camera, dxPx, dyPx); },
    pan(dxPx, dyPx) { return panCamera(camera, dxPx, dyPx, gl.drawingBufferHeight); },
    zoom(wheelDelta) { return zoomCamera(camera, wheelDelta); },

    // Clears colour and depth and latches the view-projection for this frame.
    beginFrame(clearRgb = [0.05, 0.06, 0.08]) {
      gl.viewport(0, 0, gl.drawingBufferWidth, gl.drawingBufferHeight);
      const a = aspect ?? gl.drawingBufferWidth / Math.max(1, gl.drawingBufferHeight);
      viewProj = viewProjection(camera, a);
      gl.clearColor(clearRgb[0], clearRgb[1], clearRgb[2], 1);
      gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
      gl.useProgram(program);
      gl.uniformMatrix4fv(uni.viewProj, false, viewProj);
    },

    // transform: column-major 16-element model matrix (identity when omitted).
    drawMesh(mesh, transform = null) {
      const model = transform ? Float32Array.from(transform) : identity();
      const buf = upload(mesh);
      gl.bindBuffer(gl.ARRAY_BUFFER, buf.pos);
      gl.enableVertexAttribArray(attr.pos);
      gl.vertexAttribPointer(attr.pos, 3, gl.FLOAT, false, 0, 0);
      gl.bindBuffer(gl.ARRAY_BUFFER, buf.normal);
      gl.enableVertexAttribArray(attr.normal);
      gl.vertexAttribPointer(attr.normal, 3, gl.FLOAT, false, 0, 0);
      gl.bindBuffer(gl.ARRAY_BUFFER, buf.uv);
      gl.enableVertexAttribArray(attr.uv);
      gl.vertexAttribPointer(attr.uv, 2, gl.FLOAT, false, 0, 0);
      gl.uniformMatrix4fv(uni.model, false, model);
      gl.uniformMatrix3fv(uni.normal, false, normalMatrix(model));
      gl.uniform3fv(uni.color, mesh.color || [1, 1, 1]);
      gl.uniform1i(uni.useTex, mesh.texture ? 1 : 0);
      if (mesh.texture) {
        gl.activeTexture(gl.TEXTURE0);
        gl.bindTexture(gl.TEXTURE_2D, mesh.texture);
      }
      if (buf.index) {
        gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, buf.index);
        gl.drawElements(gl.TRIANGLES, buf.count, buf.indexType, 0);
      } else {
        gl.drawArrays(gl.TRIANGLES, 0, buf.count);
      }
    },
  };
}

export function setCamera(renderer, opts) {
  return renderer.setCamera(opts);
}

export function drawMesh(renderer, mesh, transform) {
  return renderer.drawMesh(mesh, transform);
}
