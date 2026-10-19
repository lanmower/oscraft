// Item panel for the selected local hero, the centre block of the bottom bar: ten inventory slots (six main, three
// backpack, one neutral) beside a nine-slot stash. Item names come from Game.item_catalog_json, slots from
// Game.inventory_json. Click an inventory slot to sell it, click a stash slot to move it into the inventory (both need a
// shop in reach). A filled slot shows its item name and the full name on hover; an empty one is dimmed.
import UI from "./moba_ui.json" with { type: "json" };

const STYLE = `
#ui-inv{display:flex;gap:12px;align-items:flex-start;height:100%;box-sizing:border-box;color:var(--text);font:var(--t-label)/1.2 var(--font)}
#ui-inv .block{display:flex;flex-direction:column;gap:4px}
#ui-inv .label{color:var(--muted);text-transform:uppercase;letter-spacing:.06em;font-size:var(--t-label)}
#ui-inv .grid{display:grid;gap:4px}
#ui-inv .grid.main{grid-template-columns:repeat(5,48px)}
#ui-inv .grid.stash{grid-template-columns:repeat(3,48px)}
#ui-inv .cell{height:40px;border:1px solid var(--edge);border-radius:3px;background:var(--well);display:flex;align-items:center;justify-content:center;text-align:center;overflow:hidden;padding:2px 3px;box-sizing:border-box;font-size:var(--t-label);line-height:1.1;cursor:pointer;color:var(--text)}
#ui-inv .cell:hover{border-color:var(--hover-edge);background:var(--well-hi)}
#ui-inv .cell.empty{color:var(--faint);cursor:default;opacity:.55}
#ui-inv .cell.empty:hover{border-color:var(--edge);background:var(--well)}
#ui-inv .cell.stash{background:var(--well-dim);border-style:dashed;height:34px}
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
  const grid = el("div", "grid main");
  const stashGrid = el("div", "grid stash");
  const invBlock = el("div", "block", "");
  invBlock.append(el("div", "label", UI.text.inventoryLabel), grid);
  const stashBlock = el("div", "block", "");
  stashBlock.append(el("div", "label", UI.text.stashLabel), stashGrid);
  const cells = Array.from({ length: 10 }, (_, s) => {
    const c = el("div", "cell empty");
    c.dataset.slot = String(s);
    c.onclick = () => game.sell(selected(), s);
    grid.appendChild(c);
    return c;
  });
  const stashCells = Array.from({ length: 9 }, (_, s) => {
    const c = el("div", "cell stash empty");
    c.dataset.stash = String(s);
    c.onclick = () => game.unstash(selected(), s);
    stashGrid.appendChild(c);
    return c;
  });
  root.append(invBlock, stashBlock);
  const host = document.getElementById("ui-bar-mid") || document.body;
  host.appendChild(root);

  function paint(cell, kind) {
    const text = slotLabel(kind, catalog);
    cell.textContent = text;
    cell.title = text;
    cell.classList.toggle("empty", text === "");
  }

  function update() {
    const id = selected();
    const hero = id >= 0 && game.attributes(id).some((v) => v > 0);
    root.hidden = !hero;
    if (!hero) return;
    const inv = JSON.parse(game.inventory_json(id));
    inv.slots.forEach(([kind], s) => paint(cells[s], kind));
    inv.stash.forEach(([kind], s) => { if (stashCells[s]) paint(stashCells[s], kind); });
  }

  return { update, root };
}
