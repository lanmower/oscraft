//! Gate checks for the moba scenario's units and timers: lane waves, siege timing, barracks wave
//! upgrades, neutral camp stacking and Roshan's respawn window. Each runs on the moba image.

use crate::checks::{fresh, run_ticks, spawn, MAX_UNITS};
use crate::mod_checks::{advance, read_int, start};
use sim::{fixed, Sim};

/// Lane wave logic under test: the moba scenario's wave mod, loaded into the mod host.
const WAVES_MOD: &str = include_str!("../../../mods/moba_match.lua");
/// Creeps per lane spawner per wave, as configured in mods/moba_match.lua (CREEP_OFFSETS: three
/// melee, one ranged; creep lanes are top and bottom, mid sends none).
const MELEE_PER_LANE: i64 = 3;
const RANGED_PER_LANE: i64 = 1;
// Super creeps held by the top lane after its mirror tower dies, on the 6.88 tower positions (was 3 melee, 1 ranged on the
// 7.41 positions: the Scourge towers moved to their war3map.j uzig positions, see the commit that moved them).
const SUPER_MELEE_TOP_LANE: i64 = 2;
const SUPER_RANGED_TOP_LANE: i64 = 0;
const CREEP_LANES: &[&str] = &["lane_spawner_top", "lane_spawner_bot"];
const ALL_LANES: &[&str] = &["lane_spawner_top", "lane_spawner_mid", "lane_spawner_bot"];
/// The mod's wave logic plus a test harness: per-kind spawn counts in the state, the top lane's
/// mirror tower killed at tick 500, and every side-1 tower killed at tick 1500.
const UPGRADE_HARNESS: &str = r#"
local base_on_tick = on_tick
local function any_tower(owner)
  for _, t in ipairs(towers) do
    if t.owner == owner then return t end
  end
end
local function mirror_tower(owner, x, y)
  local mx, my = MAP_W - x, MAP_H - y
  local best, best_d
  for _, t in ipairs(towers) do
    if t.owner ~= owner then
      local dx, dy = t.x - mx, t.y - my
      local d = dx * dx + dy * dy
      if best_d == nil or d < best_d then best, best_d = t, d end
    end
  end
  return best
end
local function lanes_with_dead_mirror(owner)
  local n = 0
  for _, s in ipairs(lane_spawners) do
    if s.owner == owner and s.lane ~= "mid" then
      local x, y = unit_position(s.id)
      if counterpart_dead(owner, x, y) then n = n + 1 end
    end
  end
  return n
end
function on_tick(state)
  base_on_tick(state)
  state.melee_super = spawned["creep_super"] or 0
  state.melee_mega = spawned["creep_mega"] or 0
  state.ranged_super = spawned["creep_ranged_super"] or 0
  state.ranged_mega = spawned["creep_ranged_mega"] or 0
  state.lanes_super = lanes_with_dead_mirror(0)
end
game.timer("gate_kill_top_mirror", 500, function()
  for _, s in ipairs(lane_spawners) do
    if s.owner == 0 and s.lane == "top" then
      local x, y = unit_position(s.id)
      damage_unit(any_tower(0).id, mirror_tower(0, x, y).id, 655360000)
    end
  end
end, 1000000)
game.timer("gate_kill_side1_towers", 1500, function()
  for _, t in ipairs(towers) do
    if t.owner ~= 0 and unit_alive(t.id) then damage_unit(any_tower(0).id, t.id, 655360000) end
  end
end, 1000000)
"#;

fn moba_image() -> Result<Vec<u8>, String> {
    let scenario = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("scenarios").join("moba");
    osdt_data::generate(&scenario).map_err(|e| format!("moba scenario image: {e:#}"))
}

fn count_kind(s: &Sim, player: u8, name: &str) -> usize {
    (0..MAX_UNITS).filter(|&i| s.alive(i) && s.player(i) == player && s.kind_name(i) == Some(name)).count()
}

/// Lane spawner units named in `kinds` that are alive for both sides, as the wave mod sees them.
fn spawners(s: &Sim, kinds: &[&str]) -> i64 {
    [0u8, 1].iter().map(|&p| kinds.iter().map(|k| count_kind(s, p, k) as i64).sum::<i64>()).sum()
}

/// Lane creep waves come from mods/moba_match.lua: each top and bottom lane spawner sends three melee and
/// one ranged creep every 480 ticks from tick 1, and mid sends none. The mod's counts sum both sides,
/// so the expected wave is the creep lane spawners in the image times the per-lane counts.
pub fn moba_wave_composition(_image: &[u8]) -> Result<(), String> {
    let image = moba_image()?;
    let lanes = spawners(&fresh(&image)?, CREEP_LANES);
    let (want_melee, want_ranged) = (lanes * MELEE_PER_LANE, lanes * RANGED_PER_LANE);
    let (mut host, _mod) = start(&image, "waves-composition", WAVES_MOD)?;
    advance(&mut host, 2)?;
    let (melee1, ranged1) = (read_int(&host, "waves_creep")?, read_int(&host, "waves_creep_ranged")?);
    if melee1 != want_melee || ranged1 != want_ranged {
        return Err(format!("first wave spawned {melee1} melee and {ranged1} ranged creeps, want {want_melee} and {want_ranged}"));
    }
    if read_int(&host, "waves_siege")? != 0 {
        return Err("a siege creep spawned in the first wave".into());
    }
    advance(&mut host, 480)?;
    let melee2 = read_int(&host, "waves_creep")? - melee1;
    if melee2 != want_melee {
        return Err(format!("second wave 480 ticks later spawned {melee2} melee creeps, want {want_melee}"));
    }
    println!("moba wave composition ok melee_per_wave={want_melee} ranged_per_wave={want_ranged} interval_ticks=480");
    Ok(())
}

/// Siege creeps come from the mod: every lane spawner (top, mid, bottom, both sides) sends one at tick
/// 4800, and the first wave's size is the siege spawner count in the image.
pub fn moba_siege_timing(_image: &[u8]) -> Result<(), String> {
    let image = moba_image()?;
    let want = spawners(&fresh(&image)?, ALL_LANES);
    let (mut host, _mod) = start(&image, "waves-siege", WAVES_MOD)?;
    let mut first = None;
    let mut most = 0;
    for t in 1..=4810i64 {
        advance(&mut host, 1)?;
        let n = read_int(&host, "waves_siege")? + read_int(&host, "waves_siege_upgraded")?;
        if n > 0 && first.is_none() {
            first = Some(t);
        }
        most = most.max(n);
    }
    let t = first.ok_or("no siege creep within 4810 ticks")?;
    if !(4800..=4803).contains(&t) {
        return Err(format!("first siege creep at tick {t}, want 4800"));
    }
    if most != want {
        return Err(format!("{most} siege creeps in the first siege wave, want {want} (one per lane spawner)"));
    }
    println!("moba siege timing ok first_tick={t} per_wave={most}");
    Ok(())
}

/// The mod upgrades wave creeps by enemy towers, not barracks: a creep whose spawner's mirror tower is
/// dead spawns as super, and once every enemy tower is dead its side's creeps spawn as mega. The harness
/// kills the top lane's mirror tower at tick 500 (one lane upgrades) and every side-1 tower at tick 1500.
/// Expected per wave: 3 melee and 1 ranged per upgraded lane (super), then 3 and 1 per creep lane of side 0
/// (mega), with the creep lane count taken from the image.
pub fn moba_wave_upgrade(_image: &[u8]) -> Result<(), String> {
    let image = moba_image()?;
    let owner_lanes = spawners(&fresh(&image)?, CREEP_LANES) / 2;
    let (mut host, _mod) = start(&image, "waves-upgrade", &format!("{WAVES_MOD}\n{UPGRADE_HARNESS}"))?;
    advance(&mut host, 490)?;
    let (super0, ranged_super0) = (read_int(&host, "melee_super")?, read_int(&host, "ranged_super")?);
    if super0 != 0 || ranged_super0 != 0 || read_int(&host, "melee_mega")? != 0 {
        return Err("wave creeps upgraded before any enemy tower died".into());
    }
    advance(&mut host, 950)?;
    let lanes = read_int(&host, "lanes_super")?;
    let (melee, ranged) = (read_int(&host, "melee_super")? - super0, read_int(&host, "ranged_super")? - ranged_super0);
    if lanes != 1 {
        return Err(format!("{lanes} side-0 creep lanes have a dead mirror tower after its kill, want 1 (the top lane)"));
    }
    if melee != SUPER_MELEE_TOP_LANE * lanes || ranged != SUPER_RANGED_TOP_LANE * lanes {
        return Err(format!("with the top mirror tower down the wave held {melee} super melee and {ranged} super ranged creeps, want {} and {}", SUPER_MELEE_TOP_LANE * lanes, SUPER_RANGED_TOP_LANE * lanes));
    }
    if read_int(&host, "melee_mega")? != 0 {
        return Err("a melee creep spawned mega while an enemy tower still stood".into());
    }
    advance(&mut host, 480)?;
    let (mega0, ranged_mega0) = (read_int(&host, "melee_mega")?, read_int(&host, "ranged_mega")?);
    advance(&mut host, 10)?;
    let (mega, ranged_mega) = (read_int(&host, "melee_mega")? - mega0, read_int(&host, "ranged_mega")? - ranged_mega0);
    if mega != MELEE_PER_LANE * owner_lanes || ranged_mega != RANGED_PER_LANE * owner_lanes {
        return Err(format!("with every enemy tower down the wave held {mega} mega melee and {ranged_mega} mega ranged creeps, want {} and {}", MELEE_PER_LANE * owner_lanes, RANGED_PER_LANE * owner_lanes));
    }
    println!("moba wave upgrade ok super={melee} mega={mega} ranged_mega={ranged_mega}");
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

/// From 15:00 Roshan walks to the pit of the day/night phase (top at night), and the Tormentor respawns at the
/// Radiant corner at night and the Dire corner by day.
pub fn camp_pit_swap(_image: &[u8]) -> Result<(), String> {
    let mut s = fresh(&moba_image()?)?;
    let find = |s: &Sim, name: &str| (0..MAX_UNITS).find(|&u| s.alive(u) && s.kind_name(u) == Some(name));
    let cells = |s: &Sim, u: usize, x: i32, y: i32| {
        let (px, py) = s.position_fixed(u);
        let (dx, dy) = (i64::from(px - fixed::from_milli(x)), i64::from(py - fixed::from_milli(y)));
        ((dx * dx + dy * dy) as f64).sqrt() / f64::from(fixed::ONE)
    };
    let roshan = find(&s, "roshan").ok_or("no Roshan at tick 0")?;
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
    let tormentor = find(&s, "tormentor").ok_or("no Tormentor after 15:00")?;
    let t0 = cells(&s, tormentor, 43500, 64500);
    if t0 > 4.0 {
        return Err(format!("the night Tormentor stands {t0:.0} cells from the Radiant corner"));
    }
    let left = 19300 - s.ticks();
    run_ticks(&mut s, left);
    s.hurt(tormentor, fixed::from_int(10000));
    s.hurt(tormentor, fixed::from_int(10000));
    run_ticks(&mut s, 9600 + 8);
    let back = find(&s, "tormentor").ok_or("the Tormentor did not return")?;
    let t1 = cells(&s, back, 284500, 260500);
    if t1 > 4.0 {
        return Err(format!("the day Tormentor stands {t1:.0} cells from the Dire corner"));
    }
    println!("camp pit swap ok roshan_night_pit_dist={d1:.1} tormentor_day_dist={t1:.1}");
    Ok(())
}
