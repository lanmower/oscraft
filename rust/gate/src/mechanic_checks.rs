use crate::checks::{fresh, kind, scenario_rule, run_past_first_camp_mark, run_ticks, spawn, MAX_UNITS, RUN_TICKS};
use crate::checks::{pack_run, role};
use mod_host::ModHost;
use sim::{fixed, Sim};

const ORDER_MOVE: u8 = 0;
const ORDER_STOP: u8 = 1;
const MECHANIC_REGION_MOD: &str = "oscraft-gate-mechanic-region.lua";

pub fn unit_spawn(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let before = s.alive_count();
    let u = spawn(&mut s, 1, "skirmisher", 6, 6)?;
    if s.alive_count() != before + 1 {
        return Err(format!("alive_count {} want {}", s.alive_count(), before + 1));
    }
    if !s.alive(u) || s.player(u) != 1 {
        return Err(format!("spawned unit {u} alive {} player {}", s.alive(u), s.player(u)));
    }
    if s.hp(u) <= 0 {
        return Err(format!("spawned unit {u} has hp {}", s.hp(u)));
    }
    Ok(())
}

pub fn ability_use(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let a = spawn(&mut s, 0, "skirmisher", 10, 10)?;
    let t = spawn(&mut s, 1, "skirmisher", 12, 10)?;
    let (casts, refused) = (s.stats.casts, s.stats.casts_refused);
    if !s.cast_ability(a, 0, t as i32, 0, 0) {
        return Err(format!("cast refused: {}", s.stats.last_refusal.name()));
    }
    if s.stats.casts != casts + 1 {
        return Err(format!("stats.casts {} want {}", s.stats.casts, casts + 1));
    }
    if s.stats.casts_refused != refused {
        return Err(format!("a successful cast counted as refused ({} -> {})", refused, s.stats.casts_refused));
    }
    Ok(())
}

pub fn economy_transfer(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let (m0, v0, m1, v1) = (s.resource_a(0), s.resource_b(0), s.resource_a(1), s.resource_b(1));
    s.add_resources(0, 75, 25);
    if s.resource_a(0) != m0 + 75 || s.resource_b(0) != v0 + 25 {
        return Err(format!(
            "player 0 moved to {}/{} want {}/{}",
            s.resource_a(0),
            s.resource_b(0),
            m0 + 75,
            v0 + 25
        ));
    }
    if s.resource_a(1) != m1 || s.resource_b(1) != v1 {
        return Err("crediting player 0 changed player 1's ledger".into());
    }
    Ok(())
}

pub fn order_queue(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let a = spawn(&mut s, 0, "skirmisher", 10, 10)?;
    let (x, y) = (fixed::from_int(12), fixed::from_int(10));
    let base = s.order_log_len();
    s.issue_order_kind(a, ORDER_MOVE, x, y).map_err(|r| format!("move refused: {}", r.name()))?;
    s.issue_order_kind(a, ORDER_STOP, x, y).map_err(|r| format!("stop refused: {}", r.name()))?;
    if s.order_log_len() != base + 2 {
        return Err(format!("order log grew by {} want 2", s.order_log_len() - base));
    }
    let first = s.order_record(base).map(|r| (r.1, r.2));
    let second = s.order_record(base + 1).map(|r| (r.1, r.2));
    if first != Some((a as i32, ORDER_MOVE)) || second != Some((a as i32, ORDER_STOP)) {
        return Err(format!("recorded {first:?} then {second:?}, want move then stop for unit {a}"));
    }
    if s.issue_order_kind(a, 9, x, y).is_ok() || s.order_log_len() != base + 2 {
        return Err("a refused order was accepted or logged".into());
    }
    Ok(())
}

pub fn inventory_slots(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let u = spawn(&mut s, 0, "skirmisher", 10, 10)?;
    if s.inventory_slots() != 10 {
        return Err(format!("inventory_slots {} want 10", s.inventory_slots()));
    }
    if !s.inventory_add(u, 7, 3) || !s.inventory_add(u, 7, 2) {
        return Err("adding to a kind already held was refused".into());
    }
    if s.inventory_count(u, 7) != 5 || s.inventory_slot(u, 0) != (7, 5) {
        return Err(format!("kind 7 count {} slot0 {:?} want 5 and (7,5)", s.inventory_count(u, 7), s.inventory_slot(u, 0)));
    }
    for kind in 8..=12 {
        if !s.inventory_add(u, kind, 1) {
            return Err(format!("kind {kind} refused with a free slot left"));
        }
    }
    if s.inventory_add(u, 13, 1) {
        return Err("a seventh kind was accepted into a full inventory".into());
    }
    if s.inventory_remove(u, 7, 6) || s.inventory_count(u, 7) != 5 {
        return Err("removing more than held was accepted or changed the unit".into());
    }
    if !s.inventory_remove(u, 7, 5) || s.inventory_slot(u, 0) != (0, 0) {
        return Err(format!("removing all of kind 7 left slot0 {:?}", s.inventory_slot(u, 0)));
    }
    if !s.inventory_add(u, 13, 1) {
        return Err("a freed slot did not take a new kind".into());
    }
    Ok(())
}

pub fn xp_accumulates(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let u = spawn(&mut s, 0, "skirmisher", 10, 10)?;
    if s.xp(u) != 0 {
        return Err(format!("new unit holds {} xp, want 0", s.xp(u)));
    }
    s.add_xp(u, 100);
    s.add_xp(u, 250);
    if s.xp(u) != 350 {
        return Err(format!("gains of 100 and 250 left {} xp, want 350", s.xp(u)));
    }
    s.add_xp(u, u32::MAX);
    if s.xp(u) != u32::MAX {
        return Err(format!("a gain past u32::MAX left {} xp, want saturation at {}", s.xp(u), u32::MAX));
    }
    Ok(())
}

pub fn trigger_firing(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let a = spawn(&mut s, 0, "skirmisher", 10, 10)?;
    spawn(&mut s, 1, "skirmisher", 12, 10)?;
    s.take_fired();
    if s.stats.casts != 0 {
        return Err(format!("stats.casts {} before any tick, want 0", s.stats.casts));
    }
    if s.fire_trigger(a, "no_such_trigger") {
        return Err("an unknown trigger name reported as fired".into());
    }
    if s.fire_trigger(usize::MAX, "idle") {
        return Err("an out-of-range unit reported as fired".into());
    }
    if !s.fire_trigger(a, "idle") {
        return Err("the idle trigger on a live unit reported as not fired".into());
    }
    let queued = s.take_fired();
    let idle_casts = queued.iter().filter(|f| f.unit == a as u32 && f.trigger_name() == "idle" && f.action_name() == "use_ability").count();
    if queued.len() != 1 || idle_casts != 1 {
        return Err(format!("idle trigger queued {} behaviours, {idle_casts} of them an idle use_ability on unit {a}, want exactly that one", queued.len()));
    }
    run_ticks(&mut s, 200);
    if s.take_fired().is_empty() {
        return Err("no tick-driven behaviour trigger was queued over 200 ticks".to_string());
    }
    Ok(())
}

pub fn structure_damage(_image: &[u8]) -> Result<(), String> {
    let scenario = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("scenarios").join("duel-routes");
    let image = osdt_data::generate(&scenario).map_err(|e| format!("scenario image: {e:#}"))?;
    let mut s = fresh(&image)?;
    let beacon = spawn(&mut s, 1, "beacon", 20, 4)?;
    let skirmisher = spawn(&mut s, 0, "skirmisher", 18, 4)?;
    s.set_strike_target(skirmisher, beacon as i32).map_err(|e| format!("set_strike_target refused: {e}"))?;
    let before = fixed::to_int(s.hp(beacon));
    run_ticks(&mut s, 100);
    let after = fixed::to_int(s.hp(beacon));
    if after >= before {
        return Err(format!("beacon hp {before} -> {after} with an enemy skirmisher commanded to strike it in weapon range"));
    }
    println!("structure damage ok hp_before={before} hp_after={after}");
    Ok(())
}

fn scenario_group_unit() -> Result<(ModHost, usize), String> {
    let mut host = pack_run::start()?;
    run_past_first_camp_mark(&mut host)?;
    let u = (0..MAX_UNITS)
        .find(|&u| {
            let s = host.sim();
            s.alive(u) && s.player(u) == 0xFF && s.hp(u) > fixed::from_int(1) && s.hp(u) < fixed::from_int(5000)
        })
        .ok_or("moba image places no live neutral unit")?;
    Ok((host, u))
}

pub fn spawn_group_respawn(_image: &[u8]) -> Result<(), String> {
    let (mut host, u) = scenario_group_unit()?;
    let home = host.sim().position_fixed(u);
    let clear_reach = fixed::from_milli(scenario_rule("camp_clear_milli")?) as i64;
    let camp_mates: Vec<usize> = (0..MAX_UNITS)
        .filter(|&k| {
            let s = host.sim();
            k != u && s.alive(k) && s.player(k) == 0xFF && {
                let (x, y) = s.position_fixed(k);
                let (dx, dy) = (i64::from(x) - i64::from(home.0), i64::from(y) - i64::from(home.1));
                dx * dx + dy * dy <= clear_reach * clear_reach
            }
        })
        .collect();
    {
        let mut s = host.sim_mut();
        for &k in &camp_mates {
            s.hurt(k, fixed::from_int(10000));
        }
        s.hurt(u, fixed::from_int(10000));
    }
    let camp_alive = |host: &ModHost| -> usize {
        let s = host.sim();
        let n = (0..MAX_UNITS)
            .filter(|&k| {
                s.alive(k) && s.player(k) == 0xFF && s.kind_name(k).is_some_and(|n| n.starts_with("neutral_")) && {
                    let (x, y) = s.position_fixed(k);
                    let (dx, dy) = (i64::from(x) - i64::from(home.0), i64::from(y) - i64::from(home.1));
                    dx * dx + dy * dy <= clear_reach * clear_reach
                }
            })
            .count();
        n
    };
    if camp_alive(&host) != 0 {
        return Err(format!("camp unit {u} survived a lethal hit"));
    }
    let wait = (scenario_rule("camp_respawn_s")? * sim::TICKS_PER_SECOND) as u32;
    pack_run::advance(&mut host, wait - 4)?;
    if camp_alive(&host) != 0 {
        return Err(format!("the camp of unit {u} respawned before its {} s timer", (scenario_rule("camp_respawn_s")? as u32)));
    }
    pack_run::advance(&mut host, 16)?;
    let back = camp_alive(&host);
    if back < 2 {
        return Err(format!("the camp of unit {u} holds {back} units after its timer, want at least 2"));
    }
    println!("camp respawn ok unit={u} after={} ticks members={back}", wait);
    Ok(())
}

pub fn spawn_group_pickup(_image: &[u8]) -> Result<(), String> {
    let (mut host, _) = scenario_group_unit()?;
    let rune = (0..MAX_UNITS)
        .find(|&u| {
            let s = host.sim();
            s.alive(u) && s.player(u) == 0xFF && s.hp(u) == fixed::from_int(1)
        })
        .ok_or("moba image places no bounty rune")?;
    let (rx, ry) = host.sim().position_fixed(rune);
    let hero_kind = kind(&host.sim(), role("unit.hero")?)?;
    let hero = host
        .sim_mut()
        .spawn_attributed(0, hero_kind, rx + fixed::from_int(1), ry)
        .ok_or("spawn_attributed refused beside the rune")?;
    let gold = host.sim().resource_a(0);
    pack_run::advance(&mut host, 2)?;
    if host.sim().alive(rune) {
        return Err(format!("rune {rune} survived a hero in pickup range"));
    }
    let paid = host.sim().resource_a(0) - gold;
    let passive_slack = 2;
    let first_gold = scenario_rule("rune_first_gold")?;
    if paid < first_gold || paid > first_gold + passive_slack {
        return Err(format!("rune paid {paid} gold, want {first_gold} plus at most {passive_slack} passive gold"));
    }
    host.sim_mut().order(hero, rx - fixed::from_int(12), ry);
    pack_run::advance(&mut host, (scenario_rule("rune_period_s")? * sim::TICKS_PER_SECOND) as u32 + 8)?;
    if !host.sim().alive(rune) {
        return Err(format!("rune {rune} did not return on the next rune round"));
    }
    println!("camp rune pickup ok rune={rune} hero={hero}");
    Ok(())
}

pub fn spawn_group_boss_aegis(_image: &[u8]) -> Result<(), String> {
    let (mut host, _) = scenario_group_unit()?;
    let boss = (0..MAX_UNITS)
        .filter(|&u| {
            let s = host.sim();
            s.alive(u) && s.player(u) == 0xFF
        })
        .max_by_key(|&u| host.sim().hp(u))
        .ok_or("moba image places no neutral unit")?;
    let (bx, by) = host.sim().position_fixed(boss);
    let hero_kind = kind(&host.sim(), role("unit.hero")?)?;
    let hero = host
        .sim_mut()
        .spawn_attributed(0, hero_kind, bx + fixed::from_int(1), by)
        .ok_or("spawn_attributed refused beside Roshan")?;
    pack_run::hurt_to_life(&mut host, boss, fixed::from_int(1));
    let held = |host: &ModHost| {
        let s = host.sim();
        let n = (0..s.inventory_slots()).filter(|&k| s.inventory_slot(hero, k).0 != 0).count();
        n
    };
    let before = held(&host);
    host.sim_mut().order_attack(hero, boss).map_err(|r| format!("hero cannot attack roshan: {}", r.name()))?;
    for _ in 0..400 {
        pack_run::advance(&mut host, 1)?;
        if !host.sim().alive(boss) {
            break;
        }
    }
    if host.sim().alive(boss) {
        return Err(format!("hero did not kill unit {boss}"));
    }
    pack_run::advance(&mut host, 4)?;
    if held(&host) != before + 1 {
        return Err(format!("killer holds {} item slots, want {}", held(&host), before + 1));
    }
    println!("camp roshan aegis ok boss={boss} hero={hero}");
    Ok(())
}

pub fn hero_death_drop(_image: &[u8]) -> Result<(), String> {
    let (mut host, _) = scenario_group_unit()?;
    let rune = (0..MAX_UNITS)
        .find(|&u| {
            let s = host.sim();
            s.alive(u) && s.player(u) == 0xFF && s.hp(u) == fixed::from_int(1)
        })
        .ok_or("moba image places no bounty rune")?;
    let (rx, ry) = host.sim().position_fixed(rune);
    let hero_kind = kind(&host.sim(), role("unit.hero")?)?;
    let hero = host
        .sim_mut()
        .spawn_attributed(0, hero_kind, rx + fixed::from_int(1), ry)
        .ok_or("spawn_attributed refused beside the rune")?;
    let items = {
        let s = host.sim();
        let c = s.catalog();
        let found = (c.item_index("rapier"), c.item_index("gem"), c.item_index("flask"));
        found
    };
    let (rapier, gem, flask) = (
        items.0.ok_or("catalog has no rapier")?,
        items.1.ok_or("catalog has no gem")?,
        items.2.ok_or("catalog has no flask")?,
    );
    for item in [rapier, gem, flask] {
        if !host.sim_mut().grant_item(hero, item, 1) {
            return Err(format!("grant_item refused item {item}"));
        }
    }
    pack_run::advance(&mut host, 2)?;
    for _ in 0..8 {
        if !host.sim().alive(hero) {
            break;
        }
        host.sim_mut().hurt(hero, fixed::from_int(10000));
    }
    if host.sim().alive(hero) {
        return Err(format!("hero {hero} survived a lethal hit"));
    }
    let (hx, hy) = host.sim().position_fixed(hero);
    pack_run::advance(&mut host, 4)?;
    let s = host.sim();
    let on_ground = |item: usize| {
        let near = s.ground_items().iter().filter(|g| {
            let (dx, dy) = (i64::from(g.x) - i64::from(hx), i64::from(g.y) - i64::from(hy));
            g.kind == item as u32 + 1 && dx * dx + dy * dy <= i64::from(fixed::from_int(1)) * i64::from(fixed::from_int(1))
        });
        let n = near.count();
        n
    };
    let held = |item: usize| {
        let n = (0..s.inventory_slots()).filter(|&k| s.inventory_slot(hero, k).0 == item as u32 + 1).count();
        n
    };
    if on_ground(rapier) != 1 || on_ground(gem) != 1 || held(rapier) != 0 || held(gem) != 0 {
        return Err(format!(
            "rapier on ground {} held {}, gem on ground {} held {}, want 1 and 0 each",
            on_ground(rapier), held(rapier), on_ground(gem), held(gem)
        ));
    }
    if held(flask) != 1 || on_ground(flask) != 0 {
        return Err(format!("flask held {} on ground {}, want 1 and 0: only the Rapier and Gem drop", held(flask), on_ground(flask)));
    }
    println!("hero death drop ok hero={hero}");
    Ok(())
}

fn bottle_hero(with_rune: bool) -> Result<(ModHost, usize, (usize, usize, usize)), String> {
    let (mut host, _) = scenario_group_unit()?;
    let rune = (0..MAX_UNITS)
        .find(|&u| {
            let s = host.sim();
            s.alive(u) && s.player(u) == 0xFF && s.hp(u) == fixed::from_int(1)
        })
        .ok_or("moba image places no bounty rune")?;
    let (rx, ry) = host.sim().position_fixed(rune);
    if !with_rune {
        host.sim_mut().hurt(rune, fixed::from_int(10000));
    }
    let hero_kind = kind(&host.sim(), role("unit.hero")?)?;
    let hero = host
        .sim_mut()
        .spawn_attributed(0, hero_kind, rx + fixed::from_int(1), ry)
        .ok_or("spawn_attributed refused beside the rune")?;
    let items = {
        let s = host.sim();
        let c = s.catalog();
        let found = (c.item_index("bottle"), c.item_index("bottle_bounty"), c.item_index("bottle_empty"));
        found
    };
    let states = (
        items.0.ok_or("catalog has no bottle")?,
        items.1.ok_or("catalog has no bottle_bounty")?,
        items.2.ok_or("catalog has no bottle_empty")?,
    );
    if !host.sim_mut().grant_item(hero, states.0, 3) {
        return Err("grant_item refused the bottle".into());
    }
    Ok((host, hero, states))
}

fn held_of(host: &ModHost, hero: usize, item: usize) -> u32 {
    let s = host.sim();
    let n = s.inventory_count(hero, item as u32 + 1);
    n
}

fn use_held(host: &mut ModHost, hero: usize, item: usize) -> Result<(), String> {
    let slot = {
        let s = host.sim();
        let found = (0..s.inventory_slots()).find(|&k| s.inventory_slot(hero, k).0 == item as u32 + 1);
        found
    }
    .ok_or("hero holds no such item")?;
    let (x, y) = host.sim().position_fixed(hero);
    if !host.sim_mut().use_item(hero, slot, hero as i32, x, y) {
        return Err(format!("use_item refused slot {slot}: {}", host.sim().stats.last_refusal.name()));
    }
    Ok(())
}

pub fn bottle_rune(_image: &[u8]) -> Result<(), String> {
    let (mut host, hero, (full, stored, _)) = bottle_hero(true)?;
    let gold = host.sim().resource_a(0);
    pack_run::advance(&mut host, 6)?;
    if held_of(&host, hero, full) != 0 || held_of(&host, hero, stored) != 1 {
        return Err(format!("hero holds {} full bottles and {} rune bottles, want 0 and 1", held_of(&host, hero, full), held_of(&host, hero, stored)));
    }
    let passive_slack = 3;
    let paid = host.sim().resource_a(0) - gold;
    if paid > passive_slack {
        return Err(format!("the stored rune paid {paid} gold before it was drunk"));
    }
    use_held(&mut host, hero, stored)?;
    pack_run::advance(&mut host, 6)?;
    if held_of(&host, hero, stored) != 0 || held_of(&host, hero, full) != 3 {
        return Err(format!("after the drink the hero holds {} rune bottles and {} drinks, want 0 and 3", held_of(&host, hero, stored), held_of(&host, hero, full)));
    }
    let first_gold = scenario_rule("rune_first_gold")?;
    let paid = host.sim().resource_a(0) - gold;
    if paid < first_gold {
        return Err(format!("the drunk bounty rune paid {paid} gold, want at least {first_gold}"));
    }
    println!("bottle rune ok hero={hero} paid={paid}");
    Ok(())
}

pub fn bottle_rune_expires(_image: &[u8]) -> Result<(), String> {
    let (mut host, hero, (full, stored, _)) = bottle_hero(true)?;
    pack_run::advance(&mut host, 6)?;
    if held_of(&host, hero, stored) != 1 {
        return Err("the rune was not stored".into());
    }
    let store_ticks = 120 * sim::TICKS_PER_SECOND as u32;
    pack_run::advance(&mut host, store_ticks - 40)?;
    if held_of(&host, hero, stored) != 1 {
        return Err("the stored rune acted before its 120 s were up".into());
    }
    pack_run::advance(&mut host, 80)?;
    if held_of(&host, hero, stored) != 0 || held_of(&host, hero, full) != 3 {
        return Err(format!("after 120 s the hero holds {} rune bottles and {} drinks, want 0 and 3", held_of(&host, hero, stored), held_of(&host, hero, full)));
    }
    println!("bottle rune expiry ok hero={hero}");
    Ok(())
}

pub fn bottle_drink(_image: &[u8]) -> Result<(), String> {
    let (mut host, hero, (full, _, empty)) = bottle_hero(false)?;
    pack_run::advance(&mut host, 2)?;
    for left in [2u32, 1] {
        use_held(&mut host, hero, full)?;
        pack_run::advance(&mut host, 4)?;
        if held_of(&host, hero, full) != left {
            return Err(format!("after a drink the bottle holds {} drinks, want {left}", held_of(&host, hero, full)));
        }
        pack_run::advance(&mut host, 12)?;
    }
    use_held(&mut host, hero, full)?;
    pack_run::advance(&mut host, 4)?;
    if held_of(&host, hero, full) != 0 || held_of(&host, hero, empty) != 1 {
        return Err(format!("after the last drink the hero holds {} full and {} empty bottles, want 0 and 1", held_of(&host, hero, full), held_of(&host, hero, empty)));
    }
    println!("bottle drink ok hero={hero}");
    Ok(())
}

pub fn bloodstone_mana_trickle(_image: &[u8]) -> Result<(), String> {
    let (mut host, _) = scenario_group_unit()?;
    let hero_kind = kind(&host.sim(), role("unit.hero")?)?;
    let (x, y) = open_cell(&host.sim())?;
    let (holder, control) = {
        let mut s = host.sim_mut();
        let (hx, hy) = (fixed::from_int(x), fixed::from_int(y));
        let (cx, cy) = (fixed::from_int(x), fixed::from_int(y + 1));
        (s.spawn_attributed(0, hero_kind, hx, hy), s.spawn_attributed(0, hero_kind, cx, cy))
    };
    let (holder, control) = (holder.ok_or("spawn_attributed refused the holder")?, control.ok_or("spawn_attributed refused the control")?);
    let stone = host.sim().catalog().item_index("bloodstone").ok_or("catalog has no bloodstone")?;
    if !host.sim_mut().grant_item(holder, stone, 3) {
        return Err("grant_item refused the bloodstone".into());
    }
    pack_run::advance(&mut host, 2)?;
    let (charges, seconds) = (8, 4);
    {
        let mut s = host.sim_mut();
        s.set_mana_within(holder, 100);
        s.set_mana_within(control, 100);
    }
    pack_run::advance(&mut host, 16 * seconds)?;
    let (held, free) = {
        let s = host.sim();
        (s.mana_of(holder) - 100, s.mana_of(control) - 100)
    };
    let extra = held - free;
    println!("bloodstone mana trickle holder={held} control={free} extra={extra}");
    if extra < charges * seconds as i32 - charges {
        return Err(format!("a holder of {charges} charges gained {extra} more mana than a control over {seconds} s, want at least {}", charges * seconds as i32 - charges));
    }
    if extra > charges * seconds as i32 + 20 {
        return Err(format!("a holder of {charges} charges gained {extra} more mana than a control over {seconds} s, far above one per charge each second"));
    }
    Ok(())
}

fn hero_pair() -> Result<(ModHost, usize, usize), String> {
    let (mut host, _) = scenario_group_unit()?;
    let hero_kind = kind(&host.sim(), role("unit.hero")?)?;
    let (x, y) = open_cell(&host.sim())?;
    let (a, b) = {
        let mut s = host.sim_mut();
        (
            s.spawn_attributed(0, hero_kind, fixed::from_int(x), fixed::from_int(y)),
            s.spawn_attributed(0, hero_kind, fixed::from_int(x), fixed::from_int(y + 1)),
        )
    };
    Ok((host, a.ok_or("spawn_attributed refused the first hero")?, b.ok_or("spawn_attributed refused the second hero")?))
}

pub fn pipe_regain_lock(_image: &[u8]) -> Result<(), String> {
    let (mut host, caster, ally) = hero_pair()?;
    let (amount, duration, lock) = (400, 192, 800);
    host.sim_mut().shield_bonus_locked(caster, ally as i32, amount, duration, lock);
    let first = host.sim().shield_of(ally);
    if first != fixed::from_int(amount) {
        return Err(format!("the first barrier gave {first}, want {}", fixed::from_int(amount)));
    }
    pack_run::advance(&mut host, duration as u32 + 8)?;
    if host.sim().shield_of(ally) != 0 {
        return Err("the barrier outlived its 12 s".into());
    }
    host.sim_mut().shield_bonus_locked(caster, ally as i32, amount, duration, lock);
    if host.sim().shield_of(ally) != 0 {
        return Err("a hero regained the barrier before 50 s had elapsed".into());
    }
    pack_run::advance(&mut host, lock as u32)?;
    host.sim_mut().shield_bonus_locked(caster, ally as i32, amount, duration, lock);
    if host.sim().shield_of(ally) != fixed::from_int(amount) {
        return Err("a hero could not regain the barrier after 50 s".into());
    }
    println!("pipe regain lock ok");
    Ok(())
}

pub fn kelens_heavy_hit(_image: &[u8]) -> Result<(), String> {
    let (mut host, attacker, victim) = hero_pair()?;
    if host.sim().heavy_hit_age(victim).is_some() {
        return Err("a fresh hero already carries a heavy hit".into());
    }
    host.sim_mut().hit_by(victim, fixed::from_int(2), attacker as i32);
    if host.sim().heavy_hit_age(victim).is_some() {
        return Err("a hit of 2 stamped the victim, only more than 2 should".into());
    }
    host.sim_mut().hit_by(victim, fixed::from_int(3), attacker as i32);
    pack_run::advance(&mut host, 5)?;
    match host.sim().heavy_hit_age(victim) {
        Some(5) => {}
        other => return Err(format!("a hit of 3 five ticks ago reads age {other:?}, want Some(5)")),
    }
    println!("kelens heavy hit ok");
    Ok(())
}

pub fn guardian_low_life_aura(_image: &[u8]) -> Result<(), String> {
    let (mut host, holder, ally) = hero_pair()?;
    let greaves = host.sim().catalog().item_index("guardian_greaves").ok_or("catalog has no guardian_greaves")?;
    if !host.sim_mut().grant_item(holder, greaves, 3) {
        return Err("grant_item refused the guardian greaves".into());
    }
    pack_run::advance(&mut host, 40)?;
    let healthy = host.sim().eff_armour_of(ally);
    let max = host.sim().hp_max(ally);
    host.sim_mut().set_health(ally, max / 10, max);
    pack_run::advance(&mut host, 40)?;
    let wounded = host.sim().eff_armour_of(ally);
    let gain = (wounded - healthy) / fixed::ONE;
    println!("guardian low life aura healthy={healthy} wounded={wounded} gain={gain}");
    if gain != 13 {
        return Err(format!("an ally under a fifth of its life gained {gain} armour over a healthy one, want 13"));
    }
    Ok(())
}

pub fn bottle_swap_in_place(_image: &[u8]) -> Result<(), String> {
    let (mut host, hero, (full, _, empty)) = bottle_hero(false)?;
    if !host.sim_mut().grant_item(hero, empty, 1) {
        return Err("grant_item refused the empty bottle".into());
    }
    let slot_of = |host: &ModHost, item: usize| {
        let s = host.sim();
        (0..s.inventory_slots()).find(|&k| s.inventory_slot(hero, k).0 == item as u32 + 1)
    };
    let full_slot = slot_of(&host, full).ok_or("hero holds no full bottle")?;
    pack_run::advance(&mut host, 2)?;
    for _ in 0..3 {
        use_held(&mut host, hero, full)?;
        pack_run::advance(&mut host, 16)?;
    }
    let empty_slots = {
        let s = host.sim();
        (0..s.inventory_slots()).filter(|&k| s.inventory_slot(hero, k).0 == empty as u32 + 1).count()
    };
    if empty_slots != 2 || held_of(&host, hero, empty) != 2 {
        return Err(format!("the hero holds {} empty bottles in {empty_slots} slots, want 2 in 2", held_of(&host, hero, empty)));
    }
    if host.sim().inventory_slot(hero, full_slot).0 != empty as u32 + 1 {
        return Err(format!("the drunk bottle left slot {full_slot}"));
    }
    println!("bottle swap in place ok hero={hero} slot={full_slot}");
    Ok(())
}

fn bottle_item(host: &ModHost, name: &str) -> Result<usize, String> {
    host.sim().catalog().item_index(name).ok_or_else(|| format!("catalog has no {name}"))
}

pub fn bottle_regen_no_restart(_image: &[u8]) -> Result<(), String> {
    let (mut host, hero, (full, _, empty)) = bottle_hero(false)?;
    pack_run::advance(&mut host, 2)?;
    use_held(&mut host, hero, full)?;
    pack_run::advance(&mut host, 4)?;
    let until = host.sim().counter(hero, "bottle_regen_until");
    if until == 0 {
        return Err("a drink started no regeneration".into());
    }
    pack_run::advance(&mut host, 12)?;
    use_held(&mut host, hero, full)?;
    pack_run::advance(&mut host, 4)?;
    if host.sim().counter(hero, "bottle_regen_until") != until {
        return Err("a drink during a running regeneration restarted it".into());
    }
    if held_of(&host, hero, full) != 1 {
        return Err(format!("the second drink left {} drinks, want 1: it still spends a charge", held_of(&host, hero, full)));
    }
    pack_run::advance(&mut host, 3 * sim::TICKS_PER_SECOND as u32)?;
    if host.sim().counter(hero, "bottle_regen_until") != 0 {
        return Err("the regeneration did not end".into());
    }
    use_held(&mut host, hero, full)?;
    pack_run::advance(&mut host, 4)?;
    if held_of(&host, hero, empty) != 1 || host.sim().counter(hero, "bottle_regen_until") == 0 {
        return Err("a drink after the regeneration ended did not start a new one".into());
    }
    println!("bottle regen ok hero={hero}");
    Ok(())
}

fn warning_shown(host: &mut ModHost) -> bool {
    host.take_ui().iter().any(|m| m.contains("Bottled rune will be activated in 20 seconds"))
}

pub fn bottle_rune_warning(_image: &[u8]) -> Result<(), String> {
    let (mut host, hero, (_, stored, _)) = bottle_hero(true)?;
    pack_run::advance(&mut host, 6)?;
    if held_of(&host, hero, stored) != 1 {
        return Err("the rune was not stored".into());
    }
    host.take_ui();
    let tps = sim::TICKS_PER_SECOND as u32;
    pack_run::advance(&mut host, 100 * tps - 16)?;
    if warning_shown(&mut host) {
        return Err("the warning showed before the last 20 s".into());
    }
    pack_run::advance(&mut host, 32)?;
    if !warning_shown(&mut host) {
        return Err("no warning showed 20 s before the stored rune acts".into());
    }
    pack_run::advance(&mut host, 8 * tps)?;
    if warning_shown(&mut host) {
        return Err("the warning showed twice".into());
    }
    println!("bottle warning ok hero={hero}");
    Ok(())
}

pub fn bottle_capture_cast(_image: &[u8]) -> Result<(), String> {
    let (mut host, _) = scenario_group_unit()?;
    let rune = (0..MAX_UNITS)
        .find(|&u| {
            let s = host.sim();
            s.alive(u) && s.player(u) == 0xFF && s.hp(u) == fixed::from_int(1)
        })
        .ok_or("moba image places no bounty rune")?;
    let (rx, ry) = host.sim().position_fixed(rune);
    let hero_kind = kind(&host.sim(), role("unit.hero")?)?;
    let hero = host
        .sim_mut()
        .spawn_attributed(0, hero_kind, rx + fixed::from_milli(2300), ry)
        .ok_or("spawn_attributed refused beside the rune")?;
    let (empty, stored) = (bottle_item(&host, "bottle_empty")?, bottle_item(&host, "bottle_bounty")?);
    if !host.sim_mut().grant_item(hero, empty, 1) {
        return Err("grant_item refused the empty bottle".into());
    }
    host.sim_mut().set_counter(hero, "bottle_autopickup_off", 1);
    pack_run::advance(&mut host, 1)?;
    if !host.sim().alive(rune) {
        let (hx, hy) = host.sim().position_fixed(hero);return Err(format!("the rune was taken from beyond the pickup range: hero {} {} rune {} {}", hx, hy, rx, ry));
    }
    let gold = host.sim().resource_a(0);
    use_held(&mut host, hero, empty)?;
    pack_run::advance(&mut host, 6)?;
    if host.sim().alive(rune) {
        return Err("the capture cast left the rune on the map".into());
    }
    if held_of(&host, hero, stored) != 1 || held_of(&host, hero, empty) != 0 {
        return Err(format!("after the capture the hero holds {} rune bottles and {} empty, want 1 and 0", held_of(&host, hero, stored), held_of(&host, hero, empty)));
    }
    if host.sim().resource_a(0) - gold > 3 {
        return Err("the captured rune paid out before it was drunk".into());
    }
    println!("bottle capture ok hero={hero}");
    Ok(())
}

pub fn bottle_courier_slow(_image: &[u8]) -> Result<(), String> {
    let (mut host, hero, (full, _, empty)) = bottle_hero(false)?;
    let (cx, cy) = host.sim().position_fixed(hero);
    let courier_kind = kind(&host.sim(), "courier")?;
    let courier = host
        .sim_mut()
        .spawn_for(0, courier_kind, cx + fixed::from_int(3), cy)
        .ok_or("spawn_for refused a courier")?;
    if !host.sim_mut().grant_item(courier, empty, 1) {
        return Err("grant_item refused the empty bottle to the courier".into());
    }
    pack_run::advance(&mut host, 12)?;
    let slow = scenario_rule("bottle_slow_pct")? as i64;
    if host.sim().counter(courier, "bottle_slow") != -slow {
        return Err(format!("a courier with an empty bottle holds slow {}, want -{slow}", host.sim().counter(courier, "bottle_slow")));
    }
    if host.sim().counter(hero, "bottle_slow") != 0 {
        return Err("a hero holding a bottle was slowed".into());
    }
    if !host.sim_mut().inventory_remove(courier, empty as u32 + 1, 1) || !host.sim_mut().grant_item(courier, full, 3) {
        return Err("the courier's bottle could not be filled".into());
    }
    pack_run::advance(&mut host, 12)?;
    if host.sim().counter(courier, "bottle_slow") != 0 {
        return Err("a courier with a full bottle is still slowed".into());
    }
    println!("bottle courier ok courier={courier}");
    Ok(())
}

fn bottle_pair(offsets: &[(i32, i32)]) -> Result<(ModHost, usize, usize, u8, (usize, usize, usize)), String> {
    let (mut host, owner, states) = bottle_hero(false)?;
    let owner_player = host.sim().player(owner);
    let ally_player = {
        let s = host.sim();
        (1..10u8).find(|&p| s.team_of(p).is_some() && s.team_of(p) == s.team_of(owner_player)).ok_or("no allied player")?
    };
    let hero_kind = kind(&host.sim(), role("unit.hero")?)?;
    let (ox, oy) = host.sim().position_fixed(owner);
    let ally = offsets
        .iter()
        .find_map(|&(dx, dy)| host.sim_mut().spawn_attributed(ally_player, hero_kind, ox + fixed::from_milli(dx), oy + fixed::from_milli(dy)))
        .ok_or("spawn_attributed refused the ally at every offset")?;
    let (ax, ay) = host.sim().position_fixed(ally);
    let gap = (((i64::from(ax) - i64::from(ox)).pow(2) + (i64::from(ay) - i64::from(oy)).pow(2)) as f64).sqrt() / f64::from(fixed::from_milli(1));
    if offsets.len() > 4 && gap < 4500.0 {
        return Err(format!("the far ally stands only {gap} milli from the owner"));
    }
    pack_run::advance(&mut host, 2)?;
    Ok((host, owner, ally, owner_player, states))
}

fn drop_held(host: &mut ModHost, from: usize, item: usize, at: usize) -> Result<(), String> {
    let slot = {
        let s = host.sim();
        (0..s.inventory_slots()).find(|&k| s.inventory_slot(from, k).0 == item as u32 + 1)
    }
    .ok_or("the dropping hero holds no such item")?;
    let (x, y) = host.sim().position_fixed(at);
    host.sim_mut().drop_item_to_ground(from, slot, x, y).map_err(|r| format!("drop refused: {}", r.name()))?;
    Ok(())
}

pub fn bottle_share_dose(_image: &[u8]) -> Result<(), String> {
    let (mut host, owner, ally, _, (full, _, empty)) = bottle_pair(&[(2500, 0), (0, 2500), (-2500, 0), (0, -2500)])?;
    drop_held(&mut host, owner, full, ally)?;
    pack_run::advance(&mut host, 10)?;
    if host.sim().counter(ally, "bottle_regen_until") == 0 {
        return Err("the ally who took the bottle beside its owner drank no dose".into());
    }
    if held_of(&host, owner, full) != 2 || held_of(&host, ally, full) != 0 || !host.sim().ground_items().is_empty() {
        return Err(format!(
            "owner holds {} drinks, ally holds {}, {} items lie on the ground, want 2, 0 and 0",
            held_of(&host, owner, full),
            held_of(&host, ally, full),
            host.sim().ground_items().len()
        ));
    }
    let _ = empty;
    println!("bottle share ok owner={owner} ally={ally}");
    Ok(())
}

pub fn bottle_loan_and_return(_image: &[u8]) -> Result<(), String> {
    let (mut host, owner, ally, owner_player, (full, _, _)) = bottle_pair(&[(8000, 0), (-8000, 0), (0, 8000), (0, -8000), (4000, 0), (-4000, 0), (0, 4000), (0, -4000)])?;
    let loan = bottle_item(&host, "bottle_loan")?;
    drop_held(&mut host, owner, full, ally)?;
    pack_run::advance(&mut host, 10)?;
    if held_of(&host, ally, loan) != 3 || held_of(&host, owner, full) != 0 || held_of(&host, ally, full) != 0 {
        return Err(format!(
            "ally holds {} loaned drinks and {} own, owner holds {}, want 3, 0 and 0",
            held_of(&host, ally, loan),
            held_of(&host, ally, full),
            held_of(&host, owner, full)
        ));
    }
    let slot = {
        let s = host.sim();
        (0..s.inventory_slots()).find(|&k| s.inventory_slot(ally, k).0 == loan as u32 + 1)
    }
    .ok_or("the loaned bottle left the ally")?;
    if host.sim().item_owner(ally, slot) != Some(usize::from(owner_player)) {
        return Err("the loaned bottle does not name its owner".into());
    }
    if host.sim().counter(ally, "bottle_regen_until") != 0 {
        return Err("a far ally drank from the bottle".into());
    }
    drop_held(&mut host, ally, loan, owner)?;
    pack_run::advance(&mut host, 10)?;
    if held_of(&host, owner, full) != 3 || held_of(&host, owner, loan) != 0 || held_of(&host, ally, loan) != 0 {
        return Err(format!(
            "after the owner took it back the owner holds {} drinks and {} loaned, the ally {} loaned, want 3, 0 and 0",
            held_of(&host, owner, full),
            held_of(&host, owner, loan),
            held_of(&host, ally, loan)
        ));
    }
    println!("bottle loan ok owner={owner} ally={ally}");
    Ok(())
}

pub fn region_query(image: &[u8]) -> Result<(), String> {
    let src = r#"
define_region("zone", 0, 0, 655360, 655360)
function on_tick(state)
  state.inside = in_region("zone", 327680, 327680)
  state.outside = in_region("zone", 1310720, 327680)
end
"#;
    let path = std::env::temp_dir().join(MECHANIC_REGION_MOD);
    let _guard = RemoveOnDrop(path.clone());
    std::fs::write(&path, src).map_err(|e| format!("write {}: {e}", path.display()))?;
    let mut host = mod_host::ModHost::new(image).map_err(|e| format!("ModHost::new: {e:?}"))?;
    host.load_file(&path).map_err(|e| format!("mod load: {e}"))?;
    host.advance().map_err(|e| format!("advance: {e}"))?;
    let inside = host.state_bool("inside");
    let outside = host.state_bool("outside");
    if inside != Some(true) {
        return Err(format!("point inside the region reads {inside:?}"));
    }
    if outside != Some(false) {
        return Err(format!("point outside the region reads {outside:?}"));
    }
    Ok(())
}

struct RemoveOnDrop(std::path::PathBuf);

impl Drop for RemoveOnDrop {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
fn scenario_image() -> Result<Vec<u8>, String> {
    let scenario = crate::checks::pack_dir();
    osdt_data::generate(&scenario).map_err(|e| format!("moba scenario image: {e:#}"))
}

pub fn scenario_units(_image: &[u8]) -> Result<(), String> {
    let mut s = fresh(&scenario_image()?)?;
    let r_unit_base = role("unit.base")?;
    let own = (0..MAX_UNITS).find(|&i| s.alive(i) && s.player(i) == 0 && s.kind_name(i) == Some(r_unit_base)).ok_or("moba has no side 0 base")?;
    let r_unit_base = role("unit.base")?;
    let rival = (0..MAX_UNITS).find(|&i| s.alive(i) && s.player(i) == 1 && s.kind_name(i) == Some(r_unit_base)).ok_or("moba has no side 1 base")?;
    let (ox, oy) = s.position_fixed(own);
    let (tx, ty) = s.position_fixed(rival);
    let k = kind(&s, role("unit.creep")?)?;
    let creep = s.spawn_for(0, k, ox, oy).ok_or("spawn_for refused a creep at the side 0 base")?;
    let dist2 = |s: &Sim| {
        let (x, y) = s.position_fixed(creep);
        let (dx, dy) = (i64::from(tx) - i64::from(x), i64::from(ty) - i64::from(y));
        dx * dx + dy * dy
    };
    let before = dist2(&s);
    s.order(creep, tx, ty);
    run_ticks(&mut s, 200);
    let after = dist2(&s);
    if after >= before {
        return Err(format!("creep did not close on the rival base: squared distance {before} -> {after}"));
    }
    let progress = (before as f64).sqrt() - (after as f64).sqrt();
    println!("moba creeps ok lane_progress={}", progress as i64);
    Ok(())
}

pub fn scenario_route(_image: &[u8]) -> Result<(), String> {
    let mut s = fresh(&scenario_image()?)?;
    let r_unit_base = role("unit.base")?;
    let own = (0..MAX_UNITS).find(|&i| s.alive(i) && s.player(i) == 0 && s.kind_name(i) == Some(r_unit_base)).ok_or("moba has no side 0 base")?;
    let (ox, oy) = s.position_fixed(own);
    let k = kind(&s, role("unit.creep")?)?;
    let creep = s.spawn_for(0, k, ox, oy).ok_or("spawn_for refused a creep at the side 0 base")?;
    let (gx, gy) = (fixed::from_milli(61922), fixed::from_milli(177125));
    s.order(creep, gx, gy);
    let one_tile = i64::from(fixed::from_int(1));
    let mut connector = false;
    for t in 1..=600 {
        s.tick();
        if !s.alive(creep) {
            return Err(format!("creep died on the lane walk at tick {t}"));
        }
        let (x, y) = s.position_fixed(creep);
        let (cx, cy) = (x >> 16, y >> 16);
        if !s.grid().walkable(cx, cy) {
            return Err(format!("creep stands on a blocked cell ({cx}, {cy}) at tick {t}"));
        }
        connector |= (56..80).contains(&cx) && (190..216).contains(&cy);
        let (dx, dy) = (i64::from(gx) - i64::from(x), i64::from(gy) - i64::from(y));
        if dx * dx + dy * dy <= one_tile * one_tile {
            if !connector {
                return Err("creep reached the north lane without walking the connector column".into());
            }
            println!("moba lane ok ticks={t} connector=true");
            return Ok(());
        }
    }
    Err("creep did not reach the north lane point within 600 ticks".into())
}

const CREEP_SPEED_MILLI: i64 = 317;
const CREEP_REACH_MILLI: i64 = 1562;
const CREEP_WEAPON_PERIOD: u32 = 16;

pub fn unit_stops_to_attack(_image: &[u8]) -> Result<(), String> {
    let mut host = pack_run::start()?;
    let (k, ox, oy, tx, ty, friend, enemy, d0) = {
        let mut s = host.sim_mut();
        let r_unit_base = role("unit.base")?;
        let own = (0..MAX_UNITS).find(|&i| s.alive(i) && s.player(i) == 0 && s.kind_name(i) == Some(r_unit_base)).ok_or("moba has no side 0 base")?;
        let rival = (0..MAX_UNITS).find(|&i| s.alive(i) && s.player(i) == 1 && s.kind_name(i) == Some(r_unit_base)).ok_or("moba has no side 1 base")?;
        let (ox, oy) = s.position_fixed(own);
        let (tx, ty) = s.position_fixed(rival);
        let k = kind(&s, role("unit.creep")?)?;
        let friend = s.spawn_for(0, k, ox, oy).ok_or("spawn_for refused a creep at the side 0 base")?;
        let (fx, fy) = s.position_fixed(friend);
        let (dx, dy) = (f64::from(tx) - f64::from(fx), f64::from(ty) - f64::from(fy));
        let len = (dx * dx + dy * dy).sqrt();
        let tile = f64::from(fixed::from_int(1));
        let mut placed = None;
        for step in 0..=8 {
            let d = 4.0 + 0.5 * f64::from(step);
            let ex = (f64::from(fx) + dx / len * d * tile).round() as i32;
            let ey = (f64::from(fy) + dy / len * d * tile).round() as i32;
            if !s.grid().walkable(ex >> 16, ey >> 16) {
                continue;
            }
            if let Some(e) = s.spawn_for(1, k, ex, ey) {
                placed = Some((e, d));
                break;
            }
        }
        let (enemy, d0) = placed.ok_or("no walkable lane cell 4 to 8 tiles from the side 0 creep for an enemy creep")?;
        (k, ox, oy, tx, ty, friend, enemy, d0)
    };
    let tile = f64::from(fixed::from_int(1));
    let (ex0, ey0) = host.sim().position_fixed(enemy);
    {
        let mut s = host.sim_mut();
        s.order(enemy, ex0, ey0);
        s.issue_order_kind(friend, sim::ORDER_ATTACK_MOVE, tx, ty).map_err(|e| format!("attack-move refused: {}", e.name()))?;
    }
    let reach = CREEP_REACH_MILLI as f64 / 1000.0;
    let speed = CREEP_SPEED_MILLI as f64 / 1000.0;
    let approach = ((d0 - reach) / speed).ceil() as u32;
    let budget = 2 * approach + 16;
    let gap = |s: &Sim| {
        let (x, y) = s.position_fixed(friend);
        let (ex, ey) = s.position_fixed(enemy);
        let (gx, gy) = (f64::from(x) - f64::from(ex), f64::from(y) - f64::from(ey));
        (gx * gx + gy * gy).sqrt() / tile
    };
    let enemy_hp0 = host.sim().hp(enemy);
    let mut stop_tick = None;
    let mut prev = host.sim().position_fixed(friend);
    for t in 1..=budget {
        pack_run::advance(&mut host, 1)?;
        let s = host.sim();
        if !s.alive(friend) {
            return Err(format!("creep died on the approach at tick {t}"));
        }
        let now = s.position_fixed(friend);
        if now == prev && gap(&*s) <= reach + 1.0 {
            stop_tick = Some(t);
            break;
        }
        prev = now;
    }
    let t_stop = stop_tick.ok_or(format!("creep did not reach enemy reach within {budget} ticks from {d0:.1} tiles away"))?;
    let stop_pos = host.sim().position_fixed(friend);
    let hp_at_stop = host.sim().hp(enemy);
    for t in 0..3 * CREEP_WEAPON_PERIOD {
        if !host.sim().alive(enemy) {
            break;
        }
        pack_run::advance(&mut host, 1)?;
        let s = host.sim();
        if s.alive(enemy) && s.position_fixed(friend) != stop_pos {
            return Err(format!("engaged creep moved {} ticks after stopping at tick {t_stop} while its target lived", t + 1));
        }
    }
    if host.sim().hp(enemy) >= hp_at_stop {
        return Err(format!("engaged creep struck nothing: enemy hp {hp_at_stop} unchanged while in reach"));
    }
    host.sim_mut().hurt(enemy, fixed::from_int(10000));
    if host.sim().alive(enemy) {
        return Err("enemy creep survived lethal damage".into());
    }
    let mut resumed = false;
    for _ in 0..budget {
        pack_run::advance(&mut host, 1)?;
        if host.sim().position_fixed(friend) != stop_pos {
            resumed = true;
            break;
        }
    }
    if !resumed {
        return Err(format!("creep stayed put {budget} ticks after its target died"));
    }
    println!("creep engagement ok stop_tick={t_stop} enemy_hp {enemy_hp0} -> {}", host.sim().hp(enemy));
    let mut walker = pack_run::start()?;
    let free = walker.sim_mut().spawn_for(0, k, ox, oy).ok_or("spawn_for refused a walking creep at the side 0 base")?;
    walker.sim_mut().issue_order_kind(free, sim::ORDER_ATTACK_MOVE, tx, ty).map_err(|e| format!("attack-move refused: {}", e.name()))?;
    let start = walker.sim().position_fixed(free);
    pack_run::advance(&mut walker, 30)?;
    if walker.sim().position_fixed(free) == start {
        return Err("a creep with no enemy in reach did not walk its attack-move route".into());
    }
    Ok(())
}

fn open_cell(s: &Sim) -> Result<(i32, i32), String> {
    let g = s.grid();
    let (w, h) = (g.w as i32, g.h as i32);
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            if [(x, y), (x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)].iter().all(|&(a, b)| g.walkable(a, b)) {
                return Ok((x, y));
            }
        }
    }
    Err("the moba grid has no cell with four walkable neighbours".into())
}

pub fn unit_strikes_unit_before_structure(_image: &[u8]) -> Result<(), String> {
    let mut host = pack_run::start()?;
    let (friend, enemy, tower) = {
        let mut s = host.sim_mut();
        let (x, y) = open_cell(&s)?;
        let creep = kind(&s, role("unit.creep")?)?;
        let tower_kind = kind(&s, role("unit.tower")?)?;
        let friend = s.spawn_for(0, creep, fixed::from_int(x), fixed::from_int(y)).ok_or("spawn_for refused the friendly creep")?;
        let enemy = s.spawn_for(1, creep, fixed::from_int(x + 1), fixed::from_int(y)).ok_or("spawn_for refused the enemy creep")?;
        let tower = s.spawn_for(1, tower_kind, fixed::from_int(x - 1), fixed::from_int(y)).ok_or("spawn_for refused the tower")?;
        (friend, enemy, tower)
    };
    let tower_hp = host.sim().hp(tower);
    let mut enemy_hp = host.sim().hp(enemy);
    let mut strikes = 0;
    for t in 1..=3 * CREEP_WEAPON_PERIOD {
        pack_run::advance(&mut host, 1)?;
        let s = host.sim();
        if !s.alive(friend) {
            return Err(format!("friendly creep died at tick {t} before its strikes landed"));
        }
        if !s.alive(enemy) {
            break;
        }
        if s.hp(tower) != tower_hp {
            return Err(format!("tower hp {} -> {} at tick {t} while the enemy creep lived in reach", tower_hp, s.hp(tower)));
        }
        if s.hp(enemy) < enemy_hp {
            strikes += 1;
            enemy_hp = s.hp(enemy);
        }
    }
    if strikes < 3 {
        return Err(format!("friendly creep struck the enemy creep {strikes} times in {} ticks, want 3", 3 * CREEP_WEAPON_PERIOD));
    }
    println!("creep strike priority ok strikes={strikes} enemy_hp={enemy_hp} tower_hp={tower_hp}");
    Ok(())
}

pub fn reused_slot_drops_stale_target(_image: &[u8]) -> Result<(), String> {
    let mut host = pack_run::start()?;
    let (x, y, creep, friend, a) = {
        let mut s = host.sim_mut();
        let (x, y) = open_block(&s)?;
        let creep = kind(&s, role("unit.creep")?)?;
        let friend = s.spawn_for(0, creep, fixed::from_int(x), fixed::from_int(y)).ok_or("spawn_for refused the friendly creep")?;
        s.spawn_for(0, creep, fixed::from_int(x), fixed::from_int(y - 1)).ok_or("spawn_for refused the second friendly creep")?;
        s.spawn_for(0, creep, fixed::from_int(x + 1), fixed::from_int(y - 1)).ok_or("spawn_for refused the third friendly creep")?;
        let a = s.spawn_for(1, creep, fixed::from_int(x + 1), fixed::from_int(y)).ok_or("spawn_for refused enemy creep A")?;
        for i in [friend, friend + 1, friend + 2, a] {
            hold_in_place(&mut s, i)?;
        }
        (x, y, creep, friend, a)
    };
    let mut t = 0;
    while host.sim().alive(a) {
        t += 1;
        if t > RUN_TICKS {
            return Err(format!("enemy creep A in slot {a} never died in {RUN_TICKS} ticks"));
        }
        pack_run::advance(&mut host, 1)?;
        if !host.sim().alive(friend) {
            return Err(format!("friendly creep died at tick {t} before enemy creep A died"));
        }
    }
    let (c, b_hp, c_hp) = {
        let mut s = host.sim_mut();
        if !s.spawn_at(a, creep, 1, fixed::from_int(x + 1), fixed::from_int(y + 1)) {
            return Err(format!(
                "spawn_at refused the dead slot {a} for enemy creep B at cell ({}, {}): slot alive {}, cell walkable {}, block origin ({x}, {y})",
                x + 1,
                y + 1,
                s.alive(a),
                s.grid().walkable(x + 1, y + 1)
            ) + &format!(
                "; tick {}, obstacles over the cell: {:?}, flags {:#06b}",
                s.ticks(),
                s.obstacles().iter().filter(|o| (o.x0..o.x0 + o.w).contains(&(x + 1)) && (o.y0..o.y0 + o.h).contains(&(y + 1))).collect::<Vec<_>>(),
                s.grid().flags[((y + 1) * s.grid().w as i32 + x + 1) as usize]
            ));
        }
        if !s.alive(a) || s.player(a) != 1 {
            return Err(format!("reused slot {a} is not a live enemy after spawn_at"));
        }
        let c = s.spawn_for(1, creep, fixed::from_int(x), fixed::from_int(y + 1)).ok_or("spawn_for refused enemy creep C")?;
        hold_in_place(&mut s, a)?;
        hold_in_place(&mut s, c)?;
        (c, s.hp(a), s.hp(c))
    };
    let watched = [friend, friend + 1, friend + 2, a, c];
    let mut seen: Vec<i32> = watched.iter().map(|&k| host.sim().hp(k)).collect();
    let mut trace: Vec<String> = Vec::new();
    for t in 1..=3 * CREEP_WEAPON_PERIOD {
        pack_run::advance(&mut host, 1)?;
        let s = host.sim();
        for (n, &k) in watched.iter().enumerate() {
            if s.hp(k) != seen[n] {
                trace.push(format!("t{t} unit {k} hp {} -> {}", seen[n], s.hp(k)));
                seen[n] = s.hp(k);
            }
        }
        if !s.alive(friend) {
            return Err(format!("friendly creep died at tick {t} before its first strike on B or C"));
        }
        if s.hp(a) < b_hp {
            let strikers: Vec<String> = [friend, friend + 1, friend + 2]
                .iter()
                .map(|&k| format!("unit {k} at {:?} commanded {} sticky {}", s.position_fixed(k), s.commanded_strike_index(k), s.strike_of(k)))
                .collect();
            return Err(format!(
                "friendly creep struck enemy creep B in reused slot {a} at tick {t} instead of nearer enemy creep C in slot {c}: stale sticky target; B at {:?} hp {} -> {}, C at {:?}, {}; hp changes: {}",
                s.position_fixed(a),
                b_hp,
                s.hp(a),
                s.position_fixed(c),
                strikers.join("; "),
                trace.join(", ")
            ));
        }
        if s.hp(c) < c_hp {
            return Ok(());
        }
    }
    Err(format!("friendly creep struck neither B (slot {a}) nor C (slot {c}) in {} ticks", 3 * CREEP_WEAPON_PERIOD))
}

pub fn reused_slot_gets_its_own_unit_rules(_image: &[u8]) -> Result<(), String> {
    let mut host = pack_run::start()?;
    let (creep, tower_kind, x, y) = {
        let s = host.sim();
        let (x, y) = open_cell(&s)?;
        (kind(&s, role("unit.creep")?)?, kind(&s, role("unit.tower")?)?, x, y)
    };
    let at = |dx: i32, dy: i32| (fixed::from_int(x + dx), fixed::from_int(y + dy));
    let (px, py) = at(0, 0);
    let victim = host.sim_mut().spawn_for(0, creep, px, py).ok_or("spawn_for refused the creep")?;
    let (cx, cy) = at(1, 0);
    let control_creep = host.sim_mut().spawn_for(0, creep, cx, cy).ok_or("spawn_for refused the control creep")?;
    pack_run::advance(&mut host, 4)?;
    let creep_scale = host.sim().armour_scale_of(control_creep);
    if creep_scale == fixed::ONE || host.sim().armour_scale_of(victim) != creep_scale {
        return Err(format!("the pack gave the creeps armour scale {} and {}, want one value below 1.0", host.sim().armour_scale_of(victim), creep_scale));
    }
    for (name, reuse_kind, player) in [("creep", creep, 0u8), ("tower", tower_kind, 1u8)] {
        host.sim_mut().hurt(victim, fixed::from_int(10000));
        pack_run::advance(&mut host, 2)?;
        if host.sim().alive(victim) {
            return Err(format!("slot {victim} still lives after lethal damage"));
        }
        if !host.sim_mut().spawn_at(victim, reuse_kind, player, px, py) {
            return Err(format!("spawn_at refused slot {victim} for the {name}"));
        }
        let (dx, dy) = if name == "creep" { at(0, 1) } else { at(-1, 0) };
        let control = host.sim_mut().spawn_for(player, reuse_kind, dx, dy).ok_or_else(|| format!("spawn_for refused the control {name}"))?;
        pack_run::advance(&mut host, 4)?;
        let (got, want) = (host.sim().armour_scale_of(victim), host.sim().armour_scale_of(control));
        if got != want {
            return Err(format!("the {name} in reused slot {victim} holds armour scale {got}, a fresh {name} holds {want}"));
        }
        if name == "tower" && got == creep_scale {
            return Err(format!("the tower in reused slot {victim} holds the dead creep's armour scale {got}"));
        }
    }
    Ok(())
}

pub fn deny_gate(_image: &[u8]) -> Result<(), String> {
    let host = pack_run::start()?;
    let (x, y) = {
        let s = host.sim();
        open_cell(&s)?
    };
    let creep = role("unit.creep")?;
    let (attacker, ally) = {
        let mut s = host.sim_mut();
        (spawn(&mut s, 0, creep, x, y)?, spawn(&mut s, 0, creep, x + 1, y)?)
    };
    let hp_max = host.sim().hp_max(ally);
    let at_percent = |p: i64| (hp_max as i64 * p / 100) as i32;
    for percent in [100, 60, 51] {
        host.sim_mut().set_health(ally, at_percent(percent), hp_max);
        let before = host.sim().hp(ally);
        let ordered = host.sim_mut().order_attack_or_approach(attacker, ally);
        if ordered.is_ok() || host.sim().hp(ally) != before {
            return Err(format!("an ally at {percent} percent of its life was attackable"));
        }
    }
    host.sim_mut().set_health(ally, at_percent(49), hp_max);
    let before = host.sim().hp(ally);
    let ordered = host.sim_mut().order_attack_or_approach(attacker, ally);
    match ordered {
        Ok(true) => {}
        Ok(false) => return Err("the denial approached instead of striking an adjacent ally".into()),
        Err(r) => return Err(format!("an ally at 49 percent of its life was refused: {}", r.name())),
    }
    for _ in 0..40 {
        host.sim_mut().tick();
    }
    if host.sim().hp(ally) >= before {
        return Err("the denial struck but took no life".into());
    }
    Ok(())
}

pub fn pack_loads_clean(_image: &[u8]) -> Result<(), String> {
    let mut host = pack_run::start()?;
    pack_run::advance(&mut host, 16 * 60)?;
    let errors = host.hook_errors();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("{} hook(s) failed in the first minute: {}", errors.len(), errors.join("; ")))
    }
}

fn hold_in_place(s: &mut Sim, i: usize) -> Result<(), String> {
    const ORDER_HOLD: u8 = 4;
    let (x, y) = s.position_fixed(i);
    s.issue_order_kind(i, ORDER_HOLD, x, y).map_err(|r| format!("hold refused for unit {i}: {}", r.name()))
}

fn open_block(s: &Sim) -> Result<(i32, i32), String> {
    let g = s.grid();
    let (w, h) = (g.w as i32, g.h as i32);
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            if (-1..=1).all(|dy| (-1..=1).all(|dx| g.walkable(x + dx, y + dy))) {
                return Ok((x, y));
            }
        }
    }
    Err("the moba grid has no cell with eight walkable neighbours".into())
}

pub fn unit_crowded_cell_strikes_unit(_image: &[u8]) -> Result<(), String> {
    let mut host = pack_run::start()?;
    let (friend, enemies, tower) = {
        let mut s = host.sim_mut();
        let (x, y) = open_cell(&s)?;
        let creep = kind(&s, role("unit.creep")?)?;
        let tower_kind = kind(&s, role("unit.tower")?)?;
        let live_before = s.alive_count();
        let (px, py) = (fixed::from_int(x), fixed::from_int(y));
        let friend = s.spawn_for(0, creep, px, py).ok_or("spawn_for refused the friendly creep")?;
        for _ in 0..7 {
            s.spawn_for(0, creep, px, py).ok_or("spawn_for refused an allied filler creep")?;
        }
        let mut enemies = Vec::with_capacity(12);
        for _ in 0..12 {
            enemies.push(s.spawn_for(1, creep, px, py).ok_or("spawn_for refused an enemy creep in the crowded cell")?);
        }
        let tower = s.spawn_for(1, tower_kind, fixed::from_int(x - 1), py).ok_or("spawn_for refused the tower")?;
        let live_after = s.alive_count();
        if live_after != live_before + 21 {
            return Err(format!("alive_count {live_after} want {} after 21 spawns", live_before + 21));
        }
        (friend, enemies, tower)
    };
    let tower_hp = host.sim().hp(tower);
    let enemy_hp: Vec<i32> = enemies.iter().map(|&e| host.sim().hp(e)).collect();
    pack_run::advance(&mut host, 12)?;
    let s = host.sim();
    if !s.alive(friend) {
        return Err("friendly creep died in the first twelve ticks of the crowded cell".into());
    }
    if s.hp(tower) != tower_hp {
        return Err(format!("tower hp {tower_hp} -> {} in twelve ticks: the crowded cell hid the enemy creeps", s.hp(tower)));
    }
    let struck = enemies.iter().zip(&enemy_hp).filter(|(&e, &h0)| s.hp(e) < h0).count();
    if struck == 0 {
        return Err("no enemy creep among the twelve lost hp in twelve ticks with the tower untouched".into());
    }
    println!("creep crowded cell ok enemies_struck={struck} tower_hp={tower_hp}");
    Ok(())
}

pub fn scenario_idle_settle(_image: &[u8]) -> Result<(), String> {
    let mut s = fresh(&scenario_image()?)?;
    let unit = (0..MAX_UNITS).find(|&i| s.alive(i) && s.player(i) == 0).ok_or("moba has no side 0 unit")?;
    let mut xs = Vec::with_capacity(200);
    for _ in 0..200 {
        s.tick();
        xs.push(s.position_fixed(unit).0);
    }
    let milli: Vec<i64> = xs.iter().map(|&x| i64::from(x) * 1000 >> 16).collect();
    let max_step = xs.windows(2).map(|w| (i64::from(w[1]) - i64::from(w[0])).abs()).max().unwrap_or(0);
    println!("moba idle max_step_milli={} x milli-tiles: {milli:?}", max_step * 1000 >> 16);
    let step = i64::from(fixed::from_milli(20));
    let mut flips = 0usize;
    for w in xs.windows(3) {
        let (a, b, c) = (i64::from(w[0]), i64::from(w[1]), i64::from(w[2]));
        let (d1, d2) = (b - a, c - b);
        if d1.abs() >= step && d2.abs() >= step && (d1 > 0) != (d2 > 0) && (c - a).abs() < step {
            flips += 1;
        }
    }
    if flips > 4 {
        return Err(format!("unit {unit} alternates between two points on {flips} ticks of 200"));
    }
    Ok(())
}

pub fn scenario_sync(_image: &[u8]) -> Result<(), String> {
    let image = scenario_image()?;
    let mut a = fresh(&image)?;
    let mut b = fresh(&image)?;
    let ticks = 300;
    for t in 0..ticks {
        if t == 0 || t == 150 {
            let x = fixed::from_int(10 + t as i32 / 10);
            let y = fixed::from_int(12);
            a.order_all(0, x, y);
            b.order_all(0, x, y);
        }
        a.tick();
        b.tick();
    }
    let mut diff = 0usize;
    for i in 0..MAX_UNITS {
        if a.alive(i) != b.alive(i) || a.position_fixed(i) != b.position_fixed(i) {
            diff += 1;
        }
    }
    if diff != 0 || a.state_hash() != b.state_hash() {
        return Err(format!("peers disagree after {ticks} ticks: {diff} units differ"));
    }
    println!("moba sync ok ticks={ticks} diff={diff}");
    Ok(())
}

pub fn scenario_objective(_image: &[u8]) -> Result<(), String> {
    let mut host = pack_run::start()?;
    for side in 0..2 {
        let s = host.sim();
        if !s.catalog().starts.iter().any(|st| st.player == side) {
            return Err(format!("side {side} has no seat start for a picked hero"));
        }
        for name in [role("unit.base")?, role("unit.tower")?] {
            if !(0..MAX_UNITS).any(|i| s.alive(i) && s.player(i) == side && s.kind_name(i) == Some(name)) {
                return Err(format!("side {side} has no {name} at match start"));
            }
        }
    }
    let r_unit_tower = role("unit.tower")?;
    let target = (0..MAX_UNITS)
        .find(|&i| {
            let s = host.sim();
            s.alive(i) && s.player(i) == 1 && s.kind_name(i) == Some(r_unit_tower)
        })
        .ok_or("moba has no side 1 tower")?;
    let (tx, ty) = host.sim().position_fixed(target);
    let k = kind(&host.sim(), role("unit.creep")?)?;
    let creep = host.sim_mut().spawn_for(0, k, tx - fixed::from_int(4), ty).ok_or("spawn_for refused a creep beside the side 1 tower")?;
    let before = fixed::to_int(host.sim().hp(target));
    host.sim_mut().order(creep, tx, ty);
    pack_run::advance(&mut host, 300)?;
    let after = fixed::to_int(host.sim().hp(target));
    if after >= before {
        return Err(format!("side 0 creep did not hurt the side 1 tower: hp {before} -> {after}"));
    }
    println!("moba objective ok hp={before}->{after}");
    Ok(())
}

const RESPAWN_TERMS_PROBE: &str = r#"
local terms = require("respawn_terms")
local units = require("units")
local Q16 = 65536
local stage = 0

local function pack_heroes(player)
  local out = {}
  for _, id in ipairs(game.units(player)) do
    if units.is_hero(id) then out[#out + 1] = id end
  end
  table.sort(out)
  return out
end

return {
  on_tick = function(state)
    stage = stage + 1
    if stage ~= 4 then return end
    local mine, theirs = pack_heroes(0), pack_heroes(1)
    local holder, ally, enemy = mine[1], mine[2], theirs[1]
    state.heroes = (#mine == 2 and #theirs == 1) and 1 or 0
    if state.heroes ~= 1 then return end
    state.start_charges = terms.charges(holder)
    local cuts = {}
    for i = 1, 6 do
      game.set_hp(ally, 10 * Q16)
      cuts[i] = terms.cut(holder)
      state["cut" .. i] = cuts[i]
      state["charges_after" .. i] = terms.charges(holder)
      state["ally_hp" .. i] = unit_hp(ally)
    end
    game.set_hp(holder, 10 * Q16)
    state.nonholder_cut = terms.cut(ally)
    state.holder_hp_after_nonholder = unit_hp(holder)
    local enemy_before = terms.charges(enemy)
    terms.died(ally, nil)
    state.enemy_gain = terms.charges(enemy) - enemy_before
    state.ally_side_gain = terms.charges(holder)
    state.hp_max_seen = 1
  end,
}
"#;

pub fn bloodstone_terms(_image: &[u8]) -> Result<(), String> {
    let mut host = pack_run::start_with(&[("gate_respawn_probe.lua", RESPAWN_TERMS_PROBE)])?;
    let hero_kind = kind(&host.sim(), role("unit.hero")?)?;
    let rune = (0..MAX_UNITS)
        .find(|&u| {
            let s = host.sim();
            s.alive(u) && s.player(u) == 0xFF && s.hp(u) == fixed::from_int(1)
        })
        .ok_or("moba image places no bounty rune")?;
    let (bx, by) = host.sim().position_fixed(rune);
    let spots = [(bx + fixed::from_int(1), by), (bx + fixed::from_int(2), by), (bx + fixed::from_int(3), by)];
    let holder = host.sim_mut().spawn_attributed(0, hero_kind, spots[0].0, spots[0].1).ok_or("no cell for the holder")?;
    let ally = host.sim_mut().spawn_attributed(0, hero_kind, spots[1].0, spots[1].1).ok_or("no cell for the ally")?;
    let enemy = host.sim_mut().spawn_attributed(1, hero_kind, spots[2].0, spots[2].1).ok_or("no cell for the enemy")?;
    let stone = host.sim().catalog().item_index("bloodstone").ok_or("catalog has no bloodstone")?;
    for hero in [holder, enemy] {
        if !host.sim_mut().grant_item(hero, stone, 1) {
            return Err("grant_item refused the bloodstone".into());
        }
    }
    pack_run::advance(&mut host, 6)?;
    let int = |key: &str| host.state_int(key).ok_or_else(|| format!("the probe never set state.{key}"));
    if int("heroes")? != 1 {
        return Err("the probe did not see two player 0 heroes and one player 1 hero".into());
    }
    let start = scenario_rule("bloodstone_start_charges")? as i64;
    let per_charge = scenario_rule("bloodstone_cut_s_per_charge")? as i64;
    let decay = scenario_rule("bloodstone_decay_pct")? as i64;
    let (heal, heal_per) = (scenario_rule("bloodstone_heal")? as i64, scenario_rule("bloodstone_heal_per_charge")? as i64);
    let q16 = i64::from(fixed::from_int(1));
    let hp_max = i64::from(host.sim().hp_max(ally));
    if int("start_charges")? != start {
        return Err(format!("a fresh Bloodstone holds {} charges, want {start}", int("start_charges")?));
    }
    let mut charges = start;
    for i in 1..=6 {
        let cut = int(&format!("cut{i}"))?;
        if cut != charges * per_charge {
            return Err(format!("cut {i} took {cut} s off the respawn, want {} s ({charges} charges x {per_charge} s)", charges * per_charge));
        }
        let kept = if charges > 1 { charges * decay / 100 } else { 0 };
        if int(&format!("charges_after{i}"))? != kept {
            return Err(format!("after cut {i} the holder keeps {} charges, want {kept} ({decay}% of {charges}, one charge goes to none)", int(&format!("charges_after{i}"))?));
        }
        let want_hp = (10 * q16 + (heal + heal_per * charges) * q16).min(hp_max);
        let got_hp = int(&format!("ally_hp{i}"))?;
        if got_hp != want_hp {
            return Err(format!("cut {i} left the ally at {got_hp}, want {want_hp}: 10 hp + {heal} + {heal_per} x {charges} charges, capped at max hp {hp_max}"));
        }
        charges = kept;
    }
    if int("nonholder_cut")? != 0 || int("holder_hp_after_nonholder")? != 10 * q16 {
        return Err("a hero without a Bloodstone cut the respawn or healed an ally".into());
    }
    if int("enemy_gain")? != 1 {
        return Err(format!("an enemy holder in range gained {} charges from a hero death, want 1", int("enemy_gain")?));
    }
    if int("ally_side_gain")? != 0 {
        return Err("a holder on the dying hero's side gained a charge".into());
    }
    println!("bloodstone terms ok start={start} hp_max={hp_max}");
    Ok(())
}

const FOUNTAIN_USE_PROBE: &str = r#"
local protection = require("protection")
local units = require("units")
local stage = 0
return {
  on_tick = function(state)
    stage = stage + 1
    if stage ~= 3 then return end
    local hero, fountain
    for _, id in ipairs(game.units(0)) do
      if units.is_hero(id) then hero = id end
      if game.unit_kind(id) == "fountain" then fountain = id end
    end
    if hero == nil or fountain == nil then return end
    local slot
    for _, row in ipairs(game.unit_items(hero)) do
      if row.name == "bottle_empty" then slot = row.slot end
    end
    if slot == nil then return end
    state.armed = 1
    state.in_fountain_before = protection.in_fountain(hero) and 1 or 0
    game.cmd_use_item_on(hero, slot, "unit", fountain)
  end,
}
"#;

pub fn bottle_refill_on_fountain(_image: &[u8]) -> Result<(), String> {
    let mut host = pack_run::start_with(&[("gate_fountain_probe.lua", FOUNTAIN_USE_PROBE)])?;
    let fountain = (0..MAX_UNITS)
        .find(|&u| {
            let s = host.sim();
            s.alive(u) && s.player(u) == 0 && s.kind_name(u) == Some("fountain")
        })
        .ok_or("the moba image places no fountain for player 0")?;
    let (fx, fy) = host.sim().position_fixed(fountain);
    let hero_kind = kind(&host.sim(), role("unit.hero")?)?;
    let hero = (1..8)
        .find_map(|dx| host.sim_mut().spawn_attributed(0, hero_kind, fx + fixed::from_int(dx), fy))
        .ok_or("no open cell beside the fountain")?;
    let (full, empty) = {
        let s = host.sim();
        let c = s.catalog();
        (c.item_index("bottle"), c.item_index("bottle_empty"))
    };
    let (full, empty) = (full.ok_or("catalog has no bottle")?, empty.ok_or("catalog has no bottle_empty")?);
    if !host.sim_mut().grant_item(hero, empty, 1) {
        return Err("grant_item refused the empty bottle".into());
    }
    pack_run::advance(&mut host, 2)?;
    if host.state_int("armed") == Some(1) {
        return Err("the probe armed before its stage".into());
    }
    pack_run::advance(&mut host, 5)?;
    if host.state_int("armed") != Some(1) {
        return Err("the probe found no hero with an empty bottle beside a fountain".into());
    }
    if host.state_int("in_fountain_before") != Some(0) {
        return Err("the hero already counted as standing in the fountain before it used the bottle, so the use proves nothing".into());
    }
    if held_of(&host, hero, empty) != 0 || held_of(&host, hero, full) != 3 {
        return Err(format!(
            "after using the empty bottle on the fountain the hero holds {} empty and {} full charges, want 0 and 3",
            held_of(&host, hero, empty),
            held_of(&host, hero, full)
        ));
    }
    println!("bottle refill on fountain ok hero={hero} fountain={fountain}");
    Ok(())
}

fn host_with_modes(codes: &[&str]) -> Result<ModHost, String> {
    let mut host = pack_run::start()?;
    host.dispatch_action("ui", &serde_json::json!({ "call": "claim", "args": { "slot": 0 } })).map_err(|e| format!("claim: {e}"))?;
    for code in codes {
        host.dispatch_action("ui", &serde_json::json!({ "call": "mode", "args": { "code": code } })).map_err(|e| format!("mode {code}: {e}"))?;
    }
    pack_run::advance(&mut host, 2)?;
    Ok(host)
}

fn spawn_hero_with_items(host: &mut ModHost, item_count: usize) -> Result<usize, String> {
    let hero_kind = kind(&host.sim(), role("unit.hero")?)?;
    let (x, y) = host.sim().position_fixed(0);
    let hero = host.sim_mut().spawn_attributed(0, hero_kind, x + fixed::from_int(2), y).ok_or("spawn_attributed refused")?;
    let candidates: Vec<usize> = {
        let sim = host.sim();
        let catalog = sim.catalog();
        (0..catalog.items.len()).filter(|&i| !matches!(catalog.name(catalog.items[i].name), "rapier" | "gem")).collect()
    };
    let mut held = 0;
    for item in candidates {
        if held == item_count {
            break;
        }
        if host.sim_mut().grant_item(hero, item, 1) {
            held += 1;
        }
    }
    if held != item_count {
        return Err(format!("only {held} of {item_count} items could be granted"));
    }
    Ok(hero)
}

fn ground_items_beside(host: &ModHost, hero: usize) -> usize {
    let s = host.sim();
    let (hx, hy) = s.position_fixed(hero);
    let reach = i64::from(fixed::from_int(1)) * i64::from(fixed::from_int(1));
    s.ground_items().iter().filter(|g| (i64::from(g.x) - i64::from(hx)).pow(2) + (i64::from(g.y) - i64::from(hy)).pow(2) <= reach).count()
}

fn kill_and_settle(host: &mut ModHost, hero: usize) -> Result<(), String> {
    pack_run::advance(host, 2)?;
    for _ in 0..8 {
        if !host.sim().alive(hero) {
            break;
        }
        host.sim_mut().hurt(hero, fixed::from_int(10000));
    }
    if host.sim().alive(hero) {
        return Err(format!("hero {hero} survived a lethal hit"));
    }
    pack_run::advance(host, 4)
}

pub fn mode_easy_mini_item_drop(_image: &[u8]) -> Result<(), String> {
    let plain = host_with_modes(&[])?;
    if plain.sim().xp_rate(0) != 100 {
        return Err(format!("xp rate {} without Easy Mode, want 100", plain.sim().xp_rate(0)));
    }
    let mut host = host_with_modes(&["em", "mi"])?;
    for player in [0, 1] {
        if host.sim().xp_rate(player) != 150 {
            return Err(format!("Easy Mode left player {player} at xp rate {}, want 150", host.sim().xp_rate(player)));
        }
    }
    let hero = spawn_hero_with_items(&mut host, 0)?;
    pack_run::advance(&mut host, 3)?;
    if host.sim().unit_scale(hero) != 50 {
        return Err(format!("Mini Heroes left the hero at scale {}, want 50", host.sim().unit_scale(hero)));
    }
    let mut off = host_with_modes(&[])?;
    let off_hero = spawn_hero_with_items(&mut off, 6)?;
    kill_and_settle(&mut off, off_hero)?;
    let mut on = host_with_modes(&["id"])?;
    let on_hero = spawn_hero_with_items(&mut on, 6)?;
    kill_and_settle(&mut on, on_hero)?;
    let (dropped_off, dropped_on) = (ground_items_beside(&off, off_hero), ground_items_beside(&on, on_hero));
    if dropped_on != dropped_off + 1 {
        return Err(format!("Item Drop put {dropped_on} items beside the dead hero against {dropped_off} without it, want one more"));
    }
    Ok(())
}
