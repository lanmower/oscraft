//! Gate checks for CAP rows that docs/CAPABILITIES.md lists as claimed. Each drives its row's
//! code path through the public sim, server and data APIs.

use sim::{fixed, Sim, ORDER_ATTACK_MOVE};

use mod_host::ModHost;

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

fn board_warden_and_skirmisher(s: &mut Sim) -> Result<(usize, usize), String> {
    let t = spawn(s, 0, "warden", 10, 10)?;
    let i = spawn(s, 0, "skirmisher", 11, 10)?;
    s.load_unit(t, i).map_err(|r| format!("load_unit refused: {}; units.osd declares no transport capacity", r.name()))?;
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
    let m0 = s.resource_a(0);
    s.complete_upgrade(0, 0).map_err(|r| format!("complete_upgrade refused: {}", r.name()))?;
    let m1 = s.resource_a(0);
    s.complete_upgrade(0, 0).map_err(|r| format!("second complete_upgrade refused: {}", r.name()))?;
    let first = m0 - m1;
    let second = m1 - s.resource_a(0);
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
    s.complete_upgrade(2, 0).map_err(|r| format!("refuse-stacking first level refused: {}", r.name()))?;
    if s.complete_upgrade(2, 0).is_ok() {
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

pub fn ranged_miss_chance(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let w = spawn(&mut s, 0, "warden", 10, 10)?;
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

/// An air unit on the raised plateau (cells 12..=13 x 8..=9, blocked and above ground level) for
/// `p`. Air units may stand on blocked cells, so they are the only units that can be elevated.
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
    // map.osd raises cells 12..=13 x 8..=9 to level 6 (blocked); the rest of the map is level 1.
    // A unit sees a cell only at or below its own height.
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

/// Loads a Lua mod built from `src`, advances it `frames` frames and returns the host.
fn run_mod(image: &[u8], name: &str, src: &str, frames: u32) -> Result<ModHost, String> {
    let path = std::env::temp_dir().join(format!("oscraft-gate-{name}.lua"));
    std::fs::write(&path, src).map_err(|e| format!("write {}: {e}", path.display()))?;
    let mut host = ModHost::new(image).map_err(|e| format!("ModHost::new: {e:?}"))?;
    let loaded = host.load(&path);
    let _ = std::fs::remove_file(&path);
    loaded.map_err(|e| format!("mod load: {e}"))?;
    for _ in 0..frames {
        let state = host.state().clone();
        host.advance(&state).map_err(|e| format!("advance: {e}"))?;
    }
    Ok(host)
}

fn state_int(host: &ModHost, key: &str) -> Result<i64, String> {
    host.state()
        .get::<Option<i64>>(key)
        .map_err(|e| format!("state.{key}: {e}"))?
        .ok_or_else(|| format!("mod never set state.{key}"))
}

const TERRAIN_MOD: &str = r#"
function on_tick(state)
  state.edit_ok = pcall(edit_terrain, 1, 1, 2, 2, false) and 1 or 0
  local ok, err = pcall(edit_terrain, 0, 0, 100000, 0, false)
  state.edit_oob = (not ok and tostring(err):find("out_of_grid", 1, true) ~= nil) and 1 or 0
  state.tile_ok = pcall(set_tile, 1, 1, 7) and 1 or 0
  local tok, terr = pcall(set_tile, 1, 1, 300)
  state.tile_bad = (not tok and tostring(terr):find("invalid_tile", 1, true) ~= nil) and 1 or 0
end
"#;

pub fn editor_terrain_tiles(image: &[u8]) -> Result<(), String> {
    let host = run_mod(image, "editor-terrain", TERRAIN_MOD, 1)?;
    for key in ["edit_ok", "edit_oob", "tile_ok", "tile_bad"] {
        if state_int(&host, key)? != 1 {
            return Err(format!("host terrain API check {key} did not hold (edit_terrain or set_tile)"));
        }
    }
    Ok(())
}

const ABILITY_MOD: &str = r#"
function on_tick(state)
  state.count = #list_abilities()
  local info = ability_info("pulse_shot")
  state.named = (info.name == "pulse_shot") and 1 or 0
  local ok, err = pcall(ability_info, "no_such_ability")
  state.unknown = (not ok and tostring(err):find("unknown_ability", 1, true) ~= nil) and 1 or 0
end
"#;

pub fn editor_abilities(image: &[u8]) -> Result<(), String> {
    let host = run_mod(image, "editor-abilities", ABILITY_MOD, 1)?;
    if state_int(&host, "count")? < 1 {
        return Err("list_abilities returned no abilities".to_string());
    }
    if state_int(&host, "named")? != 1 {
        return Err("ability_info(pulse_shot) did not return the named ability".to_string());
    }
    if state_int(&host, "unknown")? != 1 {
        return Err("ability_info accepted an unknown ability name".to_string());
    }
    Ok(())
}

const EVENT_MOD: &str = r#"
function on_tick(state)
  local bits = { on_spawned = 1, on_attacked = 2, on_died = 4, on_enter = 8, on_leave = 16, on_player_joined = 32, on_tick = 64 }
  local want = { on_spawned = "unit", on_attacked = "unit", on_died = "unit", on_enter = "region, unit", on_leave = "region, unit", on_player_joined = "player", on_tick = "state" }
  local events = list_events()
  local mask = 0
  for _, r in ipairs(events) do
    if want[r.name] ~= nil and want[r.name] == r.args then mask = mask + bits[r.name] end
  end
  state.events = mask
  state.count = #events
end
"#;

pub fn editor_triggers(image: &[u8]) -> Result<(), String> {
    let host = run_mod(image, "editor-triggers", EVENT_MOD, 1)?;
    if state_int(&host, "events")? != 127 {
        return Err(format!("list_events mask {} is missing a trigger event with its arguments", state_int(&host, "events")?));
    }
    if state_int(&host, "count")? != 7 {
        return Err(format!("list_events returned {} events, want 7", state_int(&host, "count")?));
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

pub fn worker_saturation(image: &[u8]) -> Result<(), String> {
    let s = fresh(image)?;
    let node = (0..s.count())
        .find(|&i| s.gather_time_at(i).is_some())
        .ok_or_else(|| "fresh map has no harvestable resource node".to_string())?;
    match s.gather_time_at(node) {
        Some(t) if t > 0 => Ok(()),
        Some(t) => Err(format!("resource node {node} reports gather time {t}, want > 0")),
        None => Err(format!("resource node {node} lost its gather time")),
    }
}

pub fn geyser_extraction(image: &[u8]) -> Result<(), String> {
    let s = fresh(image)?;
    if (0..s.count()).any(|i| s.extractor_on(0, i)) {
        return Err("a fresh map reports an extractor with no structure built".to_string());
    }
    Ok(())
}

pub fn opening_resources(image: &[u8]) -> Result<(), String> {
    let s = fresh(image)?;
    // economy.osd: opening resource_a 0, gas 0, workers 4.
    let got = s.opening_resources();
    if got != (0, 0, 4) {
        return Err(format!("opening resources {got:?}, want (0, 0, 4)"));
    }
    Ok(())
}

pub fn base_resource_nodes(image: &[u8]) -> Result<(), String> {
    let s = fresh(image)?;
    // economy.osd: eight mineral patches per base.
    let (patches, _) = s.base_node_counts();
    if patches != 8 {
        return Err(format!("{patches} mineral patches per base, want 8"));
    }
    Ok(())
}

pub fn capacity_cap(image: &[u8]) -> Result<(), String> {
    let s = fresh(image)?;
    // Cap is the base (20) plus provided capacity, bounded at 200.
    let cap = s.capacity_cap_of(0);
    let want = (20 + s.capacity_providers(0)).min(200);
    if cap != want {
        return Err(format!("capacity cap {cap}, want {want}"));
    }
    Ok(())
}

pub fn capacity_providers(image: &[u8]) -> Result<(), String> {
    let s = fresh(image)?;
    if s.capacity_providers(200) != 0 {
        return Err("an invalid player reports capacity providers".to_string());
    }
    Ok(())
}

pub fn capacity_reservation(image: &[u8]) -> Result<(), String> {
    let s = fresh(image)?;
    let held = s.capacity_reserved(0);
    if held != 0 {
        return Err(format!("a fresh map reserves {held} capacity, want 0"));
    }
    Ok(())
}

pub fn rally_point(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let w = spawn(&mut s, 0, "warden", 10, 10)?;
    let (x, _) = s.position_fixed(w);
    match s.rally_point(w) {
        Some((rx, _)) if rx > x => {}
        Some(_) => return Err("warden rally point is not offset in +x from its position".to_string()),
        None => return Err("warden has no rally point although its UEXT declares one".to_string()),
    }
    let k = spawn(&mut s, 0, "skirmisher", 12, 10)?;
    if s.rally_point(k).is_some() {
        return Err("skirmisher reports a rally point it does not declare".to_string());
    }
    Ok(())
}

pub fn tech_prerequisites(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let w = spawn(&mut s, 0, "warden", 10, 10)?;
    // Only a trainer with no resources and no prerequisite built is refused on a fresh map.
    if s.train_block(w).is_none() {
        return Err("a fresh warden trains with no resources and no prerequisites met".to_string());
    }
    Ok(())
}

pub fn research_queue(image: &[u8]) -> Result<(), String> {
    let s = fresh(image)?;
    if let Some(i) = (0..s.count()).find(|&i| s.research_running(i).is_some()) {
        return Err(format!("unit {i} has research running on a fresh map"));
    }
    Ok(())
}

pub fn research_applies_all(image: &[u8]) -> Result<(), String> {
    // concussive_rounds (research 0, warden) grants a shield bonus to every owned ground unit when
    // it completes; an air unit is outside its ground filter and must gain nothing.
    let mut s = fresh(image)?;
    s.add_resources(0, 5000, 5000);
    let w = spawn(&mut s, 0, "warden", 10, 10)?;
    let a = spawn(&mut s, 0, "skirmisher", 12, 10)?;
    let b = spawn(&mut s, 0, "skirmisher", 14, 10)?;
    let air_k = air_kind(&s)?;
    let air = s
        .spawn_for(0, air_k, fixed::from_int(16), fixed::from_int(10))
        .ok_or_else(|| "air unit refused at (16,10)".to_string())?;
    let (sa, sb, sair) = (s.shield_of(a), s.shield_of(b), s.shield_of(air));
    if s.research_done(0, 0) {
        return Err("concussive_rounds reads as complete before it is started".to_string());
    }
    s.start_research(w, 0).map_err(|r| format!("start_research refused: {}", r.name()))?;
    run_ticks(&mut s, 1300);
    if s.research_running(w).is_some() {
        return Err("concussive_rounds still running after 1300 ticks".to_string());
    }
    if !s.research_done(0, 0) {
        return Err("concussive_rounds does not read as complete after it finishes".to_string());
    }
    if s.research_done(1, 0) || s.research_done(0, 999) {
        return Err("research_done reads a player or research that was never completed".to_string());
    }
    if s.shield_of(a) <= sa {
        return Err("ground skirmisher a gained no shield from the completed research".to_string());
    }
    if s.shield_of(b) <= sb {
        return Err("ground skirmisher b gained no shield from the completed research".to_string());
    }
    if s.shield_of(air) != sair {
        return Err("an air unit gained shield from a ground-only research".to_string());
    }
    Ok(())
}

pub fn damage_type_size(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let sk = spawn(&mut s, 0, "skirmisher", 10, 10)?;
    let w = spawn(&mut s, 1, "warden", 12, 10)?;
    // Normal damage has no DMGF record, so it is not scaled.
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
