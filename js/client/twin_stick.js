import { heroDirection } from "./hero_view_lock.js";

export const STICK_DEADZONE = 0.2;
export const AIM_CELLS = 8;
export const PAD_CAST_SLOTS = [0, 1, 2, 3];

function shape(value) {
  const magnitude = Math.abs(value);
  if (magnitude < STICK_DEADZONE) return 0;
  return Math.sign(value) * (magnitude - STICK_DEADZONE) / (1 - STICK_DEADZONE);
}

function stick(x, y) {
  const sx = shape(x);
  const sy = shape(y);
  return sx === 0 && sy === 0 ? null : { x: sx, y: sy };
}

export function twinStick(pads) {
  const pad = Array.from(pads || []).find((p) => p && p.connected);
  if (!pad) return { move: null, aim: null, pressed: [] };
  const [lx = 0, ly = 0, rx = 0, ry = 0] = pad.axes;
  return {
    move: stick(lx, -ly),
    aim: stick(rx, -ry),
    pressed: pad.buttons.map((button) => Boolean(button && button.pressed)),
  };
}

export function pressedDownEdges(previous, pressed) {
  const downs = [];
  pressed.forEach((on, index) => {
    if (on && !previous[index]) downs.push(index);
  });
  return downs;
}

export function aimPoint(hero, aim, yaw) {
  const direction = heroDirection([], yaw, aim);
  if (!direction) return { x: hero.x, z: hero.z };
  return { x: hero.x + direction.x * AIM_CELLS, z: hero.z + direction.z * AIM_CELLS };
}
