import { createMobaInput } from "./moba_input.js";
import { createMinimap } from "./ui_minimap.js";

const fail = (what) => { throw new Error(what); };

class FakeEvent {
  constructor(type, init = {}) {
    this.type = type;
    this.button = init.button ?? 0;
    this.clientX = init.clientX ?? 0;
    this.clientY = init.clientY ?? 0;
    this.pointerId = init.pointerId ?? 1;
    this.detail = init.detail;
    this.altKey = init.altKey === true;
    this.ctrlKey = init.ctrlKey === true;
    this.shiftKey = init.shiftKey === true;
    this.target = null;
    this.stopped = false;
    this.defaultPrevented = false;
  }
  stopPropagation() { this.stopped = true; }
  preventDefault() { this.defaultPrevented = true; }
}

class FakeNode {
  constructor() {
    this.parent = null;
    this.children = [];
    this.listeners = [];
    this.dataset = {};
    this.style = {};
    this.attributes = new Map();
    this.rect = { left: 0, top: 0, width: 0, height: 0 };
    this.clientLeft = 0;
    this.clientTop = 0;
    this.captured = null;
  }
  appendChild(child) {
    child.parent = this;
    this.children.push(child);
    return child;
  }
  remove() {
    if (this.parent) this.parent.children = this.parent.children.filter((c) => c !== this);
    this.parent = null;
  }
  addEventListener(type, fn) { this.listeners.push({ type, fn }); }
  removeEventListener(type, fn) { this.listeners = this.listeners.filter((l) => l.type !== type || l.fn !== fn); }
  setAttribute(name, value) { this.attributes.set(name, String(value)); }
  hasAttribute(name) { return this.attributes.has(name); }
  getBoundingClientRect() { return this.rect; }
  setPointerCapture(id) { this.captured = id; }
  hasPointerCapture(id) { return this.captured === id; }
  releasePointerCapture(id) { if (this.captured === id) this.captured = null; }
  closest(selector) {
    if (selector !== "[data-ui]") fail(`unsupported selector ${selector}`);
    for (let n = this; n; n = n.parent) if (n.dataset.ui !== undefined) return n;
    return null;
  }
  dispatchEvent(event) {
    event.target = this;
    for (let node = this; node && !event.stopped; node = node.parent) {
      event.currentTarget = node;
      for (const l of node.listeners) if (l.type === event.type) l.fn(event);
    }
    return !event.defaultPrevented;
  }
}

class FakeCanvas extends FakeNode {
  constructor() {
    super();
    this.width = 192;
    this.height = 144;
    this.calls = [];
    this.context = new Proxy({}, {
      get: (target, key) => (key in target ? target[key] : (target[key] = (...args) => { this.calls.push([key, ...args]); })),
      set: (target, key, value) => { target[key] = value; return true; },
    });
  }
  getContext() { return this.context; }
}

globalThis.HTMLCanvasElement = FakeCanvas;
globalThis.CustomEvent = FakeEvent;
const windowNode = new FakeNode();
windowNode.innerWidth = 800;
windowNode.innerHeight = 600;
const documentNode = new FakeNode();
documentNode.parent = windowNode;
const body = documentNode.appendChild(new FakeNode());
globalThis.window = windowNode;
globalThis.document = Object.assign(documentNode, {
  body,
  createElement: (tag) => (tag === "canvas" ? new FakeCanvas() : new FakeNode()),
});
windowNode.parent = null;

const world = body.appendChild(new FakeCanvas());
world.id = "view";
world.rect = { left: 0, top: 0, width: 800, height: 600 };
const uiBar = body.appendChild(new FakeNode());
const uiBarRight = uiBar.appendChild(new FakeNode());
const minimap = uiBarRight.appendChild(new FakeCanvas());
minimap.id = "minimap";
minimap.rect = { left: 10, top: 500, width: 192, height: 144 };

const worldEvents = [];
world.addEventListener("pointerdown", (e) => worldEvents.push(`down:${e.button}`));
world.addEventListener("pointerup", (e) => worldEvents.push(`up:${e.button}`));

const sends = [];
const centres = [];
const drags = [];
const orders = [];
const input = createMobaInput({
  game: {},
  toWorld: (x, y) => ({ x, z: y }),
  toScreen: () => ({ x: 0, y: 0 }),
  send: (name) => sends.push(name),
  dragSlopPx: 4,
});
const minimapView = createMinimap({
  canvas: minimap,
  mapSize: { w: 6400, h: 6400 },
  lanes: null,
  selected: () => input.selection()[0] ?? -1,
  localPlayer: () => 0,
  playerColor: () => [1, 0, 0],
  teams: [0, 0, 0, 0, 0, 1, 1, 1, 1, 1],
  viewRect: () => ({ x0: 0, z0: 0, x1: 800, z1: 600 }),
  centre: (x, z) => centres.push([x, z]),
  drag: (x, z) => drags.push([x, z]),
  order: (kind, x, z) => orders.push([kind, x, z]),
});
windowNode.dispatchEvent(new CustomEvent("moba:feedback", { detail: { kind: "selection", ids: [42] } }));

const press = (target, button, x, y, keys = {}) => target.dispatchEvent(new FakeEvent("pointerdown", { button, clientX: x, clientY: y, ...keys }));
const move = (target, x, y) => target.dispatchEvent(new FakeEvent("pointermove", { clientX: x, clientY: y }));
const release = (target, button, x, y) => target.dispatchEvent(new FakeEvent("pointerup", { button, clientX: x, clientY: y }));
const boxShown = () => {
  input.draw();
  return body.children.some((c) => c.dataset.ui === "box" && c.style.display === "block");
};
const reset = () => {
  sends.length = 0;
  centres.length = 0;
  drags.length = 0;
  orders.length = 0;
  worldEvents.length = 0;
};

const centre = { x: 10 + 96, y: 500 + 72 };
const selectionBefore = input.selection().slice();

reset();
press(minimap, 0, centre.x, centre.y);
release(minimap, 0, centre.x, centre.y);
const clickSelection = input.selection().slice();
if (JSON.stringify(clickSelection) !== JSON.stringify(selectionBefore)) fail(`click changed selection ${selectionBefore} -> ${clickSelection}`);
if (boxShown()) fail("click started a box select");
if (sends.includes("viewport") || worldEvents.length) fail(`click reached the world path: sends=${sends} world=${worldEvents}`);
if (centres.length !== 1 || drags.length !== 0) fail(`click did not only move the camera: centres=${centres.length} drags=${drags.length}`);
const clickLog = `click selection ${JSON.stringify(selectionBefore)} -> ${JSON.stringify(clickSelection)} centres=${centres.length}`;

reset();
press(minimap, 0, centre.x, centre.y);
move(minimap, centre.x + 40, centre.y);
move(minimap, centre.x + 80, centre.y + 20);
release(minimap, 0, centre.x + 80, centre.y + 20);
if (boxShown()) fail("drag started a box select");
if (input.selection().join() !== selectionBefore.join()) fail("drag changed selection");
if (sends.includes("viewport") || worldEvents.length) fail(`drag reached the world path: sends=${sends} world=${worldEvents}`);
if (centres.length !== 0 || drags.length < 1) fail(`drag did not only pan: centres=${centres.length} drags=${drags.length}`);
const dragLog = `drag selection ${JSON.stringify(input.selection())} drags=${drags.length} centres=${centres.length}`;

reset();
press(minimap, 2, centre.x, centre.y);
release(minimap, 2, centre.x, centre.y);
if (orders.length !== 2 || orders[0][0] !== "down" || orders[1][0] !== "up") fail(`right click lost its order: ${JSON.stringify(orders)}`);
if (centres.length !== 0 || boxShown() || input.selection().join() !== selectionBefore.join()) fail("right click on the minimap changed the selection or moved the camera");
const rightLog = `right click orders=${orders.map((o) => o[0]).join(",")}`;

reset();
press(minimap, 0, centre.x, centre.y, { altKey: true });
release(minimap, 0, centre.x, centre.y);
press(minimap, 0, centre.x, centre.y, { altKey: true, ctrlKey: true });
release(minimap, 0, centre.x, centre.y);
press(minimap, 0, centre.x, centre.y, { altKey: true, shiftKey: true });
release(minimap, 0, centre.x, centre.y);
const kinds = orders.map((o) => o[0]).join(",");
if (kinds !== "ping,danger,on_my_way") fail(`alt clicks on the minimap sent kinds ${kinds}`);
if (centres.length !== 0 || drags.length !== 0 || boxShown() || input.selection().join() !== selectionBefore.join()) fail("alt click on the minimap moved the camera or changed the selection");
const pingLog = `minimap alt clicks kinds=${kinds} at=${orders.map((o) => `${o[1].toFixed(1)},${o[2].toFixed(1)}`).join(" ")}`;

minimap.calls.length = 0;
windowNode.dispatchEvent(new CustomEvent("moba:ping", { detail: { kind: "on_my_way", x: 3200, z: 3200, from: { x: 1000, z: 1000 }, seconds: 4, colour: "#7cc7ff", text: "" } }));
windowNode.dispatchEvent(new CustomEvent("moba:ping", { detail: { kind: "item", x: 1200, z: 900, seconds: 4, colour: "#ffd54f", text: "cheese: ready" } }));
minimapView.draw([]);
const routeDash = minimap.calls.some((c) => c[0] === "setLineDash" && c[1] !== undefined && c[1].length === 2);
const routeLine = minimap.calls.find((c) => c[0] === "moveTo");
const routeEnd = minimap.calls.find((c) => c[0] === "lineTo");
const glyphFilled = minimap.calls.some((c) => c[0] === "fill");
const textDrawn = minimap.calls.some((c) => c[0] === "fillText");
if (!routeDash || !routeLine || !routeEnd || Math.abs(routeLine[1] - 1000 * 192 / 6400) > 0.01 || Math.abs(routeEnd[1] - 3200 * 192 / 6400) > 0.01) fail(`route ping did not draw a route: ${JSON.stringify(minimap.calls.slice(0, 12))}`);
if (!glyphFilled || textDrawn) fail(`ping must draw a glyph and no words: glyph=${glyphFilled} text=${textDrawn}`);
const drawLog = `minimap route from=${routeLine[1].toFixed(1)},${routeLine[2].toFixed(1)} to=${routeEnd[1].toFixed(1)},${routeEnd[2].toFixed(1)} glyph=${glyphFilled} text=${textDrawn}`;

reset();
press(world, 0, 300, 300);
move(world, 400, 400);
const groundBoxShown = boxShown();
release(world, 0, 400, 400);
if (!sends.includes("viewport") || !groundBoxShown || worldEvents.join() !== "down:0,up:0") fail(`control: a world press did not reach the world path: sends=${sends} box=${groundBoxShown} world=${worldEvents}`);

input.destroy();
console.log(clickLog);
console.log(dragLog);
console.log(rightLog);
console.log(pingLog);
console.log(drawLog);
console.log("minimap selection ok");
