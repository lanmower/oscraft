uniform mat4 uLightVP; uniform sampler2D uShadow; uniform float uShadowOn; uniform float uShadowTexel;
float sunShadow(vec3 w, vec3 n) {
  if (uShadowOn < 0.5) return 1.0;
  vec4 p = uLightVP * vec4(w, 1.0);
  vec3 s = p.xyz / p.w * 0.5 + 0.5;
  if (s.x < 0.0 || s.y < 0.0 || s.x > 1.0 || s.y > 1.0 || s.z > 1.0) return 1.0;
  vec3 sun = vec3(0.4, 1.0, 0.6) / length(vec3(0.4, 1.0, 0.6));
  float facing = clamp(dot(normalize(n), sun), 0.0, 1.0);
  float bias = max(0.0009 * (1.0 - facing), 0.00025);
  float k = 4.0 * uShadowTexel;
  vec2 t = s.xy / k - 0.5;
  vec2 f = fract(t);
  vec2 c = (floor(t) + 0.5) * k;
  float r = s.z - bias;
  float a = (r > texture(uShadow, c).r) ? 0.0 : 1.0;
  float b = (r > texture(uShadow, c + vec2(k, 0.0)).r) ? 0.0 : 1.0;
  float e = (r > texture(uShadow, c + vec2(0.0, k)).r) ? 0.0 : 1.0;
  float g = (r > texture(uShadow, c + vec2(k)).r) ? 0.0 : 1.0;
  return mix(mix(a, b, f.x), mix(e, g, f.x), f.y);
}
