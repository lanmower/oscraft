
import { loadBasis, ktx2Size, transcodeKtx2 } from './decoders.js';
import { assetJson } from './pack_load.js';

const RASTER_URI = /\.(png|jpe?g)$/i;

let shippedKtx2 = null;
function shippedKtx2Paths() {
  shippedKtx2 ??= assetJson("ktx2").catch(() => []);
  return shippedKtx2;
}

export async function loadBaseColorImage(ref, glbUrl) {
  if (!ref) return null;
  if (ref.mimeType === 'image/ktx2') return loadKtx2Image(ref, glbUrl);
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
