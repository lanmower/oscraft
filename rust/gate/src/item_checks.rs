//! Gate checks for Dota items on the moba scenario: costs and recipes against items.txt
//! @282fad29f0f74504fad465c52d197fb61f4d612b, the recipe purchase rule, shops, stacks, slots,
//! stat bonuses and consumables.

use crate::checks::{fresh, kind};
use sim::{fixed, Sim};

const HOME_SHOP_DOODAD: u16 = 5;

fn moba() -> Result<Sim, String> {
    let scenario = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("scenarios").join("moba");
    let image = osdt_data::generate(&scenario).map_err(|e| format!("moba scenario image: {e:#}"))?;
    fresh(&image)
}

fn item(s: &Sim, name: &str) -> Result<usize, String> {
    s.catalog().item_index(name).ok_or_else(|| format!("moba scenario has no item {name}"))
}

/// A hero of player 0 beside a home shop with `gold` to spend.
fn shopper(s: &mut Sim, gold: i32) -> Result<usize, String> {
    let shop = s.catalog().doodads.iter().find(|d| d.kind == HOME_SHOP_DOODAD).copied().ok_or("moba scenario has no shop doodad")?;
    let k = kind(s, "axe")?;
    let (x, y) = (fixed::from_milli(shop.x), fixed::from_milli(shop.y));
    for dx in 1..40 {
        for dy in [0, 3, -3] {
            if let Some(h) = s.spawn_hero(0, k, x + fixed::from_int(dx), y + fixed::from_int(dy)) {
                s.add_resources(0, gold - s.resource_a(0), 0);
                return Ok(h);
            }
        }
    }
    Err("no free cell beside the shop".into())
}

fn want(what: &str, got: i64, want: i64) -> Result<(), String> {
    if got != want {
        return Err(format!("{what} {got} want {want}"));
    }
    Ok(())
}

/// Costs and recipe shapes of items.txt: a basic item, a two-part recipe, a four-part recipe, a
/// consumable with charges, a secret-shop item and a neutral item.
pub fn item_catalog(_image: &[u8]) -> Result<(), String> {
    let s = moba()?;
    let cat = s.catalog();
    for (name, cost) in [("blink", 2250), ("black_king_bar", 4050), ("tpscroll", 100), ("tango", 90), ("clarity", 60), ("flask", 100), ("ward_sentry", 50), ("platemail", 1400), ("blade_mail", 2400), ("force_staff", 2200)] {
        let it = &cat.items[item(&s, name)?];
        want(&format!("{name} cost"), it.cost as i64, cost)?;
    }
    let parts = |name: &str| -> Result<Vec<&str>, String> {
        let it = &cat.items[item(&s, name)?];
        Ok(it.components.iter().filter(|&&c| c != 0xFFFF).map(|&c| cat.name(cat.items[c as usize].name)).collect())
    };
    want("blade_mail parts", parts("blade_mail")?.len() as i64, 2)?;
    if parts("black_king_bar")? != ["mithril_hammer", "ogre_axe"] {
        return Err(format!("black_king_bar parts {:?}", parts("black_king_bar")?));
    }
    let tango = &cat.items[item(&s, "tango")?];
    want("tango charges", tango.charges as i64, 3)?;
    want("platemail shops", cat.items[item(&s, "platemail")?].shops as i64, 4)?;
    let kobold = &cat.items[item(&s, "kobold_cup")?];
    want("kobold_cup tier", kobold.tier as i64, 1)?;
    println!("item catalog ok items={}", cat.items.len());
    Ok(())
}

/// A recipe consumes the components held and charges the cost less theirs; its bonuses apply; selling
/// returns half the cost.
pub fn item_recipe_buy(_image: &[u8]) -> Result<(), String> {
    let mut s = moba()?;
    let h = shopper(&mut s, 5000)?;
    let (hammer, axe, bkb) = (item(&s, "mithril_hammer")?, item(&s, "ogre_axe")?, item(&s, "black_king_bar")?);
    let damage = s.damage_of(h);
    if !s.buy_item(h, hammer) || !s.buy_item(h, axe) {
        return Err(format!("component purchases refused: {}", s.stats.last_refusal.name()));
    }
    want("gold after components", s.resource_a(0) as i64, 5000 - 1600 - 1000)?;
    if !s.buy_item(h, bkb) {
        return Err(format!("recipe purchase refused: {}", s.stats.last_refusal.name()));
    }
    want("gold after recipe", s.resource_a(0) as i64, 5000 - 4050)?;
    want("held items", (0..s.inventory_slots()).filter(|&k| s.inventory_slot(h, k).0 != 0).count() as i64, 1)?;
    want("damage bonus", s.item_bonus(h, "damage") as i64, 24)?;
    want("strength damage", ((s.damage_of(h) - damage) / fixed::ONE) as i64, 10)?;
    let gold = s.resource_a(0);
    let slot = (0..s.inventory_slots()).find(|&k| s.inventory_slot(h, k).0 != 0).ok_or("no item held")?;
    if !s.sell_item(h, slot) {
        return Err("sell refused beside the shop".into());
    }
    want("sell refund", (s.resource_a(0) - gold) as i64, 4050 / 2)?;
    want("damage after selling", s.damage_of(h) as i64, damage as i64)?;
    println!("item recipe buy ok");
    Ok(())
}

/// A missing component is paid for in gold; a secret-shop item is refused at a home shop; a purchase
/// beyond the gold held is refused.
pub fn item_shop_rules(_image: &[u8]) -> Result<(), String> {
    let mut s = moba()?;
    let h = shopper(&mut s, 3000)?;
    let (hammer, bkb, plate) = (item(&s, "mithril_hammer")?, item(&s, "black_king_bar")?, item(&s, "platemail")?);
    if s.buy_item(h, plate) {
        return Err("platemail sold at a home shop".into());
    }
    if s.buy_item(h, bkb) {
        return Err("a 4050 gold item bought with 3000 gold".into());
    }
    if !s.buy_item(h, hammer) {
        return Err("mithril hammer refused".into());
    }
    s.add_resources(0, 5000, 0);
    let gold = s.resource_a(0);
    if !s.buy_item(h, bkb) {
        return Err("recipe with one held component refused".into());
    }
    want("price with one component held", (gold - s.resource_a(0)) as i64, 4050 - 1600)?;
    let far = s.spawn_hero(0, kind(&s, "axe")?, fixed::from_int(1), fixed::from_int(1));
    if let Some(far) = far {
        if s.buy_item(far, hammer) {
            return Err("an item bought far from every shop".into());
        }
    }
    println!("item shop rules ok");
    Ok(())
}

/// A stackable item joins the stack of its kind and a consumable loses a charge per use.
pub fn item_stack_consume(_image: &[u8]) -> Result<(), String> {
    let mut s = moba()?;
    let h = shopper(&mut s, 2000)?;
    let tango = item(&s, "tango")?;
    let faerie = item(&s, "faerie_fire")?;
    s.buy_item(h, tango);
    s.buy_item(h, tango);
    let kind_id = sim_kind(tango);
    want("tango charges held", s.inventory_count(h, kind_id) as i64, 6)?;
    want("slots used", (0..s.inventory_slots()).filter(|&k| s.inventory_slot(h, k).0 == kind_id).count() as i64, 1)?;
    if !s.buy_item(h, faerie) {
        return Err("faerie fire refused".into());
    }
    s.set_health(h, 100 * fixed::ONE, 1000 * fixed::ONE);
    let slot = (0..s.inventory_slots()).find(|&k| s.inventory_slot(h, k).0 == sim_kind(faerie)).ok_or("faerie fire not held")?;
    if !s.use_item(h, slot, -1, 0, 0) {
        return Err(format!("faerie fire use refused: {}", s.stats.last_refusal.name()));
    }
    want("faerie fire heal", ((s.hp(h) - 100 * fixed::ONE) / fixed::ONE) as i64, 85)?;
    want("faerie fire left", s.inventory_count(h, sim_kind(faerie)) as i64, 0)?;
    println!("item stack consume ok");
    Ok(())
}

fn sim_kind(index: usize) -> u32 {
    index as u32 + 1
}

/// The neutral slot takes only a neutral item and a neutral item sits nowhere else among the main slots.
pub fn item_slots(_image: &[u8]) -> Result<(), String> {
    let mut s = moba()?;
    let h = shopper(&mut s, 1000)?;
    let cup = item(&s, "kobold_cup")?;
    let blink = item(&s, "blink")?;
    if s.buy_item(h, cup) {
        return Err("a neutral item was sold".into());
    }
    if !s.inventory_add(h, sim_kind(cup), 1) {
        return Err("a neutral item was refused by inventory_add".into());
    }
    want("neutral slot", s.inventory_slot(h, 9).0 as i64, sim_kind(cup) as i64)?;
    s.add_resources(0, 3000, 0);
    s.buy_item(h, blink);
    if s.move_item(h, 0, 9) {
        return Err("blink dagger moved into the neutral slot".into());
    }
    if s.move_item(h, 9, 1) {
        return Err("a neutral item moved into a main slot".into());
    }
    if !s.move_item(h, 0, 6) {
        return Err("blink dagger refused the backpack".into());
    }
    println!("item slots ok");
    Ok(())
}

/// The item stat names the data format writes and the sim reads are the same list in the same order.
pub fn item_stat_names(_image: &[u8]) -> Result<(), String> {
    let written = osdt_data::format::ITEM_STATS;
    if written != sim::ITEM_STATS.as_slice() {
        return Err(format!("data writes {written:?}, sim reads {:?}", sim::ITEM_STATS));
    }
    Ok(())
}

/// Stats that do not stack take the best item; the others add up.
pub fn item_stat_stacking(_image: &[u8]) -> Result<(), String> {
    let mut s = moba()?;
    let h = shopper(&mut s, 20000)?;
    let (crit, plate) = (item(&s, "lesser_crit")?, item(&s, "chainmail")?);
    for _ in 0..2 {
        if !s.buy_item(h, crit) || !s.buy_item(h, plate) {
            return Err(format!("purchase refused: {}", s.stats.last_refusal.name()));
        }
    }
    want("crit chance of two crits", s.item_bonus(h, "crit_chance") as i64, 30)?;
    want("damage of two crits", s.item_bonus(h, "damage") as i64, 64)?;
    want("armor of two chainmails", s.item_bonus(h, "armor") as i64, 800)?;
    Ok(())
}

/// Observer wards come from a shared team stock: two at the start, one more every 135 seconds up to four.
pub fn item_stock(_image: &[u8]) -> Result<(), String> {
    let mut s = moba()?;
    let h = shopper(&mut s, 0)?;
    let ward = item(&s, "ward_observer")?;
    let team = s.team_of(0).unwrap_or(0);
    want("initial stock", s.item_stock(team, ward).map_or(-1, |c| c as i64), 2)?;
    if !s.buy_item(h, ward) || !s.buy_item(h, ward) {
        return Err(format!("stocked wards refused: {}", s.stats.last_refusal.name()));
    }
    if s.buy_item(h, ward) {
        return Err("a ward was sold out of stock".into());
    }
    want("stock after two purchases", s.item_stock(team, ward).map_or(-1, |c| c as i64), 0)?;
    for _ in 0..135 * sim::TICKS_PER_SECOND + sim::TICKS_PER_SECOND {
        s.tick();
    }
    if s.item_stock(team, ward) != Some(1) {
        return Err(format!("stock after the restock time {:?}", s.item_stock(team, ward)));
    }
    Ok(())
}
