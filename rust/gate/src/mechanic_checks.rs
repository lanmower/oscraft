//! Gate checks for the engine's general mechanics, each run on the default data image. Each
//! asserts one postcondition through the public sim or mod-host API and names the mechanic,
//! never a genre.

use crate::checks::{fresh, kind, scenario_rule, run_ticks, spawn, MAX_UNITS, RUN_TICKS};
use crate::checks::{role};
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

/// Xp is a per-unit counter: gains accumulate and saturate at the top of u32. What the xp means (levels on the
/// curve) is the mod's rule, so only the storage is checked here.
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
    if s.stats.casts != 1 {
        return Err(format!("idle trigger left stats.casts {} want 1", s.stats.casts));
    }
    run_ticks(&mut s, 200);
    if s.stats.casts < 2 {
        return Err(format!("no tick-driven behaviour trigger fired over 200 ticks (casts {})", s.stats.casts));
    }
    Ok(())
}

/// A structure with hit points loses them when an enemy with an attack is in weapon range. The
/// beacon and skirmisher come from the duel-routes scenario, not the default data.
pub fn structure_damage(_image: &[u8]) -> Result<(), String> {
    let scenario = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("scenarios").join("duel-routes");
    let image = osdt_data::generate(&scenario).map_err(|e| format!("scenario image: {e:#}"))?;
    let mut s = fresh(&image)?;
    let beacon = spawn(&mut s, 1, "beacon", 20, 4)?;
    spawn(&mut s, 0, "skirmisher", 18, 4)?;
    let before = fixed::to_int(s.hp(beacon));
    run_ticks(&mut s, 100);
    let after = fixed::to_int(s.hp(beacon));
    if after >= before {
        return Err(format!("beacon hp {before} -> {after} with an enemy skirmisher in weapon range"));
    }
    println!("structure damage ok hp_before={before} hp_after={after}");
    Ok(())
}

/// A live neutral camp unit of the moba scenario, with the sim it lives in.
fn scenario_group_unit() -> Result<(Sim, usize), String> {
    let scenario = crate::checks::pack_dir();
    let image = osdt_data::generate(&scenario).map_err(|e| format!("scenario image: {e:#}"))?;
    let mut s = fresh(&image)?;
    run_ticks(&mut s, (scenario_rule("camp_first_spawn_s")? as u32) * sim::TICKS_PER_SECOND as u32 + 1);
    let u = (0..MAX_UNITS)
        .find(|&u| s.alive(u) && s.player(u) == 0xFF && s.hp(u) > fixed::from_int(1) && s.hp(u) < fixed::from_int(5000))
        .ok_or("moba image places no live neutral unit")?;
    Ok((s, u))
}

/// A dead camp unit respawns at its home once its timer runs out, and not before.
pub fn spawn_group_respawn(_image: &[u8]) -> Result<(), String> {
    let (mut s, u) = scenario_group_unit()?;
    let home = s.position_fixed(u);
    let clear_reach = fixed::from_milli(scenario_rule("camp_clear_milli")?) as i64;
    let camp_mates: Vec<usize> = (0..MAX_UNITS)
        .filter(|&k| k != u && s.alive(k) && s.player(k) == 0xFF && {
            let (x, y) = s.position_fixed(k);
            let (dx, dy) = (i64::from(x) - i64::from(home.0), i64::from(y) - i64::from(home.1));
            dx * dx + dy * dy <= clear_reach * clear_reach
        })
        .collect();
    for &k in &camp_mates {
        s.hurt(k, fixed::from_int(10000));
    }
    s.hurt(u, fixed::from_int(10000));
    if s.alive(u) {
        return Err(format!("camp unit {u} survived a lethal hit"));
    }
    let wait = scenario_rule("camp_respawn_s")? * sim::TICKS_PER_SECOND;
    for _ in 0..wait - 4 {
        s.tick();
    }
    if s.alive(u) {
        return Err(format!("camp unit {u} respawned before its {} s timer", (scenario_rule("camp_respawn_s")? as u32)));
    }
    for _ in 0..8 {
        s.tick();
    }
    if !s.alive(u) {
        return Err(format!("camp unit {u} did not respawn after its timer"));
    }
    if s.position_fixed(u) != home {
        return Err(format!("camp unit {u} respawned at {:?} not home {home:?}", s.position_fixed(u)));
    }
    println!("camp respawn ok unit={u} after={} ticks", wait);
    Ok(())
}

/// A hero within range of a bounty rune takes its gold, the rune dies, and it respawns on its timer.
pub fn spawn_group_pickup(_image: &[u8]) -> Result<(), String> {
    let (mut s, _) = scenario_group_unit()?;
    let rune = (0..MAX_UNITS)
        .find(|&u| s.alive(u) && s.player(u) == 0xFF && s.hp(u) == fixed::from_int(1))
        .ok_or("moba image places no bounty rune")?;
    let (rx, ry) = s.position_fixed(rune);
    let hero_kind = kind(&s, role("unit.hero")?)?;
    let hero = s
        .spawn_attributed(0, hero_kind, rx + fixed::from_int(1), ry)
        .ok_or("spawn_attributed refused beside the rune")?;
    let gold = s.unit_gold(hero);
    s.tick();
    if s.alive(rune) {
        return Err(format!("rune {rune} survived a hero in pickup range"));
    }
    let paid = s.unit_gold(hero) - gold;
    // One tick can land on a second boundary, which adds one passive gold per second of hero income.
    let passive_slack = 1;
    if paid < scenario_rule("rune_gold")? || paid > scenario_rule("rune_gold")? + passive_slack {
        return Err(format!("rune paid {paid} gold, want {} plus at most one passive tick", scenario_rule("rune_gold")?));
    }
    s.order(hero, rx - fixed::from_int(12), ry);
    for _ in 0..scenario_rule("rune_respawn_s")? * sim::TICKS_PER_SECOND + 8 {
        s.tick();
    }
    if !s.alive(rune) {
        return Err(format!("rune {rune} did not respawn after its timer"));
    }
    println!("camp rune pickup ok rune={rune} hero={hero}");
    Ok(())
}

/// A hero that kills Roshan is granted one inventory item (the Aegis reward).
pub fn spawn_group_boss_aegis(_image: &[u8]) -> Result<(), String> {
    let (mut s, _) = scenario_group_unit()?;
    let boss = (0..MAX_UNITS)
        .filter(|&u| s.alive(u) && s.player(u) == 0xFF)
        .max_by_key(|&u| s.hp(u))
        .ok_or("moba image places no neutral unit")?;
    let (bx, by) = s.position_fixed(boss);
    let hero = spawn(&mut s, 0, role("unit.hero")?, fixed::to_int(bx) + 1, fixed::to_int(by))?;
    let hp = s.hp(boss);
    s.hurt(boss, hp - fixed::from_int(1));
    let held = |s: &Sim| (0..s.inventory_slots()).filter(|&k| s.inventory_slot(hero, k).0 != 0).count();
    let before = held(&s);
    s.order_attack(hero, boss).map_err(|r| format!("hero cannot attack roshan: {}", r.name()))?;
    for _ in 0..400 {
        s.tick();
        if !s.alive(boss) {
            break;
        }
    }
    if s.alive(boss) {
        return Err(format!("hero did not kill unit {boss}"));
    }
    if held(&s) != before + 1 {
        return Err(format!("killer holds {} item slots, want {}", held(&s), before + 1));
    }
    println!("camp roshan aegis ok boss={boss} hero={hero}");
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

/// A creep ordered from its own base toward the rival base closes distance along the lane.
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

/// A creep ordered from the side 0 base to the second waypoint of player 0's top lane route walks the corridor
/// between them (rows 190 to 215) and stays on open cells, so passing this check means the route followed the lane.
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

/// Creep stats from the moba unit table (scenarios/moba/units.osd): speed and weapon reach in milli-tiles,
/// weapon period in ticks.
const CREEP_SPEED_MILLI: i64 = 317;
const CREEP_REACH_MILLI: i64 = 1562;
const CREEP_WEAPON_PERIOD: u32 = 16;

/// A creep on an attack-move order stops once an enemy creep is in weapon reach, strikes it, and resumes
/// its route after the target dies. The approach budget is the time to close the gap at creep speed plus a
/// margin of 16 ticks; the stop tolerance is the weapon reach plus one tile of hull margin; the hold window
/// is three weapon periods, so at least three strikes are attempted.
pub fn unit_stops_to_attack(_image: &[u8]) -> Result<(), String> {
    let mut s = fresh(&scenario_image()?)?;
    let r_unit_base = role("unit.base")?;
    let own = (0..MAX_UNITS).find(|&i| s.alive(i) && s.player(i) == 0 && s.kind_name(i) == Some(r_unit_base)).ok_or("moba has no side 0 base")?;
    let r_unit_base = role("unit.base")?;
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
    let (ex0, ey0) = s.position_fixed(enemy);
    s.order(enemy, ex0, ey0);
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
    s.issue_order_kind(friend, sim::ORDER_ATTACK_MOVE, tx, ty).map_err(|e| format!("attack-move refused: {}", e.name()))?;
    let enemy_hp0 = s.hp(enemy);
    let mut stop_tick = None;
    let mut prev = s.position_fixed(friend);
    for t in 1..=budget {
        s.tick();
        if !s.alive(friend) {
            return Err(format!("creep died on the approach at tick {t}"));
        }
        let now = s.position_fixed(friend);
        if now == prev && gap(&s) <= reach + 1.0 {
            stop_tick = Some(t);
            break;
        }
        prev = now;
    }
    let t_stop = stop_tick.ok_or(format!("creep did not reach enemy reach within {budget} ticks from {d0:.1} tiles away"))?;
    let stop_pos = s.position_fixed(friend);
    let hp_at_stop = s.hp(enemy);
    for t in 0..3 * CREEP_WEAPON_PERIOD {
        if !s.alive(enemy) {
            break;
        }
        s.tick();
        if s.alive(enemy) && s.position_fixed(friend) != stop_pos {
            return Err(format!("engaged creep moved {} ticks after stopping at tick {t_stop} while its target lived", t + 1));
        }
    }
    if s.hp(enemy) >= hp_at_stop {
        return Err(format!("engaged creep struck nothing: enemy hp {hp_at_stop} unchanged while in reach"));
    }
    s.hurt(enemy, fixed::from_int(10000));
    if s.alive(enemy) {
        return Err("enemy creep survived lethal damage".into());
    }
    let resumed = (0..budget).any(|_| {
        s.tick();
        s.position_fixed(friend) != stop_pos
    });
    if !resumed {
        return Err(format!("creep stayed put {budget} ticks after its target died"));
    }
    println!("creep engagement ok stop_tick={t_stop} enemy_hp {enemy_hp0} -> {}", s.hp(enemy));
    let mut walker = fresh(&scenario_image()?)?;
    let free = walker.spawn_for(0, k, ox, oy).ok_or("spawn_for refused a walking creep at the side 0 base")?;
    walker.issue_order_kind(free, sim::ORDER_ATTACK_MOVE, tx, ty).map_err(|e| format!("attack-move refused: {}", e.name()))?;
    let start = walker.position_fixed(free);
    run_ticks(&mut walker, 30);
    if walker.position_fixed(free) == start {
        return Err("a creep with no enemy in reach did not walk its attack-move route".into());
    }
    Ok(())
}

/// The moba sim with its hero rules loaded. `aggro_priority` is off until those rules are set, so without them a
/// creep takes the nearest target, and a tower's body can be nearer than a creep.
fn scenario_rules_sim() -> Result<Sim, String> {
    let mut s = fresh(&scenario_image()?)?;
    let scenario = crate::checks::pack_dir();
    let rules = std::fs::read_to_string(scenario.join(crate::checks::RULES_FILE)).map_err(|e| e.to_string())?;
    if !s.set_attr_rules_text(&rules) {
        return Err("the pack rules file has a bad line".into());
    }
    Ok(s)
}

/// First cell (row-major) whose four neighbours are walkable too, so a friendly creep, an enemy creep and a tower
/// can each stand one cell from the next without a wall between them.
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

/// Creep targeting, one friendly creep with an enemy creep and a tower each one tile away. A tile is one cell (the
/// loader refuses other cell sizes), and creep weapon reach is CREEP_REACH_MILLI (1.562 tiles), so both are in
/// reach. Target class ranks a plain creep (0) ahead of a structure (2), so the friend strikes the enemy creep
/// and the tower's hp must not move while the enemy lives. Creep weapon period is CREEP_WEAPON_PERIOD ticks, so
/// three periods (48 ticks) hold three strikes on that one creep, and the first strike lands on tick 1.
pub fn unit_strikes_unit_before_structure(_image: &[u8]) -> Result<(), String> {
    let mut s = scenario_rules_sim()?;
    let (x, y) = open_cell(&s)?;
    let creep = kind(&s, role("unit.creep")?)?;
    let tower_kind = kind(&s, role("unit.tower")?)?;
    let friend = s.spawn_for(0, creep, fixed::from_int(x), fixed::from_int(y)).ok_or("spawn_for refused the friendly creep")?;
    let enemy = s.spawn_for(1, creep, fixed::from_int(x + 1), fixed::from_int(y)).ok_or("spawn_for refused the enemy creep")?;
    let tower = s.spawn_for(1, tower_kind, fixed::from_int(x - 1), fixed::from_int(y)).ok_or("spawn_for refused the tower")?;
    let tower_hp = s.hp(tower);
    let mut enemy_hp = s.hp(enemy);
    let mut strikes = 0;
    for t in 1..=3 * CREEP_WEAPON_PERIOD {
        s.tick();
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

/// A reused slot must not inherit its previous occupant's sticky target. Friendly creep F (with two more friends south of it,
/// so three of them kill A faster) strikes enemy creep A east of F until A dies. A tower stands one
/// tile west, in reach. F's strike record keeps A's slot through its weapon cooldown after the killing blow, so the slot
/// is reused at once: spawn_at puts a new enemy creep B on it, one tile diagonal to F's north-east, and a second enemy
/// creep C spawns one tile north of F. Both are plain creeps (one class), so F's next strike must land on the nearer C
/// (squared distance 1, against B's 2). B and C share a team, so F is the only thing that can lower their hp.
pub fn reused_slot_drops_stale_target(_image: &[u8]) -> Result<(), String> {
    let mut s = scenario_rules_sim()?;
    let (x, y) = open_block(&s)?;
    let creep = kind(&s, role("unit.creep")?)?;
    let tower_kind = kind(&s, role("unit.tower")?)?;
    let friend = s.spawn_for(0, creep, fixed::from_int(x), fixed::from_int(y)).ok_or("spawn_for refused the friendly creep")?;
    s.spawn_for(0, creep, fixed::from_int(x), fixed::from_int(y - 1)).ok_or("spawn_for refused the second friendly creep")?;
    s.spawn_for(0, creep, fixed::from_int(x + 1), fixed::from_int(y - 1)).ok_or("spawn_for refused the third friendly creep")?;
    let a = s.spawn_for(1, creep, fixed::from_int(x + 1), fixed::from_int(y)).ok_or("spawn_for refused enemy creep A")?;
    for i in [friend, friend + 1, friend + 2, a] {
        hold_in_place(&mut s, i)?;
    }
    let mut t = 0;
    while s.alive(a) {
        t += 1;
        if t > RUN_TICKS {
            return Err(format!("enemy creep A in slot {a} never died in {RUN_TICKS} ticks"));
        }
        s.tick();
        if !s.alive(friend) {
            return Err(format!("friendly creep died at tick {t} before enemy creep A died"));
        }
    }
    if !s.spawn_at(a, creep, 1, fixed::from_int(x + 1), fixed::from_int(y + 1)) {
        return Err(format!("spawn_at refused the dead slot {a} for enemy creep B"));
    }
    if !s.alive(a) || s.player(a) != 1 {
        return Err(format!("reused slot {a} is not a live enemy after spawn_at"));
    }
    let c = s.spawn_for(1, creep, fixed::from_int(x), fixed::from_int(y + 1)).ok_or("spawn_for refused enemy creep C")?;
    let tower = s.spawn_for(1, tower_kind, fixed::from_int(x - 1), fixed::from_int(y)).ok_or("spawn_for refused the tower")?;
    let tower_hp = s.hp(tower);
    hold_in_place(&mut s, a)?;
    hold_in_place(&mut s, c)?;
    let (b_hp, c_hp) = (s.hp(a), s.hp(c));
    for t in 1..=3 * CREEP_WEAPON_PERIOD {
        s.tick();
        if !s.alive(friend) {
            return Err(format!("friendly creep died at tick {t} before its first strike on B or C"));
        }
        if s.hp(tower) != tower_hp {
            return Err(format!("tower hp {} -> {} at tick {t}; the tower is out of class and must not be struck", tower_hp, s.hp(tower)));
        }
        if s.hp(a) < b_hp {
            return Err(format!(
                "friendly creep struck enemy creep B in reused slot {a} at tick {t} instead of nearer enemy creep C in slot {c}: stale sticky target"
            ));
        }
        if s.hp(c) < c_hp {
            return Ok(());
        }
    }
    Err(format!("friendly creep struck neither B (slot {a}) nor C (slot {c}) in {} ticks", 3 * CREEP_WEAPON_PERIOD))
}

/// Holds unit `i` where it stands (ORDER_HOLD at its own point), so no creep walks out of the reach the check built.
fn hold_in_place(s: &mut Sim, i: usize) -> Result<(), String> {
    const ORDER_HOLD: u8 = 4;
    let (x, y) = s.position_fixed(i);
    s.issue_order_kind(i, ORDER_HOLD, x, y).map_err(|r| format!("hold refused for unit {i}: {}", r.name()))
}

/// First cell (row-major) whose eight neighbours and itself are all walkable, so the friends, the enemy and the new
/// creeps of the stale-target check each stand on their own open cell.
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

/// A crowded bucket cell keeps every unit. The friendly creep shares a one-tile cell with seven allied creeps,
/// which fill the eight inline bucket slots (BUCKET_CAP) after it, and twelve enemy creeps overflow past them. All
/// twelve are inside creep reach of the friend (1.562 tiles, against a cell diagonal of 1.414), and a tower one
/// tile west is in reach too. A dropped unit would leave the friend with only the tower, so after one tick the
/// tower must be untouched and one of the twelve enemy creeps must have lost hp. Live units after the spawns:
/// 1 friend + 7 allies + 12 enemies + 1 tower = 21 more than before, so none is lost to the spawn either.
pub fn unit_crowded_cell_strikes_unit(_image: &[u8]) -> Result<(), String> {
    let mut s = scenario_rules_sim()?;
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
    let tower_hp = s.hp(tower);
    let enemy_hp: Vec<i32> = enemies.iter().map(|&e| s.hp(e)).collect();
    s.tick();
    if !s.alive(friend) {
        return Err("friendly creep died on tick 1 of the crowded cell".into());
    }
    if s.hp(tower) != tower_hp {
        return Err(format!("tower hp {tower_hp} -> {} on tick 1: the crowded cell hid the enemy creeps", s.hp(tower)));
    }
    let struck = enemies.iter().zip(&enemy_hp).filter(|(&e, &h0)| s.hp(e) < h0).count();
    if struck == 0 {
        return Err("no enemy creep among the twelve lost hp on tick 1 with the tower untouched".into());
    }
    println!("creep crowded cell ok enemies_struck={struck} tower_hp={tower_hp}");
    Ok(())
}

/// A moba unit standing with no orders and no neighbours to move away from holds one point: its x does not
/// flip between two lanes on successive ticks. Prints the x sequence in milli-tiles for the witness.
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

/// Two sims loaded from the same moba image, given the same orders at the same ticks, end in the same state.
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

/// Both sides start with a hero, a base and a tower. A side 0 creep set beside the side 1 tower and ordered to it loses that tower hit points.
pub fn scenario_objective(_image: &[u8]) -> Result<(), String> {
    let mut s = fresh(&scenario_image()?)?;
    for side in 0..2 {
        for name in [role("unit.hero")?, role("unit.base")?, role("unit.tower")?] {
            if !(0..MAX_UNITS).any(|i| s.alive(i) && s.player(i) == side && s.kind_name(i) == Some(name)) {
                return Err(format!("side {side} has no {name} at match start"));
            }
        }
    }
    let r_unit_tower = role("unit.tower")?;
    let target = (0..MAX_UNITS).find(|&i| s.alive(i) && s.player(i) == 1 && s.kind_name(i) == Some(r_unit_tower)).ok_or("moba has no side 1 tower")?;
    let (tx, ty) = s.position_fixed(target);
    let k = kind(&s, role("unit.creep")?)?;
    let creep = s.spawn_for(0, k, tx - fixed::from_int(4), ty).ok_or("spawn_for refused a creep beside the side 1 tower")?;
    let before = fixed::to_int(s.hp(target));
    s.order(creep, tx, ty);
    run_ticks(&mut s, 300);
    let after = fixed::to_int(s.hp(target));
    if after >= before {
        return Err(format!("side 0 creep did not hurt the side 1 tower: hp {before} -> {after}"));
    }
    println!("moba objective ok hp={before}->{after}");
    Ok(())
}
