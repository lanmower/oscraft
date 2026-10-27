import fs from 'node:fs';
import path from 'node:path';
import { parseGlbAnimations } from '../js/client/glb.js';
import { parentWorldRotation, restHips } from '../js/client/anim.js';

const [source, target = 'scenarios/moba/assets/anim/library.json'] = process.argv.slice(2);
if (!source) throw new Error('usage: node scripts/anim-extract.mjs <spoint anim-lib.glb> [out.json]');

const HUMANOID_SOURCE = {
  root: 'root', hips: 'mixamorig:Hips', spine: 'mixamorig:Spine', chest: 'mixamorig:Spine1', upperChest: 'mixamorig:Spine2',
  neck: 'mixamorig:Neck', head: 'mixamorig:Head',
  leftShoulder: 'mixamorig:LeftShoulder', leftUpperArm: 'mixamorig:LeftArm', leftLowerArm: 'mixamorig:LeftForeArm', leftHand: 'mixamorig:LeftHand',
  rightShoulder: 'mixamorig:RightShoulder', rightUpperArm: 'mixamorig:RightArm', rightLowerArm: 'mixamorig:RightForeArm', rightHand: 'mixamorig:RightHand',
  leftUpperLeg: 'mixamorig:LeftUpLeg', leftLowerLeg: 'mixamorig:LeftLeg', leftFoot: 'mixamorig:LeftFoot', leftToes: 'mixamorig:LeftToeBase',
  rightUpperLeg: 'mixamorig:RightUpLeg', rightLowerLeg: 'mixamorig:RightLeg', rightFoot: 'mixamorig:RightFoot', rightToes: 'mixamorig:RightToeBase',
};

const LIBRARY_CLIPS = {
  idle: 'IdleLoop', walk: 'WalkLoop', run: 'JogFwdLoop', attack: 'SwordRegularA', death: 'Death', cast: 'SpellSimpleShoot',
};

const round = (v) => Math.round(v * 1e5) / 1e5;
const bytes = fs.readFileSync(source);
const buffer = bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength);
const { clips, nodes } = parseGlbAnimations(buffer);
const nodeIndex = new Map(nodes.map((node, i) => [node.name, i]));

const boneOf = new Map(Object.entries(HUMANOID_SOURCE).map(([bone, name]) => [nodeIndex.get(name), bone]));
const rest = {};
for (const [bone, name] of Object.entries(HUMANOID_SOURCE)) {
  const node = nodes[nodeIndex.get(name)];
  if (!node) continue;
  if (node.matrix) throw new Error(`source node ${name} uses a matrix; decompose before extracting`);
  rest[bone] = {
    translation: (node.translation ?? [0, 0, 0]).map(round),
    rotation: (node.rotation ?? [0, 0, 0, 1]).map(round),
    parentWorld: parentWorldRotation(nodes, nodeIndex.get(name)).map(round),
  };
}

const library = [];
for (const [key, sourceName] of Object.entries(LIBRARY_CLIPS)) {
  const clip = clips.find((c) => c.name.replace(/@\d+$/, '') === sourceName);
  if (!clip) throw new Error(`clip ${sourceName} missing from ${source}`);
  const channels = [];
  for (const channel of clip.channels) {
    const bone = boneOf.get(channel.node);
    if (!bone || (channel.path !== 'rotation' && channel.path !== 'translation')) continue;
    channels.push({
      bone,
      path: channel.path,
      times: Array.from(channel.times, round),
      values: Array.from(channel.values, round),
      width: channel.width,
    });
  }
  library.push({ name: key, source: sourceName, duration: round(clip.duration), channels });
}

const hipsNode = nodeIndex.get(HUMANOID_SOURCE.hips);
const sourceHips = restHips(nodes, hipsNode);
const out = {
  source: path.basename(source),
  hipsHeight: round(sourceHips.height),
  hipsParentScale: round(sourceHips.parentScale),
  rest,
  clips: library,
};
fs.mkdirSync(path.dirname(target), { recursive: true });
fs.writeFileSync(target, JSON.stringify(out));
console.log(`wrote ${target}: ${library.length} clips, ${Object.keys(rest).length} bones at rest`);
