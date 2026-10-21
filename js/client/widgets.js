// Generic widget renderer. The pack sends a widget tree for each panel as the UI event "widget" ({player, id, slot, tree}),
// or {player, id, remove: true} to close it; the lobby's start comes as "lobby". This file draws the tree into the page slot
// it names and sends a button press back to the pack as the "ui" action ({call, args}). It decides nothing about the game:
// what a panel shows, and what a button does, is the pack's (panels.lua).
// A node: {t: "panel"|"box"|"label"|"bar"|"button"|"grid"|"cell"|"input", text, title, tone, value, max, cols, dir, call, args,
// disabled, tip, kids}. A tone becomes the class t-<tone>; the stylesheet below gives the tones their colours.

// Page slots: the element a panel's slot names. Overlay slots ("top", "centre", "modal") are created on first use.
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
#ui-modal{left:50%;top:50%;transform:translate(-50%,-50%);width:min(960px,96vw);max-height:96vh;overflow:auto;box-sizing:border-box;pointer-events:auto}
.w-root{display:flex;flex-direction:column;gap:4px;min-width:0;box-sizing:border-box;color:var(--text);font:var(--t-label)/1.25 var(--font)}
.w-root>.w-panel{background:var(--panel);border:1px solid var(--edge);border-radius:6px;padding:8px 10px}
.w-panel{display:flex;flex-direction:column;gap:4px;min-width:0}
.w-panel>h4{margin:0 0 4px;font-size:var(--t-title);color:var(--accent)}
.w-box{display:flex;flex-direction:column;gap:3px;min-width:0}
.w-box.row{flex-direction:row;flex-wrap:wrap;align-items:center;gap:6px}
.w-label{min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.w-bar{height:8px;min-width:40px;background:var(--well);border-radius:2px;overflow:hidden}
.w-bar i{display:block;height:100%;background:var(--bar,var(--accent))}
.w-button{background:var(--ok-bg);color:var(--text);border:1px solid var(--ok);border-radius:3px;min-width:30px;padding:2px 6px;font:700 var(--t-label) var(--font);cursor:pointer}
.w-button:hover:not(:disabled){background:var(--ok)}
.w-button:disabled{background:var(--well-dim);color:var(--faint);border-color:var(--edge);cursor:default}
.w-input{min-width:0;box-sizing:border-box;background:var(--well);color:var(--text);border:1px solid var(--edge);border-radius:3px;padding:2px 6px;font:var(--t-label) var(--font)}
.w-grid{display:grid;gap:4px;align-content:start;min-width:0}
.w-cell{position:relative;min-width:0;min-height:34px;border:1px solid var(--edge);border-radius:3px;background:var(--well);padding:3px 4px;box-sizing:border-box;display:flex;flex-direction:column;justify-content:center;gap:2px;overflow:hidden}
.w-cell.clickable{cursor:pointer}
.w-cell.clickable:hover{border-color:var(--hover-edge);background:var(--well-hi)}
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

// One node as DOM. Buttons and clickable cells send their call through `send`.
export function buildNode(n, send) {
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
    case "input":
      e = el("input", "w-input");
      e.type = "text";
      e.placeholder = n.text ?? "";
      e.value = n.value ?? "";
      if (n.call) e.onchange = () => send(n.call, { ...(n.args ?? {}), text: e.value });
      break;
    case "grid":
      e = el("div", "w-grid");
      e.style.gridTemplateColumns = `repeat(${Math.max(1, n.cols || 1)}, minmax(0, auto))`;
      break;
    case "cell":
      e = el("div", n.call ? "w-cell clickable" : "w-cell");
      if (n.call) e.onclick = () => send(n.call, n.args ?? {});
      break;
    default:
      e = el("div", "w-unknown");
  }
  if (n.tone) e.classList.add(`t-${n.tone}`);
  if (n.tip) e.title = n.tip;
  for (const kid of n.kids ?? []) e.append(buildNode(kid, send));
  return e;
}

// The host element of a slot: an existing page element, or an overlay created once and kept.
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

export function createWidgets({ send }) {
  ensureStyle();
  const roots = new Map(); // panel id -> { host, root }

  // Draws one panel's tree, or closes it when the event has no tree.
  function apply(data) {
    const id = String(data.id);
    const open = roots.get(id);
    if (data.remove || data.tree === undefined || data.tree === null) {
      if (open) {
        open.root.remove();
        roots.delete(id);
        if (HIDE_WHEN_EMPTY.has(open.host.id)) open.host.hidden = true;
      }
      return;
    }
    const entry = open || { host: hostFor(data.slot), root: el("div", "w-root") };
    entry.root.dataset.id = id;
    entry.root.replaceChildren(buildNode(data.tree, send));
    if (!entry.root.isConnected) entry.host.appendChild(entry.root);
    if (HIDE_WHEN_EMPTY.has(entry.host.id)) entry.host.hidden = false;
    roots.set(id, entry);
  }

  return { apply };
}
