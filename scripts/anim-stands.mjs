import fs from 'node:fs';
import path from 'node:path';
import { readGlbChunks } from '../js/client/glb.js';
import { MeshoptDecoder } from '../js/client/vendor/three/libs/meshopt_decoder.module.js';

const [libraryPath, source] = process.argv.slice(2);
if (!libraryPath || !source) throw new Error('usage: node scripts/anim-stands.mjs <library.json> <character.glb>');

const TORSO_HEAD = { torso: 'spine', head: 'head' };
const WITH_ARMS = { ...TORSO_HEAD, 'arm-left': 'leftUpperArm', 'arm-right': 'rightUpperArm' };
const STAND_CLIPS = [
  { name: 'static', source: 'static', bones: TORSO_HEAD },
  { name: 'emote-yes', source: 'emote-yes', bones: TORSO_HEAD },
  { name: 'emote-no', source: 'emote-no', bones: TORSO_HEAD },
  { name: 'idle-alt', source: 'idle', bones: WITH_ARMS },
  { name: 'holding-both', source: 'holding-both', bones: WITH_ARMS },
];
const IDENTITY = [0, 0, 0, 1];
const FLOAT32 = 5126;
const WIDTH = { SCALAR: 1, VEC3: 3, VEC4: 4 };

const round = (v) => Math.round(v * 1e5) / 1e5;
const conj = (q) => [-q[0], -q[1], -q[2], q[3]];
const mul = (a, b) => [
  a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
  a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
  a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
  a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
];
const unit = (q) => {
  const len = Math.hypot(q[0], q[1], q[2], q[3]) || 1;
  return q.map((c) => c / len);
};
const dot = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];

await MeshoptDecoder.ready;

const library = JSON.parse(fs.readFileSync(libraryPath, 'utf8'));
const file = fs.readFileSync(source);
const { json: gltf, bin } = readGlbChunks(file.buffer.slice(file.byteOffset, file.byteOffset + file.byteLength));

const decoded = new Map();
function viewBytes(index) {
  if (!decoded.has(index)) {
    const view = gltf.bufferViews[index];
    const meshopt = view.extensions?.EXT_meshopt_compression;
    if (meshopt) {
      const out = new Uint8Array(meshopt.count * meshopt.byteStride);
      const offset = meshopt.byteOffset ?? 0;
      MeshoptDecoder.decodeGltfBuffer(out, meshopt.count, meshopt.byteStride, bin.subarray(offset, offset + meshopt.byteLength), meshopt.mode, meshopt.filter ?? 'NONE');
      decoded.set(index, { bytes: out, stride: meshopt.byteStride });
    } else {
      const offset = view.byteOffset ?? 0;
      decoded.set(index, { bytes: bin.subarray(offset, offset + view.byteLength), stride: view.byteStride });
    }
  }
  return decoded.get(index);
}

function readFloats(index) {
  const accessor = gltf.accessors[index];
  if (accessor.componentType !== FLOAT32) throw new Error(`accessor ${index} is not float32`);
  const width = WIDTH[accessor.type];
  const { bytes, stride: viewStride } = viewBytes(accessor.bufferView);
  const stride = viewStride || width * 4;
  const base = accessor.byteOffset ?? 0;
  const data = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const out = new Float32Array(accessor.count * width);
  for (let i = 0; i < accessor.count; i += 1) {
    for (let c = 0; c < width; c += 1) out[i * width + c] = data.getFloat32(base + i * stride + c * 4, true);
  }
  return out;
}

function standClip({ name, source: sourceName, bones }) {
  const animation = gltf.animations.find((a) => a.name === sourceName);
  if (!animation) throw new Error(`clip ${sourceName} missing from ${source}`);
  const channels = [];
  let duration = 0;
  for (const channel of animation.channels) {
    if (channel.target.path !== 'rotation') continue;
    const node = gltf.nodes[channel.target.node];
    const bone = bones[node.name];
    if (!bone) continue;
    const times = readFloats(animation.samplers[channel.sampler].input);
    const values = readFloats(animation.samplers[channel.sampler].output);
    const charRest = node.rotation ?? IDENTITY;
    const libRest = library.rest[bone].rotation;
    const out = [];
    let previous = libRest;
    for (let i = 0; i < times.length; i += 1) {
      const local = [values[i * 4], values[i * 4 + 1], values[i * 4 + 2], values[i * 4 + 3]];
      let key = unit(mul(libRest, mul(conj(charRest), local)));
      if (dot(previous, key) < 0) key = key.map((c) => -c);
      previous = key;
      out.push(...key);
    }
    duration = Math.max(duration, times[times.length - 1]);
    channels.push({ bone, path: 'rotation', times: Array.from(times, round), values: out.map(round), width: 4 });
  }
  const driven = new Set(channels.map((c) => c.bone));
  for (const frozen of library.clips.find((c) => c.name === 'idle').channels) {
    if (driven.has(frozen.bone)) continue;
    const first = frozen.values.slice(0, frozen.width).map(round);
    channels.push({ bone: frozen.bone, path: frozen.path, times: [0, round(duration)], values: [...first, ...first], width: frozen.width });
  }
  return { name, source: `${path.basename(source)}:${sourceName}`, duration: round(duration), channels };
}

for (const entry of STAND_CLIPS) {
  const clip = standClip(entry);
  const at = library.clips.findIndex((c) => c.name === entry.name);
  if (at >= 0) library.clips[at] = clip;
  else library.clips.push(clip);
}

fs.writeFileSync(libraryPath, JSON.stringify(library));
console.log(`wrote ${libraryPath}: ${library.clips.map((c) => c.name).join(', ')}`);
