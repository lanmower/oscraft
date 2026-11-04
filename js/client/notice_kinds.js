import { pingGlyphSvg } from "./ping_glyphs.js";

export const NOTICE_KINDS = Object.freeze({
  refusal: Object.freeze({ glyph: "danger", colour: "#ffb4a8" }),
  notice: Object.freeze({ glyph: "ping", colour: "#e6ecf1" }),
});

export function noticeKind(kind) {
  return Object.hasOwn(NOTICE_KINDS, kind) ? kind : "notice";
}

export function renderToast(doc, kind, text) {
  const name = noticeKind(kind);
  const { glyph, colour } = NOTICE_KINDS[name];
  const sentence = String(text);
  const item = doc.createElement("div");
  item.dataset.notice = name;
  item.setAttribute("role", "img");
  item.setAttribute("aria-label", sentence);
  item.title = sentence;
  item.style.cssText = `display:flex;align-items:center;gap:6px;background:rgba(0,0,0,0.78);color:${colour};font:600 var(--t-body, 13px) var(--font, sans-serif);padding:4px 10px;border-radius:4px;white-space:nowrap;`;
  const icon = doc.createElement("span");
  icon.style.cssText = "display:inline-block;flex:0 0 auto;width:16px;height:16px;line-height:0;";
  icon.innerHTML = pingGlyphSvg(glyph);
  const words = doc.createElement("span");
  words.textContent = sentence;
  item.append(icon, words);
  return item;
}
