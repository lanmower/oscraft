#version 300 es
in vec3 aPos; in vec3 aNormal; in vec2 aUv; in vec4 aJoints; in vec4 aWeights; in mat4 aInst;
uniform mat4 uModel; uniform mat3 uNormal; uniform mat4 uViewProj;
uniform bool uSkinned; uniform bool uInstanced; uniform mat4 uJoints[__MAX_JOINTS__];
out vec3 vNormal; out vec2 vUv; out vec3 vWorld;
void main() {
  mat4 skin = mat4(1.0);
  mat3 skinNormal = mat3(1.0);
  if (uSkinned) {
    skin = aWeights.x * uJoints[int(aJoints.x)] + aWeights.y * uJoints[int(aJoints.y)]
      + aWeights.z * uJoints[int(aJoints.z)] + aWeights.w * uJoints[int(aJoints.w)];
    skinNormal = mat3(skin);
  }
  mat4 model = uInstanced ? aInst : uModel;
  mat3 normalBasis = uInstanced ? mat3(aInst) : uNormal;
  vec4 world = model * skin * vec4(aPos, 1.0);
  vNormal = normalBasis * skinNormal * aNormal; vUv = aUv; vWorld = world.xyz;
  gl_Position = uViewProj * world;
}
