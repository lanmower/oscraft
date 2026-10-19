// Shop panel: every catalog item with cost, stats, recipe and a buy button. The catalog is read once from
// Game.item_catalog_json; Buy calls Game.buy for the selected local hero (the sim refuses it out of shop reach).
// Toggled by the shop action (B and P in moba_ui.json) that the input layer dispatches as "moba:action".
import UI from "./moba_ui.json" with { type: "json" };
import { fill } from "./moba_select.js";

const STYLE = `
#ui-shop{position:fixed;z-index:2;right:8px;top:48px;width:260px;max-height:60vh;overflow:auto;background:rgba(12,14,18,.9);border:1px solid #333;color:#eee;font:12px/1.3 monospace;padding:6px 8px}
#ui-shop h4{margin:0 0 4px;font-size:12px;color:#ffd54f}
#ui-shop .row{display:flex;justify-content:space-between;gap:6px;border-top:1px solid #222;padding:3px 0}
#ui-shop .stats{color:#aaa;font-size:11px}
#ui-shop button{background:#243a24;color:#eee;border:1px solid #4a7;cursor:pointer}
#ui-shop button:disabled{background:#222;color:#666;border-color:#333;cursor:default}
#ui-shop .msg{color:#ff8a65;min-height:14px}
`;

function ensureStyle() {
  if (document.getElementById("ui-shop-style")) return;
  const style = document.createElement("style");
  style.id = "ui-shop-style";
  style.textContent = STYLE;
  document.head.appendChild(style);
}

function el(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

export function itemStats(item) {
  return Object.entries(item.stats ?? { armor: item.armor, damage: item.damage, health: item.health })
    .filter(([, v]) => v)
    .map(([k, v]) => `${v > 0 ? "+" : ""}${v} ${k}`)
    .join(" ");
}

export const SHOP_NAMES = ["home", "side", "secret"];

// Shop kinds that stock the item, from the catalog's shop bit mask.
export function shopText(item) {
  return SHOP_NAMES.filter((_, bit) => item.shops & (1 << bit)).join("/");
}

export function recipeText(item, catalog) {
  return item.components.length ? `recipe: ${item.components.map((c) => (catalog[c] ? catalog[c].name : `#${c}`)).join(" + ")}` : "";
}

export function createShop({ game, selected, localPlayer }) {
  ensureStyle();
  const catalog = JSON.parse(game.item_catalog_json());
  const root = el("div");
  root.id = "ui-shop";
  root.hidden = true;
  const goldLine = el("h4");
  const msg = el("div", "msg");
  root.append(goldLine, msg);
  const filter = el("input");
  filter.placeholder = "filter";
  root.insertBefore(filter, msg);
  const rows = [];
  const buttons = catalog.map((item) => {
    const row = el("div", "row");
    row.hidden = !item.shops;
    rows.push(row);
    const info = el("div");
    info.append(el("div", "", `${item.name}  ${item.cost}g  [${shopText(item)}]`), el("div", "stats", [itemStats(item), recipeText(item, catalog)].filter(Boolean).join("  ")));
    const buy = el("button", "", "Buy");
    buy.onclick = () => {
      const id = selected();
      msg.textContent = id >= 0 && game.buy(id, item.id) ? "" : fill(UI.text.shopRefusal, { item: item.name });
    };
    row.append(info, buy);
    root.appendChild(row);
    return buy;
  });
  filter.oninput = () => {
    const needle = filter.value.trim().toLowerCase();
    catalog.forEach((item, i) => { rows[i].hidden = !item.shops || (needle !== "" && !item.name.includes(needle)); });
  };
  document.body.appendChild(root);
  window.addEventListener("moba:action", (e) => {
    if (e.detail && e.detail.action === "shop") root.hidden = !root.hidden;
  });

  function update() {
    if (root.hidden) return;
    const gold = game.gold(localPlayer());
    goldLine.textContent = fill(UI.text.shopTitle, { gold });
    const has = selected() >= 0;
    catalog.forEach((item, i) => { buttons[i].disabled = !has || gold < item.cost && !item.components.length || !item.shops; });
  }

  return { update, root };
}
