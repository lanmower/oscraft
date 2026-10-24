#version 300 es
precision highp float;
in vec3 vNormal; in vec3 vWorld;
// Ripple height: four travelling plane waves at 0, 45, 90 and 135 degrees with comparable weights, so the surface is
// isotropic and no single direction streaks across the river. A small drift along the river's flow is the only
// directional term.
float waveH(vec2 q, float t) {
  float h = 0.3 * sin(dot(q, vec2(1.0, 0.0)) * 2.6 + t * 1.1)
    + 0.3 * sin(dot(q, vec2(0.70711, 0.70711)) * 3.2 + t * 1.3)
    + 0.3 * sin(dot(q, vec2(0.0, 1.0)) * 4.2 + t * 1.5)
    + 0.3 * sin(dot(q, vec2(-0.70711, 0.70711)) * 5.0 + t * 1.7);
  return h + 0.1 * sin(dot(q, vec2(0.8, 0.6)) * 1.8 + t * 0.4);
}
#include "shadow.glsl"
#define GROUND_HEIGHT_BIAS 0.6
// Grass variant: the main grass frequency times 1.68 (finer), with 1.1 times the main warp amplitude.
#define VARIANT_UV_SCALE 1.68
#define VARIANT_WARP 1.1
uniform sampler2D uControl; uniform vec2 uMapSize; uniform float uUvScale; uniform float uBand;
uniform sampler2D uAlb0; uniform sampler2D uAlb1; uniform sampler2D uAlb2; uniform sampler2D uAlb3;
uniform sampler2D uDisp0; uniform sampler2D uDisp1; uniform sampler2D uDisp2; uniform sampler2D uDisp3;
uniform vec3 uShade; // ambient, diffuse, albedo gain
uniform vec3 uLayerTint[4]; // per-layer albedo tint (the sand layer takes the dirt tint)
uniform sampler2D uWaterMap; uniform vec3 uWaterTint; uniform float uTime; uniform vec3 uEye; // water: cell map, tint, clock, eye
uniform sampler2D uVariant; uniform float uVariantOn; // grass variant weight (R8 per cell), and 0 when the map marks no variant
out vec4 outColor;
// Per-layer UV warp, in tile units: a slow two-octave sine field over world space, with its own phase for each layer,
// so the four tilings drift apart instead of lining up. Peak offset is about 0.6 of one texture tile.
vec2 layerWarp(vec2 xz, float k) {
  vec2 a = vec2(sin(xz.y * 0.011 + k * 1.3 + 0.8 * sin(xz.x * 0.017 + k)), cos(xz.x * 0.013 - k * 0.7 + 0.6 * sin(xz.y * 0.021 - k)));
  vec2 b = vec2(sin(xz.x * 0.031 + xz.y * 0.007 + k * 2.1), cos(xz.y * 0.029 - xz.x * 0.005 + k * 1.9));
  return 0.6 * (0.7 * a + 0.3 * b);
}
vec3 bedAt(vec2 xz, float y, vec3 n0) {
  vec4 w = texture(uControl, xz / uMapSize);
  // Every layer weight is bilinear across the cell, the rock weight included: the control map already eases the rock weight
  // over several cells, so the rock-to-ground edge is a ramp that the height textures bias (see rw below), not a cell step.
  float cliffCell = w.w;
  // The ground band is widened (twice the uniform) so the blends run over a longer height range.
  float bandW = 2.0 * uBand;
  vec2 uv0 = xz * uUvScale + layerWarp(xz, 0.0), uv1 = xz * uUvScale + layerWarp(xz, 1.0);
  vec2 uv2 = xz * uUvScale + layerWarp(xz, 2.0), uv3 = xz * uUvScale + layerWarp(xz, 3.0);
  vec3 grassAlb = texture(uAlb0, uv0).rgb; float grassDisp = texture(uDisp0, uv0).r;
  // Grass variant (litter and vine cells): the grass albedo and height again, at the variant scale and warp on its own phase.
  // The variant map weights it, so cells it does not mark keep the main grass exactly.
  if (uVariantOn > 0.5) {
    vec2 uvV = xz * uUvScale * VARIANT_UV_SCALE + VARIANT_WARP * layerWarp(xz, 4.0);
    float vv = texture(uVariant, xz / uMapSize).r;
    grassAlb = mix(grassAlb, texture(uAlb0, uvV).rgb, vv);
    grassDisp = mix(grassDisp, texture(uDisp0, uvV).r, vv);
  }
  vec3 c0 = grassAlb * uLayerTint[0]; vec3 c1 = texture(uAlb1, uv1).rgb * uLayerTint[1];
  vec3 c2 = texture(uAlb2, uv2).rgb * uLayerTint[2];
  // The rock albedo is toned to the grass layer's mean brightness (rock024 averages 0.48, grass 0.34): 0.7 scale.
  vec3 c3 = texture(uAlb3, uv3).rgb * uLayerTint[3] * 0.7;
  vec4 d = vec4(grassDisp, texture(uDisp1, uv1).r, texture(uDisp2, uv2).r, texture(uDisp3, uv3).r);
  // Ground: grass, dirt and lane, blended by their own control weights and the band on their heights.
  // The ramp between two ground layers is biased by their height textures in the same units at every weight: a layer with a
  // higher height wins the ramp, so a stone or a root shows through the edge instead of the edge following the cell grid.
  vec3 g = w.xyz;
  vec3 score = g + (d.xyz - 0.5) * GROUND_HEIGHT_BIAS;
  float gm = max(score.x, max(score.y, score.z));
  vec3 gb = smoothstep(gm - bandW, gm, score) * g;
  float gs = gb.x + gb.y + gb.z;
  vec3 ground = gs > 1e-4 ? (gb.x * c0 + gb.y * c1 + gb.z * c2) / gs : (g.x * c0 + g.y * c1 + g.z * c2) / max(g.x + g.y + g.z, 1e-4);
  // Cliff: only cells whose rock channel is set take rock. The cutaway edge is biased by the rock024 height, and a flat
  // cell (rock channel 0) stays at 0 whatever its height, so rock never wins on flat ground.
  // The rock024 height biases the edge by at most +-0.2 of the cell's rock value, so a flat cell (rock channel 0) stays
  // below the threshold whatever its height, and a cliff cell (rock channel 1) stays above it.
  float rw = smoothstep(0.35, 0.65, cliffCell + (d.w - 0.5) * 0.4);
  vec3 base = mix(ground, c3, rw);
  vec3 light = normalize(vec3(0.4, 1.0, 0.6));
  float diffuse = max(dot(normalize(n0), light), 0.0);
  float lit = uShade.x + uShade.y * diffuse * sunShadow(vec3(xz.x, y, xz.y), n0);
  return base * uShade.z * lit;
}
// Ripple height from the rock height map when it is bound (uRippleOn 1), else the analytic waves. Each normal scale
// reads the map through its own warped position (q) and scrolls it slowly along the flow, so the scales stay out of step.
uniform sampler2D uRipple; uniform float uRippleOn;
float rippleH(vec2 q, float t, float k) {
  if (uRippleOn < 0.5) return waveH(q, t);
  vec2 uv = q * 0.03 + t * 0.002 * vec2(0.8 + 0.1 * k, 0.6 - 0.1 * k);
  return (texture(uRipple, uv).r - 0.5) * 2.0;
}
// Ripple sample position for one of the three normal scales: the scaled position plus its own slow warp (about 0.6 of
// a ripple cell at peak), with a different phase and frequency per scale, so the scales drift out of step.
vec2 ripplePos(vec2 x, float sc, float k) {
  vec2 warp = 0.6 * vec2(sin(x.y * (0.31 + 0.07 * k) + 1.7 * k + 0.8 * sin(x.x * (0.23 + 0.05 * k) + k)),
                         cos(x.x * (0.29 - 0.04 * k) - 2.3 * k + 0.7 * sin(x.y * (0.27 + 0.06 * k) - k)));
  return x * sc + warp;
}
// The ripple slope at q: the three ripple scales interfere, each sampled at its own frequency and weight, so the sum never
// repeats on one regular grid. The slope of each scale is divided back by its frequency, so the strengths stay comparable.
vec2 rippleSlope(vec2 q, float t) {
  vec2 slope = vec2(0.0);
  for (int k = 0; k < 3; k++) {
    float sc = k == 0 ? 3.4 : (k == 1 ? 5.0 : 6.8);
    float w = k == 0 ? 0.3 : (k == 1 ? 1.0 : 0.6);
    float e = 0.15 / sc;
    float hx = rippleH(ripplePos(q + vec2(e, 0.0), sc, float(k)), t, float(k)) - rippleH(ripplePos(q - vec2(e, 0.0), sc, float(k)), t, float(k));
    float hz = rippleH(ripplePos(q + vec2(0.0, e), sc, float(k)), t, float(k)) - rippleH(ripplePos(q - vec2(0.0, e), sc, float(k)), t, float(k));
    slope += vec2(hx, hz) / (2.0 * e) * 0.35 / sc * w;
  }
  return slope;
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
    near += texture(uWaterMap, (p + o * 2.5) / uMapSize).r;
    far += texture(uWaterMap, (p + o * 3.5) / uMapSize).r;
  }
  near /= 8.0; far /= 8.0;
  float depth = smoothstep(0.55, 1.0, far);
  float t = 48.0 * uTime;
  vec2 slope = rippleSlope(p, t);
  float soft = clamp(1.0 - (length(uEye - vWorld) - 20.0) / 80.0, 0.35, 1.0);
  // Fades the ripple amplitude and the refraction with the view distance: full strength near the camera, about 0.35 far off.
  slope *= soft;
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
  // Teal tint: stronger with depth, and more at grazing view angles (1 - dot(V, n)), so sharp angles read more teal.
  float graze = clamp(1.0 - dot(V, n), 0.0, 1.0);
  float teal = depth * (0.4 + 0.6 * graze);
  body = mix(body, vec3(0.1, 0.6, 0.58) * (uShade.x + uShade.y * max(dot(n, light), 0.0)), teal * 0.6);
  // Refraction: the bed is seen through the surface, so it is sampled at a small offset that follows the ripple slope at
  // half the spatial frequency of the surface ripples (p * 0.5), so the bending comes in broader waves. The gain doubles
  // to keep the bend the size it was. A small offset: enough to bend the bed under the ripples, too little to smear the ground.
  vec2 refr = -rippleSlope(p * 0.5, t) * soft * 0.24;
  vec3 bedR = bedAt(p + refr, vWorld.y, vNormal);
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
  // The water mask is warped on its own field (about 0.6 of a cell), so the shoreline cells do not repeat on a grid.
  vec2 pw = p + 0.6 * vec2(sin(p.y * 0.019 + 0.7 * sin(p.x * 0.023)), cos(p.x * 0.017 - 0.5 * sin(p.y * 0.029)));
  float wwW = texture(uWaterMap, pw / uMapSize).r;
  float wet = smoothstep(0.0, 1.0, near) * wwW;
  outColor = vec4(mix(bed, water, wet), 1.0);
}
