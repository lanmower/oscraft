// GLB (glTF 2.0 binary) parser: the first mesh's positions, normals and indices. Pure: no DOM and no
// fetch, so Node CI can import it. Takes an ArrayBuffer and returns typed arrays ready for WebGL buffers.

const MAGIC = 0x46546c67; // "glTF"
const CHUNK_JSON = 0x4e4f534a;
const CHUNK_BIN = 0x004e4942;
const COMPONENT_TYPES = Object.freeze({
  5120: Int8Array, 5121: Uint8Array, 5122: Int16Array, 5123: Uint16Array, 5125: Uint32Array, 5126: Float32Array,
});
const COMPONENT_COUNTS = Object.freeze({ SCALAR: 1, VEC2: 2, VEC3: 3, VEC4: 4 });

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

function readAccessor(gltf, bin, index) {
  const accessor = gltf.accessors[index];
  const view = gltf.bufferViews[accessor.bufferView];
  const Type = COMPONENT_TYPES[accessor.componentType];
  const width = COMPONENT_COUNTS[accessor.type];
  if (!Type || !width) throw new Error(`glb: unsupported accessor ${accessor.componentType} ${accessor.type}`);
  const stride = view.byteStride || Type.BYTES_PER_ELEMENT * width;
  const base = (view.byteOffset || 0) + (accessor.byteOffset || 0);
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

// Returns { positions: Float32Array, normals: Float32Array|null, indices: Uint32Array }
// for the first primitive of the first mesh. Triangles only; indices widened to Uint32.
export function parseGlb(buffer) {
  const { json: gltf, bin } = readChunks(buffer);
  if (!bin) throw new Error('glb: no BIN chunk');
  const primitive = gltf.meshes?.[0]?.primitives?.[0];
  if (!primitive || primitive.attributes?.POSITION === undefined) throw new Error('glb: no POSITION attribute');
  if (primitive.mode !== undefined && primitive.mode !== 4) throw new Error('glb: only triangle primitives');
  const positions = readAccessor(gltf, bin, primitive.attributes.POSITION);
  const normals = primitive.attributes.NORMAL === undefined ? null : readAccessor(gltf, bin, primitive.attributes.NORMAL);
  const indices = primitive.indices === undefined
    ? Uint32Array.from({ length: positions.length / 3 }, (_, i) => i)
    : Uint32Array.from(readAccessor(gltf, bin, primitive.indices));
  return { positions: Float32Array.from(positions), normals: normals && Float32Array.from(normals), indices };
}
