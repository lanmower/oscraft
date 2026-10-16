//! Sim-side gate checks. Each runs against the sim's public API on the map that osdt_data
//! generates from rust/data/*.osd, and returns Err with the reason it fails.

use crate::checks::{fresh, kind, run_ticks, spawn};
use sim::{fixed, Refusal};

// ORDER_* in rust/crates/sim/src/lib.rs. They are not public, so the values are repeated here.
const ORDER_MOVE: u8 = 0;
const ORDER_STOP: u8 = 1;
const ORDER_ATTACK_MOVE: u8 = 2;
const ORDER_PATROL: u8 = 3;
const ORDER_HOLD: u8 = 4;

fn refused(what: &str, r: Refusal) -> String {
    format!("{what} refused: {}", r.name())
}

/// The bit the data crate's FILTERS list assigns to `name`, as the sim catalog stores it.
fn filter_bit(name: &str) -> Result<u32, String> {
    osdt_data::format::FILTERS
        .iter()
        .position(|f| *f == name)
        .map(|i| 1u32 << i)
        .ok_or_else(|| format!("FILTERS has no {name}"))
}

pub fn load_map(image: &[u8]) -> Result<(), String> {
    let s = fresh(image)?;
    let g = s.grid();
    if (g.w, g.h) != (32, 24) {
        return Err(format!("grid is {}x{}, map.osd paints 32x24", g.w, g.h));
    }
    if g.walkable(5, 0) {
        return Err("row 0 reads walkable, map.osd paints it blocked".into());
    }
    if !g.walkable(20, 15) {
        return Err("open cell (20,15) reads blocked".into());
    }
    if s.count() < 4 {
        return Err(format!("{} units placed, map.osd places 4", s.count()));
    }
    Ok(())
}

pub fn walkable_cells(image: &[u8]) -> Result<(), String> {
    let s = fresh(image)?;
    let g = s.grid();
    if g.walkable(5, 0) {
        return Err("row 0 reads walkable, map.osd paints it blocked".into());
    }
    match g.nearest_walkable(5, 0, 4) {
        Some((x, y)) if g.walkable(x, y) => Ok(()),
        other => Err(format!("nearest_walkable(5,0,4) returned {other:?}")),
    }
}

pub fn spawn_units(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let before = s.count();
    let i = spawn(&mut s, 1, "skirmisher", 10, 10)?;
    if s.player(i) != 1 {
        return Err(format!("spawned unit owned by {} want 1", s.player(i)));
    }
    if s.count() != before + 1 {
        return Err(format!("count {} after spawn, want {}", s.count(), before + 1));
    }
    Ok(())
}

pub fn unit_stats_read(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let i = spawn(&mut s, 0, "skirmisher", 10, 10)?;
    let (x, y) = s.position_fixed(i);
    if (x, y) != (fixed::from_int(10), fixed::from_int(10)) {
        return Err(format!("position Q16 ({x},{y}) want (10,10) in whole units"));
    }
    if s.hp(i) != fixed::from_int(45) {
        return Err(format!("hp {} want 45", fixed::to_int(s.hp(i))));
    }
    if s.player(i) != 0 || !s.alive(i) {
        return Err("spawned unit is not alive for player 0".into());
    }
    Ok(())
}

pub fn catalog_load(image: &[u8]) -> Result<(), String> {
    let s = fresh(image)?;
    let c = s.catalog();
    let k = c.unit_index("skirmisher").ok_or("catalog has no skirmisher")?;
    let u = c.units[k];
    if u.health != 45 || u.speed != 3150 {
        return Err(format!("skirmisher health {} speed {} want 45 and 3150", u.health, u.speed));
    }
    if c.unit_index("warden").is_none() {
        return Err("catalog has no warden".into());
    }
    Ok(())
}

pub fn unit_classes(image: &[u8]) -> Result<(), String> {
    let s = fresh(image)?;
    let c = s.catalog();
    let bit = |name: &str| -> Result<u32, String> {
        osdt_data::format::FILTERS
            .iter()
            .position(|f| *f == name)
            .map(|p| 1u32 << p)
            .ok_or_else(|| format!("FILTERS has no {name}"))
    };
    let (air, worker, structure) = (bit("air")?, bit("worker")?, bit("structure")?);
    for u in &c.units {
        let name = c.name(u.name);
        if u.is_air() != (u.filter & air != 0) {
            return Err(format!("{name}: is_air disagrees with its filter mask"));
        }
        if u.is_worker() != (u.filter & worker != 0) {
            return Err(format!("{name}: is_worker disagrees with its filter mask"));
        }
        if u.is_structure() != (u.filter & structure != 0) {
            return Err(format!("{name}: is_structure disagrees with its filter mask"));
        }
    }
    for a in &c.abilities {
        if a.is_projectile() != (a.projectile_speed > 0) {
            return Err(format!(
                "ability with projectile_speed {} disagrees with is_projectile",
                a.projectile_speed
            ));
        }
    }
    Ok(())
}

pub fn cast_ability(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let a = spawn(&mut s, 0, "skirmisher", 10, 10)?;
    let t = spawn(&mut s, 1, "skirmisher", 12, 10)?;
    if !s.cast_ability(a, 0, t as i32, 0, 0) {
        return Err(format!("first cast refused: {}", s.stats.last_refusal.name()));
    }
    if s.cast_ability(a, 0, t as i32, 0, 0) {
        return Err("second cast during cooldown was accepted".into());
    }
    if s.stats.last_refusal != Refusal::Cooldown {
        return Err(format!("second cast refused as {} want cooldown", s.stats.last_refusal.name()));
    }
    Ok(())
}

pub fn ability_target_filter(image: &[u8]) -> Result<(), String> {
    let s = fresh(image)?;
    let c = s.catalog();
    let ground = filter_bit("ground")?;
    if !c.units.iter().any(|u| u.filter & ground == 0) {
        return Err("every catalog unit carries the ground filter, so no target filter can refuse one".into());
    }
    if !c.abilities.iter().any(|a| a.filter != 0) {
        return Err("no catalog ability declares a target filter".into());
    }
    Ok(())
}

pub fn behaviour_declared(image: &[u8]) -> Result<(), String> {
    let s = fresh(image)?;
    let c = s.catalog();
    if c.behaviours.is_empty() {
        return Err("catalog has no behaviours; behaviours.osd declares guard_patrol".into());
    }
    for b in &c.behaviours {
        if b.trigger_kind().is_none() {
            return Err(format!("{}: trigger code {} is not a declared trigger", c.name(b.name), b.trigger));
        }
    }
    Ok(())
}

pub fn behaviour_trigger_fires(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    spawn(&mut s, 0, "skirmisher", 10, 10)?;
    spawn(&mut s, 1, "skirmisher", 12, 10)?;
    run_ticks(&mut s, 200);
    if s.stats.casts == 0 {
        return Err("stats.casts stayed 0 over 200 ticks: no idle behaviour fired".into());
    }
    Ok(())
}

pub fn issue_move(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let a = spawn(&mut s, 0, "skirmisher", 4, 4)?;
    s.order(a, fixed::from_int(20), fixed::from_int(4));
    run_ticks(&mut s, 60);
    let (x, y) = s.position_fixed(a);
    let (x, y) = (fixed::to_int(x), fixed::to_int(y));
    if x < 19 || (y - 4).abs() > 1 {
        return Err(format!("unit stopped at ({x},{y}), want near (20,4)"));
    }
    Ok(())
}

pub fn move_units_tick(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let a = spawn(&mut s, 0, "skirmisher", 4, 4)?;
    let b = spawn(&mut s, 0, "skirmisher", 4, 6)?;
    let (before_a, before_b) = (s.position_fixed(a), s.position_fixed(b));
    s.order_all(0, fixed::from_int(20), fixed::from_int(10));
    run_ticks(&mut s, 60);
    for (i, p0) in [(a, before_a), (b, before_b)] {
        let p1 = s.position_fixed(i);
        let moved = (p1.0 - p0.0).abs() + (p1.1 - p0.1).abs();
        if moved < fixed::from_int(2) {
            return Err(format!("unit {i} moved {:.2} world units in 60 ticks", fixed::to_float(moved)));
        }
    }
    Ok(())
}

pub fn path_to_goal(image: &[u8]) -> Result<(), String> {
    // The blocked cells at x 12..14, y 8..10 sit between the unit and the goal, so the unit must
    // route around them.
    let mut s = fresh(image)?;
    let a = spawn(&mut s, 0, "skirmisher", 10, 9)?;
    s.order(a, fixed::from_int(16), fixed::from_int(9));
    run_ticks(&mut s, 300);
    let (x, y) = s.position_fixed(a);
    let dx = (x - fixed::from_int(16)).abs();
    let dy = (y - fixed::from_int(9)).abs();
    if dx > fixed::from_milli(1500) || dy > fixed::from_milli(1500) {
        return Err(format!(
            "unit at ({:.2},{:.2}) after 300 ticks, want (16,9) around the blocked cells",
            fixed::to_float(x),
            fixed::to_float(y)
        ));
    }
    Ok(())
}

pub fn order_kinds(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let a = spawn(&mut s, 0, "skirmisher", 10, 10)?;
    let (x, y) = (fixed::from_int(12), fixed::from_int(10));
    for kind in [ORDER_MOVE, ORDER_STOP, ORDER_ATTACK_MOVE, ORDER_PATROL] {
        s.issue_order_kind(a, kind, x, y).map_err(|r| refused(&format!("order kind {kind}"), r))?;
    }
    match s.issue_order_kind(a, 9, x, y) {
        Err(Refusal::InvalidOrderKind) => {}
        other => return Err(format!("order kind 9 gave {other:?}, want InvalidOrderKind")),
    }
    s.issue_order_kind(a, ORDER_HOLD, x, y).map_err(|r| refused("hold", r))?;
    let held = s.position_fixed(a);
    run_ticks(&mut s, 30);
    if s.position_fixed(a) != held {
        return Err("hold order moved the unit".into());
    }
    Ok(())
}

pub fn produce_unit(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    s.add_resources(0, 1000, 1000);
    s.set_production(true);
    let before = s.count();
    for _ in 0..1000 {
        s.tick();
        if s.stats.trained > 0 {
            break;
        }
    }
    if s.stats.trained == 0 {
        return Err(format!(
            "no unit trained in 1000 ticks (trains_refused {}, last {})",
            s.stats.trains_refused,
            s.stats.last_train_refusal.name()
        ));
    }
    if s.count() <= before {
        return Err("stats.trained moved but no unit appeared".into());
    }
    Ok(())
}

pub fn harvest_resources(image: &[u8]) -> Result<(), String> {
    let s = fresh(image)?;
    let worker = filter_bit("worker")?;
    if !s.catalog().units.iter().any(|u| u.filter & worker != 0) {
        return Err("catalog has no unit with the worker filter, so harvest_step never runs".into());
    }
    Ok(())
}

pub fn spend_resources(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    s.set_production(true);
    if s.minerals(0) != 0 {
        return Err(format!("player 0 starts with {} minerals, want 0", s.minerals(0)));
    }
    run_ticks(&mut s, 5);
    if s.stats.trains_refused == 0 {
        return Err("no train was refused for cost with zero minerals".into());
    }
    if s.stats.last_train_refusal != Refusal::CannotAfford {
        return Err(format!("last refusal {} want cannot-afford", s.stats.last_train_refusal.name()));
    }
    if s.minerals(0) != 0 {
        return Err(format!("refused trains changed minerals to {}", s.minerals(0)));
    }
    Ok(())
}

pub fn cancel_production(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    s.add_resources(0, 1000, 1000);
    s.set_production(true);
    let k = kind(&s, "skirmisher")? as usize;
    let cost = s.catalog().units[k].minerals;
    let w = spawn(&mut s, 0, "warden", 6, 5)?;
    let before = s.minerals(0);
    run_ticks(&mut s, 1);
    let started = s.minerals(0);
    if started >= before {
        return Err("no train started: minerals unchanged after one tick".into());
    }
    s.cancel_train(w).map_err(|r| refused("cancel_train", r))?;
    let after = s.minerals(0);
    if after != started + cost {
        return Err(format!("refund left minerals at {after}, want {}", started + cost));
    }
    Ok(())
}

pub fn attack_unit(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let a = spawn(&mut s, 0, "skirmisher", 10, 10)?;
    let t = spawn(&mut s, 1, "skirmisher", 12, 10)?;
    let hp0 = s.hp(t);
    s.issue_order_kind(a, ORDER_ATTACK_MOVE, fixed::from_int(12), fixed::from_int(10))
        .map_err(|r| refused("attack-move", r))?;
    run_ticks(&mut s, 120);
    if s.hp(t) >= hp0 {
        return Err(format!("target hp {} unchanged after 120 ticks", fixed::to_int(s.hp(t))));
    }
    Ok(())
}

pub fn projectile_hit(image: &[u8]) -> Result<(), String> {
    // The target stands 8 units away, outside the 5-unit melee range, and the attacker holds,
    // so only a launched projectile can damage it.
    let mut s = fresh(image)?;
    let a = spawn(&mut s, 0, "skirmisher", 10, 10)?;
    let t = spawn(&mut s, 1, "skirmisher", 18, 10)?;
    s.issue_order_kind(a, ORDER_HOLD, fixed::from_int(10), fixed::from_int(10))
        .map_err(|r| refused("hold", r))?;
    let before = s.hp(t);
    if !s.cast_ability(a, 0, t as i32, 0, 0) {
        return Err(format!("pulse_shot refused: {}", s.stats.last_refusal.name()));
    }
    run_ticks(&mut s, 60);
    if s.hp(t) >= before {
        return Err("target hp unchanged 60 ticks after pulse_shot at range 8".into());
    }
    Ok(())
}

pub fn shield_absorbs(image: &[u8]) -> Result<(), String> {
    // The warden's shield absorbs the skirmisher's hits first, so after 400 ticks the shield
    // is lower than it started and hp is still full.
    let mut s = fresh(image)?;
    let a = spawn(&mut s, 0, "skirmisher", 10, 10)?;
    let t = spawn(&mut s, 1, "warden", 12, 10)?;
    s.issue_order_kind(a, ORDER_ATTACK_MOVE, fixed::from_int(12), fixed::from_int(10))
        .map_err(|r| refused("attack-move", r))?;
    let full = fixed::from_int(120);
    let shield0 = s.shield_of(t);
    run_ticks(&mut s, 400);
    let (shield, hp) = (s.shield_of(t), s.hp(t));
    if shield >= shield0 {
        return Err(format!(
            "warden shield {:.2} was not reduced from {:.2} in 400 ticks",
            fixed::to_float(shield),
            fixed::to_float(shield0)
        ));
    }
    if hp != full {
        return Err(format!(
            "warden hp {:.2} changed from {:.2}; the shield should absorb the hits",
            fixed::to_float(hp),
            fixed::to_float(full)
        ));
    }
    Ok(())
}

pub fn player_ownership(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    for p in [0u8, 1, 15] {
        let i = spawn(&mut s, p, "skirmisher", 20, 20)?;
        if s.player(i) != p {
            return Err(format!("unit {i} owned by {} want {p}", s.player(i)));
        }
    }
    Ok(())
}

pub fn player_resources(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    s.add_resources(2, 300, 40);
    if s.minerals(2) != 300 {
        return Err(format!("minerals(2) {} want 300", s.minerals(2)));
    }
    if s.vespene(2) != 40 {
        return Err(format!("vespene(2) {} want 40", s.vespene(2)));
    }
    spawn(&mut s, 2, "skirmisher", 20, 20)?;
    let present = (0..s.count()).filter(|&j| s.player(j) == 2 && s.alive(j)).count();
    if present != 1 {
        return Err(format!("player 2 has {present} live units, want 1"));
    }
    Ok(())
}

fn skirmish_hp(image: &[u8], allied: bool) -> Result<i32, String> {
    let mut s = fresh(image)?;
    if allied {
        s.set_alliance(0, 1, true).map_err(|r| refused("set_alliance", r))?;
    }
    let a = spawn(&mut s, 0, "skirmisher", 10, 10)?;
    let t = spawn(&mut s, 1, "skirmisher", 12, 10)?;
    s.issue_order_kind(a, ORDER_ATTACK_MOVE, fixed::from_int(12), fixed::from_int(10))
        .map_err(|r| refused("attack-move", r))?;
    run_ticks(&mut s, 120);
    Ok(s.hp(t))
}

pub fn alliance_filter(image: &[u8]) -> Result<(), String> {
    let full = fixed::from_int(45);
    if skirmish_hp(image, true)? != full {
        return Err("allied skirmisher took damage".into());
    }
    if skirmish_hp(image, false)? >= full {
        return Err("hostile skirmisher took no damage".into());
    }
    Ok(())
}

pub fn tick_rate(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    run_ticks(&mut s, 100);
    if s.ticks() != 100 {
        return Err(format!("ticks() {} after 100 tick calls", s.ticks()));
    }
    Ok(())
}

pub fn cooldown_expires(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let a = spawn(&mut s, 0, "skirmisher", 10, 10)?;
    let t = spawn(&mut s, 1, "skirmisher", 12, 10)?;
    if !s.cast_ability(a, 0, t as i32, 0, 0) {
        return Err(format!("first cast refused: {}", s.stats.last_refusal.name()));
    }
    if s.cast_ability(a, 0, t as i32, 0, 0) {
        return Err("re-cast accepted before the cooldown elapsed".into());
    }
    run_ticks(&mut s, 30);
    if !s.cast_ability(a, 0, t as i32, 0, 0) {
        return Err(format!("cast still refused after 30 ticks: {}", s.stats.last_refusal.name()));
    }
    Ok(())
}

pub fn fixed_math(_image: &[u8]) -> Result<(), String> {
    if fixed::mul(fixed::from_int(3), fixed::from_int(4)) != fixed::from_int(12) {
        return Err("mul(3, 4) is not 12".into());
    }
    if fixed::to_int(fixed::from_int(-7)) != -7 {
        return Err("to_int(from_int(-7)) is not -7".into());
    }
    if fixed::round(fixed::from_milli(1500)) != 2 {
        return Err("round(1.5) is not 2".into());
    }
    if fixed::sqrt(fixed::from_int(16)) != fixed::from_int(4) {
        return Err("sqrt(16) is not 4".into());
    }
    if fixed::from_milli(1000) != fixed::ONE {
        return Err("from_milli(1000) is not one unit".into());
    }
    Ok(())
}

pub fn state_hash_stable(image: &[u8]) -> Result<(), String> {
    let mut a = fresh(image)?;
    let mut b = fresh(image)?;
    if a.state_hash() != b.state_hash() {
        return Err("two fresh loads of one image hash differently".into());
    }
    run_ticks(&mut a, 500);
    run_ticks(&mut b, 500);
    if a.state_hash() != b.state_hash() {
        return Err("two identical runs diverged after 500 ticks".into());
    }
    Ok(())
}

fn script_hash(image: &[u8]) -> Result<u64, String> {
    let mut s = fresh(image)?;
    s.seed(7);
    let a = spawn(&mut s, 0, "skirmisher", 10, 10)?;
    let t = spawn(&mut s, 1, "skirmisher", 12, 10)?;
    s.issue_order_kind(a, ORDER_ATTACK_MOVE, fixed::from_int(12), fixed::from_int(10))
        .map_err(|r| refused("attack-move", r))?;
    s.cast_ability(a, 0, t as i32, 0, 0);
    run_ticks(&mut s, 300);
    Ok(s.state_hash())
}

pub fn replay_hash_equal(image: &[u8]) -> Result<(), String> {
    let first = script_hash(image)?;
    let second = script_hash(image)?;
    if first != second {
        return Err(format!("first={first:016x} second={second:016x}"));
    }
    Ok(())
}

pub fn map_image_read(image: &[u8]) -> Result<(), String> {
    osdt_data::persist::check_version(image).map_err(|e| format!("check_version: {e}"))?;
    osdt_data::reader::verify(image).map_err(|e| format!("verify: {e:#}"))?;
    let mut stale = image.to_vec();
    stale[4] ^= 0xff;
    if osdt_data::persist::check_version(&stale).is_ok() {
        return Err("an image with a wrong version was accepted".into());
    }
    Ok(())
}

pub fn map_image_roundtrip(image: &[u8]) -> Result<(), String> {
    use osdt_data::persist::{assemble, split};

    let sections = split(image).map_err(|e| format!("split: {e}"))?;
    let rebuilt = assemble(&sections);
    if rebuilt != image {
        return Err(format!(
            "assemble(split(image)) is {} bytes, image is {} bytes, and they differ",
            rebuilt.len(),
            image.len()
        ));
    }
    Ok(())
}

pub fn save_load_state(image: &[u8]) -> Result<(), String> {
    let mut a = fresh(image)?;
    spawn(&mut a, 0, "skirmisher", 10, 10)?;
    spawn(&mut a, 1, "warden", 14, 10)?;
    run_ticks(&mut a, 100);
    let bytes = a.save_state();
    let mut b = fresh(image)?;
    b.load_state(&bytes).map_err(|e| format!("load_state: {e:?}"))?;
    if a.state_hash() != b.state_hash() {
        return Err(format!(
            "hash {:016x} after load_state, saved state hashed {:016x}",
            b.state_hash(),
            a.state_hash()
        ));
    }
    run_ticks(&mut a, 200);
    run_ticks(&mut b, 200);
    if a.state_hash() != b.state_hash() {
        return Err("the loaded run diverged from the uninterrupted run within 200 ticks".into());
    }
    Ok(())
}

pub fn apply_buff(image: &[u8]) -> Result<(), String> {
    // BUFF_SLOTS is 4. A held kind refreshes in place; a fifth kind is refused and counted
    // as dropped; a slot frees once its 5-tick countdown runs out.
    let mut s = fresh(image)?;
    let a = spawn(&mut s, 0, "skirmisher", 10, 10)?;
    for kind in 1..=4 {
        if !s.apply_buff(a, kind, 5) {
            return Err(format!("buff {kind} refused with a free slot"));
        }
    }
    if !s.apply_buff(a, 1, 5) {
        return Err("refreshing a held buff was refused with all slots full".into());
    }
    if s.apply_buff(a, 5, 5) {
        return Err("a fifth buff was accepted with all four slots full".into());
    }
    if s.stats.buffs_dropped == 0 {
        return Err("the refused buff was not counted as dropped".into());
    }
    run_ticks(&mut s, 6);
    if !s.apply_buff(a, 5, 5) {
        return Err("no slot freed after the 5-tick countdown".into());
    }
    Ok(())
}
