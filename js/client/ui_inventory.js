// Hero bar for the selected local hero: portrait with health and mana, six inventory slots and a six-slot
// stash. Item names come from Game.item_catalog_json, slots from Game.inventory_json. Click an inventory slot
// to sell it, click a stash slot to move it into the inventory (both need a shop in reach). Mana shows the
// hero's max pool because the sim exposes no current mana; health tracks the highest value seen as its maximum.

const STYLE = `
#ui-inv{position:fixed;left:50%;bottom:78px;transform:translateX(-50%);display:flex;gap:10px;align-items:flex-end;background:rgba(12,14,18,.9);border:1px solid #333;color:#eee;font:11px/1.3 monospace;padding:6px 8px}
#ui-inv .portrait{width:96px}
#ui-inv .face{height:40px;background:#1b1f26;border:1px solid #555;display:flex;align-items:center;justify-content:center;font-size:18px;color:#ffd54f}
#ui-inv .bar{height:9px;background:#222;margin-top:2px}
#ui-inv .bar i{display:block;height:100%}
#ui-inv .hp i{background:#4caf50}#ui-inv .mp i{background:#3d7bff}
#ui-inv .grid{display:grid;grid-template-columns:repeat(6,44px);gap:3px}
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

function bar(kind) {
  const outer = el("div", `bar ${kind}`);
  const fill = el("i");
  outer.appendChild(fill);
  return { outer, fill };
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
  const portrait = el("div", "portrait");
  const face = el("div", "face");
  const hpText = el("div");
  const hp = bar("hp");
  const mpText = el("div");
  const mp = bar("mp");
  portrait.append(face, hpText, hp.outer, mpText, mp.outer);
  const invWrap = el("div");
  const grid = el("div", "grid");
  const stashGrid = el("div", "grid");
  invWrap.append(el("div", "label", "inventory"), grid, el("div", "label", "stash"), stashGrid);
  const cells = Array.from({ length: 6 }, (_, s) => {
    const c = el("div", "cell");
    c.onclick = () => game.sell(selected(), s);
    grid.appendChild(c);
    return c;
  });
  const stashCells = Array.from({ length: 6 }, (_, s) => {
    const c = el("div", "cell stash");
    c.onclick = () => game.unstash(selected(), s);
    stashGrid.appendChild(c);
    return c;
  });
  root.append(portrait, invWrap);
  document.body.appendChild(root);
  const peakHp = new Map();

  function update() {
    const id = selected();
    const hero = id >= 0 && game.attributes(id).some((v) => v > 0);
    root.hidden = !hero;
    if (!hero) return;
    const hpNow = game.unit_hp(id);
    const peak = Math.max(peakHp.get(id) || 0, hpNow);
    peakHp.set(id, peak);
    const manaMax = game.max_mana(id);
    face.textContent = `L${game.character_level(id)}`;
    hpText.textContent = `hp ${hpNow}/${peak}`;
    hp.fill.style.width = `${peak ? (100 * hpNow) / peak : 0}%`;
    mpText.textContent = `mana ${manaMax}`;
    mp.fill.style.width = "100%";
    const inv = JSON.parse(game.inventory_json(id));
    inv.slots.forEach(([kind], s) => { cells[s].textContent = slotLabel(kind, catalog); });
    inv.stash.forEach(([kind], s) => { stashCells[s].textContent = slotLabel(kind, catalog); });
  }

  return { update, root };
}
