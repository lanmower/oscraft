// Loads a GLB base color texture for WebGL. Browser-only: fetch, Blob and createImageBitmap.
// The glb.js reference { uri, mimeType, bytes, texCoord } is resolved against the GLB's own URL,
// so the shipped Textures/colormap.png is found next to the model file.
// PNG, JPEG and WebP decode through createImageBitmap. KTX2 (KHR_texture_basisu) decodes through the vendored
// Basis transcoder and uploads compressed mip levels when the device has a compressed format, else RGBA.

import { loadBasis, ktx2Size, transcodeKtx2 } from './decoders.js';
import { assetJson } from './pack_load.js';

const RASTER_URI = /\.(png|jpe?g)$/i;

// The KTX2 files the build ships, read once from the asset catalogue. A raster whose sibling is not listed never
// requests it, so no 404 is logged and the raster is used directly.
let shippedKtx2 = null;
function shippedKtx2Paths() {
  shippedKtx2 ??= assetJson('catalogue')
    .then((catalogue) => (catalogue.items || []).map((item) => item.path).filter((p) => typeof p === 'string' && p.endsWith('.ktx2')))
    .catch(() => []);
  return shippedKtx2;
}

export async function loadBaseColorImage(ref, glbUrl) {
  if (!ref) return null;
  if (ref.mimeType === 'image/ktx2') return loadKtx2Image(ref, glbUrl);
  // A PNG or JPEG uri reads its KTX2 sibling first (same name, .ktx2) when the build ships it; the raster is the fallback.
  if (!ref.bytes && ref.uri && RASTER_URI.test(ref.uri)) {
    const siblingPath = new URL(ref.uri.replace(RASTER_URI, '.ktx2'), glbUrl ?? location.href).pathname;
    if ((await shippedKtx2Paths()).some((path) => siblingPath.endsWith('/' + path))) {
      const sibling = await loadKtx2Image({ uri: ref.uri.replace(RASTER_URI, '.ktx2'), texCoord: ref.texCoord }, glbUrl).catch(() => null);
      if (sibling) return sibling;
    }
  }
  const blob = ref.bytes
    ? new Blob([ref.bytes], { type: ref.mimeType ?? 'image/png' })
    : await fetchImageBlob(ref.uri, glbUrl);
  const bitmap = await createImageBitmap(blob);
  return { source: bitmap, width: bitmap.width, height: bitmap.height, texCoord: ref.texCoord ?? 0 };
}

async function fetchImageBlob(uri, glbUrl) {
  const url = glbUrl ? new URL(uri, glbUrl).href : uri;
  const response = await fetch(url);
  if (!response.ok) throw new Error(`texture: ${url} answered ${response.status}`);
  return response.blob();
}

// The transcoder is loaded here, so the WebGL upload stays synchronous. `source` is an RGBA ImageData built on
// demand for renderers that take a decoded image (the TSL path); the WebGL path never reads it.
async function loadKtx2Image(ref, glbUrl) {
  const bytes = ref.bytes ?? new Uint8Array(await (await fetchImageBlob(ref.uri, glbUrl)).arrayBuffer());
  const basis = await loadBasis();
  const { width, height } = ktx2Size(basis, bytes);
  let source = null;
  return {
    ktx2: { basis, bytes },
    width,
    height,
    texCoord: ref.texCoord ?? 0,
    get source() {
      source ??= new ImageData(new Uint8ClampedArray(transcodeKtx2(basis, bytes, 'rgba').levels[0].data.buffer), width, height);
      return source;
    },
  };
}

// Uploads a loaded image to a WebGL texture; returns the WebGLTexture for sampling with TEXCOORD_0.
export function uploadTexture(gl, image) {
  if (image.ktx2) return uploadKtx2(gl, image);
  const texture = gl.createTexture();
  gl.bindTexture(gl.TEXTURE_2D, texture);
  gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, false);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, image.source);
  gl.generateMipmap(gl.TEXTURE_2D);
  setSampling(gl, gl.LINEAR_MIPMAP_LINEAR);
  return texture;
}

// The GPU format a KTX2 image transcodes to: BC7, then BC3, then ETC2 (core in WebGL2), else RGBA8.
export function ktx2Target(gl) {
  if (gl.getExtension('EXT_texture_compression_bptc')) return { format: 'bc7', internalFormat: 0x8e8c };
  if (gl.getExtension('WEBGL_compressed_texture_s3tc')) return { format: 'bc3', internalFormat: 0x83f3 };
  const webgl2 = typeof WebGL2RenderingContext !== 'undefined' && gl instanceof WebGL2RenderingContext;
  if (webgl2 || gl.getExtension('WEBGL_compressed_texture_etc')) return { format: 'etc2', internalFormat: 0x9278 };
  return { format: 'rgba', internalFormat: gl.RGBA };
}

function uploadKtx2(gl, image) {
  const { basis, bytes } = image.ktx2;
  const { format, internalFormat } = ktx2Target(gl);
  const { levels } = transcodeKtx2(basis, bytes, format);
  const texture = gl.createTexture();
  gl.bindTexture(gl.TEXTURE_2D, texture);
  gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, false);
  levels.forEach((level, index) => {
    if (format === 'rgba') gl.texImage2D(gl.TEXTURE_2D, index, internalFormat, level.width, level.height, 0, gl.RGBA, gl.UNSIGNED_BYTE, level.data);
    else gl.compressedTexImage2D(gl.TEXTURE_2D, index, internalFormat, level.width, level.height, 0, level.data);
  });
  setSampling(gl, levels.length > 1 ? gl.LINEAR_MIPMAP_LINEAR : gl.LINEAR);
  return texture;
}

function setSampling(gl, minFilter) {
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, minFilter);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.REPEAT);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.REPEAT);
}
