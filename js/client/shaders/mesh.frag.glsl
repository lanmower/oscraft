#version 300 es
precision mediump float;
in vec3 vNormal; in vec2 vUv; in vec3 vWorld;
#include "shadow.glsl"
uniform vec3 uColor; uniform vec3 uTint; uniform bool uUseTex; uniform sampler2D uTex; uniform vec3 uShade; // ambient, diffuse, gain
uniform float uAlpha; uniform bool uGlow; // optional additive blend mode (fx.js)
out vec4 outColor;
void main() {
  vec3 light = normalize(vec3(0.4, 1.0, 0.6));
  float diffuse = max(dot(normalize(vNormal), light), 0.0);
  vec3 base = (uUseTex ? texture(uTex, vUv).rgb : uColor) * uTint;
  outColor = uGlow ? vec4(base, uAlpha) : vec4(base * uShade.z * (uShade.x + uShade.y * diffuse * sunShadow(vWorld, vNormal)), 1.0);
}
