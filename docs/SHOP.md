# Shop

Bottom line: a hero buys a listed item for gold from a shop within that shop's reach, or the player's courier fetches it from the nearest seller and delivers it to the hero, and the selected item shows its build tree and what a buy does.

Source tags: `6.88g` is the map data (`items_688g.tsv`, `shops_688g.tsv`); `web` is a Dota 2 reference (`items.osd` reference dump, Dota 2 wiki); `design` is an oscraft choice; `GUESS` means the value is not confirmed and the reason is given.

## What can be bought

- **SHOP-LIST.** A shop sells the items its row lists. `6.88g` `scenarios/moba/shops_688g.tsv` (generated into `shops_688g.lua`); 24 home shops, 6 side shops, 2 secret shops.
- **SHOP-STOCK-LEFT.** A limited-stock item shows its remaining quantity in the selected item's detail (`Stock left: N.`). An out-of-stock item shows `Out of stock. Restocks in Ns.`, both in the selected item's detail and in its Quick Buy entry. Source: `store.lua` `stock_status`; the seconds come from the same timer as SHOP-STOCK.
- **SHOP-STOCK.** Items with a stock limit restock on a timer per team. `web`: the limits come from the Dota 2 reference dump (`ItemStockMax`, `ItemStockInitial`, `ItemStockTime` in `items.osd`), not from 6.88g; 6.88g stock is not in the repo.
- **SHOP-NOBUY.** Items marked `no_buy` (states, proxies, placeholders) cannot be bought. `6.88g` `items_688g_na.tsv` (not_applicable rows).

## What can be built (build tree)

- **SHOP-SHORTFALL.** Alt-click on a shop item shows the gold it still needs (`Needs N gold more for this item.`, or `You can afford this item now.`). Ctrl+Alt adds the buyback cost to that figure. The click carries `alt` and `ctrl` flags from `js/client/widgets.js` (`withModifiers`). Source: `design`.
- **SHOP-TREE.** The selected item shows its build tree in the shop's detail panel: the components it is built from, each with its own cost and its own components one level at a time, and the items it is a part of. An item with no recipe shows `No build tree.` `6.88g` recipe rows, `items_688g.tsv` column `components`: a list is the 6.88g recipe and the tree matches it exactly. `KEEP` means 6.88g is silent, so the catalog keeps its reference recipe from `items.osd` (`web`; Abyssal Blade builds from Basher and Sange this way), or no recipe when the reference has none (`apply-items-688g.mjs`). A `not_in_688g` row (Holy Locket, parent of Magic Wand) has only a `web` recipe. `shop_offhand_probe` checks every listed item against this column.
- **SHOP-RECIPE-ORDER.** A recipe component is declared before its item and costs no more than the item. `6.88g` items_688g.tsv order; AGENTS.md Items and economy.

## What happens on buy

- **SHOP-BUY.** Buying pays the item cost less the value of owned parts, which are used up. Parts are taken from the inventory and the stash, recursively. The item goes to the first free inventory slot, else the stash. Source: `store.lua` `acquire`. The exact 6.88g purchase rule is not cited in the repo: `GUESS`.
- **SHOP-BUY-REFUSE.** A buy is refused when the item is not listed, the stock is 0, the gold is short, or no slot is free. Source: `store.lua` `acquire`, `sim` `apply_acquire`.

## Sell, stash and depots

- **SHOP-SELL.** Selling refunds half the item cost (rounded down). Refused for `no_sell` items, consumables (charges), locked items, and when no depot is in reach. Source: `store.lua` `sale_refund`, `sale_refusal`; the 50 percent rule is `web` (Dota 2), the 6.88g sell rate is `GUESS`.
- **SHOP-STASH.** Stash and unstash work within depot reach. Neutral items cannot be stashed. Depot reach uses the same numbers as SHOP-REACH. Source: `store.lua` `release`, `take_out`.

## Distance rule

- **SHOP-REACH.** A direct buy needs the buyer, the hero, within the reach of a shop that sells the item. Reach: home 1300 units, side 625 units, secret 700 units (`shops.lua` `REACH_MILLI`). Source: `GUESS`; the repo has no 6.88g purchase-range value, and 1300 units matches the repo's courier pickup and kill-gold radius.
- Quick buy (the build list) is direct only: each build item needs the hero in reach.

## Courier and off-hand buys

- **SHOP-COURIER.** When the hero is out of reach, Buy sends the player's courier to the nearest shop that sells the item. The courier must reach that shop (walk within its reach) to buy, then carries the item back. Source: `design` (owner to confirm).
- **SHOP-NO-COURIER.** An off-hand buy is refused, with a reason shown to the player, when the player has no courier (`noCourier`) or the courier is already on an errand (`busy`). Source: `design`.
- **SHOP-DELIVER.** The courier, once within 1300 units of the hero (`PICKUP_REACH_MILLI = 20312` in `courier.lua`), drops its items at the hero's position; the hero picks them up two ticks later; the courier returns home. If the shop refused the purchase, the courier comes back empty and the panel says so (`shopRefused`). While the errand runs the courier is held (`courier.errand` in `courier.lua`): the idle follow and the automatic delivery of `courier.lua` skip it, so they cannot override the walk to the seller. Source: `design`; the 64-unit delivery reach is `GUESS`.

## Item text

- **SHOP-TEXT.** Each grid cell shows the item name and cost, and its hover tip is the name, cost and one effect line (`priced` and `effect_sentence` in `panels.lua`). The selected item's detail panel reads in this order: the effect (stats, charges, or no passive stats), the cost, the build tree and what a buy does, then whether the buyer can buy from here (`yes, your hero is in reach`, `yes, your courier fetches it`, or `no` with the reason). Source: `moba_ui.json` `shop.tip` and `shop.offHand`.
