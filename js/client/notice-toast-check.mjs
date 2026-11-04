import { NOTICE_KINDS, noticeKind, renderToast } from "./notice_kinds.js";
import { hasPingGlyph, pingGlyphSvg } from "./ping_glyphs.js";

const fail = (what) => { throw new Error(what); };

class FakeNode {
  constructor(tag) {
    this.tag = tag;
    this.children = [];
    this.dataset = {};
    this.style = { cssText: "" };
    this.attributes = new Map();
    this.textContent = "";
    this.innerHTML = "";
    this.title = "";
  }
  append(...nodes) { this.children.push(...nodes); }
  setAttribute(name, value) { this.attributes.set(name, String(value)); }
  getAttribute(name) { return this.attributes.get(name) ?? null; }
}

const doc = { createElement: (tag) => new FakeNode(tag) };
const WORDS = "Cannot do that: no mana";
const HEX = /^#[0-9a-f]{6}$/i;

const kinds = Object.keys(NOTICE_KINDS);
const lines = kinds.map((kind) => {
  const spec = NOTICE_KINDS[kind];
  if (!hasPingGlyph(spec.glyph)) fail(`${kind}: glyph ${spec.glyph} is not in the shared glyph set`);
  if (!HEX.test(spec.colour)) fail(`${kind}: colour ${spec.colour} is not a palette hex`);
  const toast = renderToast(doc, kind, WORDS);
  if (toast.dataset.notice !== kind) fail(`${kind}: dataset.notice is ${toast.dataset.notice}`);
  if (toast.title !== WORDS) fail(`${kind}: title is ${toast.title}`);
  if (toast.getAttribute("aria-label") !== WORDS) fail(`${kind}: aria-label is ${toast.getAttribute("aria-label")}`);
  if (toast.getAttribute("role") !== "img") fail(`${kind}: role is ${toast.getAttribute("role")}`);
  if (!toast.style.cssText.includes(`color:${spec.colour}`)) fail(`${kind}: colour ${spec.colour} not on the toast`);
  const [icon] = toast.children;
  if (icon.innerHTML !== pingGlyphSvg(spec.glyph)) fail(`${kind}: icon is not the shared ${spec.glyph} glyph`);
  return `notice toast ${kind} glyph=${spec.glyph} colour=${spec.colour} title="${toast.title}"`;
});

if (new Set(kinds.map((k) => NOTICE_KINDS[k].glyph)).size !== kinds.length) fail("two notice kinds share a glyph");
if (new Set(kinds.map((k) => NOTICE_KINDS[k].colour)).size !== kinds.length) fail("two notice kinds share a colour");
if (noticeKind("refusal") !== "refusal" || noticeKind("bogus") !== "notice") fail("unknown notice kind does not fall back to notice");
if (renderToast(doc, "bogus", WORDS).dataset.notice !== "notice") fail("unknown kind is not rendered as notice");

for (const line of lines) console.log(line);
console.log(`notice toast fallback bogus -> notice`);
console.log(`notice toast ok (${kinds.length} kinds)`);
