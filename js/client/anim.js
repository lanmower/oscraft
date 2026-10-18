// Clip playback for skinned units. Pure: no DOM, no fetch. Consumes the clips parseGlbAnimations
// (glb.js) returns and yields per-node local poses { translation, rotation, scale } as Float32Arrays.
// Composing poses with the skin is the renderer's job; wiring to a unit's state is the caller's.

const STATE_CLIPS = Object.freeze({ attack: ['attack'], walk: ['walk'], idle: ['idle'] });

export function chooseState({ attacking = false, moving = false } = {}) {
  if (attacking) return 'attack';
  return moving ? 'walk' : 'idle';
}

// Exact name first, then the first clip whose name starts with the requested name (so 'attack'
// finds 'attack-melee-right').
export function findClip(clips, name) {
  return clips.find((clip) => clip.name === name) ?? clips.find((clip) => clip.name.startsWith(name)) ?? null;
}

function segment(times, t) {
  const last = times.length - 1;
  if (t <= times[0]) return { lo: 0, hi: 0, u: 0 };
  if (t >= times[last]) return { lo: last, hi: last, u: 0 };
  let lo = 0;
  let hi = last;
  while (hi - lo > 1) {
    const mid = (lo + hi) >> 1;
    if (times[mid] <= t) lo = mid;
    else hi = mid;
  }
  return { lo, hi, u: (t - times[lo]) / (times[hi] - times[lo]) };
}

function sampleLinear(channel, s, out) {
  const { values, width } = channel;
  for (let c = 0; c < width; c++) {
    out[c] = values[s.lo * width + c] + (values[s.hi * width + c] - values[s.lo * width + c]) * s.u;
  }
}

function sampleCubic(channel, s, dt, out) {
  const { values, width } = channel;
  const u = s.u;
  const u2 = u * u;
  const u3 = u2 * u;
  const h00 = 2 * u3 - 3 * u2 + 1;
  const h10 = u3 - 2 * u2 + u;
  const h01 = -2 * u3 + 3 * u2;
  const h11 = u3 - u2;
  for (let c = 0; c < width; c++) {
    const p0 = values[(3 * s.lo + 1) * width + c];
    const b0 = values[(3 * s.lo + 2) * width + c];
    const p1 = values[(3 * s.hi + 1) * width + c];
    const a1 = values[(3 * s.hi) * width + c];
    out[c] = h00 * p0 + h10 * dt * b0 + h01 * p1 + h11 * dt * a1;
  }
}

function sampleStep(channel, s, out) {
  const { values, width } = channel;
  for (let c = 0; c < width; c++) out[c] = values[s.lo * width + c];
}

function slerp(a, b, u, out) {
  let dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];
  const sign = dot < 0 ? -1 : 1;
  dot = Math.min(1, Math.abs(dot));
  if (dot > 0.9995) {
    for (let c = 0; c < 4; c++) out[c] = a[c] + (sign * b[c] - a[c]) * u;
  } else {
    const theta = Math.acos(dot);
    const wa = Math.sin((1 - u) * theta) / Math.sin(theta);
    const wb = Math.sin(u * theta) / Math.sin(theta);
    for (let c = 0; c < 4; c++) out[c] = wa * a[c] + wb * sign * b[c];
  }
  const len = Math.hypot(out[0], out[1], out[2], out[3]) || 1;
  for (let c = 0; c < 4; c++) out[c] /= len;
}

function sampleChannel(channel, time, loop, scratch) {
  const { times, interpolation, width } = channel;
  if (times.length === 0) return null;
  const s = segment(times, time);
  const out = scratch.slice(0, width);
  if (interpolation === 'STEP') sampleStep(channel, s, out);
  else if (interpolation === 'CUBICSPLINE') sampleCubic(channel, s, times[s.hi] - times[s.lo] || 0, out);
  else if (channel.path === 'rotation' && s.lo !== s.hi) {
    const a = channel.values.subarray(s.lo * 4, s.lo * 4 + 4);
    const b = channel.values.subarray(s.hi * 4, s.hi * 4 + 4);
    slerp(a, b, s.u, out);
  } else sampleLinear(channel, s, out);
  return out;
}

// Local pose per animated node at `time` seconds. Nodes without a channel keep their rest pose
// (absent from the result). With loop=false the time clamps to the clip's last key.
export function samplePose(clip, time, loop = true) {
  const duration = clip.duration;
  let t = time;
  if (duration > 0) t = loop ? ((time % duration) + duration) % duration : Math.min(Math.max(time, 0), duration);
  else t = 0;
  const pose = new Map();
  const scratch = new Float32Array(4);
  for (const channel of clip.channels) {
    const value = sampleChannel(channel, t, loop, scratch);
    if (!value) continue;
    let entry = pose.get(channel.node);
    if (!entry) {
      entry = {};
      pose.set(channel.node, entry);
    }
    entry[channel.path] = Float32Array.from(value);
  }
  return pose;
}

// Holds the active clip and its clock. play() keeps the current clip when the name is unknown.
export class AnimationPlayer {
  constructor(clips) {
    this.clips = clips;
    this.clip = null;
    this.time = 0;
    this.loop = true;
  }

  play(name, { loop = true } = {}) {
    const clip = findClip(this.clips, name);
    if (!clip) return null;
    if (clip !== this.clip) {
      this.clip = clip;
      this.time = 0;
    }
    this.loop = loop;
    return clip.name;
  }

  // Plays the clip for a unit state: attack, walk or idle, falling back to idle when absent.
  playState(state) {
    const wanted = STATE_CLIPS[chooseState(state)];
    for (const name of wanted) {
      const played = this.play(name, { loop: chooseState(state) !== 'attack' });
      if (played) return played;
    }
    return this.play('idle');
  }

  update(dt) {
    if (this.clip) this.time += dt;
    if (this.clip && !this.loop && this.time >= this.clip.duration) this.time = this.clip.duration;
  }

  pose() {
    return this.clip ? samplePose(this.clip, this.time, this.loop) : null;
  }
}

// Column-major 4x4 matrices as Float32Array(16). Skinning: a skin's joint matrices are the joint's global
// transform times its inverse bind matrix, then the model fit, so the shader only blends them per vertex.
const IDENTITY = Float32Array.of(1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1);

export function mat4Multiply(a, b) {
  const out = new Float32Array(16);
  for (let c = 0; c < 4; c++) {
    for (let r = 0; r < 4; r++) {
      let v = 0;
      for (let k = 0; k < 4; k++) v += a[k * 4 + r] * b[c * 4 + k];
      out[c * 4 + r] = v;
    }
  }
  return out;
}

function trsMatrix(t, q, s) {
  const [x, y, z, w] = q;
  const xx = x * x, yy = y * y, zz = z * z, xy = x * y, xz = x * z, yz = y * z, wx = w * x, wy = w * y, wz = w * z;
  return Float32Array.of(
    (1 - 2 * (yy + zz)) * s[0], 2 * (xy + wz) * s[0], 2 * (xz - wy) * s[0], 0,
    2 * (xy - wz) * s[1], (1 - 2 * (xx + zz)) * s[1], 2 * (yz + wx) * s[1], 0,
    2 * (xz + wy) * s[2], 2 * (yz - wx) * s[2], (1 - 2 * (xx + yy)) * s[2], 0,
    t[0], t[1], t[2], 1,
  );
}

// A node's local matrix: its animated TRS from the pose when it has one, else its rest TRS or matrix.
function localMatrix(node, animated) {
  if (!animated && node.matrix) return Float32Array.from(node.matrix);
  return trsMatrix(
    animated?.translation ?? node.translation ?? [0, 0, 0],
    animated?.rotation ?? node.rotation ?? [0, 0, 0, 1],
    animated?.scale ?? node.scale ?? [1, 1, 1],
  );
}

// Global matrix per node for a pose (Map node index -> { translation, rotation, scale }). Roots are nodes no
// other node lists as a child.
export function globalMatrices(nodes, pose = new Map()) {
  const globals = new Array(nodes.length);
  const hasParent = new Uint8Array(nodes.length);
  for (const node of nodes) for (const child of node.children) hasParent[child] = 1;
  const visit = (index, parent) => {
    const local = localMatrix(nodes[index], pose.get(index));
    globals[index] = parent ? mat4Multiply(parent, local) : local;
    for (const child of nodes[index].children) visit(child, globals[index]);
  };
  nodes.forEach((_, index) => {
    if (!hasParent[index]) visit(index, null);
  });
  return globals;
}

// Joint matrices for a skin at a pose, pre-multiplied by `fit` (the model's fit to its footprint). The result
// packs one column-major mat4 per joint, in the order of skin.joints.
export function jointMatrices(nodes, skin, pose, fit) {
  const globals = globalMatrices(nodes, pose);
  const count = skin.joints.length;
  const out = new Float32Array(16 * count);
  for (let j = 0; j < count; j++) {
    const inverseBind = skin.inverseBindMatrices ? skin.inverseBindMatrices.subarray(16 * j, 16 * j + 16) : IDENTITY;
    out.set(mat4Multiply(fit, mat4Multiply(globals[skin.joints[j]], inverseBind)), 16 * j);
  }
  return out;
}
