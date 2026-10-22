// GLB (glTF 2.0 binary) parser: the first mesh's positions, normals and indices. Pure: no DOM and no
// fetch, so Node CI can import it. Takes an ArrayBuffer and returns typed arrays ready for WebGL buffers.

const MAGIC = 0x46546c67; // "glTF"
const CHUNK_JSON = 0x4e4f534a;
const CHUNK_BIN = 0x004e4942;
const COMPONENT_TYPES = Object.freeze({
  5120: Int8Array, 5121: Uint8Array, 5122: Int16Array, 5123: Uint16Array, 5125: Uint32Array, 5126: Float32Array,
});
const COMPONENT_COUNTS = Object.freeze({ SCALAR: 1, VEC2: 2, VEC3: 3, VEC4: 4, MAT4: 16 });
const NORMALIZE_DIVISOR = Object.freeze({ 5120: 127, 5121: 255, 5122: 32767, 5123: 65535 });
const PATH_TYPE = Object.freeze({ translation: 'VEC3', rotation: 'VEC4', scale: 'VEC3' });

function readChunks(buffer) {
  const view = new DataView(buffer);
  if (view.getUint32(0, true) !== MAGIC) throw new Error('glb: bad magic');
  const total = view.getUint32(8, true);
  if (total > buffer.byteLength) throw new Error('glb: header length exceeds the buffer');
  let json = null;
  let bin = null;
  for (let at = 12; at + 8 <= total;) {
    const length = view.getUint32(at, true);
    const type = view.getUint32(at + 4, true);
    const start = at + 8;
    if (start + length > total) throw new Error('glb: chunk overruns the file');
    if (type === CHUNK_JSON) json = JSON.parse(new TextDecoder().decode(new Uint8Array(buffer, start, length)));
    else if (type === CHUNK_BIN) bin = new Uint8Array(buffer, start, length);
    at = start + length;
  }
  if (!json) throw new Error('glb: no JSON chunk');
  return { json, bin };
}

export { readChunks as readGlbChunks };

// Packs a glTF JSON object and its BIN chunk into a GLB ArrayBuffer.
export function writeGlb(json, bin) {
  const jsonBytes = new TextEncoder().encode(JSON.stringify(json));
  const jsonLength = (jsonBytes.byteLength + 3) & ~3;
  const binLength = (bin.byteLength + 3) & ~3;
  const total = 12 + 8 + jsonLength + 8 + binLength;
  const out = new ArrayBuffer(total);
  const view = new DataView(out);
  const bytes = new Uint8Array(out);
  view.setUint32(0, MAGIC, true);
  view.setUint32(4, 2, true);
  view.setUint32(8, total, true);
  view.setUint32(12, jsonLength, true);
  view.setUint32(16, CHUNK_JSON, true);
  bytes.fill(0x20, 20 + jsonBytes.byteLength, 20 + jsonLength);
  bytes.set(jsonBytes, 20);
  const binHeader = 20 + jsonLength;
  view.setUint32(binHeader, binLength, true);
  view.setUint32(binHeader + 4, CHUNK_BIN, true);
  bytes.set(bin, binHeader + 8);
  return out;
}

const VERTEX_TYPE = 'VEC3';

const LITTLE_ENDIAN = new Uint8Array(new Uint16Array([1]).buffer)[0] === 1;

function readAccessor(gltf, bin, index, expectedType) {
  const accessor = gltf.accessors[index];
  if (accessor.type !== expectedType) throw new Error(`glb: accessor ${index} is ${accessor.type}, expected ${expectedType}`);
  const view = gltf.bufferViews[accessor.bufferView];
  const Type = COMPONENT_TYPES[accessor.componentType];
  const width = COMPONENT_COUNTS[accessor.type];
  if (!Type || !width) throw new Error(`glb: unsupported accessor ${accessor.componentType} ${accessor.type}`);
  const stride = view.byteStride || Type.BYTES_PER_ELEMENT * width;
  const base = (view.byteOffset || 0) + (accessor.byteOffset || 0);
  // Fast path: packed, aligned, little-endian data is the same bytes as the typed array, so one copy replaces a
  // DataView read per component. Big-endian hosts and strided or unaligned data take the component loop below.
  const at = bin.byteOffset + base;
  if (LITTLE_ENDIAN && stride === Type.BYTES_PER_ELEMENT * width && at % Type.BYTES_PER_ELEMENT === 0) {
    return new Type(bin.buffer, at, accessor.count * width).slice();
  }
  const out = new Type(accessor.count * width);
  const raw = new DataView(bin.buffer, bin.byteOffset, bin.byteLength);
  for (let i = 0; i < accessor.count; i++) {
    for (let c = 0; c < width; c++) {
      const at = base + i * stride + c * Type.BYTES_PER_ELEMENT;
      out[i * width + c] = readComponent(raw, at, accessor.componentType);
    }
  }
  return out;
}

function readComponent(view, at, componentType) {
  switch (componentType) {
    case 5120: return view.getInt8(at);
    case 5121: return view.getUint8(at);
    case 5122: return view.getInt16(at, true);
    case 5123: return view.getUint16(at, true);
    case 5125: return view.getUint32(at, true);
    case 5126: return view.getFloat32(at, true);
    default: throw new Error(`glb: component ${componentType}`);
  }
}

// The image a texture samples: the EXT_texture_webp or KHR_texture_basisu source when present, else the core source.
function textureSource(texture) {
  const extensions = texture?.extensions;
  return extensions?.EXT_texture_webp?.source ?? extensions?.KHR_texture_basisu?.source ?? texture?.source ?? -1;
}

// Reads the base color image reference of the primitive's material. An embedded image (bufferView)
// yields its bytes and mime type; an external or data: uri yields the uri as written in the file.
function readBaseColorTexture(gltf, bin, primitive) {
  const material = gltf.materials?.[primitive.material ?? -1];
  const ref = material?.pbrMetallicRoughness?.baseColorTexture;
  if (!ref) return null;
  const image = gltf.images?.[textureSource(gltf.textures?.[ref.index])];
  if (!image) return null;
  const texCoord = ref.texCoord ?? 0;
  if (image.bufferView !== undefined) {
    const view = gltf.bufferViews[image.bufferView];
    const bytes = bin.slice(view.byteOffset || 0, (view.byteOffset || 0) + view.byteLength);
    return { uri: null, mimeType: image.mimeType ?? 'image/png', bytes, texCoord };
  }
  return { uri: image.uri ?? null, mimeType: image.mimeType ?? null, bytes: null, texCoord };
}

// Returns { positions: Float32Array, normals: Float32Array|null, texcoords: Float32Array|null,
//   indices: Uint32Array, baseColorTexture: {uri, mimeType, bytes, texCoord}|null }
// for the first primitive of the first mesh. Triangles only; indices widened to Uint32.
// texcoords is TEXCOORD_0 (two floats per vertex); baseColorTexture is the material's image reference.
export function parseGlb(buffer) {
  const { json: gltf, bin } = readChunks(buffer);
  if (!bin) throw new Error('glb: no BIN chunk');
  const primitive = gltf.meshes?.[0]?.primitives?.[0];
  if (!primitive || primitive.attributes?.POSITION === undefined) throw new Error('glb: no POSITION attribute');
  if (primitive.mode !== undefined && primitive.mode !== 4) throw new Error('glb: only triangle primitives');
  const positions = readFloats(gltf, bin, primitive.attributes.POSITION, VERTEX_TYPE);
  const normals = primitive.attributes.NORMAL === undefined ? null : readFloats(gltf, bin, primitive.attributes.NORMAL, VERTEX_TYPE);
  const texcoords = primitive.attributes.TEXCOORD_0 === undefined ? null : readFloats(gltf, bin, primitive.attributes.TEXCOORD_0, 'VEC2');
  const vertexCount = positions.length / COMPONENT_COUNTS[VERTEX_TYPE];
  const indices = primitive.indices === undefined
    ? Uint32Array.from({ length: vertexCount }, (_, i) => i)
    : Uint32Array.from(readAccessor(gltf, bin, primitive.indices, 'SCALAR'));
  const joints = primitive.attributes.JOINTS_0 === undefined ? null : Float32Array.from(readAccessor(gltf, bin, primitive.attributes.JOINTS_0, 'VEC4'));
  const weights = primitive.attributes.WEIGHTS_0 === undefined ? null : readFloats(gltf, bin, primitive.attributes.WEIGHTS_0, 'VEC4');
  return {
    positions: Float32Array.from(positions),
    normals: normals && Float32Array.from(normals),
    texcoords,
    indices,
    joints,
    weights,
    baseColorTexture: readBaseColorTexture(gltf, bin, primitive),
  };
}

// Reads an accessor as float32 values, dividing normalized integer components into [-1, 1] or [0, 1].
function readFloats(gltf, bin, index, expectedType) {
  const values = readAccessor(gltf, bin, index, expectedType);
  const accessor = gltf.accessors[index];
  const divisor = accessor.normalized ? NORMALIZE_DIVISOR[accessor.componentType] : undefined;
  if (!divisor) return Float32Array.from(values);
  const out = new Float32Array(values.length);
  for (let i = 0; i < values.length; i++) out[i] = Math.max(values[i] / divisor, -1);
  return out;
}

// Returns { clips, skins }. Each clip is { name, duration, channels } where a channel is
// { node, path: 'translation'|'rotation'|'scale', interpolation, times, values, width }; values hold
// width floats per key, or three per key (in-tangent, value, out-tangent) for CUBICSPLINE. Weight
// channels are skipped. Each skin is { joints, inverseBindMatrices, skeleton }.
export function parseGlbAnimations(buffer) {
  const { json: gltf, bin } = readChunks(buffer);
  if (!bin) throw new Error('glb: no BIN chunk');
  const clips = (gltf.animations ?? []).map((animation) => {
    const samplers = animation.samplers ?? [];
    const channels = [];
    let duration = 0;
    for (const channel of animation.channels ?? []) {
      const path = channel.target?.path;
      const type = PATH_TYPE[path];
      if (channel.target?.node === undefined || !type) continue;
      const sampler = samplers[channel.sampler];
      const times = readFloats(gltf, bin, sampler.input, 'SCALAR');
      const values = readFloats(gltf, bin, sampler.output, type);
      const interpolation = sampler.interpolation ?? 'LINEAR';
      const width = COMPONENT_COUNTS[type];
      duration = Math.max(duration, times[times.length - 1] ?? 0);
      channels.push({ node: channel.target.node, path, interpolation, times, values, width });
    }
    return { name: animation.name ?? '', duration, channels };
  });
  const skins = (gltf.skins ?? []).map((skin) => ({
    joints: Uint32Array.from(skin.joints ?? []),
    inverseBindMatrices: skin.inverseBindMatrices === undefined
      ? null
      : readFloats(gltf, bin, skin.inverseBindMatrices, 'MAT4'),
    skeleton: skin.skeleton ?? null,
  }));
  const nodes = (gltf.nodes ?? []).map((node) => ({
    name: node.name ?? null,
    children: node.children ?? [],
    translation: node.translation ?? null,
    rotation: node.rotation ?? null,
    scale: node.scale ?? null,
    matrix: node.matrix ?? null,
  }));
  return { clips, skins, nodes };
}
