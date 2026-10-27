use std::cell::RefMut;
use std::path::{Path, PathBuf};
use crate::checks::{role};

use mod_host::ModHost;
use sim::{fixed, Refusal, Sim};

use crate::checks::kind;

const HOME_SHOP_PROP: u16 = 5;

struct Moba {
    host: ModHost,
}

fn lua_files(dir: &Path, rel: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let here = dir.join(rel);
    for entry in std::fs::read_dir(&here).map_err(|e| format!("read {}: {e}", here.display()))? {
        let path = entry.map_err(|e| e.to_string())?.path();
        let name = path.file_name().map(|n| n.to_owned()).unwrap_or_default();
        let next = rel.join(&name);
        if path.is_dir() {
            if name != "fixtures" {
                lua_files(dir, &next, out)?;
            }
        } else if path.extension().is_some_and(|e| e == "lua") {
            out.push(next);
        }
    }
    Ok(())
}

fn requires(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = src;
    while let Some(i) = rest.find("require(\"") {
        let after = &rest[i + "require(\"".len()..];
        let Some(j) = after.find('"') else { break };
        out.push(after[..j].to_owned());
        rest = &after[j..];
    }
    out
}

fn visit(dir: &Path, files: &[PathBuf], module: &str, out: &mut Vec<(String, String)>) -> Result<(), String> {
    let Some(file) = files.iter().find(|f| f.file_stem().is_some_and(|s| s == module)) else { return Ok(()) };
    let name = file.to_string_lossy().replace('\\', "/");
    if out.iter().any(|(f, _)| *f == name) {
        return Ok(());
    }
    let src = std::fs::read_to_string(dir.join(file)).map_err(|e| format!("read {name}: {e}"))?;
    let reqs = requires(&src);
    out.push((name, src));
    for r in reqs {
        visit(dir, files, &r, out)?;
    }
    Ok(())
}

fn pack_mods(dir: &Path, roots: &[&str]) -> Result<Vec<(String, String)>, String> {
    let mut files = Vec::new();
    lua_files(dir, Path::new(""), &mut files)?;
    let mut out = Vec::new();
    for root in roots {
        let stem = Path::new(root).file_stem().and_then(|s| s.to_str()).ok_or("root module name")?;
        visit(dir, &files, stem, &mut out)?;
    }
    Ok(out)
}

fn scenario() -> Result<Moba, String> {
    scenario_with(&["store.lua"])
}

fn scenario_with(roots: &[&str]) -> Result<Moba, String> {
    scenario_with_probe(roots, None)
}

fn scenario_with_probe(roots: &[&str], probe: Option<(&str, &str)>) -> Result<Moba, String> {
    let scenario = crate::checks::pack_dir();
    let image = osdt_data::generate(&scenario).map_err(|e| format!("moba scenario image: {e:#}"))?;
    let mut host = ModHost::new(&image).map_err(|e| format!("ModHost::new: {e:?}"))?;
    let rules = std::fs::read_to_string(scenario.join(crate::checks::RULES_FILE)).map_err(|e| e.to_string())?;
    if !host.sim_mut().set_attr_rules_text(&rules) {
        return Err("the pack rules file has a bad line".into());
    }
    host.sim_mut().set_fog(false);
    let mut mods = pack_mods(&scenario, roots)?;
    mods.extend(probe.map(|(name, src)| (name.to_owned(), src.to_owned())));
    host.load_mods(&mods).map_err(|e| format!("{roots:?}: {e}"))?;
    Ok(Moba { host })
}

impl Moba {
    fn sim(&self) -> RefMut<'_, Sim> {
        self.host.sim_mut()
    }

    fn buy(&mut self, h: usize, item: usize) -> bool {
        self.trade(|s| s.buy_item(h, item))
    }

    fn use_item(&mut self, h: usize, slot: usize, target: i32, x: i32, y: i32) -> bool {
        self.host.cast_allowed(h, slot, target, x, y, true) && self.sim().use_item(h, slot, target, x, y)
    }

    fn sell(&mut self, h: usize, slot: usize) -> bool {
        self.trade(|s| s.sell_item(h, slot))
    }

    fn trade(&mut self, queue: impl FnOnce(&mut Sim) -> bool) -> bool {
        let queued = {
            let mut s = self.sim();
            s.stats.last_refusal = Refusal::Unset;
            queue(&mut s)
        };
        self.settle();
        queued && self.sim().stats.last_refusal == Refusal::Unset
    }

    fn settle(&mut self) {
        let reqs = self.sim().drain_requests();
        for r in reqs {
            self.host.feed_request(&r);
        }
    }

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
    s.catalog().item_index(name).ok_or_else(|| format!("moba scenario has no item {name}"))
}

fn slot_of(m: &Moba, h: usize, index: usize) -> Option<usize> {
    let s = m.sim();
    (0..s.inventory_slots()).find(|&k| s.inventory_slot(h, k).0 == sim_kind(index))
}

fn shopper(m: &Moba, gold: i32) -> Result<usize, String> {
    let (k, x, y) = {
        let s = m.sim();
        let shop = s.catalog().props.iter().find(|d| d.kind == HOME_SHOP_PROP).copied().ok_or("moba scenario has no shop prop")?;
        (kind(&s, role("hero.a")?)?, fixed::from_milli(shop.x), fixed::from_milli(shop.y))
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

pub fn item_catalog(_image: &[u8]) -> Result<(), String> {
    let m = scenario()?;
    let s = m.sim();
    let cat = s.catalog();
    for (name, cost) in [(role("item.5")?, 2250), (role("item.3")?, 3975), (role("item.19")?, 75), (role("item.18")?, 125), (role("item.8")?, 50), (role("item.10")?, 110), (role("unit.ward_b")?, 100), (role("item.16")?, 1400), (role("item.4")?, 2200), (role("item.11")?, 2250)] {
        let it = &cat.items[item(&s, name)?];
        want(&format!("{name} cost"), it.cost as i64, cost)?;
    }
    let parts = |name: &str| -> Result<Vec<&str>, String> {
        let it = &cat.items[item(&s, name)?];
        Ok(it.components.iter().filter(|&&c| c != 0xFFFF).map(|&c| cat.name(cat.items[c as usize].name)).collect())
    };
    want("blade_mail parts", parts(role("item.4")?)?.len() as i64, 3)?;
    if parts(role("item.3")?)? != [role("item.14")?, role("item.15")?] {
        return Err(format!("black_king_bar parts {:?}", parts(role("item.3")?)?));
    }
    let tango = &cat.items[item(&s, role("item.18")?)?];
    want("tango charges", tango.charges as i64, 4)?;
    want("platemail tags", cat.items[item(&s, role("item.16")?)?].tags as i64, 4)?;
    let kobold = &cat.items[item(&s, role("item.12")?)?];
    want("kobold_cup tier", kobold.tier as i64, 1)?;
    println!("item catalog ok items={}", cat.items.len());
    Ok(())
}

pub fn item_stack_consume(_image: &[u8]) -> Result<(), String> {
    let mut m = scenario()?;
    let h = shopper(&m, 2000)?;
    let tango = item(&m.sim(), role("item.18")?)?;
    let faerie = item(&m.sim(), role("item.9")?)?;
    m.buy(h, tango);
    m.buy(h, tango);
    let kind_id = sim_kind(tango);
    want("tango charges held", m.sim().inventory_count(h, kind_id) as i64, 8)?;
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

pub fn item_slots(_image: &[u8]) -> Result<(), String> {
    let mut m = scenario()?;
    let h = shopper(&m, 1000)?;
    let cup = item(&m.sim(), role("item.12")?)?;
    let blink = item(&m.sim(), role("item.5")?)?;
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

pub fn item_stat_names(_image: &[u8]) -> Result<(), String> {
    let written = osdt_data::format::ITEM_STATS;
    if written != sim::ITEM_STATS.as_slice() {
        return Err(format!("data writes {written:?}, sim reads {:?}", sim::ITEM_STATS));
    }
    Ok(())
}

pub fn item_stat_stacking(_image: &[u8]) -> Result<(), String> {
    let mut m = scenario()?;
    let h = shopper(&m, 20000)?;
    let (crit, plate) = {
        let s = m.sim();
        (item(&s, role("item.13")?)?, item(&s, role("item.7")?)?)
    };
    for _ in 0..2 {
        if !m.buy(h, crit) || !m.buy(h, plate) {
            return Err(format!("purchase refused: {}", m.refusal()));
        }
    }
    want("crit chance of two crits", m.sim().item_bonus(h, "crit_chance") as i64, 20)?;
    want("damage of two crits", m.sim().item_bonus(h, "damage") as i64, 60)?;
    want("armor of two chainmails", m.sim().item_bonus(h, "armor") as i64, 1000)?;
    Ok(())
}

fn guard(m: &mut Moba, h: usize) -> Result<(), String> {
    let max = m.sim().hp_max(h);
    m.sim().set_health(h, max, max);
    m.tick()
}

pub fn item_town_portal(_image: &[u8]) -> Result<(), String> {
    let mut m = scenario_with(&["store.lua", "spells.lua"])?;
    let h = shopper(&m, 1000)?;
    let scroll = item(&m.sim(), role("item.19")?)?;
    let r_unit_tower = role("unit.tower")?;
    let find = |s: &Sim, player: u8| {
        (0..s.count()).find(|&u| s.alive(u) && s.player(u) == player && s.kind_name(u).is_some_and(|n| n.starts_with(r_unit_tower)))
    };
    let (own, enemy) = {
        let s = m.sim();
        (find(&s, 0).ok_or("no friendly tower")?, find(&s, 1).ok_or("no enemy tower")?)
    };
    if !m.buy(h, scroll) {
        return Err(format!("scroll refused: {}", m.refusal()));
    }
    let slot = slot_of(&m, h, scroll).ok_or("scroll not held")?;
    m.sim().add_resources(0, 1000, 0);
    m.use_item(h, slot, enemy as i32, 0, 0);
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
    if !m.use_item(h, slot, own as i32, 0, 0) {
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

pub fn item_aghanim_and_refresh(_image: &[u8]) -> Result<(), String> {
    let mut m = scenario_with(&["store.lua", "upgrades.lua"])?;
    let h = shopper(&m, 20000)?;
    let (scepter, blink, refresher) = {
        let s = m.sim();
        (item(&s, role("item.20")?)?, item(&s, role("item.5")?)?, item(&s, role("item.17")?)?)
    };
    if !m.buy(h, scepter) {
        return Err(format!("scepter refused: {}", m.refusal()));
    }
    m.tick()?;
    want("scepter upgrade bit", (m.sim().upgrade_bits(h) & 1) as i64, 1)?;
    let slot = slot_of(&m, h, scepter).ok_or("scepter not held")?;
    if !m.sell(h, slot) {
        return Err("scepter not sold".into());
    }
    m.tick()?;
    want("scepter upgrade bit after selling", (m.sim().upgrade_bits(h) & 1) as i64, 0)?;
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

const FORM_MODS: &[&str] = &["store.lua", "item_forms.lua", "spells.lua", "upgrades.lua"];

fn give(m: &Moba, h: usize, name: &str, charges: u32) -> Result<usize, String> {
    let index = item(&m.sim(), name)?;
    if !m.sim().inventory_add(h, sim_kind(index), charges) {
        return Err(format!("{name} was not added"));
    }
    slot_of(m, h, index).ok_or_else(|| format!("{name} not held"))
}

fn held_name(m: &Moba, h: usize, slot: usize) -> String {
    let s = m.sim();
    let (kind, _) = s.inventory_slot(h, slot);
    if kind == 0 {
        return String::new();
    }
    s.catalog().name(s.catalog().items[kind as usize - 1].name).to_owned()
}

fn run(m: &mut Moba, ticks: u32) -> Result<(), String> {
    for _ in 0..ticks {
        m.tick()?;
    }
    Ok(())
}

fn hero_of_kind(m: &Moba, player: u8, name: &str, near: usize) -> Result<usize, String> {
    let (k, x, y) = {
        let s = m.sim();
        let (x, y) = s.position_fixed(near);
        (kind(&s, name)?, x, y)
    };
    for dx in 1..60 {
        if let Some(h) = m.sim().spawn_attributed(player, k, x + fixed::from_int(dx), y) {
            return Ok(h);
        }
    }
    Err(format!("no free cell for {name}"))
}

pub fn item_form_toggle(_image: &[u8]) -> Result<(), String> {
    let mut m = scenario_with(FORM_MODS)?;
    let h = shopper(&m, 20000)?;
    let index = item(&m.sim(), "armlet")?;
    if !m.buy(h, index) {
        return Err(format!("armlet refused: {}", m.refusal()));
    }
    let slot = slot_of(&m, h, index).ok_or("armlet not held")?;
    want("a bought armlet starts off: damage", m.sim().item_bonus(h, "damage") as i64, 9)?;
    let max = m.sim().hp_max(h);
    m.sim().set_health(h, max, max);
    if !m.use_item(h, slot, -1, 0, 0) {
        return Err(format!("armlet toggle refused: {}", m.refusal()));
    }
    run(&mut m, 2)?;
    want("armlet on: strength", m.sim().item_bonus(h, "strength") as i64, 25)?;
    if held_name(&m, h, slot) != "armlet_on" {
        return Err(format!("armlet slot holds {}", held_name(&m, h, slot)));
    }
    let before = m.sim().hp(h);
    run(&mut m, 16)?;
    let lost = (before - m.sim().hp(h)) / fixed::ONE;
    if !(30..=45).contains(&lost) {
        return Err(format!("armlet drained {lost} hp in a second, want about 40"));
    }
    m.sim().set_health(h, 2 * fixed::ONE, max);
    run(&mut m, 40)?;
    if m.sim().hp(h) < fixed::ONE {
        return Err("the armlet drain killed its holder".into());
    }
    run(&mut m, 17)?;
    if !m.use_item(h, slot, -1, 0, 0) {
        return Err(format!("armlet toggle off refused: {}", m.refusal()));
    }
    run(&mut m, 2)?;
    want("armlet off again: strength", m.sim().item_bonus(h, "strength") as i64, 0)?;
    Ok(())
}

pub fn item_holder_forms(_image: &[u8]) -> Result<(), String> {
    let mut m = scenario_with(FORM_MODS)?;
    let melee = shopper(&m, 20000)?;
    let ranged = hero_of_kind(&m, 0, "sniper", melee)?;
    let (a, b) = (give(&m, melee, "skadi_ranged", 1)?, give(&m, ranged, "skadi", 1)?);
    let (c, d) = (give(&m, melee, "manta_ranged", 1)?, give(&m, ranged, "manta", 1)?);
    run(&mut m, 3)?;
    for (who, slot, want_name) in [(melee, a, "skadi"), (ranged, b, "skadi_ranged"), (melee, c, "manta"), (ranged, d, "manta_ranged")] {
        let got = held_name(&m, who, slot);
        if got != want_name {
            return Err(format!("unit {who} slot {slot} holds {got}, want {want_name}"));
        }
    }
    let gem = give(&m, melee, "gem_courier", 1)?;
    run(&mut m, 3)?;
    want("a hero's gem", (held_name(&m, melee, gem) == "gem") as i64, 1)?;
    Ok(())
}

pub fn item_damage_copies(_image: &[u8]) -> Result<(), String> {
    let mut m = scenario_with(FORM_MODS)?;
    let h = shopper(&m, 20000)?;
    let enemy = hero_of_kind(&m, 1, "axe", h)?;
    let heart = give(&m, h, "heart", 1)?;
    run(&mut m, 3)?;
    m.sim().hit_by(h, 10 * fixed::ONE, enemy as i32);
    run(&mut m, 4)?;
    if held_name(&m, h, heart) != "heart_disabled" {
        return Err(format!("heart holds {} after enemy damage", held_name(&m, h, heart)));
    }
    run(&mut m, 5 * sim::TICKS_PER_SECOND as u32 + 8)?;
    if held_name(&m, h, heart) != "heart" {
        return Err(format!("heart holds {} after the window", held_name(&m, h, heart)));
    }
    let boots = give(&m, h, "tranquil_boots", 1)?;
    run(&mut m, 3)?;
    m.sim().hit_by(h, 10 * fixed::ONE, enemy as i32);
    run(&mut m, 4)?;
    if held_name(&m, h, boots) != "tranquil_boots_broken" {
        return Err("tranquil boots did not break when their holder was hit".into());
    }
    run(&mut m, 13 * sim::TICKS_PER_SECOND as u32 + 8)?;
    if held_name(&m, h, boots) != "tranquil_boots" {
        return Err("tranquil boots did not mend after 13 seconds".into());
    }
    Ok(())
}

pub fn item_spell_block_recharge(_image: &[u8]) -> Result<(), String> {
    let mut m = scenario_with(FORM_MODS)?;
    let h = shopper(&m, 20000)?;
    let enemy = hero_of_kind(&m, 1, "axe", h)?;
    let sphere = give(&m, h, "sphere", 1)?;
    let blade = give(&m, enemy, "diffusal_blade", 8)?;
    run(&mut m, 3)?;
    m.sim().grant_energy(enemy, 1000).ok();
    if !m.use_item(enemy, blade, h as i32, 0, 0) {
        return Err(format!("the enemy purge was refused: {}", m.refusal()));
    }
    run(&mut m, 4)?;
    if held_name(&m, h, sphere) != "sphere_recharging" {
        return Err(format!("sphere holds {} after absorbing", held_name(&m, h, sphere)));
    }
    want("the absorbed purge spent no charge", m.sim().inventory_slot(enemy, blade).1 as i64, 8)?;
    run(&mut m, 16 * sim::TICKS_PER_SECOND as u32)?;
    if held_name(&m, h, sphere) != "sphere" {
        return Err(format!("sphere holds {} after 16 s", held_name(&m, h, sphere)));
    }
    Ok(())
}

pub fn item_bkb_counter(_image: &[u8]) -> Result<(), String> {
    let mut m = scenario_with(FORM_MODS)?;
    let h = shopper(&m, 20000)?;
    let slot = give(&m, h, "black_king_bar", 1)?;
    let mut seen = Vec::new();
    for _ in 0..7 {
        m.sim().grant_energy(h, 1000).ok();
        m.sim().set_item_cooldown(h, slot, 0);
        if !m.use_item(h, slot, -1, 0, 0) {
            return Err(format!("bkb refused: {}", m.refusal()));
        }
        run(&mut m, 2)?;
        let left = m.sim().status_left(h, 8);
        seen.push(((left + 15) / 16) as i64);
        m.sim().clear_status(h, 8);
    }
    if seen != [10, 9, 8, 7, 6, 5, 5] {
        return Err(format!("avatar seconds {seen:?}, want [10, 9, 8, 7, 6, 5, 5]"));
    }
    want("form after seven uses", (held_name(&m, h, slot) == "black_king_bar_5") as i64, 1)?;
    Ok(())
}

pub fn item_recipe_forms(_image: &[u8]) -> Result<(), String> {
    let mut m = scenario_with(FORM_MODS)?;
    let h = shopper(&m, 20000)?;
    let empty = give(&m, h, "diffusal_blade_empty", 1)?;
    let l2 = item(&m.sim(), "diffusal_blade_2")?;
    let gold = m.sim().resource_a(0);
    if !m.buy(h, l2) {
        return Err(format!("level 2 refused with an empty level 1 held: {}", m.refusal()));
    }
    want("level 2 price with the empty copy", (gold - m.sim().resource_a(0)) as i64, 700)?;
    let slot = slot_of(&m, h, l2).ok_or("level 2 not held")?;
    want("level 2 charges", m.sim().inventory_slot(h, slot).1 as i64, 8)?;
    let empty_item = item(&m.sim(), "diffusal_blade_empty")?;
    if slot_of(&m, h, empty_item).is_some() || empty == usize::MAX {
        return Err("the empty level 1 was not consumed".into());
    }
    let janggo_empty = give(&m, h, "ancient_janggo_empty", 1)?;
    let janggo = item(&m.sim(), "ancient_janggo")?;
    let gold = m.sim().resource_a(0);
    if !m.buy(h, janggo) {
        return Err(format!("janggo recharge refused: {}", m.refusal()));
    }
    want("janggo recharge price", (gold - m.sim().resource_a(0)) as i64, 875)?;
    let slot = slot_of(&m, h, janggo).ok_or("janggo not held")?;
    want("janggo charges", m.sim().inventory_slot(h, slot).1 as i64, 6)?;
    if held_name(&m, h, janggo_empty) == "ancient_janggo_empty" {
        return Err("the empty janggo was kept".into());
    }
    Ok(())
}

pub fn item_alchemist_scepter(_image: &[u8]) -> Result<(), String> {
    let mut m = scenario_with(FORM_MODS)?;
    let h = shopper(&m, 20000)?;
    let ally = hero_of_kind(&m, 0, "axe", h)?;
    let slot = give(&m, h, "ultimate_scepter_alchemist", 1)?;
    run(&mut m, 3)?;
    want("carried scepter bit", (m.sim().upgrade_bits(h) & 1) as i64, 1)?;
    want("ally has no bit", (m.sim().upgrade_bits(ally) & 1) as i64, 0)?;
    if !m.use_item(h, slot, ally as i32, 0, 0) {
        return Err(format!("scepter blessing refused: {}", m.refusal()));
    }
    run(&mut m, 3)?;
    want("blessed ally bit", (m.sim().upgrade_bits(ally) & 1) as i64, 1)?;
    want("blessed ally strength", m.sim().stat(ally, 22) as i64, 10)?;
    let scepter_item = item(&m.sim(), "ultimate_scepter_alchemist")?;
    if slot_of(&m, h, scepter_item).is_some() {
        return Err("the caster kept the scepter".into());
    }
    Ok(())
}

const TICKS_PER_SECOND: u32 = 16;

fn gold(m: &Moba) -> i64 {
    i64::from(m.sim().resource_a(0))
}

fn sale_credit(m: &mut Moba, h: usize, it: usize, ticks: u32) -> Result<i64, String> {
    if !m.buy(h, it) {
        return Err(format!("purchase refused: {}", m.refusal()));
    }
    run(m, ticks)?;
    let slot = slot_of(m, h, it).ok_or("purchase not held")?;
    let before = gold(m);
    if !m.sell(h, slot) {
        return Err(format!("sale refused: {}", m.refusal()));
    }
    Ok(gold(m) - before)
}

fn refused_sale(m: &mut Moba, h: usize, slot: usize, name: &str) -> Result<(), String> {
    let before = gold(m);
    if m.sell(h, slot) {
        return Err(format!("{name} was sold"));
    }
    want(&format!("refused sale of {name} pays nothing"), gold(m) - before, 0)?;
    if held_name(m, h, slot) != name {
        return Err(format!("{name} left its slot after a refused sale"));
    }
    Ok(())
}

pub fn item_sale_refund(_image: &[u8]) -> Result<(), String> {
    let (blink, bottle, blink_cost, bottle_cost) = {
        let m = scenario()?;
        let s = m.sim();
        let cat = s.catalog();
        let blink = item(&s, role("item.5")?)?;
        let bottle = item(&s, "bottle")?;
        (blink, bottle, cat.items[blink].cost as i64, cat.items[bottle].cost as i64)
    };
    let probe = format!("local store = require(\"store\")\nlocal charged = game.item_info({bottle})\nfunction on_tick(state)\n  state.refund_charged = store.sale_refund(charged)\nend\n");
    let mut m = scenario_with_probe(&["store.lua"], Some(("sale-refund-probe.lua", probe.as_str())))?;
    let h = shopper(&m, 20000)?;
    want("fresh sale pays half the cost", sale_credit(&mut m, h, blink, 0)?, blink_cost / 2)?;
    want("sale after the old ten-second window pays half the cost", sale_credit(&mut m, h, blink, 12 * TICKS_PER_SECOND)?, blink_cost / 2)?;
    m.tick()?;
    let charged = m.host.state_int("refund_charged").ok_or("the refund probe never set state.refund_charged")?;
    want("chargeable item refund is cost halved, charges are not multiplied", charged, bottle_cost / 2)?;
    Ok(())
}

pub fn item_sale_refusal(_image: &[u8]) -> Result<(), String> {
    let (bottle, faerie) = {
        let m = scenario()?;
        let s = m.sim();
        (item(&s, "bottle")?, item(&s, "faerie_fire")?)
    };
    let probe = format!("local store = require(\"store\")\nlocal charged = game.item_info({bottle})\nlocal consumable = game.item_info({faerie})\nfunction on_tick(state)\n  state.refusal_charged = store.sale_refusal(charged) == \"consumable\" and 1 or 0\n  state.refusal_consumable = store.sale_refusal(consumable) == \"consumable\" and 1 or 0\nend\n");
    let mut m = scenario_with_probe(&["store.lua"], Some(("sale-refusal-probe.lua", probe.as_str())))?;
    let h = shopper(&m, 20000)?;
    let charged = give(&m, h, "bottle", 3)?;
    let consumable = give(&m, h, "faerie_fire", 1)?;
    refused_sale(&mut m, h, charged, "bottle")?;
    refused_sale(&mut m, h, consumable, "faerie_fire")?;
    m.tick()?;
    let charged_reason = m.host.state_int("refusal_charged").ok_or("the refusal probe never set state.refusal_charged")?;
    want("a charged item is refused as a consumable", charged_reason, 1)?;
    let consumable_reason = m.host.state_int("refusal_consumable").ok_or("the refusal probe never set state.refusal_consumable")?;
    want("a consumable is refused as a consumable", consumable_reason, 1)?;
    Ok(())
}

pub fn item_sphere_transfer(_image: &[u8]) -> Result<(), String> {
    let mut m = scenario_with(FORM_MODS)?;
    let h = shopper(&m, 20000)?;
    let ally = hero_of_kind(&m, 0, "axe", h)?;
    let enemy = hero_of_kind(&m, 1, "axe", h)?;
    let sphere = give(&m, h, "sphere", 1)?;
    let blade = give(&m, enemy, "diffusal_blade", 8)?;
    run(&mut m, 3)?;
    if !m.use_item(h, sphere, ally as i32, 0, 0) {
        return Err(format!("the transfer was refused: {}", m.refusal()));
    }
    run(&mut m, 4)?;
    if held_name(&m, h, sphere) != "sphere_recharging" {
        return Err(format!("sphere holds {} after the transfer", held_name(&m, h, sphere)));
    }
    if m.sim().spell_block_left(ally) == 0 {
        return Err("the ally holds no borrowed block".into());
    }
    m.sim().grant_energy(enemy, 1000).ok();
    if !m.use_item(enemy, blade, ally as i32, 0, 0) {
        return Err(format!("the enemy purge was refused: {}", m.refusal()));
    }
    run(&mut m, 2)?;
    want("the borrowed block absorbed the purge", m.sim().inventory_slot(enemy, blade).1 as i64, 8)?;
    want("the borrowed block ended", m.sim().spell_block_left(ally) as i64, 0)?;
    Ok(())
}

pub fn item_treads_start_state(_image: &[u8]) -> Result<(), String> {
    let mut m = scenario_with(FORM_MODS)?;
    let h = shopper(&m, 20000)?;
    let names = ["boots", "gloves", "belt_of_strength"];
    for name in names {
        let index = item(&m.sim(), name)?;
        if !m.buy(h, index) {
            return Err(format!("{name} refused: {}", m.refusal()));
        }
    }
    let treads = item(&m.sim(), "power_treads")?;
    let gold = m.sim().resource_a(0);
    if !m.buy(h, treads) {
        return Err(format!("power treads refused: {}", m.refusal()));
    }
    want("price with belt, boots and gloves held", (gold - m.sim().resource_a(0)) as i64, 0)?;
    run(&mut m, 4)?;
    let slots = m.sim().inventory_slots();
    let slot = (0..slots).find(|&k| held_name(&m, h, k).starts_with("power_treads")).ok_or("treads not held")?;
    if held_name(&m, h, slot) != "power_treads_strength" {
        return Err(format!("a belt left the treads in state {}", held_name(&m, h, slot)));
    }
    Ok(())
}

pub fn item_bkb_drop_lock(_image: &[u8]) -> Result<(), String> {
    let mut m = scenario_with(FORM_MODS)?;
    let h = shopper(&m, 20000)?;
    let slot = give(&m, h, "black_king_bar", 1)?;
    if !m.use_item(h, slot, -1, 0, 0) {
        return Err(format!("bkb refused: {}", m.refusal()));
    }
    run(&mut m, 2)?;
    if m.sell(h, slot) {
        return Err("a locked black king bar was sold".into());
    }
    if m.sim().drop_item_to_ground(h, slot, 0, 0).is_ok() {
        return Err("a locked black king bar was dropped".into());
    }
    Ok(())
}

pub fn item_rapier_owner(_image: &[u8]) -> Result<(), String> {
    let mut m = scenario_with(FORM_MODS)?;
    let h = shopper(&m, 20000)?;
    let own = give(&m, h, "rapier", 1)?;
    run(&mut m, 3)?;
    if held_name(&m, h, own) != "rapier" {
        return Err(format!("a rapier its holder owns became {}", held_name(&m, h, own)));
    }
    m.sim().set_item_owner(h, own, 1).map_err(|r| r.name().to_string())?;
    run(&mut m, 4)?;
    if held_name(&m, h, own) != "rapier_undroppable" {
        return Err(format!("a rapier owned by an enemy became {}", held_name(&m, h, own)));
    }
    want("the owner is kept", m.sim().item_owner(h, own).map_or(-1, |p| p as i64), 1)?;
    Ok(())
}
