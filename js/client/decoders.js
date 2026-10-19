// Lazy loaders for the vendored glTF decoders (three r186 examples/jsm/libs): meshopt (ES module), Draco and
// Basis/KTX2 (emscripten classic scripts). Each loads once, on first use; a model that needs none pays nothing.
// Browser: classic scripts are injected and read from the global. Node (CI check): the same files run in a
// CommonJS shim, so the static test exercises the shipped bytes.

const LIBS = new URL('./vendor/three/libs/', import.meta.url);
const memo = new Map();

function once(key, load) {
  if (!memo.has(key)) {
    memo.set(key, load().catch((error) => { memo.delete(key); throw error; }));
  }
  return memo.get(key);
}

const isBrowser = typeof document !== 'undefined';

async function readLibBytes(url) {
  if (url.protocol === 'file:') {
    const { readFile } = await import('node:fs/promises');
    const { fileURLToPath } = await import('node:url');
    return new Uint8Array(await readFile(fileURLToPath(url)));
  }
  const response = await fetch(url);
  if (!response.ok) throw new Error(`decoder: ${url.href} answered ${response.status}`);
  return new Uint8Array(await response.arrayBuffer());
}

async function loadClassic(url, globalName) {
  if (!isBrowser) {
    const { createRequire } = await import('node:module');
    const { fileURLToPath } = await import('node:url');
    const path = fileURLToPath(url);
    const source = new TextDecoder().decode(await readLibBytes(url));
    const shim = { exports: {} };
    new Function('module', 'exports', 'require', '__filename', '__dirname', source)(
      shim, shim.exports, createRequire(url), path, path.slice(0, path.lastIndexOf('/')),
    );
    return shim.exports;
  }
  if (!window[globalName]) {
    await new Promise((resolve, reject) => {
      const script = document.createElement('script');
      script.src = url.href;
      script.onload = resolve;
      script.onerror = () => reject(new Error(`decoder: ${url.href} failed to load`));
      document.head.append(script);
    });
  }
  return window[globalName];
}

// meshopt: resolves once the wasm is compiled. Decodes with decodeGltfBufferAsync(count, size, source, mode, filter).
export function loadMeshopt() {
  return once('meshopt', async () => {
    const { MeshoptDecoder } = await import(new URL('meshopt_decoder.module.js', LIBS).href);
    await MeshoptDecoder.ready;
    return MeshoptDecoder;
  });
}

// Draco (glTF build): a factory that resolves to the decoder module.
export function loadDraco() {
  return once('draco', async () => {
    const factory = await loadClassic(new URL('draco/draco_wasm_wrapper.js', LIBS), 'DracoDecoderModule');
    const wasmBinary = await readLibBytes(new URL('draco/draco_decoder.wasm', LIBS));
    return factory({ wasmBinary });
  });
}

// Basis (KTX2 transcoder): a factory that resolves to the module, initialised for transcoding.
export function loadBasis() {
  return once('basis', async () => {
    const factory = await loadClassic(new URL('basis/basis_transcoder.js', LIBS), 'BASIS');
    const wasmBinary = await readLibBytes(new URL('basis/basis_transcoder.wasm', LIBS));
    const basis = await factory({ wasmBinary });
    basis.initializeBasis();
    return basis;
  });
}

// KTX2 transcoding, synchronous once loadBasis() has resolved (pass its module as basis).
// format: 'bc7' | 'bc3' | 'etc2' | 'rgba'. transcodeKtx2 returns { width, height, levels: [{ width, height, data }] }.
const TARGET_ENUM = { bc7: 'cTFBC7_RGBA', bc3: 'cTFBC3_RGBA', etc2: 'cTFETC2_RGBA', rgba: 'cTFRGBA32' };

export function ktx2Size(basis, bytes) {
  const file = new basis.KTX2File(bytes);
  try {
    if (!file.isValid()) throw new Error('ktx2: invalid container');
    return { width: file.getWidth(), height: file.getHeight() };
  } finally {
    file.close();
    file.delete?.();
  }
}

export function transcodeKtx2(basis, bytes, format) {
  const target = basis.transcoder_texture_format[TARGET_ENUM[format]].value;
  const file = new basis.KTX2File(bytes);
  try {
    if (!file.isValid()) throw new Error('ktx2: invalid container');
    if (!file.startTranscoding()) throw new Error('ktx2: transcoding not started');
    const width = file.getWidth();
    const height = file.getHeight();
    const levels = [];
    for (let level = 0; level < Math.max(1, file.getLevels()); level++) {
      const data = new Uint8Array(file.getImageTranscodedSizeInBytes(level, 0, 0, target));
      if (!file.transcodeImage(data, level, 0, 0, target, 0, -1, -1)) throw new Error(`ktx2: level ${level} failed`);
      levels.push({ width: Math.max(1, width >> level), height: Math.max(1, height >> level), data });
    }
    return { width, height, levels };
  } finally {
    file.close();
    file.delete?.();
  }
}

// Draco-decodes one triangle mesh. Returns { numPoints, attributes: { [name]: { components, values: Float32Array } },
// indices: Uint32Array }. ids maps glTF attribute names to the Draco unique ids.
export async function decodeDracoMesh(bytes, ids) {
  const draco = await loadDraco();
  const decoder = new draco.Decoder();
  const buffer = new draco.DecoderBuffer();
  const geometry = new draco.Mesh();
  const scratch = [];
  try {
    buffer.Init(new Int8Array(bytes.buffer, bytes.byteOffset, bytes.byteLength), bytes.byteLength);
    if (decoder.GetEncodedGeometryType(buffer) !== draco.TRIANGULAR_MESH) throw new Error('draco: only triangle meshes');
    const status = decoder.DecodeBufferToMesh(buffer, geometry);
    if (!status.ok() || geometry.ptr === 0) throw new Error(`draco: ${status.error_msg()}`);
    const numPoints = geometry.num_points();
    const attributes = {};
    for (const [name, uniqueId] of Object.entries(ids)) {
      const attribute = decoder.GetAttributeByUniqueId(geometry, uniqueId);
      const components = attribute.num_components();
      const array = new draco.DracoFloat32Array();
      scratch.push(array);
      decoder.GetAttributeFloatForAllPoints(geometry, attribute, array);
      const values = new Float32Array(numPoints * components);
      for (let i = 0; i < values.length; i++) values[i] = array.GetValue(i);
      attributes[name] = { components, values };
    }
    const numFaces = geometry.num_faces();
    const face = new draco.DracoInt32Array();
    scratch.push(face);
    const indices = new Uint32Array(numFaces * 3);
    for (let f = 0; f < numFaces; f++) {
      decoder.GetFaceFromMesh(geometry, f, face);
      indices[f * 3] = face.GetValue(0);
      indices[f * 3 + 1] = face.GetValue(1);
      indices[f * 3 + 2] = face.GetValue(2);
    }
    return { numPoints, attributes, indices };
  } finally {
    for (const item of scratch) draco.destroy(item);
    draco.destroy(geometry);
    draco.destroy(buffer);
    draco.destroy(decoder);
  }
}
