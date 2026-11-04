const CELL_PX = 44;
const CELL_GAP_PX = 6;
const CELL_FACE_RGB = "#1d242c";
const CELL_EDGE_RGB = "rgba(255,255,255,0.35)";
const COOLDOWN_SHADE_RGB = "rgba(0,0,0,0.6)";
const WARN_TEXT_RGB = "#ffd27a";
const DIM_ALPHA = 0.5;
const CELL_FONT = "600 11px sans-serif";
const CANVAS_ID = "hud-canvas-bar";
const STYLE_ID = "hud-canvas-style";
const DOM_CELL_SELECTOR = "[data-cd-unit][data-cd-slot]";
const HIDE_DOM_CELLS_CSS = 'body[data-hud="canvas"] [data-cd-unit][data-cd-slot]{visibility:hidden}';

export function hudCanvasRequested(search) {
  return new URLSearchParams(search).get("hud") === "canvas";
}

export function withModifiers(args, ev) {
  const extra = {};
  if (ev?.altKey) extra.alt = true;
  if (ev?.ctrlKey) extra.ctrl = true;
  return Object.keys(extra).length ? { ...(args ?? {}), ...extra } : args ?? {};
}

function cellOf(node) {
  const kids = node.kids ?? [];
  const bar = kids.find((kid) => kid.t === "bar");
  return {
    slot: node.slot,
    unit: node.unit,
    tone: node.tone ?? null,
    call: node.call ?? null,
    args: node.args ?? {},
    cooldownFraction: bar && bar.max > 0 ? Math.min(1, Math.max(0, bar.value / bar.max)) : 0,
    warn: kids.find((kid) => kid.t === "label" && kid.tone === "warn")?.text ?? null,
  };
}

export function abilityCellsOf(tree) {
  const cells = [];
  const visit = (node) => {
    if (!node || typeof node !== "object") return;
    if (node.t === "cell" && node.unit !== undefined && node.slot !== undefined) cells.push(cellOf(node));
    for (const kid of node.kids ?? []) visit(kid);
  };
  visit(tree);
  return cells;
}

export function layoutRow(count, width, height) {
  if (count <= 0) return [];
  const side = Math.max(1, Math.min(CELL_PX, height, (width - CELL_GAP_PX * (count - 1)) / count));
  const top = (height - side) / 2;
  return Array.from({ length: count }, (_, i) => ({ x: i * (side + CELL_GAP_PX), y: top, w: side, h: side }));
}

export function cellAt(rects, x, y) {
  return rects.findIndex((r) => x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h);
}

export function drawAbilityCells(ctx, cells, rects, icons) {
  let draws = 0;
  cells.forEach((cell, i) => {
    const r = rects[i];
    ctx.save();
    ctx.globalAlpha = cell.tone === "dim" ? DIM_ALPHA : 1;
    ctx.fillStyle = CELL_FACE_RGB;
    ctx.fillRect(r.x, r.y, r.w, r.h);
    draws += 1;
    const icon = icons[i];
    if (icon && icon.complete && icon.naturalWidth > 0) {
      ctx.drawImage(icon, r.x, r.y, r.w, r.h);
      draws += 1;
    }
    if (cell.cooldownFraction > 0) {
      ctx.fillStyle = COOLDOWN_SHADE_RGB;
      ctx.fillRect(r.x, r.y, r.w, r.h * cell.cooldownFraction);
      draws += 1;
    }
    ctx.strokeStyle = CELL_EDGE_RGB;
    ctx.strokeRect(r.x + 0.5, r.y + 0.5, r.w - 1, r.h - 1);
    draws += 1;
    if (cell.warn !== null) {
      ctx.fillStyle = WARN_TEXT_RGB;
      ctx.font = CELL_FONT;
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.fillText(cell.warn, r.x + r.w / 2, r.y + r.h / 2);
      draws += 1;
    }
    ctx.restore();
  });
  return draws;
}

export function createHudCanvas({ send, doc = globalThis.document, win = globalThis.window }) {
  const style = doc.createElement("style");
  style.id = STYLE_ID;
  style.textContent = HIDE_DOM_CELLS_CSS;
  doc.head.appendChild(style);
  doc.body.dataset.hud = "canvas";

  const canvas = doc.createElement("canvas");
  canvas.id = CANVAS_ID;
  canvas.dataset.ui = "hud-canvas";
  canvas.setAttribute("aria-hidden", "true");
  canvas.style.position = "fixed";
  canvas.style.zIndex = "3";
  canvas.style.display = "none";
  doc.body.appendChild(canvas);
  const ctx = canvas.getContext("2d");

  const panels = new Map();
  let cells = [];
  let rects = [];

  function domCells() {
    return Array.from(doc.querySelectorAll(DOM_CELL_SELECTOR));
  }

  function place() {
    const dom = domCells();
    if (cells.length === 0 || dom.length === 0) {
      canvas.style.display = "none";
      rects = [];
      canvas.dataset.cells = "0";
      return 0;
    }
    const boxes = dom.map((node) => node.getBoundingClientRect());
    const left = Math.min(...boxes.map((b) => b.left));
    const top = Math.min(...boxes.map((b) => b.top));
    const width = Math.max(...boxes.map((b) => b.left + b.width)) - left;
    const height = Math.max(...boxes.map((b) => b.top + b.height)) - top;
    const dpr = win.devicePixelRatio || 1;
    canvas.style.display = "block";
    canvas.style.left = `${left}px`;
    canvas.style.top = `${top}px`;
    canvas.style.width = `${width}px`;
    canvas.style.height = `${height}px`;
    canvas.width = Math.round(width * dpr);
    canvas.height = Math.round(height * dpr);
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, width, height);
    rects = layoutRow(cells.length, width, height);
    const icons = cells.map((_, i) => dom[i]?.querySelector("img") ?? null);
    const draws = drawAbilityCells(ctx, cells, rects, icons);
    canvas.dataset.cells = String(cells.length);
    canvas.dataset.draws = String(draws);
    return draws;
  }

  function apply(data) {
    const id = String(data.id);
    if (data.remove || data.tree === undefined || data.tree === null) panels.delete(id);
    else panels.set(id, abilityCellsOf(data.tree));
    cells = [...panels.values()].flat();
    return place();
  }

  canvas.addEventListener("click", (ev) => {
    const box = canvas.getBoundingClientRect();
    const at = cellAt(rects, ev.clientX - box.left, ev.clientY - box.top);
    if (at < 0) return;
    const cell = cells[at];
    if (cell.call !== null) send(cell.call, withModifiers(cell.args, ev));
  });
  win.addEventListener("resize", place);

  return {
    apply,
    dispose() {
      win.removeEventListener("resize", place);
      canvas.remove();
      style.remove();
      delete doc.body.dataset.hud;
    },
  };
}
