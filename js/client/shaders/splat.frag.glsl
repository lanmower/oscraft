#version 300 es
precision highp float;
in vec3 vNormal; in vec3 vWorld;
// Ripple height: four travelling plane waves at 0, 45, 90 and 135 degrees with comparable weights, so the surface is
// isotropic and no single direction streaks across the river. A small drift along the river's flow is the only
// directional term.
float waveH(vec2 q, float t) {
  float h = 0.3 * sin(dot(q, vec2(1.0, 0.0)) * 5.2 + t * 1.1)
    + 0.3 * sin(dot(q, vec2(0.70711, 0.70711)) * 6.4 + t * 1.3)
    + 0.3 * sin(dot(q, vec2(0.0, 1.0)) * 8.4 + t * 1.5)
    + 0.3 * sin(dot(q, vec2(-0.70711, 0.70711)) * 10.0 + t * 1.7);
  return h + 0.1 * sin(dot(q, vec2(0.8, 0.6)) * 3.6 + t * 0.4);
}
#include "shadow.glsl"
uniform sampler2D uControl; uniform vec2 uMapSize; uniform float uUvScale; uniform float uBand;
uniform sampler2D uAlb0; uniform sampler2D uAlb1; uniform sampler2D uAlb2; uniform sampler2D uAlb3;
uniform sampler2D uDisp0; uniform sampler2D uDisp1; uniform sampler2D uDisp2; uniform sampler2D uDisp3;
uniform vec3 uShade; // ambient, diffuse, albedo gain
uniform vec3 uLayerTint[4]; // per-layer albedo tint (the sand layer takes the dirt tint)
uniform sampler2D uWaterMap; uniform vec3 uWaterTint; uniform float uTime; uniform vec3 uEye; // water: cell map, tint, clock, eye
out vec4 outColor;
vec3 bedAt(vec2 xz, float y, vec3 n0) {
  vec4 w = texture(uControl, xz / uMapSize);
  // Cliff (rock) weight is read per cell, so a cliff edge stays hard while the ground layers blend.
  w.w = texelFetch(uControl, ivec2(floor(xz / uMapSize * vec2(textureSize(uControl, 0)))), 0).w;
  vec2 uv = xz * uUvScale;
  vec3 c0 = texture(uAlb0, uv).rgb * uLayerTint[0]; vec3 c1 = texture(uAlb1, uv).rgb * uLayerTint[1];
  vec3 c2 = texture(uAlb2, uv).rgb * uLayerTint[2]; vec3 c3 = texture(uAlb3, uv).rgb * uLayerTint[3];
  vec4 d = vec4(texture(uDisp0, uv).r, texture(uDisp1, uv).r, texture(uDisp2, uv).r, texture(uDisp3, uv).r);
  vec4 h = w * (d + 0.5);
  float m = max(max(h.x, h.y), max(h.z, h.w));
  vec4 b = max(h - (m - uBand), 0.0) * w;
  float s = b.x + b.y + b.z + b.w;
  vec3 mixed = (b.x * c0 + b.y * c1 + b.z * c2 + b.w * c3) / max(s, 1e-4);
  vec3 plain = (w.x * c0 + w.y * c1 + w.z * c2 + w.w * c3) / max(w.x + w.y + w.z + w.w, 1e-4);
  vec3 base = s > 1e-4 ? mixed : plain;
  vec3 light = normalize(vec3(0.4, 1.0, 0.6));
  float diffuse = max(dot(normalize(n0), light), 0.0);
  float lit = uShade.x + uShade.y * diffuse * sunShadow(vec3(xz.x, y, xz.y), n0);
  return base * uShade.z * lit;
}
void main() {
  vec2 p = vWorld.xz;
  vec3 bed = bedAt(p, vWorld.y, vNormal);
  float ww = texture(uWaterMap, p / uMapSize).r;
  if (ww <= 0.0) { outColor = vec4(bed, 1.0); return; }
  // Procedural water (ported from spoint's water-tsl.js: the surface, ripples and fresnel; the planet geometry is dropped).
  // Depth is how much of the neighbourhood is water: 1 well inside the river, lower towards its banks.
  float near = 0.0; float far = 0.0;
  for (int k = 0; k < 8; k++) {
    float a = float(k) * 0.7853982;
    vec2 o = vec2(cos(a), sin(a));
    near += texture(uWaterMap, (p + o * 1.2) / uMapSize).r;
    far += texture(uWaterMap, (p + o * 3.5) / uMapSize).r;
  }
  near /= 8.0; far /= 8.0;
  float depth = smoothstep(0.55, 1.0, far);
  float t = 3.0 * uTime;
  // Three ripple scales interfere: each samples the wave pattern at its own frequency and weight, so the sum never repeats
  // on one regular grid. The slope of each scale is divided back by its frequency, so the strengths stay comparable.
  vec2 slope = vec2(0.0);
  for (int k = 0; k < 3; k++) {
    float sc = k == 0 ? 3.4 : (k == 1 ? 5.0 : 6.8);
    float w = k == 0 ? 0.3 : (k == 1 ? 1.0 : 0.6);
    float e = 0.15 / sc;
    float hx = waveH((p + vec2(e, 0.0)) * sc, t) - waveH((p - vec2(e, 0.0)) * sc, t);
    float hz = waveH((p + vec2(0.0, e)) * sc, t) - waveH((p - vec2(0.0, e)) * sc, t);
    slope += vec2(hx, hz) / (2.0 * e) * 0.35 / sc * w;
  }
  vec3 n = normalize(vec3(-slope.x, 1.0, -slope.y));
  vec3 light = normalize(vec3(0.4, 1.0, 0.6));
  vec3 V = normalize(uEye - vec3(p.x, vWorld.y, p.y));
  float fres = 0.02 + 0.98 * pow(1.0 - max(dot(n, V), 0.0), 5.0);
  vec3 R = reflect(-V, n);
  vec3 sky = mix(vec3(0.62, 0.78, 0.86), vec3(0.22, 0.46, 0.66), clamp(R.y, 0.0, 1.0));
  float glint = pow(max(dot(R, light), 0.0), 140.0) * 1.2;
  vec3 shallow = vec3(0.34, 0.62, 0.56) * uWaterTint;
  vec3 deep = vec3(0.04, 0.22, 0.30) * uWaterTint;
  vec3 body = mix(shallow, deep, depth) * (uShade.x + uShade.y * max(dot(n, light), 0.0));
  // Refraction: the bed is seen through the surface, so it is sampled at a small offset driven by the surface normal
  // (0.3 world units: enough to bend the bed under the ripples, too little to smear the ground texture). The caustic
  // light is laid on that refracted bed as high-contrast bands, so the bending reads as light through the water.
  vec2 refr = n.xz * 0.01875;
  vec3 bedR = bedAt(p + refr, vWorld.y, vNormal);
  // Caustic from the isotropic ripple sum, not one plane wave, so no single direction is traced across the pool.
  float caustic = 0.5 + 0.5 * clamp(waveH((p + refr) * 6.0, t) / 1.1, -1.0, 1.0);
  caustic = smoothstep(0.6, 0.95, caustic);
  bedR *= 1.0 + 0.35 * caustic;
  // The water is transparent everywhere: the riverbed shows through the shallows and, less, the deep water.
  float alpha = mix(0.18, 0.38, depth);
  vec3 water = mix(bedR, body, alpha);
  // The sky reflection and the sun glint are the shine of deep water only: they fade out with depth, so the shallow shore
  // tiles stay matte. SPECULAR scales both: at 0.1 the shine is a tenth of its full strength.
  const float SPECULAR = 0.1;
  water = mix(water, sky, fres * 0.8 * depth * SPECULAR);
  water += vec3(1.0, 0.97, 0.9) * glint * depth * SPECULAR;
  // Wet only where the neighbourhood is all water: a shore cell with land beside it stays dry ground, so the water does
  // not spill over the edge tiles.
  float wet = smoothstep(0.97, 1.0, near) * ww;
  outColor = vec4(mix(bed, water, wet), 1.0);
}
