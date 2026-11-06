import { keyNameOf, zoomKeyName } from "./key_names.js";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "../..");
const ui = JSON.parse(readFileSync(resolve(root, "scenarios/moba/moba_ui.json"), "utf8"));
const camera = ui.camera ?? {};
const zoomNames = [...(camera.zoomKeys?.increase ?? []), ...(camera.zoomKeys?.decrease ?? []), ...(camera.rotationLockKeys ?? [])];
if (zoomNames.some((name) => /^num[0-9]$/.test(name))) throw new Error("a camera binding relies on a Numpad name");
for (const key of ["q", "w", "1", "End", "ArrowLeft", " "]) {
  const e = { key, code: "" };
  if (keyNameOf(e) !== zoomKeyName(e)) throw new Error("normalizers disagree on " + key);
}
const numpad = { key: "End", code: "Numpad1" };
if (keyNameOf(numpad) === zoomKeyName(numpad)) throw new Error("the documented Numpad divergence disappeared");
const play = readFileSync(resolve(here, "play.js"), "utf8");
const zoom = readFileSync(resolve(here, "zoom_keys.js"), "utf8");
if (!play.includes("key_names.js") || !zoom.includes("key_names.js")) throw new Error("the normalizers are not shared");
console.log("key names ok (6 plain keys agree, Numpad divergence is latent and unbound)");
