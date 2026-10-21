// Shop panel: every catalog item as a card with its name, cost, shop tags, stats, recipe and a buy button. The catalog is
// read once from Game.item_catalog_json; Buy calls Game.buy for the selected local hero (the sim refuses it out of shop
// reach). A card is marked affordable or short of gold, and the header shows the gold the hero holds. Toggled by the
// shop action (B and P in moba_ui.json) that the input layer dispatches as "moba:action".
import UI from "./moba_ui.json" with { type: "json" };
import { fill } from "./moba_select.js";

const STYLE = `
#ui-shop{position:relative;width:360px;max-height:calc(100vh - 290px);display:flex;flex-direction:column;gap:6px;box-sizing:border-box;background:var(--panel);border:1px solid var(--edge);border-radius:6px;color:var(--text);font:var(--t-body)/1.3 var(--font);padding:8px 10px}
#ui-shop .top{display:flex;justify-content:space-between;align-items:center;gap:8px}
#ui-shop h4{margin:0;font-size:var(--t-title);color:var(--accent)}
#ui-shop input{flex:1;min-width:0;max-width:150px;background:var(--well);border:1px solid var(--edge);border-radius:3px;color:var(--text);font:var(--t-label) var(--font);padding:3px 6px}
#ui-shop .grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:6px;overflow:auto;min-height:0}
#ui-shop .card{display:flex;flex-direction:column;gap:3px;background:var(--well);border:1px solid var(--edge);border-radius:4px;padding:6px;min-width:0}
#ui-shop .card:hover{border-color:var(--hover-edge)}
#ui-shop .card[data-afford=no]{opacity:.7}
#ui-shop .card .name{font-weight:700;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
#ui-shop .card .cost{color:var(--gold);font-weight:700;font-variant-numeric:tabular-nums}
#ui-shop .card[data-afford=no] .cost{color:var(--bad)}
#ui-shop .tags{color:var(--faint);font-size:var(--t-label)}
#ui-shop .stats{color:var(--muted);font-size:var(--t-label);min-height:14px}
#ui-shop .recipe{color:var(--faint);font-size:var(--t-label)}
#ui-shop .buy{margin-top:auto;background:var(--ok-bg);color:var(--text);border:1px solid var(--ok);border-radius:3px;padding:3px 6px;font:700 var(--t-label) var(--font);cursor:pointer}
#ui-shop .buy:hover:not(:disabled){background:var(--ok)}
#ui-shop .buy:disabled{background:var(--well-dim);color:var(--faint);border-color:var(--edge);cursor:default}
#ui-shop .msg{color:var(--warn);min-height:14px;font-size:var(--t-label)}
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

// Whether a card reads as affordable: a recipe item is always buyable (its parts are the cost), else gold must cover it.
export function affordable(item, gold) {
  return item.components.length > 0 || gold >= item.cost;
}

export function createShop({ game, selected, localPlayer }) {
  hot.set("ui-shop", { game, selected, localPlayer });
  ensureStyle();
  const catalog = JSON.parse(game.item_catalog_json());
  const root = el("div");
  root.id = "ui-shop";
  root.hidden = true;
  const goldLine = el("h4");
  const filter = el("input");
  filter.placeholder = "filter";
  const top = el("div", "top");
  top.append(goldLine, filter);
  const msg = el("div", "msg");
  const grid = el("div", "grid");
  root.append(top, grid, msg);
  const cards = [];
  const buttons = catalog.map((item) => {
    const card = el("div", "card");
    card.dataset.item = String(item.id);
    cards.push(card);
    const name = el("div", "name", item.name);
    name.title = item.name;
    const cost = el("div", "cost", `${item.cost}g`);
    const tags = el("div", "tags", `[${shopText(item)}]`);
    const stats = el("div", "stats", itemStats(item));
    const recipe = el("div", "recipe", recipeText(item, catalog));
    const buy = el("button", "buy", "Buy");
    buy.onclick = () => {
      const id = selected();
      msg.textContent = id >= 0 && typeof game.buy === "function" && game.buy(id, item.id) ? "" : fill(UI.text.shopRefusal, { item: item.name });
    };
    card.append(name, cost, tags, stats, recipe, buy);
    grid.appendChild(card);
    return buy;
  });
  filter.oninput = () => {
    const needle = filter.value.trim().toLowerCase();
    catalog.forEach((item, i) => { cards[i].hidden = !item.shops || (needle !== "" && !item.name.toLowerCase().includes(needle)); });
  };
  catalog.forEach((item, i) => { cards[i].hidden = !item.shops; });
  const host = document.getElementById("ui-right") || document.body;
  host.appendChild(root);
  window.addEventListener("moba:action", (e) => {
    if (e.detail && e.detail.action === "shop") root.hidden = !root.hidden;
  });

  function update() {
    if (root.hidden) return;
    const gold = game.gold(localPlayer());
    goldLine.textContent = fill(UI.text.shopTitle, { gold });
    const has = selected() >= 0;
    catalog.forEach((item, i) => {
      buttons[i].disabled = !has || gold < item.cost && !item.components.length || !item.shops;
      cards[i].dataset.afford = affordable(item, gold) ? "yes" : "no";
    });
  }

  return { update, root };
}

// Hot-update hook (see hmr.js): replaces the #ui-shop root with a fresh one built from the inputs of the last createShop call
// (same id, same host). The new panel is the hook value for its owner; listeners of the old panel are not detached.
const hot = (globalThis.__oscraftHot ??= new Map());
export function hmrReload() {
  const args = hot.get("ui-shop");
  if (!args) return null;
  document.getElementById("ui-shop")?.remove();
  return createShop(args);
}
