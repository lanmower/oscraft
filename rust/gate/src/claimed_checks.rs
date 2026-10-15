//! Gate checks for CAP rows that docs/CAPABILITIES.md lists as claimed. Each drives its row's
//! code path through the public sim, server and data APIs.

use sim::{fixed, Sim, ORDER_ATTACK_MOVE};

use crate::checks::{fresh, kind, run_ticks, spawn, MAX_UNITS};

fn air_kind(s: &Sim) -> Result<u32, String> {
    s.catalog()
        .units
        .iter()
        .position(|u| u.is_air())
        .map(|k| k as u32)
        .ok_or_else(|| "catalog has no air unit".to_string())
}

pub fn sight_reveal(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    s.set_fog(true);
    let (w, h) = (s.grid().w as i32, s.grid().h as i32);
    spawn(&mut s, 0, "skirmisher", 10, 10)?;
    let near = spawn(&mut s, 1, "skirmisher", 12, 10)?;
    let far = spawn(&mut s, 1, "skirmisher", w - 2, h - 2)?;
    run_ticks(&mut s, 2);
    if !s.visible(0, near) {
        return Err("enemy two cells away is not visible to player 0's skirmisher".to_string());
    }
    if s.visible(0, far) {
        return Err(format!("enemy at ({},{}) is visible to player 0 out of sight", w - 2, h - 2));
    }
    Ok(())
}

pub fn last_seen_memory(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    s.set_fog(true);
    let (w, h) = (s.grid().w as i32, s.grid().h as i32);
    spawn(&mut s, 0, "skirmisher", 10, 10)?;
    let enemy = spawn(&mut s, 1, "skirmisher", 12, 10)?;
    run_ticks(&mut s, 2);
    if !s.visible(0, enemy) {
        return Err("enemy was not visible before it left sight".to_string());
    }
    let seen_at = s.position_fixed(enemy);
    if !s.teleport(enemy, fixed::from_int(w - 2), fixed::from_int(h - 2)) {
        return Err("teleport refused the enemy move out of sight".to_string());
    }
    run_ticks(&mut s, 2);
    if s.visible(0, enemy) {
        return Err("enemy is still visible after it left sight".to_string());
    }
    match s.last_seen(0, enemy) {
        Some(p) if p == seen_at => Ok(()),
        Some(p) => Err(format!("last seen at {p:?}, want {seen_at:?}")),
        None => Err("no last-seen position was kept for the enemy".to_string()),
    }
}

pub fn air_terrain_crossing(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let air = air_kind(&s)?;
    let (w, h) = (s.grid().w as i32, s.grid().h as i32);
    let blocked = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).find(|&(x, y)| !s.grid().walkable(x, y));
    let Some((x, y)) = blocked else {
        return Err("map has no unwalkable cell for an air unit to cross".to_string());
    };
    match s.spawn_for(0, air, fixed::from_int(x), fixed::from_int(y)) {
        Some(_) => Ok(()),
        None => Err(format!("air unit refused at unwalkable cell ({x},{y})")),
    }
}

pub fn ground_only_air_refusal(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let air = air_kind(&s)?;
    let attacker = spawn(&mut s, 0, "skirmisher", 10, 10)?;
    let target = s
        .spawn_for(1, air, fixed::from_int(11), fixed::from_int(10))
        .ok_or_else(|| "air target refused at (11,10)".to_string())?;
    if s.order_attack(attacker, target).is_ok() {
        return Err("a ground weapon was allowed to attack an adjacent air unit".to_string());
    }
    Ok(())
}

pub fn unit_collision(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let a = spawn(&mut s, 0, "skirmisher", 10, 10)?;
    let b = spawn(&mut s, 0, "skirmisher", 10, 10)?;
    run_ticks(&mut s, 30);
    if s.position_fixed(a) == s.position_fixed(b) {
        return Err("two units placed on one point are still at the same point after 30 ticks".to_string());
    }
    Ok(())
}

pub fn narrow_passage_limit(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let skirmisher = kind(&s, "skirmisher")?;
    let mut placed = 0u32;
    for _ in 0..64 {
        if s.spawn_for(0, skirmisher, fixed::from_int(10), fixed::from_int(10)).is_some() {
            placed += 1;
        }
    }
    if placed == 64 {
        return Err("64 units entered one cell; the cell limit never refused".to_string());
    }
    Ok(())
}

pub fn weapon_range(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let (w, h) = (s.grid().w as i32, s.grid().h as i32);
    let attacker = spawn(&mut s, 0, "skirmisher", 10, 10)?;
    let target = spawn(&mut s, 1, "skirmisher", w - 2, h - 2)?;
    if s.order_attack(attacker, target).is_ok() {
        return Err("order_attack accepted a target out of weapon reach".to_string());
    }
    let hp0 = s.hp(target);
    s.order_attack_or_approach(attacker, target)
        .map_err(|r| format!("order_attack_or_approach refused: {}", r.name()))?;
    run_ticks(&mut s, 300);
    if s.hp(target) >= hp0 {
        return Err(format!("target hp {} unchanged after 300 ticks of approach", fixed::to_int(s.hp(target))));
    }
    Ok(())
}

pub fn order_recording(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let a = spawn(&mut s, 0, "skirmisher", 10, 10)?;
    let before = s.order_log_len();
    s.issue_order_kind(a, ORDER_ATTACK_MOVE, fixed::from_int(12), fixed::from_int(10))
        .map_err(|r| format!("issue_order_kind refused: {}", r.name()))?;
    if s.order_log_len() != before + 1 {
        return Err(format!("order log went from {before} to {} after one order", s.order_log_len()));
    }
    if s.order_record(before).is_none() {
        return Err("the accepted order is missing from the log".to_string());
    }
    Ok(())
}

pub fn mod_package_version(image: &[u8]) -> Result<(), String> {
    let mut stale = image.to_vec();
    stale[4] ^= 0xff;
    match sim::load(&stale, MAX_UNITS) {
        Err(sim::LoadError::BadVersion(_)) => Ok(()),
        Err(e) => Err(format!("a wrong-version package failed with {e:?}, want BadVersion")),
        Ok(_) => Err("a wrong-version package was loaded".to_string()),
    }
}

fn missing_api(name: &str) -> Result<(), String> {
    Err(format!("missing API: {name}"))
}

fn board_warden_and_skirmisher(s: &mut Sim) -> Result<(usize, usize), String> {
    let t = spawn(s, 0, "warden", 10, 10)?;
    let i = spawn(s, 0, "skirmisher", 11, 10)?;
    s.load_unit(t, i).map_err(|r| format!("load_unit refused: {}; units.star declares no transport capacity", r.name()))?;
    Ok((t, i))
}

pub fn transport_load(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    board_warden_and_skirmisher(&mut s).map(|_| ())
}

pub fn transport_unload_point(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let (t, _) = board_warden_and_skirmisher(&mut s)?;
    match s.unload_transport_at(t, fixed::from_int(20), fixed::from_int(10)) {
        Ok(1) => Ok(()),
        Ok(n) => Err(format!("unloaded {n} units at the point, want 1")),
        Err(r) => Err(format!("unload_transport_at refused: {}", r.name())),
    }
}

pub fn buff_stacking_rule(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let u = spawn(&mut s, 0, "skirmisher", 10, 10)?;
    if !s.apply_buff(u, 1, 40) {
        return Err("first apply_buff refused".to_string());
    }
    if !s.apply_buff(u, 1, 80) {
        return Err("a second buff of the same kind was refused instead of refreshing".to_string());
    }
    s.clear_buff(u, 1);
    if !s.apply_buff(u, 1, 40) {
        return Err("the buff slot was still held after clear_buff".to_string());
    }
    Ok(())
}

pub fn shield_regen_delay(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let t = spawn(&mut s, 1, "warden", 12, 10)?;
    let full = s.shield_of(t);
    s.hurt(t, 10);
    let hit = s.shield_of(t);
    if hit >= full {
        return Err(format!("hurt left the shield at {hit}, want below {full}"));
    }
    run_ticks(&mut s, 100);
    if s.shield_of(t) != hit {
        return Err("the shield regenerated inside its delay".to_string());
    }
    run_ticks(&mut s, 1000);
    if s.shield_of(t) <= hit {
        return Err("the shield did not regenerate after its delay".to_string());
    }
    Ok(())
}

pub fn upgrade_levels(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    s.add_resources(0, 5000, 5000);
    let m0 = s.minerals(0);
    s.complete_upgrade(0, 0).map_err(|r| format!("complete_upgrade refused: {}", r.name()))?;
    let m1 = s.minerals(0);
    s.complete_upgrade(0, 0).map_err(|r| format!("second complete_upgrade refused: {}", r.name()))?;
    let first = m0 - m1;
    let second = m1 - s.minerals(0);
    if first != 100 || second != 200 {
        return Err(format!("upgrade levels cost {first} then {second}, want 100 then 200"));
    }
    Ok(())
}

pub fn upgrade_stacking(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    s.add_resources(0, 50000, 50000);
    s.complete_upgrade(0, 0).map_err(|r| format!("add-stacking first level refused: {}", r.name()))?;
    s.complete_upgrade(0, 0).map_err(|r| format!("add-stacking second level refused: {}", r.name()))?;
    s.complete_upgrade(1, 0).map_err(|r| format!("refuse-stacking first level refused: {}", r.name()))?;
    if s.complete_upgrade(1, 0).is_ok() {
        return Err("a refuse-stacking upgrade was applied twice".to_string());
    }
    Ok(())
}

pub fn upgrade_applies_existing(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    s.add_resources(0, 5000, 5000);
    let w = spawn(&mut s, 0, "warden", 10, 10)?;
    let before = s.shield_of(w);
    s.complete_upgrade(0, 0).map_err(|r| format!("complete_upgrade refused: {}", r.name()))?;
    let after = s.shield_of(w);
    if after <= before {
        return Err(format!("existing warden shield {before} unchanged by an upgrade that applies to existing units"));
    }
    Ok(())
}

pub fn ranged_miss_chance(_image: &[u8]) -> Result<(), String> {
    missing_api("Sim::misses (pub(crate)): the miss roll is private")
}

pub fn replay_playback_controls(image: &[u8]) -> Result<(), String> {
    let mut source = fresh(image)?;
    spawn(&mut source, 0, "skirmisher", 10, 10)?;
    run_ticks(&mut source, 20);
    let start = fresh(image)?;
    let mut p = server::replay::Playback::new(&source, start, &[]).map_err(|e| format!("Playback::new: {e:?}"))?;
    let end = p.end();
    if end == 0 {
        return Err("playback ends at tick 0 for a 20-tick source".to_string());
    }
    let mid = end / 2;
    let got = p.seek(mid).map_err(|e| format!("seek({mid}): {e:?}"))?;
    if got != mid {
        return Err(format!("seek({mid}) landed on tick {got}"));
    }
    p.pause();
    p.resume();
    p.set_speed(2).map_err(|e| format!("set_speed(2): {e:?}"))?;
    Ok(())
}

pub fn lockstep_input_exchange(_image: &[u8]) -> Result<(), String> {
    use std::time::{Duration, Instant};
    let mut ls = server::lockstep::Lockstep::new(&[0, 1], 0, Duration::from_secs(1));
    ls.submit(0, 0, Vec::new()).map_err(|e| format!("submit player 0: {e:?}"))?;
    ls.submit(0, 1, Vec::new()).map_err(|e| format!("submit player 1: {e:?}"))?;
    match ls.poll(Instant::now()) {
        Ok(Some(r)) if r.tick == 0 => Ok(()),
        Ok(Some(r)) => Err(format!("released tick {}, want 0", r.tick)),
        Ok(None) => Err("no tick released after both players submitted".to_string()),
        Err(e) => Err(format!("poll: {e:?}")),
    }
}

const NEUTRAL_OWNER: u8 = 0xFF;

pub fn player_start_positions(image: &[u8]) -> Result<(), String> {
    let s = fresh(image)?;
    let starts = &s.catalog().starts;
    for p in 0..2u8 {
        let count = starts.iter().filter(|st| st.player == p).count();
        if count != 1 {
            return Err(format!("player {p} has {count} start positions, want 1"));
        }
    }
    Ok(())
}

pub fn neutral_objects(image: &[u8]) -> Result<(), String> {
    let s = fresh(image)?;
    if !(0..s.count()).any(|i| s.alive(i) && s.player(i) == NEUTRAL_OWNER) {
        return Err("no live unit belongs to no player after the map loads".to_string());
    }
    Ok(())
}

pub fn map_author_placement(image: &[u8]) -> Result<(), String> {
    let s = fresh(image)?;
    let starts = &s.catalog().starts;
    for p in 0..2u8 {
        if !starts.iter().any(|st| st.player == p) {
            return Err(format!("map data has no start for player {p}"));
        }
    }
    if !s.catalog().placements.iter().any(|p| p.player == NEUTRAL_OWNER) {
        return Err("map data places no neutral object".to_string());
    }
    Ok(())
}

pub fn elevation_sight(_image: &[u8]) -> Result<(), String> {
    missing_api("elevation-aware vision: Sim::visible does not read elevation_milli")
}

pub fn hp_regen(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let k = {
        let cat = s.catalog();
        cat.regen
            .iter()
            .enumerate()
            .position(|(k, r)| r.hp_per_tick > 0 && cat.units.get(k).map_or(false, |u| u.shield == 0))
            .ok_or_else(|| "no shieldless unit kind declares hp regeneration".to_string())?
    };
    let u = s
        .spawn_for(0, k as u32, fixed::from_int(10), fixed::from_int(10))
        .ok_or_else(|| "regenerating unit refused at (10,10)".to_string())?;
    s.hurt(u, 5);
    let hp0 = s.hp(u);
    run_ticks(&mut s, 200);
    if s.hp(u) <= hp0 {
        return Err(format!("hp stayed at {} after 200 ticks of regeneration", fixed::to_int(hp0)));
    }
    Ok(())
}

pub fn editor_terrain_tiles(_image: &[u8]) -> Result<(), String> {
    missing_api("graphical terrain editor: no editor API on main")
}

pub fn editor_abilities(_image: &[u8]) -> Result<(), String> {
    missing_api("graphical ability editor: no editor API on main")
}

pub fn editor_triggers(_image: &[u8]) -> Result<(), String> {
    missing_api("graphical trigger editor: no editor API on main")
}

pub fn channel_interrupt(_image: &[u8]) -> Result<(), String> {
    missing_api("Sim channel state: channel_left and channel_ai are pub(crate) with no public read")
}

pub fn worker_saturation(_image: &[u8]) -> Result<(), String> {
    missing_api("Sim harvest income per patch: no public read of income per worker")
}
