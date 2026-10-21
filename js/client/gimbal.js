// Debug gimbals for the hand pose. Active only with ?gimbal in the page URL: a panel of sliders sets an extra rotation (degrees,
// XYZ) on each hand, applied after the sampled pose the same way the retarget's hand roll is. The values are read back as
// the copy button's text, so a chosen offset can go straight into anim.js.

const HAND = { leftHand: /^left\s*hand$/i, rightHand: /^right\s*hand$/i };
const DEG = Math.PI / 180;
export const gimbal = { enabled: false, leftHand: [0, 0, 0], rightHand: [0, 0, 0] };

const quatMul = (a, b) => [
  a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
  a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
  a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
  a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
];
// XYZ euler (degrees) to a quaternion, applied in X then Y then Z.
export function quatFromEuler([x, y, z]) {
  const qx = [Math.sin(x * DEG / 2), 0, 0, Math.cos(x * DEG / 2)];
  const qy = [0, Math.sin(y * DEG / 2), 0, Math.cos(y * DEG / 2)];
  const qz = [0, 0, Math.sin(z * DEG / 2), Math.cos(z * DEG / 2)];
  return quatMul(quatMul(qx, qy), qz);
}

// Node indices of the two hands, found by name once per rig.
export function handNodes(nodes) {
  const out = {};
  for (const side of Object.keys(HAND)) out[side] = nodes.findIndex((n) => n.name && HAND[side].test(n.name));
  return out;
}

// The pose with the gimbal offsets applied to the hand nodes. Returns the pose unchanged when the gimbal is off or zero.
export function gimbalPose(pose, nodes, hands) {
  if (!gimbal.enabled) return pose;
  const out = new Map(pose);
  for (const side of ['leftHand', 'rightHand']) {
    const offset = gimbal[side];
    const i = hands[side];
    if (i < 0 || (offset[0] === 0 && offset[1] === 0 && offset[2] === 0)) continue;
    const base = pose.get(i);
    const rotation = base ? base.rotation : nodes[i].rotation ?? [0, 0, 0, 1];
    out.set(i, { translation: base?.translation ?? nodes[i].translation, rotation: quatMul(rotation, quatFromEuler(offset)), scale: base?.scale });
  }
  return out;
}

// The sliders, mounted once when the page asks for them. Each change updates gimbal[side] at once.
export function mountGimbalPanel() {
  gimbal.enabled = true;
  const panel = document.createElement('div');
  panel.id = 'gimbal';
  panel.style.cssText = 'position:fixed;left:8px;bottom:150px;z-index:9;background:rgba(0,0,0,.7);color:#fff;font:12px monospace;padding:8px;border-radius:6px;width:260px';
  const rows = [];
  for (const side of ['leftHand', 'rightHand']) {
    const head = document.createElement('div');
    head.textContent = side;
    panel.appendChild(head);
    ['x', 'y', 'z'].forEach((axis, k) => {
      const row = document.createElement('label');
      row.style.display = 'block';
      const value = document.createElement('span');
      value.textContent = '0';
      const input = document.createElement('input');
      input.type = 'range';
      input.min = -180;
      input.max = 180;
      input.step = 1;
      input.value = 0;
      input.style.width = '170px';
      input.oninput = () => {
        gimbal[side][k] = Number(input.value);
        value.textContent = input.value;
        output.textContent = JSON.stringify({ leftHand: gimbal.leftHand, rightHand: gimbal.rightHand });
      };
      row.append(`${axis} `, input, ' ', value);
      panel.appendChild(row);
      rows.push(input);
    });
  }
  const output = document.createElement('div');
  output.style.cssText = 'margin-top:6px;word-break:break-all;user-select:all';
  output.textContent = '{"leftHand":[0,0,0],"rightHand":[0,0,0]}';
  panel.appendChild(output);
  document.body.appendChild(panel);
  window.__gimbal = gimbal;
}
