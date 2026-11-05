import { cameraDragAction, MIDDLE_BUTTON, RIGHT_BUTTON, pointerReleaseKinds } from "./camera_mouse.js";
import { orbitCamera, panCamera } from "./render3d.js";

const LEFT_BUTTON = 0;
const BACK_BUTTON = 3;
const FORWARD_BUTTON = 4;

const fresh = () => ({ target: [100, 0, 200], distance: 800, yaw: 0.5, pitch: 0.9, fovY: Math.PI / 3, bounds: null });
const same = (a, b) => a.length === b.length && a.every((v, i) => v === b[i]);

const rotated = orbitCamera(fresh(), 40, 20);
const panned = panCamera(fresh(), 30, 10, 600);

const cases = [
  ["middle drag rotates the view", cameraDragAction(MIDDLE_BUTTON, true), "rotate"],
  ["right drag pans the view", cameraDragAction(RIGHT_BUTTON, true), "pan"],
  ["left drag leaves the camera alone", cameraDragAction(LEFT_BUTTON, true), null],
  ["right click without movement does not pan", cameraDragAction(RIGHT_BUTTON, false), null],
  ["middle click without movement does not rotate", cameraDragAction(MIDDLE_BUTTON, false), null],
  ["back button drag is unbound", cameraDragAction(BACK_BUTTON, true), null],
  ["forward button drag is unbound", cameraDragAction(FORWARD_BUTTON, true), null],
  ["rotate changes yaw and pitch, target fixed", rotated.yaw !== 0.5 && rotated.pitch !== 0.9 && same(rotated.target, [100, 0, 200]), true],
  ["pan moves the target, yaw and pitch fixed", !same(panned.target, [100, 0, 200]) && panned.yaw === 0.5 && panned.pitch === 0.9, true],
  ["right camera drag forwards no pointer kinds", JSON.stringify(pointerReleaseKinds(RIGHT_BUTTON, true)), "[]"],
  ["middle camera drag forwards no pointer kinds", JSON.stringify(pointerReleaseKinds(MIDDLE_BUTTON, true)), "[]"],
  ["right click forwards down and up", JSON.stringify(pointerReleaseKinds(RIGHT_BUTTON, false)), '["down","up"]'],
  ["left release forwards up", JSON.stringify(pointerReleaseKinds(LEFT_BUTTON, true)), '["up"]'],
];

let failed = 0;
for (const [name, got, want] of cases) {
  const ok = got === want;
  if (!ok) failed += 1;
  console.log(`${ok ? "ok  " : "FAIL"} ${name}: got ${got} want ${want}`);
}
if (failed > 0) {
  console.log(`${failed} of ${cases.length} camera mouse cases failed`);
  process.exit(1);
}
console.log(`camera mouse ok (${cases.length} cases)`);
