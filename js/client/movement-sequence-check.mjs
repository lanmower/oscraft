import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { approachTarget, createHeroViewLock, followTarget, heroDirection } from "./hero_view_lock.js";
import { REORDER_S, createHeroRun, enemiesInSight, stepHeroRun } from "./hero_run.js";
import { createModUi } from "./modui.js";

const here = dirname(fileURLToPath(import.meta.url));
const playSource = readFileSync(resolve(here, "play.js"), "utf8");
const fail = (what) => { throw new Error(what); };
const DT = 1 / 60;
const HERO_ID = 7;
const START = [100, 0, 100];
const DRAG = { x: 400, z: 300 };
const CAMERA_DRAG = { x: 600, y: 500 };
const LOCK_FOLLOW_RATE = Number(/const LOCK_FOLLOW_RATE = (\d+);/.exec(playSource)[1]);
const PAN_HEADER = "function panByKeys(dtS) {";
const DRIVE_HEADER = "function driveLockedHero(dtS) {";
const NAMES = [
  "heroViewLock", "approachTarget", "followTarget", "stepHeroRun", "createHeroRun", "heroDirection",
  "enemiesInSight", "drawnPosition", "groundAt", "currentUnits", "localHeroUnit", "isHostile",
  "renderer3d", "keysDown", "commands", "PAN_RULES", "pointer", "drag", "canvas", "LOCK_FOLLOW_RATE", "W", "H",
  "padMove", "pollPad", "syncOneHand",
];

function extract(header) {
  const start = playSource.indexOf(header);
  if (start < 0) fail(`missing function header in play.js: ${header}`);
  const end = playSource.indexOf("\n}\n", start);
  if (end < 0) fail(`unterminated function in play.js: ${header}`);
  return playSource.slice(start, end + 3);
}

function buildDriver(panSource) {
  return new Function("deps", [
    `const { ${NAMES.join(", ")} } = deps;`,
    "let heroRun = createHeroRun();",
    "let lockedFocus = null;",
    extract(DRIVE_HEADER),
    panSource,
    "return { driveLockedHero, panByKeys };",
  ].join("\n"));
}

function installNow(clock) {
  const previous = Object.getOwnPropertyDescriptor(globalThis, "performance");
  Object.defineProperty(globalThis, "performance", { value: { now: () => clock.t * 1000 }, configurable: true, writable: true });
  return () => {
    if (previous) Object.defineProperty(globalThis, "performance", previous);
    else delete globalThis.performance;
  };
}

function makeRenderer(lock, trace) {
  const camera = { target: [...START], distance: 1000, minDistance: 200, maxDistance: 3000, yaw: 0, pitch: 0.9 };
  return {
    camera,
    setCamera(opts = {}) {
      if (opts.target) camera.target = [...opts.target];
      if (opts.distance !== undefined) camera.distance = opts.distance;
      trace.writes.push({ phase: trace.phase, target: [...camera.target], lockedAtWrite: lock.isLocked() });
    },
    pan() {},
  };
}

function makeWindow() {
  const listeners = new Map();
  return {
    innerWidth: 1280,
    innerHeight: 720,
    addEventListener(type, fn) { listeners.set(type, [...(listeners.get(type) ?? []), fn]); },
    removeEventListener(type, fn) { listeners.set(type, (listeners.get(type) ?? []).filter((f) => f !== fn)); },
    dispatchEvent(ev) {
      for (const fn of listeners.get(ev.type) ?? []) fn(ev);
      return true;
    },
  };
}

function makeDocument(win) {
  const doc = {
    defaultView: win,
    getElementById: () => null,
    createTextNode: (text) => ({ nodeValue: text, parent: null, remove() {} }),
    createElement: (tag) => makeElement(doc, tag, ""),
  };
  return doc;
}

function makeElement(doc, tag, className) {
  const node = {
    tagName: tag,
    ownerDocument: doc,
    className,
    style: {},
    children: [],
    parent: null,
    textContent: "",
    get childNodes() { return this.children; },
    get firstChild() { return this.children[0] ?? null; },
    appendChild(child) { this.children.push(child); child.parent = this; return child; },
    prepend(child) { this.children.unshift(child); child.parent = this; return child; },
    replaceChildren(...nodes) { this.children = []; for (const n of nodes) this.appendChild(n); },
    remove() {
      if (this.parent) this.parent.children = this.parent.children.filter((c) => c !== this);
      this.parent = null;
    },
    setAttribute() {},
    querySelector(selector) { return findByClass(this, selector.slice(1)); },
    set innerHTML(html) {
      this.children = [];
      for (const m of String(html).matchAll(/<div class="([^"]+)"/g)) this.appendChild(makeElement(doc, "div", m[1]));
    },
  };
  return node;
}

function findByClass(node, cls) {
  if (node.className === cls) return node;
  for (const child of node.children) {
    const hit = findByClass(child, cls);
    if (hit) return hit;
  }
  return null;
}

async function sequence(mode, { panSource = extract(PAN_HEADER), releaseAfterWrite = false, wireLock = true } = {}) {
  const trace = { phase: "start", writes: [], orders: [] };
  const clock = { t: 0 };
  const restoreNow = installNow(clock);
  try {
    const lock = createHeroViewLock();
    const keysDown = new Set();
    const units = [{ id: HERO_ID, player: 0, x: START[0], z: START[2] }];
    const renderer3d = makeRenderer(lock, trace);
    const win = makeWindow();
    const doc = makeDocument(win);
    const commands = {
      hold: (id) => trace.orders.push({ kind: "hold", id, t: clock.t, phase: trace.phase }),
      order: (id, x, z) => trace.orders.push({ kind: "order", id, x, z, t: clock.t, phase: trace.phase }),
    };
    const onCameraTake = releaseAfterWrite ? () => queueMicrotask(() => lock.release()) : () => lock.release();
    const modUi = createModUi({
      root: makeElement(doc, "body", ""),
      renderer: () => renderer3d,
      localPlayer: () => 0,
      defaultDistance: () => 1000,
      onCameraTake,
      isLocked: wireLock ? () => lock.isLocked() : undefined,
    });
    const driver = buildDriver(panSource)({
      heroViewLock: lock,
      approachTarget,
      followTarget,
      stepHeroRun,
      createHeroRun,
      heroDirection,
      enemiesInSight,
      drawnPosition: (u) => u,
      groundAt: () => 0,
      currentUnits: () => units,
      localHeroUnit: (all) => all.find((u) => u.id === HERO_ID) ?? null,
      isHostile: () => false,
      renderer3d,
      keysDown,
      commands,
      PAN_RULES: { edgePx: 0, panPxPerS: 600 },
      pointer: null,
      drag: null,
      canvas: null,
      LOCK_FOLLOW_RATE,
      W: 2048,
      H: 2048,
      padMove: null,
      pollPad: () => {},
      syncOneHand: () => {},
    });
    const frame = () => {
      driver.panByKeys(DT);
      modUi.step(DT, lock.isLocked());
      clock.t += DT;
    };

    lock.toggle();
    keysDown.add("ArrowUp");
    trace.phase = "held";
    for (let i = 0; i < 30; i += 1) frame();

    if (mode === "minimap") {
      trace.phase = "drag";
      win.dispatchEvent({ type: "moba:minimap-drag", detail: { x: DRAG.x, z: DRAG.z } });
      await Promise.resolve();
    } else if (mode === "camera") {
      trace.phase = "glide";
      modUi.push({ kind: "camera", player: 0, x: CAMERA_DRAG.x, y: CAMERA_DRAG.y, zoom: 1, seconds: 1 });
      frame();
      trace.phase = "drag";
      clock.t += 0.05 - DT;
      modUi.push({ kind: "camera", player: 0, x: CAMERA_DRAG.x, y: CAMERA_DRAG.y, zoom: 1, seconds: 1 });
      await Promise.resolve();
    } else if (mode === "centre") {
      trace.phase = "centre";
      win.dispatchEvent({ type: "moba:minimap-centre", detail: { x: 300, z: 300, seconds: 0 } });
      clock.t += 0.2;
      win.dispatchEvent({ type: "moba:minimap-centre", detail: { x: 300, z: 300, seconds: 1 } });
      trace.phase = "centreFrame";
      frame();
    } else {
      trace.phase = "instant";
      modUi.push(mode === "distance"
        ? { kind: "camera_distance", player: 0, zoom: 2, seconds: 0 }
        : { kind: "camera", player: 0, x: CAMERA_DRAG.x, y: CAMERA_DRAG.y, zoom: 1, seconds: 0 });
      trace.phase = "instantFrame";
      frame();
    }

    trace.phase = "after";
    frame();
    trace.phase = "end";
    return trace;
  } finally {
    restoreNow();
  }
}

function violations(trace, mode) {
  const out = [];
  const drag = trace.writes.filter((w) => w.phase === "drag");
  if (drag.length === 0) out.push(`${mode}: no drag write recorded`);
  for (const w of drag) {
    if (w.lockedAtWrite) out.push(`release-after-write: ${mode} drag wrote the camera while the hero lock was engaged`);
  }
  for (const w of trace.writes.filter((x) => x.phase === "glide" && !x.lockedAtWrite)) {
    out.push(`${mode}: locked glide wrote the camera to ${w.target} without the hero lock`);
  }
  const after = trace.writes.filter((w) => w.phase === "after");
  if (after.length > 0) {
    const dragTarget = drag.length ? drag.at(-1).target : null;
    out.push(`snap-back: following frame wrote ${after[0].target}${dragTarget ? ` after the drag target ${dragTarget}` : ""}`);
  }
  const held = trace.orders.filter((o) => o.phase === "held");
  if (held.length < 2) out.push(`held cadence: ${held.length} orders in 0.5 s of held ArrowUp`);
  for (let i = 1; i < held.length; i += 1) {
    const gap = held[i].t - held[i - 1].t;
    if (gap > REORDER_S + 2 * DT) out.push(`held cadence: order gap ${gap.toFixed(4)} s exceeds ${REORDER_S + 2 * DT} s`);
  }
  if (trace.orders.some((o) => o.kind === "hold" && o.phase === "held")) out.push("held cadence: hold issued while the key stayed down");
  if (trace.orders.some((o) => o.phase === "after")) out.push("after release: hero commands issued from the unlocked frame");
  return out;
}

function instantViolations(trace) {
  return trace.writes
    .filter((w) => w.phase === "instant" && w.lockedAtWrite)
    .map((w) => `zero-second mod camera event wrote ${w.target} while the hero lock was engaged`);
}

function lockedWriteViolations(trace, phase, label) {
  return trace.writes.filter((w) => w.phase === phase).map((w) => `${label} wrote ${w.target} while the hero lock was engaged`);
}

const minimapRun = await sequence("minimap");
const cameraRun = await sequence("camera");
const positiveFound = [...violations(minimapRun, "minimap"), ...violations(cameraRun, "camera")];
if (positiveFound.length > 0) fail(positiveFound.join("; "));

const releaseAfter = violations(await sequence("minimap", { releaseAfterWrite: true }), "minimap");
if (!releaseAfter.some((v) => v.startsWith("release-after-write"))) fail("negative control release-after-write did not fail the gate");

const panSource = extract(PAN_HEADER);
const snapSource = panSource.replace("if (heroViewLock.isLocked()) {", "if (heroViewLock.isLocked() || lockedFocus !== null) {");
if (snapSource === panSource) fail("negative control snap-back: the panByKeys condition was not found to mutate");
const snapBack = violations(await sequence("minimap", { panSource: snapSource }), "minimap");
if (!snapBack.some((v) => v.startsWith("snap-back"))) fail("negative control snap-back did not fail the gate");

const instantFound = instantViolations(await sequence("instant"));
if (instantFound.length > 0) fail(instantFound.join("; "));
const instantControl = instantViolations(await sequence("instant", { wireLock: false }));
if (instantControl.length === 0) fail("negative control zero-second write while locked did not fail the gate");
const distanceFound = instantViolations(await sequence("distance"));
if (distanceFound.length > 0) fail(distanceFound.join("; "));
const distanceControl = instantViolations(await sequence("distance", { wireLock: false }));
if (distanceControl.length === 0) fail("negative control zero-second camera_distance write while locked did not fail the gate");
const centreWrites = lockedWriteViolations(await sequence("centre"), "centre", "minimap-centre");
if (centreWrites.length > 0) fail(centreWrites.join("; "));
const centreControl = lockedWriteViolations(await sequence("centre", { wireLock: false }), "centre", "minimap-centre");
if (centreControl.length === 0) fail("negative control minimap-centre write while locked did not fail the gate");

const heldOrders = minimapRun.orders.filter((o) => o.phase === "held").length;
console.log(`movement sequence ok: ${heldOrders} held ArrowUp orders at uninterrupted cadence, minimap and camera drags release before their write, no write on the following frame`);
console.log(`negative control release-after-write: fails with "${releaseAfter[0]}"`);
console.log(`negative control snap-back: fails with "${snapBack[0]}"`);
console.log(`negative control zero-second write while locked (lock state not passed to modui): fails with "${instantControl[0]}"`);
console.log(`negative control zero-second camera_distance while locked (lock state not passed): fails with "${distanceControl[0]}"`);
console.log(`minimap-centre claim: writes nothing while locked (seconds 0 and seconds 1); unwired control fails with "${centreControl[0]}"`);

