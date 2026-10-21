//! Gate checks for reference items on the scenario scenario: costs and recipes against items.txt
//! @282fad29f0f74504fad465c52d197fb61f4d612b, the recipe purchase rule, shops, stacks, slots,
//! stat bonuses and consumables. The store rule is the mod's (scenarios/moba/store.lua): a trade is
//! asked of the mod through the host, and its verdict is applied to the host's sim here.

use std::cell::RefMut;

use mod_host::ModHost;
use sim::{fixed, Refusal, Sim};

use crate::checks::kind;

const HOME_SHOP_DOODAD: u16 = 5;

/// The scenario scenario as a mod host: the host's sim is the world, and the store and courier rules are the
/// mod's. Trades and deliveries are answered by the mod, and each answer is applied to the sim.
struct Moba {
    host: ModHost,
}

fn scenario() -> Result<Moba, String> {
    let scenario = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("scenarios").join("moba");
    let image = osdt_data::generate(&scenario).map_err(|e| format!("scenario scenario image: {e:#}"))?;
    let mut host = ModHost::new(&image).map_err(|e| format!("ModHost::new: {e:?}"))?;
    let rules = std::fs::read_to_string(scenario.join("hero_rules.txt")).map_err(|e| e.to_string())?;
    if !host.sim().set_attr_rules_text(&rules) {
        return Err("hero_rules.txt has a bad line".into());
    }
    host.sim().set_fog(false);
    let store = std::fs::read_to_string(scenario.join("store.lua")).map_err(|e| e.to_string())?;
    host.load_source("store.lua", &store).map_err(|e| format!("store.lua: {e}"))?;
    Ok(Moba { host })
}

impl Moba {
    fn sim(&self) -> RefMut<'_, Sim> {
        self.host.sim()
    }

    /// Queues a purchase, has the mod answer it, and reports whether the item was bought.
    fn buy(&mut self, h: usize, item: usize) -> bool {
        self.trade(|s| s.buy_item(h, item))
    }

    fn sell(&mut self, h: usize, slot: usize) -> bool {
        self.trade(|s| s.sell_item(h, slot))
    }

    // A trade succeeded when it queued and no refusal was named by its answer.
    fn trade(&mut self, queue: impl FnOnce(&mut Sim) -> bool) -> bool {
        let queued = {
            let mut s = self.sim();
            s.stats.last_refusal = Refusal::Unset;
            queue(&mut s)
        };
        self.settle();
        queued && self.sim().stats.last_refusal == Refusal::Unset
    }

    /// Answers the sim's queued requests through the mod, as the room does.
    fn settle(&mut self) {
        let reqs = self.sim().drain_requests();
        for r in reqs {
            self.host.feed_request(&r);
        }
    }

    /// One step of the scenario: the mod's on_tick and the sim's tick, then the requests the step queued.
    fn tick(&mut self) -> Result<(), String> {
        self.host.advance().map_err(|e| format!("mod step: {e}"))?;
        self.settle();
        Ok(())
    }

    fn refusal(&self) -> &'static str {
        self.sim().stats.last_refusal.name()
    }
}

fn item(s: &Sim, name: &str) -> Result<usize, String> {
    s.catalog().item_index(name).ok_or_else(|| format!("scenario scenario has no item {name}"))
}

/// The slot of hero `h` that holds catalog item `index`, if one does.
fn slot_of(m: &Moba, h: usize, index: usize) -> Option<usize> {
    let s = m.sim();
    (0..s.inventory_slots()).find(|&k| s.inventory_slot(h, k).0 == sim_kind(index))
}

fn held_slots(m: &Moba, h: usize) -> usize {
    let s = m.sim();
    (0..s.inventory_slots()).filter(|&k| s.inventory_slot(h, k).0 != 0).count()
}

/// A hero of player 0 beside a home shop with `gold` to spend.
fn shopper(m: &Moba, gold: i32) -> Result<usize, String> {
    let (k, x, y) = {
        let s = m.sim();
        let shop = s.catalog().doodads.iter().find(|d| d.kind == HOME_SHOP_DOODAD).copied().ok_or("scenario scenario has no shop doodad")?;
        (kind(&s, "axe")?, fixed::from_milli(shop.x), fixed::from_milli(shop.y))
    };
    for dx in 1..40 {
        for dy in [0, 3, -3] {
            let placed = m.sim().spawn_attributed(0, k, x + fixed::from_int(dx), y + fixed::from_int(dy));
            if let Some(h) = placed {
                let mut s = m.sim();
                let now = s.resource_a(0);
                s.add_resources(0, gold - now, 0);
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
    let m = scenario()?;
    let s = m.sim();
    let cat = s.catalog();
    for (name, cost) in [("blink", 2250), ("black_king_bar", 4050), ("tpscroll", 100), ("tango", 90), ("clarity", 60), ("flask", 100), ("ward_sentry", 50), ("platemail", 1400), ("blade_mail", 2200), ("force_staff", 2250)] {
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
    want("platemail tags", cat.items[item(&s, "platemail")?].tags as i64, 4)?;
    let kobold = &cat.items[item(&s, "kobold_cup")?];
    want("kobold_cup tier", kobold.tier as i64, 1)?;
    println!("item catalog ok items={}", cat.items.len());
    Ok(())
}

/// A recipe consumes the components held and charges the cost less theirs; its bonuses apply; selling
/// returns half the cost.
pub fn item_recipe_buy(_image: &[u8]) -> Result<(), String> {
    let mut m = scenario()?;
    let h = shopper(&m, 5000)?;
    let (hammer, axe, bkb) = (item(&m.sim(), "mithril_hammer")?, item(&m.sim(), "ogre_axe")?, item(&m.sim(), "black_king_bar")?);
    let damage = m.sim().damage_of(h);
    if !m.buy(h, hammer) || !m.buy(h, axe) {
        return Err(format!("component purchases refused: {}", m.refusal()));
    }
    want("gold after components", m.sim().resource_a(0) as i64, 5000 - 1600 - 1000)?;
    if !m.buy(h, bkb) {
        return Err(format!("recipe purchase refused: {}", m.refusal()));
    }
    want("gold after recipe", m.sim().resource_a(0) as i64, 5000 - 4050)?;
    want("held items", held_slots(&m, h) as i64, 1)?;
    want("damage bonus", m.sim().item_bonus(h, "damage") as i64, 24)?;
    want("strength damage", ((m.sim().damage_of(h) - damage) / fixed::ONE) as i64, 10)?;
    let gold = m.sim().resource_a(0);
    let slot = (0..m.sim().inventory_slots()).find(|&k| m.sim().inventory_slot(h, k).0 != 0).ok_or("no item held")?;
    if !m.sell(h, slot) {
        return Err("sell refused beside the shop".into());
    }
    want("sell refund", (m.sim().resource_a(0) - gold) as i64, 4050 / 2)?;
    want("damage after selling", m.sim().damage_of(h) as i64, damage as i64)?;
    println!("item recipe buy ok");
    Ok(())
}

/// A missing component is paid for in gold; a secret-shop item is refused at a home shop; a purchase
/// beyond the gold held is refused.
pub fn item_shop_rules(_image: &[u8]) -> Result<(), String> {
    let mut m = scenario()?;
    let h = shopper(&m, 3000)?;
    let (hammer, bkb, plate) = (item(&m.sim(), "mithril_hammer")?, item(&m.sim(), "black_king_bar")?, item(&m.sim(), "platemail")?);
    if m.buy(h, plate) {
        return Err("platemail sold at a home shop".into());
    }
    if m.buy(h, bkb) {
        return Err("a 4050 gold item bought with 3000 gold".into());
    }
    if !m.buy(h, hammer) {
        return Err("mithril hammer refused".into());
    }
    m.sim().add_resources(0, 5000, 0);
    let gold = m.sim().resource_a(0);
    if !m.buy(h, bkb) {
        return Err("recipe with one held component refused".into());
    }
    want("price with one component held", (gold - m.sim().resource_a(0)) as i64, 4050 - 1600)?;
    let far = m.sim().spawn_attributed(0, kind(&m.sim(), "axe")?, fixed::from_int(1), fixed::from_int(1));
    if let Some(far) = far {
        if m.buy(far, hammer) {
            return Err("an item bought far from every shop".into());
        }
    }
    println!("item shop rules ok");
    Ok(())
}

/// A stackable item joins the stack of its kind and a consumable loses a charge per use.
pub fn item_stack_consume(_image: &[u8]) -> Result<(), String> {
    let mut m = scenario()?;
    let h = shopper(&m, 2000)?;
    let tango = item(&m.sim(), "tango")?;
    let faerie = item(&m.sim(), "faerie_fire")?;
    m.buy(h, tango);
    m.buy(h, tango);
    let kind_id = sim_kind(tango);
    want("tango charges held", m.sim().inventory_count(h, kind_id) as i64, 6)?;
    let stacks = {
        let s = m.sim();
        (0..s.inventory_slots()).filter(|&k| s.inventory_slot(h, k).0 == kind_id).count()
    };
    want("slots used", stacks as i64, 1)?;
    if !m.buy(h, faerie) {
        return Err("faerie fire refused".into());
    }
    m.sim().set_health(h, 100 * fixed::ONE, 1000 * fixed::ONE);
    let slot = slot_of(&m, h, faerie).ok_or("faerie fire not held")?;
    if !m.sim().use_item(h, slot, -1, 0, 0) {
        return Err(format!("faerie fire use refused: {}", m.refusal()));
    }
    want("faerie fire heal", ((m.sim().hp(h) - 100 * fixed::ONE) / fixed::ONE) as i64, 85)?;
    want("faerie fire left", m.sim().inventory_count(h, sim_kind(faerie)) as i64, 0)?;
    println!("item stack consume ok");
    Ok(())
}

fn sim_kind(index: usize) -> u32 {
    index as u32 + 1
}

/// The neutral slot takes only a neutral item and a neutral item sits nowhere else among the main slots.
pub fn item_slots(_image: &[u8]) -> Result<(), String> {
    let mut m = scenario()?;
    let h = shopper(&m, 1000)?;
    let cup = item(&m.sim(), "kobold_cup")?;
    let blink = item(&m.sim(), "blink")?;
    if m.buy(h, cup) {
        return Err("a neutral item was sold".into());
    }
    if !m.sim().inventory_add(h, sim_kind(cup), 1) {
        return Err("a neutral item was refused by inventory_add".into());
    }
    want("neutral slot", m.sim().inventory_slot(h, 9).0 as i64, sim_kind(cup) as i64)?;
    m.sim().add_resources(0, 3000, 0);
    m.buy(h, blink);
    if m.sim().move_item(h, 0, 9) {
        return Err("blink dagger moved into the neutral slot".into());
    }
    if m.sim().move_item(h, 9, 1) {
        return Err("a neutral item moved into a main slot".into());
    }
    if !m.sim().move_item(h, 0, 6) {
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
    let mut m = scenario()?;
    let h = shopper(&m, 20000)?;
    let (crit, plate) = (item(&m.sim(), "lesser_crit")?, item(&m.sim(), "chainmail")?);
    for _ in 0..2 {
        if !m.buy(h, crit) || !m.buy(h, plate) {
            return Err(format!("purchase refused: {}", m.refusal()));
        }
    }
    want("crit chance of two crits", m.sim().item_bonus(h, "crit_chance") as i64, 30)?;
    want("damage of two crits", m.sim().item_bonus(h, "damage") as i64, 64)?;
    want("armor of two chainmails", m.sim().item_bonus(h, "armor") as i64, 800)?;
    Ok(())
}

/// Observer wards come from a shared team stock: two at the start, one more every 135 seconds up to four.
/// The stock is the store rule's (store.lua): a purchase is refused while the stock is out, and a unit
/// arrives once the restock time has passed.
pub fn item_stock(_image: &[u8]) -> Result<(), String> {
    let mut m = scenario()?;
    let h = shopper(&m, 0)?;
    let ward = item(&m.sim(), "ward_observer")?;
    m.sim().add_resources(0, 1000, 0);
    if !m.buy(h, ward) || !m.buy(h, ward) {
        return Err(format!("stocked wards refused: {}", m.refusal()));
    }
    if m.buy(h, ward) {
        return Err("a ward was sold out of stock".into());
    }
    if m.sim().stats.last_refusal != Refusal::NoItem {
        return Err(format!("a sold-out ward was refused as {}, not no-item", m.refusal()));
    }
    for _ in 0..135 * sim::TICKS_PER_SECOND + sim::TICKS_PER_SECOND {
        m.tick()?;
    }
    if !m.buy(h, ward) {
        return Err(format!("no ward after the restock time: {}", m.refusal()));
    }
    Ok(())
}

fn kill(m: &Moba, h: usize) {
    m.sim().hurt(h, i32::MAX / 2);
}

/// A hero dying with the Aegis returns in 5 seconds where it fell, without a death booked and without the
/// Aegis; the next death sends it home after the respawn timer, and its items and their bonuses come back.
pub fn item_aegis_and_death(_image: &[u8]) -> Result<(), String> {
    let mut m = scenario()?;
    let h = shopper(&m, 5000)?;
    let (sword, aegis) = (item(&m.sim(), "broadsword")?, item(&m.sim(), "aegis")?);
    if !m.buy(h, sword) || !m.sim().inventory_add(h, sim_kind(aegis), 1) {
        return Err("could not equip the hero".into());
    }
    let damage = m.sim().damage_of(h);
    let spot = m.sim().position_fixed(h);
    kill(&m, h);
    if m.sim().alive(h) {
        return Err("hero survived lethal damage".into());
    }
    for _ in 0..4 * sim::TICKS_PER_SECOND {
        m.tick()?;
    }
    if m.sim().alive(h) {
        return Err("hero reincarnated before 5 seconds".into());
    }
    for _ in 0..2 * sim::TICKS_PER_SECOND {
        m.tick()?;
    }
    if !m.sim().alive(h) {
        return Err("hero did not reincarnate".into());
    }
    want("deaths booked", m.sim().deaths_of(h) as i64, 0)?;
    want("aegis left", m.sim().inventory_count(h, sim_kind(aegis)) as i64, 0)?;
    let now = m.sim().position_fixed(h);
    if (now.0 - spot.0).abs() > fixed::ONE || (now.1 - spot.1).abs() > fixed::ONE {
        return Err(format!("reincarnated at {now:?}, died at {spot:?}"));
    }
    want("damage after reincarnating", m.sim().damage_of(h) as i64, damage as i64)?;
    let hp_max = m.sim().hp_max(h);
    kill(&m, h);
    let mut waited = 0;
    while !m.sim().alive(h) && waited < 70 * sim::TICKS_PER_SECOND {
        m.tick()?;
        waited += 1;
    }
    if !m.sim().alive(h) {
        return Err("hero did not respawn after the timer".into());
    }
    want("deaths booked after the second death", m.sim().deaths_of(h) as i64, 1)?;
    want("health after respawning", m.sim().hp_max(h) as i64, hp_max as i64)?;
    want("sword kept through death", m.sim().inventory_count(h, sim_kind(sword)) as i64, 1)?;
    want("damage after respawning", m.sim().damage_of(h) as i64, damage as i64)?;
    Ok(())
}

/// One tick with the hero topped up, so a stray creep cannot interrupt the channel under test.
fn guard(m: &mut Moba, h: usize) -> Result<(), String> {
    let max = m.sim().hp_max(h);
    m.sim().set_health(h, max, max);
    m.tick()
}

/// A town portal scroll channels for 3 seconds and lands the hero beside a friendly tower; it cannot target
/// an enemy tower; the scroll is spent.
pub fn item_town_portal(_image: &[u8]) -> Result<(), String> {
    let mut m = scenario()?;
    let h = shopper(&m, 1000)?;
    let scroll = item(&m.sim(), "tpscroll")?;
    let find = |s: &Sim, player: u8| {
        (0..s.count()).find(|&u| s.alive(u) && s.player(u) == player && s.kind_name(u).is_some_and(|n| n.starts_with("tower")))
    };
    let (own, enemy) = (find(&m.sim(), 0).ok_or("no friendly tower")?, find(&m.sim(), 1).ok_or("no enemy tower")?);
    if !m.buy(h, scroll) {
        return Err(format!("scroll refused: {}", m.refusal()));
    }
    let slot = slot_of(&m, h, scroll).ok_or("scroll not held")?;
    m.sim().add_resources(0, 1000, 0);
    m.sim().use_item(h, slot, enemy as i32, 0, 0);
    for _ in 0..4 * sim::TICKS_PER_SECOND {
        guard(&mut m, h)?;
    }
    let (ex, ey) = m.sim().position_fixed(enemy);
    let (hx, hy) = m.sim().position_fixed(h);
    if (hx - ex).abs() < 30 * fixed::ONE && (hy - ey).abs() < 30 * fixed::ONE {
        return Err("the scroll landed the hero beside an enemy tower".into());
    }
    if m.sim().inventory_count(h, sim_kind(scroll)) == 0 {
        let _ = m.buy(h, scroll);
    }
    let slot = slot_of(&m, h, scroll).ok_or("scroll not held")?;
    if !m.sim().use_item(h, slot, own as i32, 0, 0) {
        return Err(format!("scroll use refused: {}", m.refusal()));
    }
    for _ in 0..2 * sim::TICKS_PER_SECOND {
        guard(&mut m, h)?;
    }
    let (ox, oy) = m.sim().position_fixed(own);
    let before = m.sim().position_fixed(h);
    if (before.0 - ox).abs() < 30 * fixed::ONE && (before.1 - oy).abs() < 30 * fixed::ONE {
        return Err("the hero landed before the channel ended".into());
    }
    for _ in 0..2 * sim::TICKS_PER_SECOND {
        guard(&mut m, h)?;
    }
    let (nx, ny) = m.sim().position_fixed(h);
    for r in 1..40 {
        let ok = m.sim().teleport(h, ox + r * fixed::ONE / 2, oy);
        if ok {
            break;
        }
    }
    let d2 = ((nx - ox) as i64).pow(2) + ((ny - oy) as i64).pow(2);
    if d2 > (60 * fixed::ONE as i64).pow(2) {
        return Err(format!("hero at {:?}, tower at {:?}", (nx, ny), (ox, oy)));
    }
    Ok(())
}

/// Aghanim's Scepter gives the hero its scepter upgrade while carried; the Shard is spent on use and keeps
/// the shard upgrade; a Refresher Orb resets the cooldowns of the hero's items except its own.
pub fn item_aghanim_and_refresh(_image: &[u8]) -> Result<(), String> {
    let mut m = scenario()?;
    let h = shopper(&m, 20000)?;
    let (scepter, shard, blink, refresher) = (item(&m.sim(), "ultimate_scepter")?, item(&m.sim(), "aghanims_shard")?, item(&m.sim(), "blink")?, item(&m.sim(), "refresher")?);
    if !m.buy(h, scepter) {
        return Err(format!("scepter refused: {}", m.refusal()));
    }
    want("scepter upgrade bit", (m.sim().upgrade_bits(h) & 1) as i64, 1)?;
    let slot = slot_of(&m, h, scepter).ok_or("scepter not held")?;
    if !m.sell(h, slot) {
        return Err("scepter not sold".into());
    }
    want("scepter upgrade bit after selling", (m.sim().upgrade_bits(h) & 1) as i64, 0)?;
    if !m.sim().inventory_add(h, sim_kind(shard), 1) {
        return Err("shard not added".into());
    }
    let slot = slot_of(&m, h, shard).ok_or("shard not held")?;
    if !m.sim().use_item(h, slot, -1, 0, 0) {
        return Err(format!("shard use refused: {}", m.refusal()));
    }
    want("shard upgrade bit", ((m.sim().upgrade_bits(h) >> 1) & 1) as i64, 1)?;
    want("shard left", m.sim().inventory_count(h, sim_kind(shard)) as i64, 0)?;
    if !m.buy(h, blink) || !m.buy(h, refresher) {
        return Err(format!("purchases refused: {}", m.refusal()));
    }
    let blink_slot = slot_of(&m, h, blink).ok_or("blink not held")?;
    let refresher_slot = slot_of(&m, h, refresher).ok_or("refresher not held")?;
    let (x, y) = m.sim().position_fixed(h);
    if !m.sim().use_item(h, blink_slot, -1, x + 5 * fixed::ONE, y) {
        return Err(format!("blink refused: {}", m.refusal()));
    }
    if m.sim().item_cooldown(h, blink_slot) == 0 {
        return Err("blink did not start its cooldown".into());
    }
    let _ = m.sim().grant_energy(h, 2000);
    if !m.sim().use_item(h, refresher_slot, -1, 0, 0) {
        return Err(format!("refresher refused: {}", m.refusal()));
    }
    want("blink cooldown after refresh", m.sim().item_cooldown(h, blink_slot) as i64, 0)?;
    if m.sim().item_cooldown(h, refresher_slot) == 0 {
        return Err("the refresher refreshed itself".into());
    }
    Ok(())
}
