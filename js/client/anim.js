// Clip playback for skinned units. Pure: no DOM, no fetch. Consumes the clips parseGlbAnimations
// (glb.js) returns and yields per-node local poses { translation, rotation, scale } as Float32Arrays.
// Composing poses with the skin is the renderer's job; wiring to a unit's state is the caller's.

// A moving unit plays the library's run clip; walk is the fallback for a clip set that has no run.
const STATE_CLIPS = Object.freeze({ attack: ['attack'], walk: ['run', 'walk'], idle: ['idle'] });

export function chooseState({ attacking = false, moving = false } = {}) {
  if (attacking) return 'attack';
  return moving ? 'walk' : 'idle';
}

// Exact name first, then the first clip whose name starts with the requested name (so 'attack'
// finds 'attack-melee-right').
export function findClip(clips, name) {
  return clips.find((clip) => clip.name === name) ?? clips.find((clip) => clip.name.startsWith(name)) ?? null;
}

// Keyframe segment for the sampled time, written into one shared record (sampling reads it straight away).
const SEGMENT = { lo: 0, hi: 0, u: 0 };
function segment(times, t) {
  const last = times.length - 1;
  if (t <= times[0]) { SEGMENT.lo = 0; SEGMENT.hi = 0; SEGMENT.u = 0; return SEGMENT; }
  if (t >= times[last]) { SEGMENT.lo = last; SEGMENT.hi = last; SEGMENT.u = 0; return SEGMENT; }
  let lo = 0;
  let hi = last;
  while (hi - lo > 1) {
    const mid = (lo + hi) >> 1;
    if (times[mid] <= t) lo = mid;
    else hi = mid;
  }
  SEGMENT.lo = lo; SEGMENT.hi = hi; SEGMENT.u = (t - times[lo]) / (times[hi] - times[lo]);
  return SEGMENT;
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

// Quaternion slerp between values[ao..ao+4) and values[bo..bo+4) into out.
function slerp(values, ao, bo, u, out) {
  let dot = values[ao] * values[bo] + values[ao + 1] * values[bo + 1] + values[ao + 2] * values[bo + 2] + values[ao + 3] * values[bo + 3];
  const sign = dot < 0 ? -1 : 1;
  dot = Math.min(1, Math.abs(dot));
  if (dot > 0.9995) {
    for (let c = 0; c < 4; c++) out[c] = values[ao + c] + (sign * values[bo + c] - values[ao + c]) * u;
  } else {
    const theta = Math.acos(dot);
    const wa = Math.sin((1 - u) * theta) / Math.sin(theta);
    const wb = Math.sin(u * theta) / Math.sin(theta);
    for (let c = 0; c < 4; c++) out[c] = wa * values[ao + c] + wb * sign * values[bo + c];
  }
  const len = Math.hypot(out[0], out[1], out[2], out[3]) || 1;
  for (let c = 0; c < 4; c++) out[c] /= len;
}

// Writes the channel value at time into out (width floats). False when the channel has no keys.
function sampleChannel(channel, time, out) {
  const { times, interpolation } = channel;
  if (times.length === 0) return false;
  const s = segment(times, time);
  if (interpolation === "STEP") sampleStep(channel, s, out);
  else if (interpolation === "CUBICSPLINE") sampleCubic(channel, s, times[s.hi] - times[s.lo] || 0, out);
  else if (channel.path === "rotation" && s.lo !== s.hi) slerp(channel.values, s.lo * 4, s.hi * 4, s.u, out);
  else sampleLinear(channel, s, out);
  return true;
}

const SAMPLE = new Float32Array(4);

// Local pose per animated node at `time` seconds. Nodes without a channel keep their rest pose
// (absent from the result). With loop=false the time clamps to the clip's last key. Pass `into` (a Map from an
// earlier call on the same clip) to refill its entries in place instead of allocating a new pose.
export function samplePose(clip, time, loop = true, into = null) {
  const duration = clip.duration;
  let t = time;
  if (duration > 0) t = loop ? ((time % duration) + duration) % duration : Math.min(Math.max(time, 0), duration);
  else t = 0;
  const pose = into ?? new Map();
  for (const channel of clip.channels) {
    if (!sampleChannel(channel, t, SAMPLE)) continue;
    let entry = pose.get(channel.node);
    if (!entry) {
      entry = {};
      pose.set(channel.node, entry);
    }
    let dst = entry[channel.path];
    if (!dst) dst = entry[channel.path] = new Float32Array(channel.width);
    for (let c = 0; c < channel.width; c++) dst[c] = SAMPLE[c];
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
    this.poseCache = new Map();
    this.poseClip = null;
    this.rate = 1; // playback rate: the clock advances dt * rate per update (set from the unit's draw-side speed)
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
    if (this.clip) this.time += dt * this.rate;
    if (this.clip && !this.loop && this.time >= this.clip.duration) this.time = this.clip.duration;
  }

  pose() {
    if (!this.clip) return null;
    if (this.poseClip !== this.clip) {
      this.poseCache.clear();
      this.poseClip = this.clip;
    }
    return samplePose(this.clip, this.time, this.loop, this.poseCache);
  }
}

// Column-major 4x4 matrices as Float32Array(16). Skinning: a skin's joint matrices are the joint's global
// transform times its inverse bind matrix, then the model fit, so the shader only blends them per vertex.
const IDENTITY = Float32Array.of(1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1);

// out must not alias a or b. Operands are read at aOff/bOff so one packed array can hold several matrices.
function mulInto(out, oo, a, ao, b, bo) {
  for (let c = 0; c < 4; c++) {
    for (let r = 0; r < 4; r++) {
      let v = 0;
      for (let k = 0; k < 4; k++) v += a[ao + k * 4 + r] * b[bo + c * 4 + k];
      out[oo + c * 4 + r] = v;
    }
  }
  return out;
}

export function mat4Multiply(a, b) {
  return mulInto(new Float32Array(16), 0, a, 0, b, 0);
}

function trsInto(out, o, t, q, s) {
  const x = q[0], y = q[1], z = q[2], w = q[3];
  const xx = x * x, yy = y * y, zz = z * z, xy = x * y, xz = x * z, yz = y * z, wx = w * x, wy = w * y, wz = w * z;
  out[o] = (1 - 2 * (yy + zz)) * s[0]; out[o + 1] = 2 * (xy + wz) * s[0]; out[o + 2] = 2 * (xz - wy) * s[0]; out[o + 3] = 0;
  out[o + 4] = 2 * (xy - wz) * s[1]; out[o + 5] = (1 - 2 * (xx + zz)) * s[1]; out[o + 6] = 2 * (yz + wx) * s[1]; out[o + 7] = 0;
  out[o + 8] = 2 * (xz + wy) * s[2]; out[o + 9] = 2 * (yz - wx) * s[2]; out[o + 10] = (1 - 2 * (xx + yy)) * s[2]; out[o + 11] = 0;
  out[o + 12] = t[0]; out[o + 13] = t[1]; out[o + 14] = t[2]; out[o + 15] = 1;
  return out;
}

const TRANSLATION_0 = [0, 0, 0];
const ROTATION_0 = [0, 0, 0, 1];
const SCALE_0 = [1, 1, 1];

// A node's local matrix written at out[o]: its animated TRS from the pose when it has one, else its rest TRS or matrix.
function localInto(out, o, node, animated) {
  if (!animated && node.matrix) {
    for (let k = 0; k < 16; k++) out[o + k] = node.matrix[k];
    return out;
  }
  return trsInto(out, o, animated?.translation ?? node.translation ?? TRANSLATION_0,
    animated?.rotation ?? node.rotation ?? ROTATION_0, animated?.scale ?? node.scale ?? SCALE_0);
}

// Per node.children array: the roots (nodes no other node lists as a child) and one scratch block of 16 floats per
// node for globals, plus a local and a joint-product scratch. Built once per nodes array, reused every frame.
const nodeScratch = new WeakMap();
function scratchFor(nodes) {
  let s = nodeScratch.get(nodes);
  if (!s) {
    const hasParent = new Uint8Array(nodes.length);
    for (const node of nodes) for (const child of node.children) hasParent[child] = 1;
    const roots = [];
    for (let i = 0; i < nodes.length; i++) if (!hasParent[i]) roots.push(i);
    s = { roots, globals: new Float32Array(16 * nodes.length), local: new Float32Array(16), product: new Float32Array(16) };
    nodeScratch.set(nodes, s);
  }
  return s;
}

const EMPTY_POSE = new Map();

function visitGlobals(nodes, index, parent, pose, s) {
  const o = index * 16;
  localInto(s.local, 0, nodes[index], pose.get(index));
  if (parent < 0) s.globals.set(s.local, o);
  else mulInto(s.globals, o, s.globals, parent * 16, s.local, 0);
  for (const child of nodes[index].children) visitGlobals(nodes, child, index, pose, s);
}

// Global matrix per node for a pose (Map node index -> { translation, rotation, scale }), packed as 16 floats per
// node into the shared scratch that the caller must read before the next call for the same nodes array.
export function globalMatrices(nodes, pose = EMPTY_POSE) {
  const s = scratchFor(nodes);
  for (const root of s.roots) visitGlobals(nodes, root, -1, pose, s);
  return s.globals;
}

// Joint matrices for a skin at a pose, pre-multiplied by `fit` (the model's fit to its footprint). Writes one
// column-major mat4 per joint, in the order of skin.joints, into `out` (length 16 * joints) and returns it. Pass a
// reused `out` per character so a frame allocates nothing here.
export function jointMatrices(nodes, skin, pose, fit, out = new Float32Array(16 * skin.joints.length)) {
  const s = scratchFor(nodes);
  globalMatrices(nodes, pose ?? EMPTY_POSE);
  const ib = skin.inverseBindMatrices;
  const count = skin.joints.length;
  for (let j = 0; j < count; j++) {
    const g = skin.joints[j] * 16;
    if (ib) mulInto(s.product, 0, s.globals, g, ib, 16 * j);
    else mulInto(s.product, 0, s.globals, g, IDENTITY, 0);
    mulInto(out, 16 * j, fit, 0, s.product, 0);
  }
  return out;
}

// World rotation of a node's parent at rest, from the root down the chain of parent rest rotations.
export function parentWorldRotation(nodes, index) {
  const parent = nodes.findIndex((node) => node.children.includes(index));
  if (parent < 0) return [0, 0, 0, 1];
  return quatMul(parentWorldRotation(nodes, parent), nodes[parent].rotation ?? [0, 0, 0, 1]);
}

// Rest-pose hips height in world units and the world scale of the hips' parent, for one rig's node tree.
export function restHips(nodes, hipsIndex) {
  const globals = globalMatrices(nodes);
  let parent = -1;
  nodes.forEach((node, i) => { if (node.children.includes(hipsIndex)) parent = i; });
  const parentScale = parent < 0 ? 1 : Math.hypot(globals[parent * 16], globals[parent * 16 + 1], globals[parent * 16 + 2]);
  return { height: globals[hipsIndex * 16 + 13], parentScale };
}

// Lowest skinned vertex height at one joint-matrix set (fit space): the vertex's weighted joint transform.
function lowestSkinned(M, positions, joints, weights) {
  let lo = Infinity;
  const n = positions.length / 3;
  for (let v = 0; v < n; v++) {
    const px = positions[v * 3], py = positions[v * 3 + 1], pz = positions[v * 3 + 2];
    let y = 0;
    for (let k = 0; k < 4; k++) {
      const w = weights[v * 4 + k];
      if (!w) continue;
      const o = joints[v * 4 + k] * 16;
      y += w * (M[o + 1] * px + M[o + 5] * py + M[o + 9] * pz + M[o + 13]);
    }
    if (y < lo) lo = y;
  }
  return lo;
}

// The rig's floor: the lowest skinned vertex over sampled frames of the given clips, in fit space. Subtracting it from the
// fit's y translation stands the rig's feet on y = 0 in those poses (the raw-position fit does not match the bind pose).
export function skinFloor(nodes, skin, fit, positions, joints, weights, clips, frames = 12) {
  let lo = Infinity;
  for (const clip of clips) {
    for (let i = 0; i <= frames; i++) {
      const M = jointMatrices(nodes, skin, samplePose(clip, clip.duration * i / frames, true), fit);
      lo = Math.min(lo, lowestSkinned(M, positions, joints, weights));
    }
  }
  return lo;
}

// The gait's world speed at playback rate 1: twice the largest hips-to-foot offset over one cycle, per clip duration.
// A clip plays at the unit's draw speed divided by this, so the feet keep pace with the ground. 0 when the rig has no
// hips or feet to measure.
export function cycleSpeed(nodes, fit, clip, frames = 24) {
  const hips = nodes.findIndex((n) => /^hips$/i.test(n.name ?? ""));
  const feet = nodes.flatMap((n, i) => (/foot$/i.test(n.name ?? "") ? [i] : []));
  if (hips < 0 || feet.length === 0 || !(clip.duration > 0)) return 0;
  const s = fit[0];
  let reach = 0;
  for (let i = 0; i < frames; i++) {
    const G = globalMatrices(nodes, samplePose(clip, clip.duration * i / frames, true));
    const hx = s * G[hips * 16 + 12], hz = s * G[hips * 16 + 14];
    for (const f of feet) reach = Math.max(reach, Math.hypot(s * G[f * 16 + 12] - hx, s * G[f * 16 + 14] - hz));
  }
  return (2 * reach) / clip.duration;
}

function quatMul(a, b) {
  return [
    a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
    a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
    a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
    a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
  ];
}

function quatConj(q) { return [-q[0], -q[1], -q[2], q[3]]; }

function rotateVector(q, v) {
  const r = quatMul(quatMul(q, [v[0], v[1], v[2], 0]), quatConj(q));
  return [r[0], r[1], r[2]];
}

// Re-expresses a humanoid library (js/client/anim/library.json) on one rig. map.bones maps each humanoid bone to the
// rig's node name. Each rotation is carried through world space: the source motion is a delta on its rest pose, applied
// to the rig's rest pose in the rig's own bone frame, so rigs whose bone axes differ still pose the same way.
// Translations are rotated the same way and scaled by the ratio of the rig's hips height to the source's. Bones the rig
// lacks are dropped; the rig keeps its rest pose there.
export function retargetLibrary(library, map, nodes) {
  const byName = new Map();
  nodes.forEach((node, i) => { if (node.name) byName.set(node.name, i); });
  const boneNode = (bone) => byName.get(map.bones[bone]) ?? -1;
  const hips = boneNode('hips');
  const rigHips = hips < 0 ? null : restHips(nodes, hips);
  const scale = rigHips ? (rigHips.height / library.hipsHeight) * (library.hipsParentScale / rigHips.parentScale) : 1;
  const frames = new Map();
  const frameOf = (bone) => {
    if (frames.has(bone)) return frames.get(bone);
    const node = boneNode(bone);
    const source = library.rest[bone];
    if (node < 0 || !source) return null;
    const rigRest = nodes[node].rotation ?? [0, 0, 0, 1];
    const rigParent = parentWorldRotation(nodes, node);
    const A = quatMul(quatConj(rigParent), source.parentWorld);
    const B = quatMul(quatConj(source.rotation), quatMul(quatConj(source.parentWorld), quatMul(rigParent, rigRest)));
    const frame = { node, A, B, source };
    frames.set(bone, frame);
    return frame;
  };
  return library.clips.map((clip) => {
    const channels = [];
    for (const channel of clip.channels) {
      const frame = frameOf(channel.bone);
      if (!frame) continue;
      const count = channel.times.length;
      const values = new Float32Array(channel.values.length);
      if (channel.path === 'rotation') {
        for (let i = 0; i < count; i++) {
          const q = quatMul(quatMul(frame.A, channel.values.slice(i * 4, i * 4 + 4)), frame.B);
          const len = Math.hypot(q[0], q[1], q[2], q[3]) || 1;
          for (let c = 0; c < 4; c++) values[i * 4 + c] = q[c] / len;
        }
      } else {
        const rest = nodes[frame.node].translation ?? [0, 0, 0];
        const k = channel.bone === 'hips' || channel.bone === 'root' ? scale : 1;
        for (let i = 0; i < count; i++) {
          const delta = [0, 1, 2].map((c) => channel.values[i * 3 + c] - frame.source.translation[c]);
          const moved = rotateVector(frame.A, delta);
          for (let c = 0; c < 3; c++) values[i * 3 + c] = rest[c] + moved[c] * k;
        }
      }
      channels.push({ node: frame.node, path: channel.path, interpolation: 'LINEAR', times: Float32Array.from(channel.times), values, width: channel.path === 'rotation' ? 4 : 3 });
    }
    return { name: clip.name, duration: clip.duration, channels };
  });
}
