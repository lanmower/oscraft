// Item panel for the selected local hero: ten inventory slots (six main, three backpack, one neutral) and a nine-slot
// stash. Item names come from Game.item_catalog_json, slots from Game.inventory_json. Click an inventory slot
// to sell it, click a stash slot to move it into the inventory (both need a shop in reach). The portrait and bars
// are in the selection panel (ui_selection.js). Labels come from moba_ui.json.
import UI from "./moba_ui.json" with { type: "json" };

const STYLE = `
#ui-inv{position:fixed;left:50%;bottom:130px;transform:translateX(-50%);display:flex;gap:10px;align-items:flex-end;background:rgba(12,14,18,.9);border:1px solid #333;color:#eee;font:11px/1.3 monospace;padding:6px 8px}
#ui-inv .grid{display:grid;grid-template-columns:repeat(5,44px);gap:3px}
#ui-inv .cell{height:36px;border:1px solid #555;background:#1b1f26;display:flex;align-items:center;justify-content:center;text-align:center;overflow:hidden;font-size:10px;cursor:pointer}
#ui-inv .cell.stash{background:#161616;border-style:dashed}
#ui-inv .label{color:#aaa;margin:0 0 2px}
`;

function ensureStyle() {
  if (document.getElementById("ui-inv-style")) return;
  const style = document.createElement("style");
  style.id = "ui-inv-style";
  style.textContent = STYLE;
  document.head.appendChild(style);
}

function el(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

// Item name for a slot kind (catalog index plus one); empty for kind 0.
export function slotLabel(kind, catalog) {
  return kind > 0 && catalog[kind - 1] ? catalog[kind - 1].name : "";
}

export function createInventory({ game, selected, localPlayer }) {
  ensureStyle();
  const catalog = JSON.parse(game.item_catalog_json());
  const root = el("div");
  root.id = "ui-inv";
  root.hidden = true;
  const invWrap = el("div");
  const grid = el("div", "grid");
  const stashGrid = el("div", "grid");
  invWrap.append(el("div", "label", UI.text.inventoryLabel), grid, el("div", "label", UI.text.stashLabel), stashGrid);
  const cells = Array.from({ length: 10 }, (_, s) => {
    const c = el("div", "cell");
    c.onclick = () => game.sell(selected(), s);
    grid.appendChild(c);
    return c;
  });
  const stashCells = Array.from({ length: 9 }, (_, s) => {
    const c = el("div", "cell stash");
    c.onclick = () => game.unstash(selected(), s);
    stashGrid.appendChild(c);
    return c;
  });
  root.append(invWrap);
  document.body.appendChild(root);

  function update() {
    const id = selected();
    const hero = id >= 0 && game.attributes(id).some((v) => v > 0);
    root.hidden = !hero;
    if (!hero) return;
    const inv = JSON.parse(game.inventory_json(id));
    inv.slots.forEach(([kind], s) => { cells[s].textContent = slotLabel(kind, catalog); });
    inv.stash.forEach(([kind], s) => { if (stashCells[s]) stashCells[s].textContent = slotLabel(kind, catalog); });
  }

  return { update, root };
}
