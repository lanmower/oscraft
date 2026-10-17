//! Gate checks for the engine's general mechanics, each run on the default data image. Each
//! asserts one postcondition through the public sim or mod-host API and names the mechanic,
//! never a genre.

use crate::checks::{fresh, kind, run_ticks, spawn, MAX_UNITS};
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
    if s.inventory_slots() != 6 {
        return Err(format!("inventory_slots {} want 6", s.inventory_slots()));
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

pub fn xp_level_up(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let u = spawn(&mut s, 0, "skirmisher", 10, 10)?;
    let curve = s.xp_curve().to_vec();
    let first = *curve.first().ok_or("xp curve is empty")?;
    if s.level(u) != 0 {
        return Err(format!("new unit starts at level {}", s.level(u)));
    }
    if s.add_xp(u, first - 1) != 0 || s.level(u) != 0 {
        return Err(format!("xp {} below the first threshold {first} levelled up", first - 1));
    }
    if s.add_xp(u, 1) != 1 || s.level(u) != 1 || s.xp(u) != first {
        return Err(format!("reaching {first} xp: level {} xp {}", s.level(u), s.xp(u)));
    }
    if !s.set_xp_curve(&[10, 20]) || s.set_xp_curve(&[20, 10]) || s.xp_curve() != [10, 20] {
        return Err("a nondecreasing curve was refused or a decreasing one replaced the curve".into());
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

/// Destroying every tagged structure of one side declares the other side the winner. The side-1
/// beacons of the duel-routes scenario carry the structure tag; the last one standing decides it.
pub fn win_trigger(_image: &[u8]) -> Result<(), String> {
    let scenario = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("scenarios").join("duel-routes");
    let image = osdt_data::generate(&scenario).map_err(|e| format!("scenario image: {e:#}"))?;
    let mut s = fresh(&image)?;
    let mut tagged: Vec<usize> = (0..MAX_UNITS).filter(|&i| s.alive(i) && s.player(i) == 1 && s.kind_name(i) == Some("beacon")).collect();
    let last = tagged.pop().ok_or("scenario has no side 1 structure")?;
    for &i in &tagged {
        s.hurt(i, fixed::from_int(10000));
    }
    run_ticks(&mut s, 1);
    if s.winner().is_some() {
        return Err("a winner was declared while a tagged structure of side 1 still stands".into());
    }
    s.hurt(last, fixed::from_int(10000));
    run_ticks(&mut s, 1);
    let winner = s.winner().ok_or("no winner after every tagged structure of side 1 was destroyed")?;
    if winner != 0 {
        return Err(format!("winner {winner} want 0, the side that is not side 1"));
    }
    println!("win trigger ok winner={winner}");
    Ok(())
}

/// Destroying a creep pays its bounty into the rival side's resource pool at once.
pub fn kill_bounty(_image: &[u8]) -> Result<(), String> {
    let scenario = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("scenarios").join("duel-routes");
    let image = osdt_data::generate(&scenario).map_err(|e| format!("scenario image: {e:#}"))?;
    let mut s = fresh(&image)?;
    let creep = spawn(&mut s, 1, "creep", 30, 20)?;
    let gold_before = s.resource_a(0);
    s.hurt(creep, fixed::from_int(10000));
    let gold_after = s.resource_a(0);
    if gold_after - gold_before != 25 {
        return Err(format!("creep bounty paid {} want 25", gold_after - gold_before));
    }
    println!("kill bounty ok gold_before={gold_before} gold_after={gold_after}");
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
    host.load(&path).map_err(|e| format!("mod load: {e}"))?;
    let state = host.state().clone();
    host.advance(&state).map_err(|e| format!("advance: {e}"))?;
    let inside = host.state().get::<Option<bool>>("inside").map_err(|e| format!("state.inside: {e}"))?;
    let outside = host.state().get::<Option<bool>>("outside").map_err(|e| format!("state.outside: {e}"))?;
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
fn moba_image() -> Result<Vec<u8>, String> {
    let scenario = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("scenarios").join("moba");
    osdt_data::generate(&scenario).map_err(|e| format!("moba scenario image: {e:#}"))
}

/// The bound on ticks from a base's destruction to a declared winner.
const MATCH_TICK_LIMIT: u32 = 200;

/// Destroying a side's only base ends the moba match with the rival as winner, within a bounded tick count.
pub fn moba_match_end(_image: &[u8]) -> Result<(), String> {
    let mut s = fresh(&moba_image()?)?;
    run_ticks(&mut s, 50);
    let base = (0..MAX_UNITS).find(|&i| s.alive(i) && s.player(i) == 1 && s.kind_name(i) == Some("base")).ok_or("moba has no side 1 base")?;
    if s.winner().is_some() {
        return Err("a winner was declared before any base fell".into());
    }
    s.hurt(base, fixed::from_int(10000));
    let mut ticks = 0;
    while s.winner().is_none() && ticks < MATCH_TICK_LIMIT {
        s.tick();
        ticks += 1;
    }
    let winner = s.winner().ok_or(format!("no winner within {MATCH_TICK_LIMIT} ticks of the base falling"))?;
    if winner != 0 {
        return Err(format!("winner {winner} want 0, the side that is not side 1"));
    }
    println!("moba match ok winner={winner} ticks={ticks}");
    Ok(())
}

/// A creep ordered from its own base toward the rival base closes distance along the lane.
pub fn moba_creeps(_image: &[u8]) -> Result<(), String> {
    let mut s = fresh(&moba_image()?)?;
    let own = (0..MAX_UNITS).find(|&i| s.alive(i) && s.player(i) == 0 && s.kind_name(i) == Some("base")).ok_or("moba has no side 0 base")?;
    let rival = (0..MAX_UNITS).find(|&i| s.alive(i) && s.player(i) == 1 && s.kind_name(i) == Some("base")).ok_or("moba has no side 1 base")?;
    let (ox, oy) = s.position_fixed(own);
    let (tx, ty) = s.position_fixed(rival);
    let k = kind(&s, "creep")?;
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

/// Two sims loaded from the same moba image, given the same orders at the same ticks, end in the same state.
pub fn moba_sync(_image: &[u8]) -> Result<(), String> {
    let image = moba_image()?;
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
