import { createRequire as __gmCreateRequire } from 'node:module';
const require = __gmCreateRequire(process.cwd() + '/');
const fs = require("fs");
const src = fs.readFileSync("js/client/render3d.js", "utf8");
const grab = (name) => { const i = src.indexOf("export function " + name + "("); if (i < 0) throw new Error("missing " + name); const e = src.indexOf("\n}\n", i) + 3; return src.slice(i, e).replace(/export /g, ""); };
const names = ["createCamera", "clampTarget", "panCamera", "viewProjection"];
let mods;
try { mods = new Function(names.map(grab).join("\n") + "\nreturn { " + names.join(", ") + " };")(); }
catch (e) { console.log(JSON.stringify({ error: String(e).slice(0, 300), RESULT: "REFUSED" })); process.exit(0); }
const { createCamera, panCamera, viewProjection } = mods;
const project = (vp, p) => { const m = vp; const x = m[0]*p[0]+m[4]*p[1]+m[8]*p[2]+m[12]; const y = m[1]*p[0]+m[5]*p[1]+m[9]*p[2]+m[13]; const w = m[3]*p[0]+m[7]*p[1]+m[11]*p[2]+m[15]; return [x/w, y/w]; };
const res = {};
for (const pitch of [0.3, 0.9, 1.2]) {
  const cam = createCamera({ target: [0,0,0], yaw: 0.4, pitch, distance: 30, fovY: 0.9 });
  const vh = 600;
  const before = project(viewProjection(cam, 1280/720), cam.target);
  panCamera(cam, 0, 1, vh);
  const after = project(viewProjection(cam, 1280/720), cam.target);
  const dyNdc = after[1] - before[1];
  res["pitch" + pitch] = Math.abs(dyNdc) > 0 ? "ndc-shift " + dyNdc.toExponential(3) : "zero";
}
console.log(JSON.stringify(res));
