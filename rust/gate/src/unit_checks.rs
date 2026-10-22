//! Gate checks for the moba scenario's neutral camps and timers: camp stacking, first spawn, Roshan's
//! respawn window, the Tormentor's timing and the pit swap. Each runs on the moba image.

use crate::checks::{fresh, scenario_rule, run_ticks, MAX_UNITS};
use crate::checks::role;
use sim::{fixed, Sim};

fn scenario_image() -> Result<Vec<u8>, String> {
    let scenario = crate::checks::pack_dir();
    osdt_data::generate(&scenario).map_err(|e| format!("moba scenario image: {e:#}"))
}

/// Roshan respawns a random 8 to 11 minutes after death, never earlier or later.
pub fn spawn_group_boss_window(_image: &[u8]) -> Result<(), String> {
    let mut s = fresh(&scenario_image()?)?;
    let boss = (0..MAX_UNITS)
        .filter(|&u| s.alive(u) && s.player(u) == 0xFF)
        .max_by_key(|&u| s.hp(u))
        .ok_or("moba image places no neutral unit")?;
    s.hurt(boss, fixed::from_int(10000));
    let died = s.ticks();
    let min = (scenario_rule("roshan_respawn_min_s")? as u32) * sim::TICKS_PER_SECOND as u32;
    let max = min + (scenario_rule("roshan_respawn_spread_s")? as u32) * sim::TICKS_PER_SECOND as u32;
    let mut back = None;
    for _ in 0..max + 16 {
        s.tick();
        if s.alive(boss) {
            back = Some(s.ticks() - died);
            break;
        }
    }
    let waited = back.ok_or("Roshan never respawned within 11 minutes")?;
    if waited < min || waited > max + 2 {
        return Err(format!("Roshan respawned after {waited} ticks, want {min} to {max}"));
    }
    println!("camp roshan window ok waited_ticks={waited} window={min}..{max}");
    Ok(())
}

/// A camp refills on whole-minute marks, and only when every unit of the camp is dead: a camp with a
/// survivor stays as it is (the stacking rule).
pub fn spawn_group_minute_refill(_image: &[u8]) -> Result<(), String> {
    let mut s = fresh(&scenario_image()?)?;
    run_ticks(&mut s, (scenario_rule("camp_first_spawn_s")? as u32) * sim::TICKS_PER_SECOND as u32 + 1);
    let u = (0..MAX_UNITS).find(|&u| s.alive(u) && s.player(u) == 0xFF && s.kind_name(u).is_some_and(|n| n.starts_with("neutral_"))).ok_or("moba image places no neutral camp unit")?;
    let (hx, hy) = s.position_fixed(u);
    let reach = fixed::from_milli(scenario_rule("camp_clear_milli")?) as i64;
    let mates: Vec<usize> = (0..MAX_UNITS)
        .filter(|&k| {
            let (x, y) = s.position_fixed(k);
            let (dx, dy) = ((x - hx) as i64, (y - hy) as i64);
            s.alive(k) && s.player(k) == 0xFF && dx * dx + dy * dy <= reach * reach && s.kind_name(k).is_some_and(|n| n.starts_with("neutral_"))
        })
        .collect();
    if mates.len() < 2 {
        return Err(format!("camp around unit {u} holds {} units, want at least 2", mates.len()));
    }
    let mark = (scenario_rule("camp_respawn_s")? as u32) * sim::TICKS_PER_SECOND as u32;
    s.hurt(u, fixed::from_int(10000));
    run_ticks(&mut s, mark + 8);
    if s.alive(u) {
        return Err(format!("camp unit {u} respawned on the minute mark while {} camp mates still lived", mates.len() - 1));
    }
    for &k in mates.iter().filter(|&&k| k != u) {
        s.hurt(k, fixed::from_int(10000));
    }
    run_ticks(&mut s, mark - 24);
    if mates.iter().any(|&k| s.alive(k)) {
        return Err("camp respawned before the next minute mark".into());
    }
    run_ticks(&mut s, 32);
    if !mates.iter().all(|&k| s.alive(k)) {
        return Err("a cleared camp did not refill on the next minute mark".into());
    }
    println!("camp minute refill ok camp_size={} mark_ticks={mark}", mates.len());
    Ok(())
}

/// Regular neutral camps are absent until 1:00; Roshan and the bounty runes are present from the start.
pub fn spawn_group_first_spawn(_image: &[u8]) -> Result<(), String> {
    let mut s = fresh(&scenario_image()?)?;
    let regular = |s: &Sim| (0..MAX_UNITS).filter(|&u| s.alive(u) && s.player(u) == 0xFF && s.kind_name(u).is_some_and(|n| n.starts_with("neutral_"))).count();
    let r_unit_boss = role("unit.boss")?;
    let boss = (0..MAX_UNITS).filter(|&u| s.alive(u) && s.player(u) == 0xFF && s.kind_name(u) == Some(r_unit_boss)).count();
    if regular(&s) != 0 || boss == 0 {
        return Err(format!("at tick 0 {} camp units live and {boss} bosses, want 0 and at least 1", regular(&s)));
    }
    let first = (scenario_rule("camp_first_spawn_s")? as u32) * sim::TICKS_PER_SECOND as u32;
    run_ticks(&mut s, first - 4);
    if regular(&s) != 0 {
        return Err("a camp unit lived before 1:00".into());
    }
    run_ticks(&mut s, 8);
    let n = regular(&s);
    if n == 0 {
        return Err("no camp unit lived after 1:00".into());
    }
    println!("camp first spawn ok at_tick={first} camp_units={n}");
    Ok(())
}

/// The Tormentor first spawns at 15:00, and returns 10 minutes after its death.
pub fn spawn_group_timed_boss_timing(_image: &[u8]) -> Result<(), String> {
    let mut s = fresh(&scenario_image()?)?;
    let r_unit_timed_boss = role("unit.timed_boss")?;
    let find = |s: &Sim| (0..MAX_UNITS).find(|&u| s.alive(u) && s.kind_name(u) == Some(r_unit_timed_boss));
    if find(&s).is_some() {
        return Err("a Tormentor lives at tick 0".into());
    }
    let tps = sim::TICKS_PER_SECOND as u32;
    run_ticks(&mut s, 900 * tps - 4);
    if find(&s).is_some() {
        return Err("a Tormentor lived before 15:00".into());
    }
    run_ticks(&mut s, 8);
    let t = find(&s).ok_or("no Tormentor after 15:00")?;
    s.hurt(t, fixed::from_int(10000));
    s.hurt(t, fixed::from_int(10000));
    if s.alive(t) {
        return Err("the Tormentor survived lethal damage".into());
    }
    let died = s.ticks();
    run_ticks(&mut s, 600 * tps - 4);
    if find(&s).is_some() {
        return Err("the Tormentor returned before 10 minutes".into());
    }
    run_ticks(&mut s, 8);
    if find(&s).is_none() {
        return Err("the Tormentor did not return 10 minutes after its death".into());
    }
    println!("camp tormentor timing ok first_tick={} died_tick={died}", 900 * tps);
    Ok(())
}

/// From 15:00 Roshan walks to the pit of the day/night phase (top at night), and the Tormentor respawns at the
/// Radiant corner at night and the Dire corner by day.
pub fn spawn_group_swap(_image: &[u8]) -> Result<(), String> {
    let mut s = fresh(&scenario_image()?)?;
    let find = |s: &Sim, name: &str| (0..MAX_UNITS).find(|&u| s.alive(u) && s.kind_name(u) == Some(name));
    let cells = |s: &Sim, u: usize, x: i32, y: i32| {
        let (px, py) = s.position_fixed(u);
        let (dx, dy) = (i64::from(px - fixed::from_milli(x)), i64::from(py - fixed::from_milli(y)));
        ((dx * dx + dy * dy) as f64).sqrt() / f64::from(fixed::ONE)
    };
    let roshan = find(&s, role("unit.boss")?).ok_or("no Roshan at tick 0")?;
    let d0 = cells(&s, roshan, 207734, 206312);
    if d0 > 12.0 {
        return Err(format!("Roshan starts {d0:.0} cells from the bottom pit"));
    }
    run_ticks(&mut s, 14400 + 1700);
    if !s.alive(roshan) {
        return Err("Roshan died on the way to the top pit".into());
    }
    let d1 = cells(&s, roshan, 113589, 126084);
    if d1 > 12.0 {
        return Err(format!("at 15:00 plus 1700 ticks Roshan stands {d1:.0} cells from the night pit"));
    }
    let tormentor = find(&s, role("unit.timed_boss")?).ok_or("no Tormentor after 15:00")?;
    let t0 = cells(&s, tormentor, 43500, 64500);
    if t0 > 4.0 {
        return Err(format!("the night Tormentor stands {t0:.0} cells from the Radiant corner"));
    }
    let left = 19300 - s.ticks();
    run_ticks(&mut s, left);
    s.hurt(tormentor, fixed::from_int(10000));
    s.hurt(tormentor, fixed::from_int(10000));
    run_ticks(&mut s, 9600 + 8);
    let back = find(&s, role("unit.timed_boss")?).ok_or("the Tormentor did not return")?;
    let t1 = cells(&s, back, 284500, 260500);
    if t1 > 4.0 {
        return Err(format!("the day Tormentor stands {t1:.0} cells from the Dire corner"));
    }
    println!("camp pit swap ok roshan_night_pit_dist={d1:.1} tormentor_day_dist={t1:.1}");
    Ok(())
}
