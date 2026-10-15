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
