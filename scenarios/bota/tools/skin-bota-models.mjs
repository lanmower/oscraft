#!/usr/bin/env node
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const pack = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const models = path.join(pack, "assets", "models");
const staticSourceDir = process.env.BOTA_STATIC_SOURCE_DIR ?? null;
const ready = JSON.parse(fs.readFileSync(path.resolve(pack, "..", "..", "..", "hy3d", "output", "bota-ready.json"), "utf8"));
const LEGACY_ACRONYMS = { "001": "nec", "002": "cha", "004": "woj", "005": "blo", "006": "tra", "007": "boo", "008": "coo", "009": "gon", "010": "spu", "011": "kar", "012": "how", "013": "san", "014": "lan", "015": "fin", "016": "wil", "017": "nos", "018": "din", "019": "ecc", "020": "bot", "021": "sho", "022": "npc", "023": "for", "024": "col", "025": "gre", "026": "ghi", "027": "got" };
const ACTIVE_IDS = { "001": "neckbeard", "002": "chad", "004": "wojak", "005": "bloomer", "006": "tradwife", "007": "boomer", "008": "coomer", "009": "gondola", "010": "spurdo", "011": "karen", "012": "howard", "013": "sanic", "014": "lankdad", "015": "finger_dan", "016": "willie", "017": "noscope", "018": "dino", "019": "ecce", "020": "bot_bernard", "021": "shoop", "022": "npc", "023": "forever", "024": "cole", "025": "grey", "026": "ghillie", "027": "gothgirl" };
const sha256 = (buf) => crypto.createHash("sha256").update(buf).digest("hex");
const pad4 = (buf, fill = 0) => buf.length % 4 ? Buffer.concat([buf, Buffer.alloc(4 - buf.length % 4, fill)]) : buf;

function readGlb(file) {
  const bytes = fs.readFileSync(file);
  if (bytes.readUInt32LE(0) !== 0x46546c67 || bytes.readUInt32LE(4) !== 2 || bytes.readUInt32LE(8) !== bytes.length) throw new Error(`${file}: bad GLB header`);
  let json = null, bin = null;
  for (let at = 12; at + 8 <= bytes.length;) {
    const length = bytes.readUInt32LE(at), type = bytes.readUInt32LE(at + 4), start = at + 8;
    if (type === 0x4e4f534a) json = JSON.parse(bytes.subarray(start, start + length).toString("utf8"));
    else if (type === 0x004e4942) bin = bytes.subarray(start, start + length);
    at = start + length;
  }
  if (!json || !bin) throw new Error(`${file}: missing GLB chunks`);
  return { bytes, json, bin };
}
function writeGlb(json, bin) {
  const jsonBytes = pad4(Buffer.from(JSON.stringify(json), "utf8"), 0x20);
  const binBytes = pad4(Buffer.from(bin));
  const total = 12 + 8 + jsonBytes.length + 8 + binBytes.length;
  const out = Buffer.alloc(total);
  out.writeUInt32LE(0x46546c67, 0); out.writeUInt32LE(2, 4); out.writeUInt32LE(total, 8);
  out.writeUInt32LE(jsonBytes.length, 12); out.writeUInt32LE(0x4e4f534a, 16); jsonBytes.copy(out, 20);
  const at = 20 + jsonBytes.length;
  out.writeUInt32LE(binBytes.length, at); out.writeUInt32LE(0x004e4942, at + 4); binBytes.copy(out, at + 8);
  return out;
}
function readAccessor(gltf, bin, index, width) {
  const accessor = gltf.accessors[index];
  const view = gltf.bufferViews[accessor.bufferView];
  const stride = view.byteStride ?? width * 4;
  const base = (view.byteOffset ?? 0) + (accessor.byteOffset ?? 0);
  const out = new Float32Array(accessor.count * width);
  const dv = new DataView(bin.buffer, bin.byteOffset, bin.byteLength);
  for (let i = 0; i < accessor.count; i++) for (let c = 0; c < width; c++) out[i * width + c] = dv.getFloat32(base + i * stride + c * 4, true);
  return out;
}
function nodeGlobal(json, index) {
  const parent = [];
  json.nodes.forEach((node, i) => (node.children ?? []).forEach((child) => parent[child] = i));
  const qmul = (a, b) => [a[3]*b[0]+a[0]*b[3]+a[1]*b[2]-a[2]*b[1], a[3]*b[1]-a[0]*b[2]+a[1]*b[3]+a[2]*b[0], a[3]*b[2]+a[0]*b[1]-a[1]*b[3]+a[2]*b[2], a[3]*b[3]-a[0]*b[0]-a[1]*b[1]-a[2]*b[2]];
  const qrot = (q, v) => { const r=qmul(qmul(q,[v[0],v[1],v[2],0]),[-q[0],-q[1],-q[2],q[3]]); return [r[0],r[1],r[2]]; };
  let pos = [0, 0, 0], rot = [0, 0, 0, 1], scale = [1, 1, 1];
  const chain = []; for (let n = index; n !== undefined; n = parent[n]) chain.push(n);
  for (let k = chain.length - 1; k >= 0; k--) {
    const node = json.nodes[chain[k]];
    const local = node.translation ?? [0, 0, 0];
    const turned = qrot(rot, local.map((v, c) => v * scale[c]));
    pos = pos.map((v, c) => v + turned[c]);
    rot = qmul(rot, node.rotation ?? [0, 0, 0, 1]);
    const localScale = node.scale ?? [1, 1, 1];
    scale = scale.map((v, c) => v * localScale[c]);
  }
  return { pos, rot, scale };
}
function inverseBind(node) {
  const { pos, rot, scale } = node;
  const [x,y,z,w] = rot;
  const r = [
    1-2*(y*y+z*z), 2*(x*y+z*w), 2*(x*z-y*w),
    2*(x*y-z*w), 1-2*(x*x+z*z), 2*(y*z+x*w),
    2*(x*z+y*w), 2*(y*z-x*w), 1-2*(x*x+y*y),
  ];
  const invScale = scale.map((v) => 1 / (v || 1));
  const m = [r[0]*invScale[0],r[1]*invScale[0],r[2]*invScale[0],0, r[3]*invScale[1],r[4]*invScale[1],r[5]*invScale[1],0, r[6]*invScale[2],r[7]*invScale[2],r[8]*invScale[2],0, pos[0],pos[1],pos[2],1];
  const rt = [r[0],r[3],r[6], r[1],r[4],r[7], r[2],r[5],r[8]];
  const t = [-(rt[0]*pos[0]+rt[1]*pos[1]+rt[2]*pos[2]), -(rt[3]*pos[0]+rt[4]*pos[1]+rt[5]*pos[2]), -(rt[6]*pos[0]+rt[7]*pos[1]+rt[8]*pos[2])];
  return [r[0]*invScale[0],r[3]*invScale[0],r[6]*invScale[0],0, r[1]*invScale[1],r[4]*invScale[1],r[7]*invScale[1],0, r[2]*invScale[2],r[5]*invScale[2],r[8]*invScale[2],0, t[0],t[1],t[2],1];
}

const rig = readGlb(path.join(models, "roshan-skinned.glb"));
const rigSkin = rig.json.skins[0];
const rigNodes = rigSkin.joints.map((index) => ({ ...rig.json.nodes[index] }));
rigNodes.forEach((node) => { node.children = (node.children ?? []).filter((child) => rigSkin.joints.includes(child)); });
const rigParent = [];
rig.json.nodes.forEach((node, i) => (node.children ?? []).forEach((child) => rigParent[child] = i));
const rigGlobals = rigSkin.joints.map((index) => nodeGlobal(rig.json, index));
const boneMap = JSON.parse(fs.readFileSync(path.join(pack, "assets", "anim", "maps", "humanoid.json"), "utf8"));
const jointNames = rigSkin.joints.map((i) => rig.json.nodes[i].name);
const resolved = Object.values(boneMap.bones).filter((name) => jointNames.includes(name)).length;
if (!jointNames.includes(boneMap.bones.hips) || resolved < Object.keys(boneMap.bones).length * 0.8) throw new Error(`Roshan rig is not humanoid: ${resolved}/${Object.keys(boneMap.bones).length}`);

const manifest = { schema: 1, sourceManifest: "/workspace/hy3d/output/bota-ready.json", rig: "roshan-skinned.glb", animationLibrary: "assets/anim/library.json", boneMap: "assets/anim/maps/humanoid.json", clips: 8, heroes: {} };
const packedHeroes = new Set(Object.keys(JSON.parse(fs.readFileSync(path.join(pack, "bota_ui.json"), "utf8")).heroes));
for (const row of ready.characters.filter((candidate) => packedHeroes.has(ACTIVE_IDS[candidate.slot]))) {
  const active = ACTIVE_IDS[row.slot];
  const legacy = LEGACY_ACRONYMS[row.slot];
  const file = path.join(models, `${active}-skinned.glb`);
  const legacySource = path.join(models, `${legacy}-skinned.glb`);
  const sourcePath = staticSourceDir ? path.join(staticSourceDir, `${active}-skinned.glb`) : legacySource;
  const source = fs.readFileSync(sourcePath);
  if (sha256(source) !== row.sha256 || source.length !== row.bytes) throw new Error(`${active}: source static mesh does not match bota-ready.json; restore it before generating skins`);
  const { json, bin } = readGlb(sourcePath);
  const primitive = json.meshes[0].primitives[0];
  const positions = readAccessor(json, bin, primitive.attributes.POSITION, 3);
  const count = positions.length / 3;
  let minY = Infinity, maxY = -Infinity, minX = Infinity, maxX = -Infinity;
  for (let i = 0; i < count; i++) {
    minY = Math.min(minY, positions[i*3+1]); maxY = Math.max(maxY, positions[i*3+1]);
    minX = Math.min(minX, positions[i*3]); maxX = Math.max(maxX, positions[i*3]);
  }
  const height = maxY - minY || 1;
  const joints = new Uint16Array(count * 4);
  const weights = new Float32Array(count * 4);
  const boneIndex = (name) => rigSkin.joints.indexOf(rig.json.nodes.findIndex((node) => node.name === name));
  const bones = [
    boneIndex("Head"), boneIndex("Spine2"), boneIndex("Spine1"), boneIndex("Spine"), boneIndex("Hips"),
    boneIndex("LeftUpLeg"), boneIndex("RightUpLeg"), boneIndex("LeftLeg"), boneIndex("RightLeg"),
    boneIndex("LeftArm"), boneIndex("RightArm"), boneIndex("LeftForeArm"), boneIndex("RightForeArm"),
  ];
  const centers = [
    [0, .90, 0], [0, .72, 0], [0, .60, 0], [0, .48, 0], [0, .43, 0],
    [-.14, .28, 0], [.14, .28, 0], [-.14, .15, 0], [.14, .15, 0],
    [-.33, .67, 0], [.33, .67, 0], [-.38, .52, 0], [.38, .52, 0],
  ];
  for (let i = 0; i < count; i++) {
    const x = (positions[i*3] - (minX + maxX) / 2) / (maxX - minX || 1);
    const y = (positions[i*3+1] - minY) / height;
    const z = positions[i*3+2];
    const near = centers.map(([cx, cy, cz], k) => ({ k, d: (x - cx) ** 2 + ((y - cy) * 1.25) ** 2 + (z - cz) ** 2 })).sort((a, b) => a.d - b.d).slice(0, 4);
    const inv = near.map((n) => 1 / Math.max(0.0025, n.d));
    const sum = inv.reduce((n, v) => n + v, 0);
    for (let j = 0; j < 4; j++) {
      const pick = near[j] ?? near[0];
      joints[i*4+j] = bones[pick.k];
      weights[i*4+j] = near[j] ? inv[j] / sum : 0;
    }
  }
  const pieces = [Buffer.from(bin)];
  const bufferViews = json.bufferViews.map((view) => ({ ...view }));
  const accessors = json.accessors.map((accessor) => ({ ...accessor }));
  const pushBytes = (buf, target) => {
    const aligned = pad4(Buffer.alloc((4 - (Buffer.concat(pieces).length % 4)) % 4));
    if (aligned.length) { pieces.push(aligned); for (const v of bufferViews) if (v.byteOffset !== undefined && v.byteOffset >= Buffer.concat(pieces).length - aligned.length) v.byteOffset += aligned.length; }
    const offset = Buffer.concat(pieces).length;
    pieces.push(Buffer.from(buf.buffer, buf.byteOffset, buf.byteLength));
    const view = bufferViews.length;
    bufferViews.push({ buffer: 0, byteOffset: offset, byteLength: buf.byteLength, target });
    return view;
  };
  const ibm = new Float32Array(rigGlobals.flatMap(inverseBind));
  const ibmView = pushBytes(ibm, undefined);
  const jointsView = pushBytes(joints, 34962);
  const weightsView = pushBytes(weights, 34962);
  const ibmAccessor = accessors.length; accessors.push({ bufferView: ibmView, componentType: 5126, count: rigSkin.joints.length, type: "MAT4" });
  const jointsAccessor = accessors.length; accessors.push({ bufferView: jointsView, componentType: 5123, count, type: "VEC4" });
  const weightsAccessor = accessors.length; accessors.push({ bufferView: weightsView, componentType: 5126, count, type: "VEC4" });
  const nodeStart = json.nodes.length;
  rigNodes.forEach((node, k) => {
    const oldIndex = rigSkin.joints[k];
    const children = (node.children ?? []).map((child) => nodeStart + rigSkin.joints.indexOf(child)).filter((n) => n >= nodeStart);
    json.nodes.push({ name: node.name, translation: node.translation, rotation: node.rotation, scale: node.scale, children: children.length ? children : undefined });
  });
  const meshNode = json.nodes.findIndex((node) => node.mesh === 0);
  const rootNode = nodeStart + rigSkin.joints.indexOf(27);
  if (meshNode >= 0) json.nodes[meshNode].skin = 0;
  else json.nodes.push({ mesh: 0, skin: 0, name: `${active}_mesh` });
  json.skins = [{ inverseBindMatrices: ibmAccessor, joints: rigSkin.joints.map((_, k) => nodeStart + k) }];
  json.scenes = [{ nodes: [...new Set([...(json.scenes?.[0]?.nodes ?? []), rootNode])] }];
  json.scene = 0;
  primitive.attributes.JOINTS_0 = jointsAccessor;
  primitive.attributes.WEIGHTS_0 = weightsAccessor;
  json.bufferViews = bufferViews;
  json.accessors = accessors;
  const finalBin = Buffer.concat(pieces);
  json.buffers = [{ byteLength: finalBin.length }];
  const output = writeGlb(json, finalBin);
  fs.writeFileSync(file, output);
  manifest.heroes[active] = {
    slot: row.slot,
    sourceStaticSha256: row.sha256,
    sourceStaticBytes: row.bytes,
    skinnedSha256: sha256(output),
    skinnedBytes: output.length,
    skinJoints: rigSkin.joints.length,
    vertices: count,
    libraryClips: 8,
  };
}
fs.writeFileSync(path.join(pack, "bota-skinned.json"), JSON.stringify(manifest, null, 2) + "\n");
console.log(`skinned ${Object.keys(manifest.heroes).length} fullname models with ${manifest.clips} library clips and ${rigSkin.joints.length} joints`);
