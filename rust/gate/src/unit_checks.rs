//! Gate checks for the moba scenario's units and timers: lane waves, siege timing, barracks wave
//! upgrades, neutral camp stacking and Roshan's respawn window. Each runs on the moba image.

use crate::checks::{fresh, run_ticks, spawn, MAX_UNITS};
use sim::{fixed, Sim};

fn moba_image() -> Result<Vec<u8>, String> {
    let scenario = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("scenarios").join("moba");
    osdt_data::generate(&scenario).map_err(|e| format!("moba scenario image: {e:#}"))
}

fn count_kind(s: &Sim, player: u8, name: &str) -> usize {
    (0..MAX_UNITS).filter(|&i| s.alive(i) && s.player(i) == player && s.kind_name(i) == Some(name)).count()
}

/// Every 30 s each base sends three melee creeps and one ranged creep down each of the three lanes.
pub fn moba_wave_composition(_image: &[u8]) -> Result<(), String> {
    let mut s = fresh(&moba_image()?)?;
    let (melee0, ranged0, siege0) = (count_kind(&s, 0, "creep"), count_kind(&s, 0, "creep_ranged"), count_kind(&s, 0, "creep_siege"));
    run_ticks(&mut s, 2);
    let (melee1, ranged1) = (count_kind(&s, 0, "creep"), count_kind(&s, 0, "creep_ranged"));
    if melee1 - melee0 != 9 || ranged1 - ranged0 != 3 {
        return Err(format!("first wave added {} melee and {} ranged creeps, want 9 and 3", melee1 - melee0, ranged1 - ranged0));
    }
    if count_kind(&s, 0, "creep_siege") != siege0 {
        return Err("a siege creep spawned in the first wave".into());
    }
    run_ticks(&mut s, 480);
    let melee2 = count_kind(&s, 0, "creep");
    if melee2 - melee1 != 9 {
        return Err(format!("second wave 480 ticks later added {} melee creeps, want 9", melee2 - melee1));
    }
    println!("moba wave composition ok melee_per_wave=9 ranged_per_wave=3 interval_ticks=480");
    Ok(())
}

/// The first siege creeps of a side appear at 5:00 (4800 ticks), one per lane, and not before.
pub fn moba_siege_timing(_image: &[u8]) -> Result<(), String> {
    let mut s = fresh(&moba_image()?)?;
    let mut first = None;
    let mut most = 0;
    for _ in 0..4810 {
        s.tick();
        let n = count_kind(&s, 0, "creep_siege");
        if n > 0 && first.is_none() {
            first = Some(s.ticks());
        }
        most = most.max(n);
    }
    let t = first.ok_or("no siege creep within 4810 ticks")?;
    if !(4800..=4803).contains(&t) {
        return Err(format!("first siege creep at tick {t}, want 4800"));
    }
    if most != 3 {
        return Err(format!("{most} siege creeps in the first siege wave, want 3 (one per lane)"));
    }
    println!("moba siege timing ok first_tick={t} per_wave={most}");
    Ok(())
}

/// Destroying the enemy melee barracks makes melee wave creeps spawn as super creeps; once every enemy
/// barracks is gone, melee and ranged wave creeps spawn as mega creeps.
pub fn moba_wave_upgrade(_image: &[u8]) -> Result<(), String> {
    let mut s = fresh(&moba_image()?)?;
    let rax = |s: &Sim, name: &str| -> Vec<usize> { (0..MAX_UNITS).filter(|&i| s.alive(i) && s.player(i) == 1 && s.kind_name(i) == Some(name)).collect() };
    let (melee, ranged) = (rax(&s, "barracks_melee"), rax(&s, "barracks_ranged"));
    if melee.is_empty() || ranged.is_empty() {
        return Err("moba image places no side 1 barracks".into());
    }
    run_ticks(&mut s, 2);
    for &i in &melee {
        s.hurt(i, fixed::from_int(10000));
    }
    run_ticks(&mut s, 480);
    let (sup, mega) = (count_kind(&s, 0, "creep_super"), count_kind(&s, 0, "creep_mega"));
    if sup != 9 || mega != 0 {
        return Err(format!("with the melee barracks down the wave held {sup} super and {mega} mega melee creeps, want 9 and 0"));
    }
    if count_kind(&s, 0, "creep_ranged_super") != 0 {
        return Err("a ranged super creep spawned while the ranged barracks stand".into());
    }
    for &i in &ranged {
        s.hurt(i, fixed::from_int(10000));
    }
    run_ticks(&mut s, 480);
    let (mega, ranged_mega) = (count_kind(&s, 0, "creep_mega"), count_kind(&s, 0, "creep_ranged_mega"));
    if mega != 9 || ranged_mega != 3 {
        return Err(format!("with every barracks down the wave held {mega} mega melee and {ranged_mega} mega ranged creeps, want 9 and 3"));
    }
    println!("moba wave upgrade ok super={sup} mega={mega} ranged_mega={ranged_mega}");
    Ok(())
}

/// Roshan respawns a random 8 to 11 minutes after death, never earlier or later.
pub fn camp_roshan_window(_image: &[u8]) -> Result<(), String> {
    let mut s = fresh(&moba_image()?)?;
    let boss = (0..MAX_UNITS)
        .filter(|&u| s.alive(u) && s.player(u) == 0xFF)
        .max_by_key(|&u| s.hp(u))
        .ok_or("moba image places no neutral unit")?;
    s.hurt(boss, fixed::from_int(10000));
    let died = s.ticks();
    let min = sim::ROSHAN_RESPAWN_MIN_S * sim::TICKS_PER_SECOND as u32;
    let max = min + sim::ROSHAN_RESPAWN_SPREAD_S * sim::TICKS_PER_SECOND as u32;
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
pub fn camp_minute_refill(_image: &[u8]) -> Result<(), String> {
    let mut s = fresh(&moba_image()?)?;
    run_ticks(&mut s, sim::CAMP_FIRST_SPAWN_S * sim::TICKS_PER_SECOND as u32 + 1);
    let u = (0..MAX_UNITS).find(|&u| s.alive(u) && s.player(u) == 0xFF && s.kind_name(u).is_some_and(|n| n.starts_with("neutral_"))).ok_or("moba image places no neutral camp unit")?;
    let (hx, hy) = s.position_fixed(u);
    let reach = fixed::from_milli(sim::CAMP_CLEAR_MILLI) as i64;
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
    let mark = sim::CAMP_RESPAWN_S * sim::TICKS_PER_SECOND as u32;
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
pub fn camp_first_spawn(_image: &[u8]) -> Result<(), String> {
    let mut s = fresh(&moba_image()?)?;
    let regular = |s: &Sim| (0..MAX_UNITS).filter(|&u| s.alive(u) && s.player(u) == 0xFF && s.kind_name(u).is_some_and(|n| n.starts_with("neutral_"))).count();
    let boss = (0..MAX_UNITS).filter(|&u| s.alive(u) && s.player(u) == 0xFF && s.kind_name(u) == Some("roshan")).count();
    if regular(&s) != 0 || boss == 0 {
        return Err(format!("at tick 0 {} camp units live and {boss} bosses, want 0 and at least 1", regular(&s)));
    }
    let first = sim::CAMP_FIRST_SPAWN_S * sim::TICKS_PER_SECOND as u32;
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
pub fn camp_tormentor_timing(_image: &[u8]) -> Result<(), String> {
    let mut s = fresh(&moba_image()?)?;
    let find = |s: &Sim| (0..MAX_UNITS).find(|&u| s.alive(u) && s.kind_name(u) == Some("tormentor"));
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
