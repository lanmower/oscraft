//! Mod-host gate checks. Each writes a Lua mod to the temp directory and drives it through
//! mod_host::ModHost's public API: load, reload_if_changed, tick, advance, set_limits, state.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use mod_host::ModHost;

struct TempMod(PathBuf);

impl Drop for TempMod {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn file_name(name: &str) -> String {
    format!("oscraft-gate-{name}.lua")
}

fn start(image: &[u8], name: &str, src: &str) -> Result<(ModHost, TempMod), String> {
    let path = std::env::temp_dir().join(file_name(name));
    let guard = TempMod(path.clone());
    std::fs::write(&path, src).map_err(|e| format!("write {}: {e}", path.display()))?;
    let mut host = ModHost::new(image).map_err(|e| format!("ModHost::new: {e:?}"))?;
    host.load(&path).map_err(|e| format!("mod load: {e}"))?;
    Ok((host, guard))
}

fn advance(host: &mut ModHost, frames: u32) -> Result<(), String> {
    for _ in 0..frames {
        let state = host.state().clone();
        host.advance(&state).map_err(|e| format!("advance: {e}"))?;
    }
    Ok(())
}

fn read_int(host: &ModHost, key: &str) -> Result<i64, String> {
    host.state()
        .get::<Option<i64>>(key)
        .map_err(|e| format!("state.{key}: {e}"))?
        .ok_or_else(|| format!("mod never set state.{key}"))
}

fn read_bool(host: &ModHost, key: &str) -> Result<bool, String> {
    host.state()
        .get::<Option<bool>>(key)
        .map_err(|e| format!("state.{key}: {e}"))?
        .ok_or_else(|| format!("mod never set state.{key}"))
}

pub fn event_delivery(image: &[u8]) -> Result<(), String> {
    // The attacker is the image's first player 0 unit and the target its first player 1 unit.
    // The sim refuses an attack out of weapon reach, so the mod first walks the attacker to a
    // point one world unit beside the target, then retries order_attack each frame (pcall)
    // until it lands, and keeps attacking until the target dies.
    let src = r#"
local attacker, target, moved = nil, nil, false
local hits, deaths = 0, 0
function on_attacked(unit) hits = hits + 1 end
function on_died(unit) deaths = deaths + 1 end
function on_tick(state)
  if attacker == nil then
    for i = 0, get_unit_count() - 1 do
      if attacker == nil and unit_player(i) == 0 then attacker = i end
      if target == nil and unit_player(i) == 1 then target = i end
    end
  end
  if attacker ~= nil and target ~= nil and unit_alive(attacker) and unit_alive(target) then
    if not moved then
      local tx, ty = unit_position(target)
      pcall(order_unit, attacker, tx - 65536, ty)
      moved = true
    end
    pcall(order_attack, attacker, target)
  end
  state.attacked = hits
  state.died = deaths
end
"#;
    const FRAMES: u32 = 4000;
    let (mut host, _mod) = start(image, "event-delivery", src)?;
    for _ in 0..FRAMES {
        advance(&mut host, 1)?;
        if read_int(&host, "attacked")? > 0 && read_int(&host, "died")? > 0 {
            return Ok(());
        }
    }
    Err(format!(
        "after {FRAMES} frames: on_attacked fired {} times, on_died fired {} times",
        read_int(&host, "attacked")?,
        read_int(&host, "died")?
    ))
}

pub fn lua_load(image: &[u8]) -> Result<(), String> {
    let (mut host, _mod) = start(image, "lua-load", "function on_tick(state)\n  state.loaded = 1\nend\n")?;
    advance(&mut host, 1)?;
    if read_int(&host, "loaded")? != 1 {
        return Err("on_tick did not set state.loaded after load".into());
    }
    Ok(())
}

pub fn lua_reload(image: &[u8]) -> Result<(), String> {
    let (mut host, temp) = start(image, "lua-reload", "function on_tick(state)\n  state.version = 1\nend\n")?;
    advance(&mut host, 1)?;
    std::fs::write(&temp.0, "function on_tick(state)\n  state.version = 2\nend\n")
        .map_err(|e| format!("rewrite mod: {e}"))?;
    // The reload is keyed on mtime, so move it forward explicitly rather than sleeping.
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(&temp.0)
        .map_err(|e| format!("open mod: {e}"))?;
    file.set_modified(SystemTime::now() + Duration::from_secs(120))
        .map_err(|e| format!("set mtime: {e}"))?;
    host.reload_if_changed();
    advance(&mut host, 1)?;
    if read_int(&host, "version")? != 2 {
        return Err("reload_if_changed did not pick up the edited mod".into());
    }
    Ok(())
}

pub fn lua_state(image: &[u8]) -> Result<(), String> {
    let src = "function on_tick(state)\n  state.n = (state.n or 0) + 1\nend\n";
    let (mut host, _mod) = start(image, "lua-state", src)?;
    advance(&mut host, 5)?;
    if read_int(&host, "n")? != 5 {
        return Err(format!("state.n {} after 5 frames, want 5", read_int(&host, "n")?));
    }
    Ok(())
}

pub fn lua_error_location(image: &[u8]) -> Result<(), String> {
    let name = file_name("lua-error-location");
    let src = "function on_tick(state)\n  local x = nil\n  state.v = x.field\nend\n";
    let (host, _mod) = start(image, "lua-error-location", src)?;
    let state = host.state().clone();
    match host.tick(&state) {
        Ok(()) => Err("a runtime error in on_tick was not reported".into()),
        Err(e) => {
            let want = format!("{name}:3:");
            let msg = e.to_string();
            if msg.contains(&want) {
                Ok(())
            } else {
                Err(format!("error lacks {want}: {msg}"))
            }
        }
    }
}

pub fn lua_host_api(image: &[u8]) -> Result<(), String> {
    // Q16.16 coordinates: 1343488 is 20.5 world units, the centre of cell (20,20). Cell (4,4) is
    // player 0's start cell: the map's placements and opening workers fill it to the per-cell limit.
    let src = r#"
local spawned = false
function on_tick(state)
  if not spawned then
    spawn_unit("skirmisher", 1343488, 1343488)
    spawned = true
  end
  state.count = get_unit_count()
end
"#;
    let (mut host, _mod) = start(image, "lua-host-api", src)?;
    let before = host.unit_count() as i64;
    advance(&mut host, 1)?;
    let count = read_int(&host, "count")?;
    if count != before + 1 {
        return Err(format!("get_unit_count {count} after spawn_unit, want {}", before + 1));
    }
    Ok(())
}

pub fn edit_terrain(image: &[u8]) -> Result<(), String> {
    // Cell (10,20) is open in map.osd. Spawn coordinates are Q16.16, so 688128 and 1343488
    // are the centre (10.5, 20.5) of that cell.
    let src = r#"
function on_tick(state)
  if state.step ~= nil then return end
  local before = get_unit_count()
  edit_terrain(10, 20, 10, 20, false)
  spawn_unit("skirmisher", 688128, 1343488)
  state.blocked_delta = get_unit_count() - before
  edit_terrain(10, 20, 10, 20, true)
  spawn_unit("skirmisher", 688128, 1343488)
  state.open_delta = get_unit_count() - before - state.blocked_delta
  state.step = 1
end
"#;
    let (mut host, _mod) = start(image, "edit-terrain", src)?;
    advance(&mut host, 1)?;
    let blocked = read_int(&host, "blocked_delta")?;
    let open = read_int(&host, "open_delta")?;
    if blocked != 0 {
        return Err(format!("a unit spawned on a cell edit_terrain had blocked (delta {blocked})"));
    }
    if open != 1 {
        return Err(format!("no unit spawned on the reopened cell (delta {open})"));
    }
    Ok(())
}

pub fn lua_host_api_full(image: &[u8]) -> Result<(), String> {
    // map.osd spawns unit 0 as player 0's skirmisher and unit 2 as player 1's skirmisher.
    // Damage and buff values are Q16.16, so 655360 is 10 world units of damage.
    let src = r#"
function on_tick(state)
  if state.done then return end
  state.owner = unit_player(2)
  state.hp0 = unit_hp(2)
  damage_unit(0, 2, 655360)
  state.hp1 = unit_hp(2)
  state.buffed = buff_unit(2, 1, 5)
  state.done = true
end
"#;
    let (mut host, _mod) = start(image, "lua-host-api-full", src)?;
    advance(&mut host, 1)?;
    let owner = read_int(&host, "owner")?;
    if owner != 1 {
        return Err(format!("unit 2 is owned by player {owner}, want 1: map.osd placement order changed"));
    }
    let lost = read_int(&host, "hp0")? - read_int(&host, "hp1")?;
    if lost != 655360 {
        return Err(format!("damage_unit removed {lost} raw Q16.16 from unit 2, want 655360"));
    }
    if !read_bool(&host, "buffed")? {
        return Err("buff_unit returned false on a live unit with free slots".into());
    }
    Ok(())
}

pub fn lua_sandbox_limits(image: &[u8]) -> Result<(), String> {
    let src = "function on_tick(state)\n  local n = 0\n  while true do n = n + 1 end\nend\n";
    let (mut host, _mod) = start(image, "lua-sandbox-limits", src)?;
    host.set_limits(100_000, 16 << 20).map_err(|e| format!("set_limits: {e}"))?;
    let state = host.state().clone();
    match host.tick(&state) {
        Ok(()) => Err("an unbounded loop returned".into()),
        Err(e) if e.to_string().contains("instruction_limit") => Ok(()),
        Err(e) => Err(format!("stopped with: {e}")),
    }
}

pub fn region_define(image: &[u8]) -> Result<(), String> {
    let src = r#"
define_region("zone", 0, 0, 655360, 655360)
function on_tick(state)
  state.inside = in_region("zone", 327680, 327680)
  state.outside = in_region("zone", 1310720, 327680)
end
"#;
    let (mut host, _mod) = start(image, "region-define", src)?;
    advance(&mut host, 1)?;
    if !read_bool(&host, "inside")? {
        return Err("a point inside the 10x10 region reads outside".into());
    }
    if read_bool(&host, "outside")? {
        return Err("a point outside the region reads inside".into());
    }
    Ok(())
}

pub fn trigger_zone(image: &[u8]) -> Result<(), String> {
    // The unit starts inside the zone, then the mod orders it out on the second frame.
    let src = r#"
local entered, left, moves = 0, 0, 0
define_region("zone", 0, 0, 655360, 655360)
spawn_unit("skirmisher", 327680, 327680)
local me = get_unit_count() - 1
function on_enter(name, unit) if unit == me then entered = entered + 1 end end
function on_leave(name, unit) if unit == me then left = left + 1 end end
function on_tick(state)
  moves = moves + 1
  if moves == 2 then order_unit(me, 1310720, 327680) end
  state.entered = entered
  state.left = left
end
"#;
    let (mut host, _mod) = start(image, "trigger-zone", src)?;
    advance(&mut host, 40)?;
    let entered = read_int(&host, "entered")?;
    let left = read_int(&host, "left")?;
    if entered < 1 {
        return Err("on_enter never fired for a unit spawned inside the zone".into());
    }
    if left < 1 {
        return Err("on_leave never fired after the unit was ordered out of the zone".into());
    }
    Ok(())
}

pub fn scheduled_action(image: &[u8]) -> Result<(), String> {
    let src = r#"
fired = 0
schedule("a", 3, function() fired = fired + 1 end)
schedule("b", 2, function() fired = fired + 100 end)
cancelled = cancel("b")
function on_tick(state)
  state.fired = fired
  state.cancelled = cancelled
end
"#;
    let (mut host, _mod) = start(image, "scheduled-action", src)?;
    advance(&mut host, 3)?;
    if !read_bool(&host, "cancelled")? {
        return Err("cancel of a pending timer returned false".into());
    }
    let fired = read_int(&host, "fired")?;
    if fired != 1 {
        return Err(format!("fired {fired} after 3 frames, want 1 (a fires, b was cancelled)"));
    }
    Ok(())
}
