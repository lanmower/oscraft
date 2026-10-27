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
    for _ in 0..300 {
        if let Err(r) = s.order_attack_or_approach(attacker, target) {
            if r.name() != "cooldown" {
                return Err(format!("order_attack_or_approach refused: {}", r.name()));
            }
        }
        if s.hp(target) < hp0 {
            break;
        }
        s.tick();
    }
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

pub fn ranged_miss_chance(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let w = spawn(&mut s, 0, "warden", 10, 10)?;
    let chance = s.data_miss_chance(w);
    if chance <= 0 {
        return Err(format!("the warden's unit data holds no ranged miss chance ({chance})"));
    }
    s.set_miss_chance(w, chance);
    let (mut missed, mut landed) = (0u32, 0u32);
    for _ in 0..2000 {
        if s.misses(w) {
            missed += 1;
        } else {
            landed += 1;
        }
    }
    if missed == 0 {
        return Err("a warden with a nonzero miss chance never missed in 2000 ranged rolls".to_string());
    }
    if landed == 0 {
        return Err("a warden never landed a ranged shot in 2000 rolls".to_string());
    }
    Ok(())
}

pub fn replay_playback_controls(image: &[u8]) -> Result<(), String> {
    let mut source = fresh(image)?;
    spawn(&mut source, 0, "skirmisher", 10, 10)?;
    run_ticks(&mut source, 20);
    let start = fresh(image)?;
    let mut p = replay::Playback::new(&source, start, &[]).map_err(|e| format!("Playback::new: {e:?}"))?;
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

fn spawn_air_on_plateau(s: &mut Sim, p: u8) -> Result<usize, String> {
    let air = air_kind(s)?;
    let ground = s.elevation_milli(10, 9);
    for y in 8..=9 {
        for x in 12..=13 {
            if s.elevation_milli(x, y) <= ground {
                continue;
            }
            if let Some(i) = s.spawn_for(p, air, fixed::from_int(x), fixed::from_int(y)) {
                return Ok(i);
            }
        }
    }
    Err("no raised-plateau cell accepts an air unit".to_string())
}

pub fn elevation_sight(image: &[u8]) -> Result<(), String> {
    let mut low = fresh(image)?;
    low.set_fog(true);
    spawn(&mut low, 0, "skirmisher", 10, 9)?;
    let elevated = spawn_air_on_plateau(&mut low, 1)?;
    run_ticks(&mut low, 2);
    if low.visible(0, elevated) {
        return Err("a ground unit on lower ground sees an air unit over the raised plateau".to_string());
    }
    let mut high = fresh(image)?;
    high.set_fog(true);
    spawn_air_on_plateau(&mut high, 0)?;
    let lower = spawn(&mut high, 1, "skirmisher", 10, 9)?;
    run_ticks(&mut high, 2);
    if !high.visible(0, lower) {
        return Err("an air unit over the raised plateau cannot see a ground unit below".to_string());
    }
    Ok(())
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

pub fn channel_interrupt(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let u = spawn(&mut s, 0, "skirmisher", 10, 10)?;
    if s.channel_left(u) != 0 {
        return Err(format!("idle unit reports {} ticks left on a channel", s.channel_left(u)));
    }
    if s.channel_ai(u) != -1 {
        return Err(format!("idle unit reports channel ability {}, want -1", s.channel_ai(u)));
    }
    Ok(())
}

pub fn damage_type_size(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let sk = spawn(&mut s, 0, "skirmisher", 10, 10)?;
    let w = spawn(&mut s, 1, "warden", 12, 10)?;
    let got = s.scale_hit(100, sk, w);
    if got != 100 {
        return Err(format!("normal damage 100 scaled to {got} against a massive armored target"));
    }
    Ok(())
}

pub fn cloak_detection(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let e = spawn(&mut s, 1, "skirmisher", 12, 10)?;
    if !s.targetable(0, e) {
        return Err("a plain unit is not targetable on a fresh map".to_string());
    }
    if s.detected(0, e) {
        return Err("a unit is detected with no detector on a fresh map".to_string());
    }
    Ok(())
}
