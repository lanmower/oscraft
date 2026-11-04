import { readFileSync } from "node:fs";
import { abilityCellsOf, createHudCanvas, withModifiers } from "./hud_canvas.js";

const expect = (cond, what) => { if (!cond) throw new Error(what); };

const ui = JSON.parse(readFileSync(new URL("../../scenarios/moba/moba_ui.json", import.meta.url), "utf8"));
const castSlot = ui.keys?.cast ?? {};
const keys = Object.keys(castSlot).sort((a, b) => castSlot[a] - castSlot[b]);
const slots = keys.map((key) => castSlot[key]);
expect(keys.join() === "q,w,e,r", `cast keys ${keys.join()}`);
expect(new Set(slots).size === slots.length, `cast slots collide ${slots.join()}`);

const HERO = 7;
const tree = {
  t: "panel",
  kids: [{
    t: "box",
    dir: "row",
    kids: keys.map((key) => ({
      t: "cell", unit: HERO, slot: castSlot[key], alt: key, icon: `icons/abilities/${key}.png`,
      call: "cast", args: { slot: castSlot[key] }, kids: [],
    })),
  }],
};
const cells = abilityCellsOf(tree);
expect(cells.length === keys.length, `cells=${cells.length} keys=${keys.length}`);

function recordingContext() {
  return new Proxy({}, {
    get: (target, key) => (key in target ? target[key] : (target[key] = () => {})),
    set: (target, key, value) => { target[key] = value; return true; },
  });
}

function fakeNode(tag, context = null) {
  const listeners = new Map();
  return {
    tag,
    id: "",
    style: {},
    dataset: {},
    attrs: {},
    children: [],
    addEventListener(type, fn) { listeners.set(type, [...(listeners.get(type) ?? []), fn]); },
    dispatch(type, event) { for (const fn of listeners.get(type) ?? []) fn(event); },
    appendChild(child) { this.children.push(child); return child; },
    remove() { this.removed = true; },
    setAttribute(name, value) { this.attrs[name] = String(value); },
    getContext() { return context; },
    querySelector() { return null; },
    getBoundingClientRect() {
      return {
        left: parseFloat(this.style.left ?? "0"),
        top: parseFloat(this.style.top ?? "0"),
        width: parseFloat(this.style.width ?? "0"),
        height: parseFloat(this.style.height ?? "0"),
      };
    },
  };
}

const canvas = fakeNode("canvas", recordingContext());
const domCells = keys.map((_, i) => ({
  getBoundingClientRect: () => ({ left: 100 + i * 50, top: 500, width: 44, height: 44 }),
  querySelector: () => null,
}));
const doc = {
  head: fakeNode("head"),
  body: fakeNode("body"),
  createElement: (tag) => (tag === "canvas" ? canvas : fakeNode(tag)),
  querySelectorAll: (selector) => (selector === "[data-cd-unit][data-cd-slot]" ? domCells : []),
};
const sends = [];
const hud = createHudCanvas({ send: (call, args) => sends.push({ call, args }), doc, win: { devicePixelRatio: 1, addEventListener() {}, removeEventListener() {} } });
const draws = hud.apply({ id: "select", slot: "bar-left", tree });
expect(canvas.dataset.cells === String(keys.length) && draws > 0, `canvas cells=${canvas.dataset.cells} draws=${draws}`);

const reached = keys.map((key, i) => {
  const slot = castSlot[key];
  expect(cells[i].slot === slot && cells[i].call === "cast", `${key}: canvas cell slot=${cells[i].slot} call=${cells[i].call}`);
  const before = sends.length;
  canvas.dispatch("click", { clientX: 100 + i * 50 + 22, clientY: 522 });
  expect(sends.length === before + 1, `${key}: click sent ${sends.length - before} messages`);
  const sent = sends[before];
  expect(sent.call === "cast" && JSON.stringify(sent.args) === JSON.stringify({ slot }), `${key}: sent ${JSON.stringify(sent)}`);
  return `${key}=${slot}`;
});

const beforeAlt = sends.length;
canvas.dispatch("click", { clientX: 100 + 22, clientY: 522, altKey: true });
const altSent = sends[beforeAlt];
expect(altSent?.call === "cast" && JSON.stringify(altSent.args) === JSON.stringify(withModifiers({ slot: castSlot.q }, { altKey: true })), `alt click sent ${JSON.stringify(altSent)}`);

console.log(`hud canvas keys probe ok keys=${keys.map((key) => `${key}:${castSlot[key]}`).join(",")} canvas=${reached.join(",")} alt=${JSON.stringify(altSent)}`);
