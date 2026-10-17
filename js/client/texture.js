// Loads a GLB base color texture for WebGL. Browser-only: fetch, Blob and createImageBitmap.
// The glb.js reference { uri, mimeType, bytes, texCoord } is resolved against the GLB's own URL,
// so the shipped Textures/colormap.png is found next to the model file.

export async function loadBaseColorImage(ref, glbUrl) {
  if (!ref) return null;
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

// Uploads a loaded image to a WebGL texture; returns the WebGLTexture for sampling with TEXCOORD_0.
export function uploadTexture(gl, image) {
  const texture = gl.createTexture();
  gl.bindTexture(gl.TEXTURE_2D, texture);
  gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, false);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, image.source);
  gl.generateMipmap(gl.TEXTURE_2D);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.REPEAT);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.REPEAT);
  return texture;
}
