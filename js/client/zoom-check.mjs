import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { packCameraRules } from "./camera_rules.js";
import { createZoomKeys, stepDistance } from "./zoom_keys.js";

const here = path.dirname(fileURLToPath(import.meta.url));
const ui = JSON.parse(fs.readFileSync(path.join(here, "../../scenarios/moba/moba_ui.json"), "utf8"));
const rules = packCameraRules(ui);
const tile = (units) => units / ui.camera.tileUnits;
const fail = (what) => { throw new Error(what); };

const camera = { distance: rules.distance, yawLocked: rules.rotationLock };
const renderer = { camera, setCamera(opts) { Object.assign(camera, opts); } };
const zoom = createZoomKeys({ renderer: () => renderer, rules });
const press = (key, extra = {}) => zoom.onKeyDown({ key, ctrlKey: false, metaKey: false, altKey: false, repeat: false, ...extra });

if (camera.yawLocked !== true) fail("rotation lock default is not on");
if (Math.abs(rules.step - tile(150)) > 1e-12) fail("zoom step is not 150 world units");
if (!press("PageDown") || Math.abs(camera.distance - (rules.distance + rules.step)) > 1e-12) fail("increase did not step by 150");
for (let i = 0; i < 20; i++) press("PageDown");
if (Math.abs(camera.distance - rules.max) > 1e-12) fail("increase does not stop at 3000");
for (let i = 0; i < 20; i++) press("PageUp");
if (Math.abs(camera.distance - rules.min) > 1e-12) fail("decrease does not stop at 1500");
if (stepDistance(rules.min, -1, rules) !== rules.min) fail("stepDistance leaves the lower bound");
if (press("PageUp", { ctrlKey: true })) fail("a modified key was taken");
if (press("q")) fail("an unbound key was taken");
if (press("ScrollLock") || camera.yawLocked !== true) fail("the pack binds no rotation lock key: 6.88g toggles it with the -w chat command");
const lockable = createZoomKeys({ renderer: () => renderer, rules: { ...rules, rotationLockKeys: ["scrolllock"] } });
if (!lockable.onKeyDown({ key: "ScrollLock" }) || camera.yawLocked !== false) fail("a pack lock key did not free the rotation");
if (!lockable.onKeyDown({ key: "ScrollLock" }) || camera.yawLocked !== true) fail("a pack lock key did not lock the rotation");
console.log("zoom keys ok");
