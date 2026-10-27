
const SLOT_HOSTS = {
  left: "ui-left",
  hud: "hud",
  win: "win",
  right: "ui-right",
  "bar-left": "ui-bar-left",
  "bar-mid": "ui-bar-mid",
};
const HIDE_WHEN_EMPTY = new Set(["win"]);

const STYLE = `
.w-overlay{position:fixed;z-index:2;pointer-events:none;color:var(--text);font:var(--t-label)/1.25 var(--font)}
.w-overlay>.w-root{pointer-events:auto}
#ui-top{top:8px;left:50%;transform:translateX(-50%)}
#ui-centre{left:50%;top:50%;transform:translate(-50%,-50%)}
#ui-banner{top:14%;left:50%;transform:translateX(-50%)}
#ui-modal{left:50%;top:50%;transform:translate(-50%,-50%);width:min(960px,96vw);max-height:96vh;max-height:96dvh;overflow:auto;box-sizing:border-box;pointer-events:auto}
.w-root{display:flex;flex-direction:column;gap:6px;min-width:0;box-sizing:border-box;color:var(--text);font:var(--t-label)/1.25 var(--font)}
.w-root>.w-panel{background:var(--panel);border:1px solid var(--edge);border-radius:6px;padding:8px 10px}
.w-panel{display:flex;flex-direction:column;gap:4px;min-width:0}
.w-panel>h4{margin:0;font-size:var(--t-title);color:var(--accent)}
.w-box{display:flex;flex-direction:column;gap:3px;min-width:0}
.w-box.row{flex-direction:row;flex-wrap:wrap;align-items:center;gap:4px}
.w-label{min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.w-bar{height:8px;min-width:40px;background:var(--well);border-radius:2px;overflow:hidden}
.w-bar i{display:block;height:100%;background:var(--bar,var(--accent))}
.w-button{background:var(--ok-bg);color:var(--text);border:1px solid var(--ok);border-radius:3px;min-width:30px;padding:2px 6px;font:700 var(--t-label) var(--font);cursor:pointer}
.w-button:hover:not(:disabled){background:var(--ok)}
.w-button:disabled{background:var(--well-dim);color:var(--faint);border-color:var(--edge);cursor:default}
.w-grid{display:grid;gap:4px;align-content:start;min-width:0}
.w-cell{position:relative;min-width:0;min-height:34px;border:1px solid var(--edge);border-radius:3px;background:var(--well);padding:3px 4px;box-sizing:border-box;display:flex;flex-direction:column;justify-content:center;gap:2px;overflow:hidden}
.w-cell.clickable{cursor:pointer}
.w-cell.clickable:hover{border-color:var(--hover-edge);background:var(--well-hi)}
.w-cell.clickable:focus-visible,.w-button:focus-visible,.w-panel-toggle:focus-visible{outline:2px solid var(--sel);outline-offset:2px}
.w-panel-toggle{display:block;width:100%;margin:0;padding:0;border:0;background:none;color:var(--accent);font:inherit;font-size:var(--t-title);font-weight:700;text-align:left;cursor:pointer}
.w-panel-toggle::before{content:"- "}
.w-panel-toggle[aria-expanded="false"]::before{content:"+ "}
.w-panel-body{display:flex;flex-direction:column;gap:4px;min-width:0}
.w-panel-body[hidden]{display:none}
.w-cell.t-empty{background:var(--well-dim);border-style:dashed;opacity:.55}
.w-cell.t-active{border-color:var(--sel);box-shadow:inset 0 0 0 1px var(--sel)}
.w-cell.t-face{width:56px;min-height:56px;align-items:center}
.t-strong{font-weight:700}
.t-accent{color:var(--accent)}
.t-gold{color:var(--gold);font-weight:700;font-variant-numeric:tabular-nums}
.t-bad{color:var(--bad)}
.t-ok{color:var(--ok)}
.t-warn{color:var(--warn)}
.t-muted{color:var(--muted)}
.t-faint{color:var(--faint)}
.t-mp{color:#8fb4ff;font-weight:700;font-variant-numeric:tabular-nums}
.t-me{color:var(--text);font-weight:700}
.t-team0{color:var(--team0)}
.t-team1{color:var(--team1)}
.w-cell.t-ok{background:var(--well);border-color:var(--ok)}
.w-cell.t-bad{border-color:var(--bad)}
.w-cell.t-stash{background:var(--well-dim);border-style:dashed}
.w-cell.t-muted{opacity:.8}
.w-bar.t-hp{--bar:var(--hp)}
.w-bar.t-mp{--bar:var(--mp)}
.w-root .w-bar{min-height:4px}
#ui-full{top:0;right:0;bottom:0;left:0}
#ui-full>.w-root{width:100%;height:100%}
.w-regions{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));gap:8px;box-sizing:border-box;width:100%;height:100%;min-height:min(64vh,640px);min-height:min(64dvh,640px);padding:8px}
.w-regions.col{grid-template-columns:minmax(0,1fr);grid-template-rows:repeat(3,minmax(0,1fr))}
.w-region{display:flex;flex-direction:column;gap:6px;min-width:0;min-height:0;overflow:auto}
.w-region>.w-box{flex:1 1 auto;min-height:0}
.w-portraits{display:grid;gap:4px;align-content:start;justify-content:start;min-width:0;max-width:100%}
.w-portrait.t-faint img{opacity:.45}
.w-portrait{padding:2px;align-items:center;justify-content:flex-start;overflow:hidden}
.w-portrait img{display:block;flex:1 1 auto;min-height:0;width:100%;height:auto;object-fit:contain}
.w-portrait-badge{flex:0 0 auto;align-self:stretch;text-align:center;overflow:hidden;pointer-events:none;user-select:none;color:var(--text);opacity:.7;font-weight:700;line-height:1.1;letter-spacing:.04em;white-space:nowrap}
.w-portraits.fit{width:100%;height:100%;flex:1 1 0;min-height:0}
.w-swipe{display:flex;flex-direction:column;width:100%;height:100%;min-height:0}
.w-swipe>.w-regions{flex:1 1 auto;height:auto}
.w-region:focus{outline:none}
.w-region-title{display:block;flex:0 0 auto;width:100%;margin:0;padding:2px 4px;border:0;border-bottom:2px solid var(--edge);border-radius:0;background:none;color:var(--accent);font:700 var(--t-title)/1.2 var(--font);text-align:center;cursor:pointer}
.w-region-title[aria-pressed="true"]{border-bottom-color:var(--sel);color:var(--text)}
.w-region-title:focus-visible{outline:2px solid var(--sel);outline-offset:2px}
#ui-modal:has(.w-regions){display:flex;flex-direction:column;height:96vh;height:96dvh}
#ui-modal:has(.w-regions)>.w-root,#ui-modal:has(.w-regions)>.w-root>.w-panel{flex:1 1 auto;min-height:0}
#ui-modal .w-panel>.w-box.row:has(.w-regions),#ui-modal .w-panel>.w-box.row:has(.w-swipe){flex:1 1 auto;min-height:0;align-items:stretch}
#ui-modal .w-box.row>.w-box:has(>.w-regions),#ui-modal .w-box.row>.w-box:has(>.w-swipe){flex:1 1 360px;min-width:0}
#ui-modal .w-box:has(>.w-regions)>.w-regions,#ui-modal .w-box:has(>.w-swipe)>.w-swipe,#ui-modal .w-swipe>.w-regions{flex:1 1 auto;min-height:0}
@media (max-width:640px){
#ui-modal .w-panel>.w-box.row:has(.w-swipe),#ui-modal .w-box:has(>.w-swipe)>.w-swipe{min-height:auto}
#ui-modal .w-swipe>.w-regions{min-height:min(60dvh,520px)}
}
.w-dots{display:none;justify-content:center;gap:6px;padding:4px 0}
.w-dot{width:8px;height:8px;border-radius:50%;background:var(--muted);opacity:.4}
.w-dot.on{opacity:1;background:var(--text)}
@media (max-width:640px){
.w-regions.swipe{display:flex;gap:0;overflow-x:auto;overflow-y:hidden;scroll-snap-type:x mandatory;touch-action:pan-x}
.w-regions.swipe>.w-region{flex:0 0 100%;box-sizing:border-box;scroll-snap-align:start;scroll-snap-stop:always}
.w-swipe>.w-dots{display:flex}
}
.w-rows{display:flex;flex-direction:column;gap:2px;min-width:0}
.w-row{display:flex;justify-content:space-between;align-items:baseline;gap:8px;min-width:0}
.w-row-label{min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;color:var(--muted)}
.w-row-value{flex:0 0 auto;font-weight:700;font-variant-numeric:tabular-nums;white-space:nowrap}
.w-statbar{display:grid;grid-template-columns:auto minmax(40px,1fr) auto;align-items:center;gap:6px;min-width:0}
.w-statbar-value{font-variant-numeric:tabular-nums;white-space:nowrap}
.w-model{position:relative;flex:0 0 auto;overflow:hidden;border-radius:3px;background:var(--well)}
.w-model img,.w-model canvas{display:block;width:100%;height:100%;object-fit:contain}
@media (max-width: 600px){
#ui-left{max-height:calc(100vh - 170px);max-height:calc(100dvh - 170px);overflow-y:auto}
#ui-right{max-height:calc(100vh - 473px);max-height:calc(100dvh - 473px);overflow-y:auto}
#ui-bar-mid .w-cell{min-height:32px}
.w-panel-toggle{min-height:32px}
.w-button{min-height:32px;padding:4px 8px}
.w-label,.w-button,.w-row,.w-statbar{font-size:max(11px,var(--t-label))}
#ui-top .w-box:has(> .t-gold){display:grid;grid-template-columns:1fr 1fr;column-gap:4px}
#ui-top .t-gold{font-size:11px}
#ui-modal{top:auto;bottom:124px;transform:translateX(-50%);max-height:calc(100vh - 140px);max-height:calc(100dvh - 140px)}
}
`;

function ensureStyle() {
  if (document.getElementById("widgets-style")) return;
  const style = document.createElement("style");
  style.id = "widgets-style";
  style.textContent = STYLE;
  document.head.appendChild(style);
}

function el(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = String(text);
  return node;
}

const percent = (value, max) => (max > 0 ? Math.max(0, Math.min(100, (100 * (value || 0)) / max)) : 0);

let srcResolver = (src) => src;
export function setImageSrcResolver(fn) { srcResolver = fn; }
let previewRigSource = null;
export function setPreviewRig(fn) { previewRigSource = fn; }
function resolveSrc(src) {
  if (/^(https?:|data:|blob:)/.test(src)) return src;
  return srcResolver(src);
}
function setImageSrc(img, src) {
  const url = resolveSrc(src);
  if (typeof url === "string") img.src = url;
  else url.then((resolved) => { img.src = resolved; });
}

const PHONE = typeof matchMedia === "function" ? matchMedia("(max-width: 600px)") : null;
const FOLDABLE_SLOTS = new Set(["left", "right"]);
const FOLDED_AT_START = new Set(["left"]);
const folded = new Map();

function foldablePanel(n, send, panel) {
  if (!folded.has(panel.id)) folded.set(panel.id, FOLDED_AT_START.has(panel.slot));
  const startFolded = folded.get(panel.id);
  const toggle = el("button", "w-panel-toggle", n.title);
  toggle.type = "button";
  toggle.setAttribute("aria-expanded", String(!startFolded));
  const body = el("div", "w-panel-body");
  body.hidden = startFolded;
  for (const kid of n.kids ?? []) body.append(buildNode(kid, send));
  toggle.onclick = () => {
    const nowFolded = !body.hidden;
    body.hidden = nowFolded;
    folded.set(panel.id, nowFolded);
    toggle.setAttribute("aria-expanded", String(!nowFolded));
  };
  const head = el("h4");
  head.append(toggle);
  const section = el("section", "w-panel w-foldable");
  section.append(head, body);
  return section;
}

const SWIPE_PHONE = typeof matchMedia === "function" ? matchMedia("(max-width: 640px)") : null;
const regionPicksByPanelId = new Map();

function regionsOf(n, send) {
  const swipe = n.swipe === true && n.dir !== "column";
  const e = el("div", n.dir === "column" ? "w-regions col" : swipe ? "w-regions swipe" : "w-regions");
  const titles = Array.isArray(n.titles) ? n.titles : [];
  const buttons = [];
  let picked = -1;
  const setPicked = (i) => {
    picked = i;
    buttons.forEach((b, k) => b.setAttribute("aria-pressed", String(k === i)));
    if (n.id) regionPicksByPanelId.set(n.id, i);
  };
  const pick = (i) => {
    if (swipe && SWIPE_PHONE?.matches) e.scrollTo({ left: i * e.clientWidth, behavior: "smooth" });
    else {
      const region = e.children[i];
      region.tabIndex = -1;
      region.focus({ preventScroll: true });
    }
    setPicked(i);
  };
  for (let i = 0; i < 3; i++) {
    const region = el("div", "w-region");
    if (titles[i] !== undefined) {
      const title = el("button", "w-region-title", titles[i]);
      title.type = "button";
      title.setAttribute("aria-label", titles[i]);
      title.setAttribute("aria-pressed", "false");
      title.onclick = () => pick(i);
      buttons.push(title);
      region.append(title);
    }
    if (n.kids && n.kids[i]) region.append(buildNode(n.kids[i], send));
    e.append(region);
  }
  if (n.id && regionPicksByPanelId.has(n.id)) setPicked(regionPicksByPanelId.get(n.id));
  if (!swipe) return e;
  const wrap = el("div", "w-swipe");
  const dots = el("div", "w-dots");
  const marks = Array.from(e.children, () => dots.appendChild(el("span", "w-dot")));
  const show = () => {
    const at = Math.max(0, Math.min(marks.length - 1, Math.round(e.scrollLeft / Math.max(1, e.clientWidth))));
    marks.forEach((d, i) => d.classList.toggle("on", i === at));
    return at;
  };
  e.addEventListener("scroll", () => {
    const at = show();
    if (at !== picked) setPicked(at);
  }, { passive: true });
  const at = show();
  if (picked < 0 && SWIPE_PHONE?.matches) setPicked(at);
  wrap.append(e, dots);
  return wrap;
}

const BADGE_EM_PER_LETTER = 0.94;
const BADGE_FILL = 0.9;
const BADGE_MAX_EM_OF_WIDTH = 0.4;
const PORTRAIT_PAD = 4;

function badgeFontPx(tilePx, letters) {
  const inner = Math.max(1, tilePx - PORTRAIT_PAD);
  const fitted = (inner * BADGE_FILL) / (Math.max(1, letters) * BADGE_EM_PER_LETTER);
  return Math.max(6, Math.floor(Math.min(fitted, inner * BADGE_MAX_EM_OF_WIDTH)));
}

function portraitGrid(n, send) {
  const fit = n.fit === true;
  const e = el("div", fit ? "w-portraits fit" : "w-portraits");
  const px = Math.max(16, Math.min(256, n.size || 56));
  e.style.gridTemplateColumns = "repeat(" + Math.max(1, n.cols || 1) + ", minmax(0, " + px + "px))";
  for (const item of n.items ?? []) {
    const cell = el("div", item.call ? "w-cell w-portrait clickable" : "w-cell w-portrait");
    cell.style.aspectRatio = "1";
    cell.style.maxWidth = px + "px";
    cell.style.minHeight = "0";
    if (item.tone) cell.classList.add("t-" + item.tone);
    if (item.selected === true) cell.classList.add("t-active");
    if (item.badge) {
      cell.classList.add("has-badge");
      const badge = el("span", "w-portrait-badge", String(item.badge).toUpperCase());
      badge.style.fontSize = badgeFontPx(px, badge.textContent.length) + "px";
      cell.append(badge);
    }
    if (item.src) {
      const img = document.createElement("img");
      setImageSrc(img, item.src);
      img.alt = item.alt ?? item.text ?? "";
      img.loading = "lazy";
      img.draggable = false;
      img.onerror = () => { img.remove(); };
      cell.append(img);
    }
    if (item.text) cell.append(el("span", "w-label", item.text));
    if (item.tip) cell.title = item.tip;
    if (item.call) {
      cell.setAttribute("aria-pressed", String(item.selected === true));
      actionable(cell, send, item.call, item.args ?? {});
    }
    e.append(cell);
  }
  if (fit && typeof ResizeObserver === "function") observeFit(e, n.cols);
  return e;
}

function observeFit(grid, maxCols) {
  const gap = 4;
  const lay = () => {
    const cells = grid.querySelectorAll(".w-portrait");
    const w = grid.clientWidth, h = grid.clientHeight, count = cells.length;
    if (!w || !h || !count) return;
    let best = { s: 0, c: 1 };
    for (let c = 1; c <= Math.min(count, Math.max(1, maxCols || count)); c++) {
      const rows = Math.ceil(count / c);
      const s = Math.min((w - gap * (c - 1)) / c, (h - gap * (rows - 1)) / rows);
      if (s > best.s) best = { s, c };
    }
    const s = Math.max(16, Math.floor(best.s));
    grid.style.gridTemplateColumns = "repeat(" + best.c + ", " + s + "px)";
    for (const cell of cells) {
      cell.style.maxWidth = s + "px";
      const badge = cell.querySelector(".w-portrait-badge");
      if (badge) badge.style.fontSize = badgeFontPx(s, badge.textContent.length) + "px";
    }
  };
  new ResizeObserver(lay).observe(grid);
}

function actionable(e, send, call, args) {
  e.tabIndex = 0;
  e.setAttribute("role", "button");
  e.onclick = () => send(call, args);
  e.onkeydown = (ev) => {
    if (ev.key !== "Enter" && ev.key !== " ") return;
    ev.preventDefault();
    e.click();
  };
}

function rowGroup(n) {
  const e = el("div", "w-rows");
  for (const row of n.rows ?? []) {
    const line = el("div", "w-row");
    const value = el("span", "w-row-value", row.value ?? "");
    if (row.tone) value.classList.add("t-" + row.tone);
    line.append(el("span", "w-row-label", row.label ?? ""), value);
    if (row.tip) line.title = row.tip;
    e.append(line);
  }
  return e;
}

function statBar(n) {
  const e = el("div", "w-statbar");
  const bar = el("div", "w-bar");
  const fill = el("i");
  fill.style.width = percent(n.value, n.max) + "%";
  bar.append(fill);
  if (n.tone) bar.classList.add("t-" + n.tone);
  const text = n.text ?? (n.max > 0 ? (n.value ?? 0) + "/" + n.max : String(n.value ?? 0));
  e.append(el("span", "w-label", n.label ?? ""), bar, el("span", "w-statbar-value", text));
  if (n.tip) e.title = n.tip;
  return e;
}

const loadedModels = new Set();
const previews = new Map();
const PREVIEW_PX = 256;

if (typeof window !== "undefined") {
  window.addEventListener("oscraft:model-loaded", (e) => {
    const model = e.detail?.model;
    if (typeof model !== "string" || loadedModels.has(model)) return;
    loadedModels.add(model);
    for (const node of document.querySelectorAll(".w-model")) {
      if (node.dataset.model === model) showPreview(node);
    }
  });
}

function modelNode(n) {
  const px = Math.max(16, Math.min(256, n.size || 56));
  const e = el("div", "w-model");
  e.dataset.model = String(n.model);
  e.dataset.px = String(px);
  e.style.width = px + "px";
  e.style.height = px + "px";
  if (n.tip) e.title = n.tip;
  if (n.src) {
    const img = document.createElement("img");
    setImageSrc(img, n.src);
    img.alt = n.alt ?? String(n.model);
    img.loading = "lazy";
    img.draggable = false;
    img.onerror = () => { img.remove(); };
    e.append(img);
  }
  if (loadedModels.has(e.dataset.model)) showPreview(e);
  return e;
}

async function showPreview(node) {
  if (node.dataset.previewing === "1") return;
  node.dataset.previewing = "1";
  const bitmap = await previewOf(node.dataset.model);
  if (!bitmap) {
    node.dataset.previewing = "";
    return;
  }
  const px = Number(node.dataset.px) || 56;
  const dpr = Math.min(2, window.devicePixelRatio || 1);
  const canvas = el("canvas", "w-model-canvas");
  canvas.width = Math.round(px * dpr);
  canvas.height = Math.round(px * dpr);
  canvas.getContext("2d").drawImage(bitmap, 0, 0, canvas.width, canvas.height);
  node.replaceChildren(canvas);
}

function previewOf(model) {
  let job = previews.get(model);
  if (!job) {
    job = renderPreview(model).catch((err) => {
      console.warn(`[model] preview ${model}: ${err && err.message ? err.message : err}`);
      return null;
    });
    previews.set(model, job);
  }
  return job;
}

async function renderPreview(model) {
  const render3d = await import("./render3d.js");
  const canvas = document.createElement("canvas");
  canvas.width = PREVIEW_PX;
  canvas.height = PREVIEW_PX;
  if (!(await render3d.drawModelPreview(canvas, (gl) => previewRigSource(model, gl)))) throw new Error("no WebGL2 context");
  return createImageBitmap(canvas);
}

export function buildNode(n, send, panel = null) {
  if (n.t === "panel" && panel && n.title && PHONE?.matches && FOLDABLE_SLOTS.has(panel.slot)) return foldablePanel(n, send, panel);
  let e;
  switch (n.t) {
    case "panel":
      e = el("section", "w-panel");
      if (n.title) e.append(el("h4", "", n.title));
      break;
    case "box":
      e = el("div", n.dir === "row" ? "w-box row" : "w-box");
      break;
    case "label":
      e = el("span", "w-label", n.text ?? "");
      break;
    case "bar":
      e = el("div", "w-bar");
      e.append(el("i"));
      e.firstChild.style.width = `${percent(n.value, n.max)}%`;
      break;
    case "button":
      e = el("button", "w-button", n.text ?? "");
      e.type = "button";
      e.disabled = n.disabled === true;
      if (n.call) e.onclick = () => send(n.call, n.args ?? {});
      break;
    case "grid":
      e = el("div", "w-grid");
      e.style.gridTemplateColumns = `repeat(${Math.max(1, n.cols || 1)}, minmax(0, auto))`;
      break;
    case "image": {
      e = el("div", "w-image");
      const px = Math.max(1, Math.min(512, n.size || 64));
      e.style.width = `${px}px`;
      e.style.height = `${px}px`;
      if (n.src) {
        const img = document.createElement("img");
        setImageSrc(img, n.src);
        img.width = px;
        img.height = px;
        img.alt = n.alt ?? "";
        img.loading = "lazy";
        img.draggable = false;
        img.onerror = () => { img.remove(); };
        e.append(img);
      }
      break;
    }
    case "cell":
      e = el("div", n.call ? "w-cell clickable" : "w-cell");
      if (n.call) {
        e.tabIndex = 0;
        e.setAttribute("role", "button");
        e.onclick = () => send(n.call, n.args ?? {});
        e.onkeydown = (ev) => {
          if (ev.key !== "Enter" && ev.key !== " ") return;
          ev.preventDefault();
          e.click();
        };
      }
      break;
    case "regions":
      return regionsOf(n, send);
    case "portraits":
      return portraitGrid(n, send);
    case "rows":
      return rowGroup(n);
    case "statbar":
      return statBar(n);
    case "model":
      return modelNode(n);
    default:
      e = el("div", "w-unknown");
  }
  if (n.tone) e.classList.add(`t-${n.tone}`);
  if (n.tip) e.title = n.tip;
  for (const kid of n.kids ?? []) e.append(buildNode(kid, send));
  return e;
}

function hostFor(slot) {
  const named = SLOT_HOSTS[slot];
  if (named) return document.getElementById(named) || document.body;
  const id = `ui-${slot}`;
  let host = document.getElementById(id);
  if (!host) {
    host = el("div", `w-overlay w-slot-${slot}`);
    host.id = id;
    document.body.appendChild(host);
  }
  return host;
}

let modalOpener = null;
function labelModal(host, root, opening) {
  host.setAttribute("role", "dialog");
  host.setAttribute("aria-modal", "true");
  const title = root.querySelector("h4");
  if (title) {
    title.id = "ui-modal-title";
    host.setAttribute("aria-labelledby", title.id);
  }
  if (!opening) return;
  modalOpener = document.activeElement;
  host.tabIndex = -1;
  host.focus({ preventScroll: true });
}
function closeModal(host) {
  host.removeAttribute("aria-labelledby");
  if (modalOpener?.isConnected && modalOpener.focus) modalOpener.focus({ preventScroll: true });
  modalOpener = null;
}

export function createWidgets({ send }) {
  ensureStyle();
  const roots = new Map();

  const SCROLLERS = ".w-region,.w-panel,.w-portraits,.w-panel-body";
  const SWIPES = ".w-regions.swipe";
  function rebuild(entry) {
    const root = entry.root;
    const active = document.activeElement;
    let path = null;
    if (active && active !== root && root.contains(active)) {
      path = [];
      for (let n = active; n !== root; n = n.parentElement) path.unshift(Array.prototype.indexOf.call(n.parentElement.children, n));
    }
    const tops = Array.from(root.querySelectorAll(SCROLLERS), (n) => n.scrollTop);
    const lefts = Array.from(root.querySelectorAll(SWIPES), (n) => n.scrollLeft);
    const hostTop = entry.host.scrollTop;
    root.replaceChildren(buildNode(entry.tree, send, entry.panel));
    Array.from(root.querySelectorAll(SCROLLERS)).forEach((n, i) => { if (tops[i]) n.scrollTop = tops[i]; });
    Array.from(root.querySelectorAll(SWIPES)).forEach((n, i) => { if (lefts[i]) n.scrollLeft = lefts[i]; });
    entry.host.scrollTop = hostTop;
    if (!path) return;
    let n = root;
    for (const i of path) { n = n?.children[i]; }
    if (n && n !== root && n.focus) n.focus({ preventScroll: true });
  }

  PHONE?.addEventListener?.("change", () => {
    for (const entry of roots.values()) rebuild(entry);
  });

  function apply(data) {
    const id = String(data.id);
    const open = roots.get(id);
    if (data.remove || data.tree === undefined || data.tree === null) {
      if (open) {
        open.root.remove();
        roots.delete(id);
        if (open.host.id === "ui-modal") closeModal(open.host);
        if (HIDE_WHEN_EMPTY.has(open.host.id)) open.host.hidden = true;
      }
      return;
    }
    const entry = open || { host: hostFor(data.slot), root: el("div", "w-root"), sig: null, tree: null, panel: null };
    entry.root.dataset.id = id;
    entry.panel = { id, slot: data.slot };
    const sig = JSON.stringify(data.tree);
    if (entry.sig !== sig) {
      entry.tree = data.tree;
      rebuild(entry);
      entry.sig = sig;
    }
    const opening = !entry.root.isConnected;
    if (opening) entry.host.appendChild(entry.root);
    if (entry.host.id === "ui-modal") labelModal(entry.host, entry.root, opening);
    if (HIDE_WHEN_EMPTY.has(entry.host.id)) entry.host.hidden = false;
    roots.set(id, entry);
  }

  return { apply };
}
