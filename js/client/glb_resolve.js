// Rewrites a GLB that uses EXT_meshopt_compression or KHR_draco_mesh_compression into a plain GLB that glb.js reads
// unchanged: decoded bytes are appended to the BIN chunk, bufferViews and accessors are repointed at them, and the
// extension entries are removed. A file with neither extension is returned as-is, without loading any decoder.

import { readGlbChunks, writeGlb } from './glb.js';
import { decodeDracoMesh, loadMeshopt } from './decoders.js';

const MESHOPT = 'EXT_meshopt_compression';
const DRACO = 'KHR_draco_mesh_compression';
const FLOAT = 5126;
const UNSIGNED_INT = 5125;
const TYPE_BY_COUNT = Object.freeze({ 1: 'SCALAR', 2: 'VEC2', 3: 'VEC3', 4: 'VEC4' });

export async function resolveGlbExtensions(buffer) {
  const { json, bin } = readGlbChunks(buffer);
  const usesMeshopt = json.bufferViews?.some((view) => view.extensions?.[MESHOPT]);
  const usesDraco = json.meshes?.some((mesh) => mesh.primitives?.some((prim) => prim.extensions?.[DRACO]));
  if (!usesMeshopt && !usesDraco) return buffer;
  if (!bin) throw new Error('glb: compressed file has no BIN chunk');
  const gltf = structuredClone(json);
  let appended = new AppendBuffer(bin);
  if (usesMeshopt) {
    const decoder = await loadMeshopt();
    for (const view of gltf.bufferViews) {
      const ext = view.extensions?.[MESHOPT];
      if (!ext) continue;
      if (gltf.buffers?.[ext.buffer]?.uri !== undefined) throw new Error('glb: meshopt source must be the GLB BIN chunk');
      const source = bin.subarray(ext.byteOffset ?? 0, (ext.byteOffset ?? 0) + ext.byteLength);
      const decoded = await decoder.decodeGltfBufferAsync(ext.count, ext.byteStride, source, ext.mode, ext.filter ?? 'NONE');
      view.buffer = 0;
      view.byteOffset = appended.add(decoded);
      view.byteLength = decoded.byteLength;
      delete view.extensions[MESHOPT];
      if (Object.keys(view.extensions).length === 0) delete view.extensions;
    }
  }
  if (usesDraco) {
    const sourceBin = appended.finish();
    appended = new AppendBuffer(sourceBin);
    for (const mesh of gltf.meshes) {
      for (const prim of mesh.primitives) {
        const ext = prim.extensions?.[DRACO];
        if (!ext) continue;
        const view = gltf.bufferViews[ext.bufferView];
        const start = view.byteOffset ?? 0;
        const decoded = await decodeDracoMesh(sourceBin.subarray(start, start + view.byteLength), ext.attributes);
        for (const [name, values] of Object.entries(decoded.attributes)) {
          prim.attributes[name] = addAccessor(gltf, appended.add(new Uint8Array(values.values.buffer)), decoded.numPoints, values.components, FLOAT);
        }
        prim.indices = addAccessor(gltf, appended.add(new Uint8Array(decoded.indices.buffer)), decoded.indices.length, 1, UNSIGNED_INT);
        delete prim.extensions[DRACO];
        if (Object.keys(prim.extensions).length === 0) delete prim.extensions;
      }
    }
  }
  const out = appended.finish();
  gltf.buffers[0].byteLength = out.byteLength;
  for (const list of [gltf.extensionsUsed, gltf.extensionsRequired]) {
    if (!list) continue;
    const kept = list.filter((name) => name !== MESHOPT && name !== DRACO);
    list.splice(0, list.length, ...kept);
  }
  return writeGlb(gltf, out);
}

// Appends typed bytes to a BIN chunk, each entry 4-byte aligned, and returns its byteOffset.
class AppendBuffer {
  constructor(bin) {
    this.parts = [bin];
    this.length = bin.byteLength;
  }

  add(bytes) {
    const pad = (4 - (this.length % 4)) % 4;
    if (pad) {
      this.parts.push(new Uint8Array(pad));
      this.length += pad;
    }
    const offset = this.length;
    this.parts.push(new Uint8Array(bytes.buffer, bytes.byteOffset, bytes.byteLength));
    this.length += bytes.byteLength;
    return offset;
  }

  finish() {
    const out = new Uint8Array(this.length);
    let at = 0;
    for (const part of this.parts) {
      out.set(part, at);
      at += part.byteLength;
    }
    return out;
  }
}

function addAccessor(gltf, byteOffset, count, components, componentType) {
  const bufferView = gltf.bufferViews.length;
  const byteLength = count * components * 4;
  gltf.bufferViews.push({ buffer: 0, byteOffset, byteLength });
  gltf.accessors.push({ bufferView, componentType, count, type: TYPE_BY_COUNT[components] });
  return gltf.accessors.length - 1;
}
