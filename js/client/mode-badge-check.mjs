import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { buildNode } from "./widgets.js";
import { pingGlyphSvg } from "./ping_glyphs.js";

const fail = (what) => { throw new Error(what); };
const here = path.dirname(fileURLToPath(import.meta.url));
const ui = JSON.parse(fs.readFileSync(path.resolve(here, "../../scenarios/moba/moba_ui.json"), "utf8")).text;

class FakeNode {
  constructor(tag) {
    this.tag = tag;
    this.children = [];
    this.dataset = {};
    this.style = {};
    this.attributes = new Map();
    this.classes = new Set();
    this.classList = { add: (...names) => names.forEach((n) => this.classes.add(n)) };
    this.textContent = "";
    this.innerHTML = "";
    this.title = "";
  }
  get className() { return [...this.classes].join(" "); }
  set className(value) { this.classes = new Set(String(value).split(/\s+/).filter(Boolean)); }
  append(...nodes) { this.children.push(...nodes); }
  setAttribute(name, value) { this.attributes.set(name, String(value)); }
  getAttribute(name) { return this.attributes.get(name) ?? null; }
}

globalThis.document = { createElement: (tag) => new FakeNode(tag) };

const LOOK = {
  hero: { glyph: "item", tone: "ok" },
  rules: { glyph: "ping", tone: "warn" },
  draft: { glyph: "on_my_way", tone: "accent" },
};
const PICKS = [["aa", "hero"], ["sc", "rules"], ["cd", "draft"]];

const badges = PICKS.map(([code, kind]) => {
  const words = `${ui.lobby.modes[code]}: ${ui.modeRules[code]}`;
  return { t: "badge", glyph: LOOK[kind].glyph, tone: LOOK[kind].tone, tip: words, alt: words };
});
const root = buildNode({ t: "box", dir: "row", kids: badges }, () => {});

if (root.children.length !== PICKS.length) fail(`box drew ${root.children.length} nodes for ${PICKS.length} badges`);
const tones = new Set();
root.children.forEach((el, i) => {
  const [code, kind] = PICKS[i];
  const { glyph, tone } = LOOK[kind];
  const words = badges[i].tip;
  if (!el.classes.has("w-badge")) fail(`${code}: badge has classes "${el.className}"`);
  if (!el.classes.has(`t-${tone}`)) fail(`${code}: badge is not toned ${tone}`);
  if (el.innerHTML !== pingGlyphSvg(glyph)) fail(`${code}: badge is not the shared ${glyph} glyph`);
  if (el.title !== words) fail(`${code}: title is "${el.title}"`);
  if (el.getAttribute("aria-label") !== words) fail(`${code}: aria-label is "${el.getAttribute("aria-label")}"`);
  if (el.getAttribute("role") !== "img") fail(`${code}: role is ${el.getAttribute("role")}`);
  if (el.textContent !== "" || el.children.length !== 0) fail(`${code}: badge shows visible text`);
  tones.add(tone);
  console.log(`mode badge ${code} glyph=${glyph} tone=${tone} title="${el.title}"`);
});
if (tones.size !== PICKS.length) fail("two mode kinds share a tone");
console.log(`mode badge ok (${PICKS.length} badges: words in title and aria-label, no visible text)`);
