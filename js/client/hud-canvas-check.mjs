import { abilityCellsOf, cellAt, createHudCanvas, hudCanvasRequested, layoutRow, withModifiers } from "./hud_canvas.js";

const expect = (cond, what) => { if (!cond) throw new Error(what); };

function recordingContext() {
  const calls = [];
  const ctx = new Proxy({}, {
    get: (target, key) => (key in target ? target[key] : (target[key] = (...args) => { calls.push([key, ...args]); })),
    set: (target, key, value) => { target[key] = value; return true; },
  });
  return { ctx, calls };
}

function fakeElement(tag, context = null) {
  const listeners = new Map();
  const node = {
    tag,
    id: "",
    style: {},
    dataset: {},
    attrs: {},
    children: [],
    addEventListener(type, fn) { listeners.set(type, [...(listeners.get(type) ?? []), fn]); },
    dispatch(type, event) { for (const fn of listeners.get(type) ?? []) fn(event); },
    appendChild(child) { node.children.push(child); return child; },
    remove() { node.removed = true; },
    setAttribute(name, value) { node.attrs[name] = String(value); },
    getContext() { return context; },
    querySelector() { return null; },
    getBoundingClientRect() {
      return {
        left: parseFloat(node.style.left ?? "0"),
        top: parseFloat(node.style.top ?? "0"),
        width: parseFloat(node.style.width ?? "0"),
        height: parseFloat(node.style.height ?? "0"),
      };
    },
  };
  return node;
}

expect(hudCanvasRequested("?hud=canvas") === true, "flag on");
expect(hudCanvasRequested("") === false && hudCanvasRequested("?hud=dom") === false, "flag off by default");

const tree = {
  t: "panel",
  kids: [{
    t: "box",
    dir: "row",
    kids: [
      { t: "cell", unit: 7, slot: 1, icon: "icons/abilities/frost_nova.png", alt: "frost_nova", call: "cast", args: { slot: 1 },
        kids: [{ t: "label", text: "75", tone: "mp" }, { t: "label", text: "3.0s", tone: "warn" }, { t: "bar", value: 4, max: 10 }] },
      { t: "cell", unit: 7, slot: 2, icon: "icons/abilities/blink.png", alt: "blink", call: "cast", args: { slot: 2 }, kids: [] },
      { t: "cell", unit: 7, slot: 3, icon: "icons/abilities/dim.png", alt: "dim", tone: "dim", kids: [] },
      { t: "label", text: "not an ability", tone: "muted" },
    ],
  }],
};
const cellNodes = tree.kids[0].kids;
const cells = abilityCellsOf(tree);
expect(cells.length === 3, `cells=${cells.length}`);

const { ctx, calls } = recordingContext();
const canvas = fakeElement("canvas", ctx);
const domCells = [0, 1, 2].map((i) => ({
  getBoundingClientRect: () => ({ left: 100 + i * 50, top: 500, width: 44, height: 44 }),
  querySelector: () => null,
}));
const doc = {
  head: fakeElement("head"),
  body: fakeElement("body"),
  createElement: (tag) => (tag === "canvas" ? canvas : fakeElement(tag)),
  querySelectorAll: (selector) => (selector === "[data-cd-unit][data-cd-slot]" ? domCells : []),
};
const sends = [];
const hud = createHudCanvas({ send: (call, args) => sends.push({ call, args }), doc, win: { devicePixelRatio: 1, addEventListener() {}, removeEventListener() {} } });

const draws = hud.apply({ id: "select", slot: "bar-left", tree });
expect(canvas.dataset.cells === "3", `canvas cells=${canvas.dataset.cells}`);
expect(Number(canvas.dataset.draws) === draws && draws === 8, `draws=${draws}`);
expect(calls.filter(([k]) => k === "fillRect").length === 4, "one face fill per cell plus cooldown shade");
expect(calls.some(([k, text]) => k === "fillText" && text === "3.0s"), "cooldown text drawn");
expect(doc.body.dataset.hud === "canvas", "DOM cells hidden by body flag");
expect(canvas.style.display === "block" && canvas.style.left === "100px" && canvas.style.width === "144px", "canvas placed over DOM row");

const rects = layoutRow(3, 144, 44);
const clickAt = (i, extra = {}) => canvas.dispatch("click", { clientX: 100 + rects[i].x + rects[i].w / 2, clientY: 500 + 22, ...extra });
expect(cellAt(rects, 47, 10) === -1, "gap between cells is not a hit");

clickAt(0);
expect(sends.length === 1, `sends after cell 1 click=${sends.length}`);
expect(JSON.stringify(sends[0]) === JSON.stringify({ call: cellNodes[0].call, args: cellNodes[0].args }), `sent ${JSON.stringify(sends[0])}`);

clickAt(1, { altKey: true });
expect(JSON.stringify(sends[1]) === JSON.stringify({ call: "cast", args: withModifiers({ slot: 2 }, { altKey: true }) }), `alt sent ${JSON.stringify(sends[1])}`);

clickAt(2);
expect(sends.length === 2, "dim cell without a call sends nothing");
canvas.dispatch("click", { clientX: 100 + 47, clientY: 522 });
expect(sends.length === 2, "gap click sends nothing");

hud.apply({ id: "select", remove: true });
expect(canvas.dataset.cells === "0" && canvas.style.display === "none", "removing the select panel hides the canvas");

console.log(`hud canvas probe ok cells=${cells.length} draws=${draws} sent=${JSON.stringify(sends)} dom_cell_call=${JSON.stringify({ call: cellNodes[0].call, args: cellNodes[0].args })}`);
