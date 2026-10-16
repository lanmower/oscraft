// Browser client: the Rust sim (wasm) steps the map; three.js draws the terrain kit and units.
import init, { Game } from "./pkg/web.js";
import * as THREE from "three";
import { GLTFLoader } from "https://unpkg.com/three@0.160.0/examples/jsm/loaders/GLTFLoader.js";

// The map grid from data/map.star: grid(32, 24), one tile per cell.
const W = 32;
const H = 24;
const BACKGROUND = 0x1d252c;
const PLAYER_COLORS = [0x3a7bd5, 0xd5543a];
const MIN_ZOOM = 0.5;
const MAX_ZOOM = 4;
const PICK_RADIUS = 1.0;
const FRAME_WINDOW = 120;

// Scales a model so its largest side is `target` world units, then centres it on the origin with its base on y = 0.
function normalize(model, target) {
  const wrap = new THREE.Group();
  wrap.add(model);
  const size = new THREE.Box3().setFromObject(wrap).getSize(new THREE.Vector3());
  wrap.scale.setScalar(target / (Math.max(size.x, size.y, size.z) || 1));
  wrap.updateMatrixWorld(true);
  const box = new THREE.Box3().setFromObject(wrap);
  const c = box.getCenter(new THREE.Vector3());
  wrap.position.set(-c.x, -box.min.y, -c.z);
  return wrap;
}

// Replaces skinned meshes with plain meshes in their bind pose. Skinned vertices follow their bones, not the
// holder's transform, so every unit would collapse to the bones' place at the origin.
function unskin(root) {
  const skinned = [];
  root.traverse((o) => {
    if (o.isSkinnedMesh) skinned.push(o);
  });
  for (const sm of skinned) {
    const mesh = new THREE.Mesh(sm.geometry, sm.material);
    mesh.name = sm.name;
    mesh.position.copy(sm.position);
    mesh.quaternion.copy(sm.quaternion);
    mesh.scale.copy(sm.scale);
    sm.parent.add(mesh);
    sm.parent.remove(sm);
  }
}

// Gives every mesh under root one material.
function paint(root, material) {
  root.traverse((o) => {
    if (o.isMesh) o.material = material;
  });
}

async function main() {
  await init();
  const image = new Uint8Array(await (await fetch("map.bin")).arrayBuffer());
  let game = new Game(image);

  const renderer = new THREE.WebGLRenderer({ antialias: true, preserveDrawingBuffer: true });
  renderer.setPixelRatio(window.devicePixelRatio);
  document.body.appendChild(renderer.domElement);

  const scene = new THREE.Scene();
  scene.background = new THREE.Color(BACKGROUND);
  scene.add(new THREE.HemisphereLight(0xffffff, 0x445566, 1.5));
  const sun = new THREE.DirectionalLight(0xffffff, 1.0);
  sun.position.set(10, 30, 10);
  scene.add(sun);

  // Top-down orthographic view; screen up is map north (-z), so the grid is not seen edge-on.
  const camera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0.1, 200);
  camera.up.set(0, 0, -1);
  const view = { cx: W / 2, cz: H / 2, zoom: 1 };
  let halfW = 1;
  let halfH = 1;

  function applyView() {
    camera.position.set(view.cx, 50, view.cz);
    camera.lookAt(view.cx, 0, view.cz);
    camera.left = -halfW / view.zoom;
    camera.right = halfW / view.zoom;
    camera.top = halfH / view.zoom;
    camera.bottom = -halfH / view.zoom;
    camera.updateProjectionMatrix();
  }

  function resize() {
    const aspect = window.innerWidth / window.innerHeight;
    halfW = Math.max(W / 2 + 1, (H / 2 + 1) * aspect);
    halfH = halfW / aspect;
    renderer.setSize(window.innerWidth, window.innerHeight);
    applyView();
  }
  window.addEventListener("resize", resize);
  resize();

  // Pan by dragging, zoom with the wheel, select a unit by clicking it.
  const canvas = renderer.domElement;
  const drag = { active: false, moved: false, x: 0, y: 0 };
  // The selection is the sim's unit id, so a unit that dies does not hand its selection to another unit.
  let selectedId = -1;
  let ids = [];
  canvas.addEventListener("contextmenu", (e) => e.preventDefault());

  function groundPoint(clientX, clientY) {
    const rect = canvas.getBoundingClientRect();
    const ndc = new THREE.Vector2(((clientX - rect.left) / rect.width) * 2 - 1, -((clientY - rect.top) / rect.height) * 2 + 1);
    const ray = new THREE.Raycaster();
    ray.setFromCamera(ndc, camera);
    const hit = new THREE.Vector3();
    return ray.ray.intersectPlane(new THREE.Plane(new THREE.Vector3(0, 1, 0), 0), hit) ? hit : null;
  }

  canvas.addEventListener("pointerdown", (e) => {
    drag.active = true;
    drag.moved = false;
    drag.x = e.clientX;
    drag.y = e.clientY;
    canvas.setPointerCapture(e.pointerId);
  });
  canvas.addEventListener("pointermove", (e) => {
    if (!drag.active) return;
    const dx = e.clientX - drag.x;
    const dy = e.clientY - drag.y;
    if (!drag.moved && Math.hypot(dx, dy) < 4) return;
    drag.moved = true;
    // Content follows the pointer: the view moves the opposite way in world units.
    const perPxX = (2 * halfW) / view.zoom / canvas.clientWidth;
    const perPxY = (2 * halfH) / view.zoom / canvas.clientHeight;
    view.cx -= dx * perPxX;
    view.cz -= dy * perPxY;
    drag.x = e.clientX;
    drag.y = e.clientY;
    applyView();
  });
  canvas.addEventListener("pointerup", (e) => {
    const wasClick = drag.active && !drag.moved;
    drag.active = false;
    if (!wasClick) return;
    if (e.button === 2) {
      // Right-click: order the selected unit to walk to the ground point under the pointer.
      const target = groundPoint(e.clientX, e.clientY);
      if (target && selectedId >= 0) {
        const ok = game.order_move(selectedId, target.x, target.z);
        document.body.dataset.order = ok ? "accepted" : "refused";
      }
      return;
    }
    const p = groundPoint(e.clientX, e.clientY);
    selectedId = -1;
    if (p) {
      let best = PICK_RADIUS;
      pool.forEach((entry, i) => {
        if (!entry.obj.visible) return;
        const d = Math.hypot(entry.obj.position.x - p.x, entry.obj.position.z - p.z);
        if (d < best) {
          best = d;
          selectedId = ids[i];
        }
      });
    }
    document.body.dataset.selected = String(selectedId);
  });
  canvas.addEventListener(
    "wheel",
    (e) => {
      e.preventDefault();
      view.zoom = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, view.zoom * (e.deltaY < 0 ? 1.1 : 1 / 1.1)));
      applyView();
    },
    { passive: false },
  );

  const loader = new GLTFLoader();
  const colormap = await new THREE.TextureLoader().loadAsync("assets/units/Textures/colormap.png");
  colormap.colorSpace = THREE.SRGBColorSpace;
  const terrainGltf = await loader.loadAsync("assets/terrain/terrain.glb");
  const tile = normalize(terrainGltf.scene, 1);
  // The kit's UVs sample the shared colour map, the same palette the unit model uses.
  paint(tile, new THREE.MeshBasicMaterial({ map: colormap }));
  for (let y = 0; y < H; y++) {
    for (let x = 0; x < W; x++) {
      const t = tile.clone(true);
      t.position.add(new THREE.Vector3(x + 0.5, 0, y + 0.5));
      scene.add(t);
    }
  }

  const unitGltf = await loader.loadAsync("assets/units/character-male-c.glb");
  unskin(unitGltf.scene);
  const unitModel = normalize(unitGltf.scene, 0.8);
  const playerMaterials = PLAYER_COLORS.map((color) => new THREE.MeshLambertMaterial({ color }));
  const pool = [];

  // Rolling p99 over the last FRAME_WINDOW frames of drawing work, measured on the page's clock.
  const frameTimes = [];
  function recordFrame(ms) {
    frameTimes.push(ms);
    if (frameTimes.length > FRAME_WINDOW) frameTimes.shift();
    const sorted = [...frameTimes].sort((a, b) => a - b);
    document.body.dataset.frameP99Ms = sorted[Math.ceil(0.99 * sorted.length) - 1].toFixed(2);
  }

  function frame() {
    const started = performance.now();
    try {
      drawFrame();
    } catch (e) {
      failPage("frame", e);
      return;
    }
    recordFrame(performance.now() - started);
    if (document.body.dataset.loadMs === undefined) document.body.dataset.loadMs = String(Math.round(performance.now()));
    requestAnimationFrame(frame);
  }

  function drawFrame() {
    game.tick();
    const data = game.units();
    const n = data.length / 3;
    ids = game.unit_ids();
    const selected = ids.indexOf(selectedId);
    if (selected < 0) selectedId = -1;
    while (pool.length < n) {
      // The holder carries the sim position; the model keeps the offset normalize() gave it.
      const holder = new THREE.Group();
      holder.add(unitModel.clone(true));
      scene.add(holder);
      pool.push({ obj: holder, player: -1 });
    }
    for (let i = 0; i < pool.length; i++) {
      const entry = pool[i];
      entry.obj.visible = i < n;
      if (i >= n) continue;
      entry.obj.position.set(data[3 * i], 0, data[3 * i + 1]);
      entry.obj.scale.setScalar(i === selected ? 1.4 : 1);
      const p = data[3 * i + 2];
      if (entry.player !== p) {
        paint(entry.obj, playerMaterials[p] || playerMaterials[0]);
        entry.player = p;
      }
    }
    renderer.render(scene, camera);

    // The pixel at the map centre: the check uses it to confirm the terrain is drawn, not a blank canvas.
    const gl = renderer.getContext();
    const px = new Uint8Array(4);
    gl.readPixels(Math.floor(gl.drawingBufferWidth / 2), Math.floor(gl.drawingBufferHeight / 2), 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, px);
    document.body.dataset.center = `${px[0]},${px[1]},${px[2]}`;
    document.body.dataset.units = String(n);
    if (selected >= 0 && selected < n) {
      document.body.dataset.selectedPos = `${data[3 * selected].toFixed(2)},${data[3 * selected + 1].toFixed(2)}`;
    }
    document.body.dataset.selectedId = String(selectedId);
    document.body.dataset.ticks = String(game.ticks());
    document.body.dataset.sample = n > 0 ? [data[0], data[1], data[2]].join(",") : "none";
    document.body.dataset.view = `${view.cx.toFixed(2)},${view.cz.toFixed(2)},${view.zoom.toFixed(2)}`;
  }
  frame();
  if (document.body.dataset.error) return;
  document.body.dataset.ready = "1";
  let rejectsLeft = Number(new URLSearchParams(location.search).get("rejectReloads") ?? 0);
  watchMod((bytes) => {
    if (rejectsLeft > 0) {
      rejectsLeft--;
      throw new Error("rejectReloads test flag");
    }
    game = new Game(bytes);
  });
  if (new URLSearchParams(location.search).has("probeErrors")) {
    setTimeout(() => {
      throw new Error("probe uncaught error");
    }, 0);
    Promise.reject(new Error("probe rejected promise"));
  }
}

// Polls the server's mod version once a second and reloads only the map image when the version moves on.
const MOD_POLL_MS = 1000;
const MOD_MAX_FAILURES = 5;
const MOD_RETRY_BASE_MS = 1000;
const MOD_RETRY_MAX_MS = 8000;
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

async function getJson(url) {
  const r = await fetch(url, { cache: "no-store" });
  if (!r.ok) throw new Error(`${url} answered ${r.status}`);
  return r.json();
}

// apply() builds the new sim before the page takes it, so a rejected image leaves the previous state in place.
// A dropped poll (a server restart included) retries with bounded backoff and re-reads /mod-version on reconnect.
async function watchMod(apply) {
  let since = null;
  let failures = 0;
  let rejected = 0;
  for (;;) {
    await sleep(failures === 0 ? MOD_POLL_MS : Math.min(MOD_RETRY_MAX_MS, MOD_RETRY_BASE_MS * 2 ** (failures - 1)));
    try {
      if (since === null) {
        since = (await getJson("/mod-version")).version;
        document.body.dataset.modVersion = String(since);
      }
      const ev = await getJson(`/mod-events?since=${since}`);
      if (failures > 0) reportClientError("reconnect", `resumed at version ${since} after ${failures} failed polls`);
      failures = 0;
      if (!ev.changed) continue;
      const r = await fetch(`map.bin?v=${ev.version}`, { cache: "no-store" });
      if (!r.ok) throw new Error(`map.bin answered ${r.status}`);
      const bytes = new Uint8Array(await r.arrayBuffer());
      try {
        apply(bytes);
      } catch (e) {
        rejected++;
        reportClientError("reload", e);
        if (rejected >= MOD_MAX_FAILURES) {
          console.error(`mod watch stopped after ${rejected} rejected reloads: ${e}`);
          reportClientError("watch-stop", `after ${rejected} rejected reloads: ${e}`);
          return;
        }
        continue;
      }
      rejected = 0;
      since = ev.version;
      document.body.dataset.modVersion = String(ev.version);
    } catch (e) {
      if (failures === 0) console.error(`mod watch lost the server: ${e}`);
      failures++;
      since = null;
    }
  }
}

const clientSession = Math.random().toString(36).slice(2, 10);

// Posts one failure to the server log; the page's tick, when known, rides in the message.
function reportClientError(path, e) {
  const ticks = document.body.dataset.ticks;
  const tick = ticks === undefined ? "" : ` (tick ${ticks})`;
  const query = new URLSearchParams({ session: clientSession, message: `${path}: ${e}${tick}` });
  fetch("/client-error?" + query, { method: "POST" }).catch((err) => console.error("client-error relay failed: " + err));
}

// Every page failure goes through here: it marks the body, logs to the console and relays to the server.
function failPage(path, e) {
  document.body.dataset.error = String(e);
  console.error(e);
  reportClientError(path, e);
}

window.addEventListener("error", (e) => failPage("window.error", e.error ?? e.message));
window.addEventListener("unhandledrejection", (e) => failPage("unhandledrejection", e.reason));
// Polls /site/version at the mod poll interval. A changed page stamp (main.js edited on the server) reloads the page
// once, and only when the new main.js compiles; a rejected edit keeps the running page and is reported once per stamp.
const AsyncFunction = Object.getPrototypeOf(async function () {}).constructor;
async function editRejection() {
  const text = await (await fetch("main.js", { cache: "no-store" })).text();
  const body = text.replace(/^\s*import\s[^;]*;\s*$/gm, "").replace(/^export\s+/gm, "");
  try {
    new AsyncFunction(body);
    return null;
  } catch (e) {
    return String(e);
  }
}
let pageVersion = null;
let rejectedPageVersion = null;
async function watchPage() {
  for (;;) {
    await sleep(MOD_POLL_MS);
    try {
      const { page } = await getJson("/site/version");
      if (pageVersion === null) {
        pageVersion = page;
        document.body.dataset.pageVersion = page;
      } else if (page !== pageVersion) {
        const rejection = await editRejection();
        if (rejection === null) {
          location.reload();
          return;
        }
        if (page !== rejectedPageVersion) {
          rejectedPageVersion = page;
          reportClientError("page edit rejected", rejection);
        }
      }
    } catch (e) {
      // a dropped poll is retried on the next tick
    }
  }
}
watchPage();
main().catch((e) => failPage("main", e));
