use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use mod_host::ModHost;

use crate::checks::{fresh, kind};

pub(crate) struct TempMod(PathBuf);

impl Drop for TempMod {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn file_name(name: &str) -> String {
    format!("oscraft-gate-{name}.lua")
}

pub(crate) fn start(image: &[u8], name: &str, src: &str) -> Result<(ModHost, TempMod), String> {
    let path = std::env::temp_dir().join(file_name(name));
    let guard = TempMod(path.clone());
    std::fs::write(&path, src).map_err(|e| format!("write {}: {e}", path.display()))?;
    let mut host = ModHost::new(image).map_err(|e| format!("ModHost::new: {e:?}"))?;
    host.load_file(&path).map_err(|e| format!("mod load: {e}"))?;
    Ok((host, guard))
}

pub(crate) fn advance(host: &mut ModHost, frames: u32) -> Result<(), String> {
    for _ in 0..frames {
        host.advance().map_err(|e| format!("advance: {e}"))?;
    }
    Ok(())
}

pub(crate) fn read_int(host: &ModHost, key: &str) -> Result<i64, String> {
    host.state_int(key).ok_or_else(|| format!("mod never set state.{key}"))
}

fn hook_disabled(host: &ModHost, name: &str, hook: &str) -> bool {
    let want = format!("{}:{hook}", file_name(name).trim_end_matches(".lua"));
    host.disabled_hooks().contains(&want)
}

fn read_bool(host: &ModHost, key: &str) -> Result<bool, String> {
    host.state_bool(key).ok_or_else(|| format!("mod never set state.{key}"))
}

pub fn event_delivery(image: &[u8]) -> Result<(), String> {
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
      order_unit(attacker, tx - 65536, ty)
      moved = true
    end
    game.cmd_strike(attacker, target)
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
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(&temp.0)
        .map_err(|e| format!("open mod: {e}"))?;
    file.set_modified(SystemTime::now() + Duration::from_secs(120))
        .map_err(|e| format!("set mtime: {e}"))?;
    if !host.reload().map_err(|e| format!("reload: {e}"))? {
        return Err("reload did not see the edited mod".into());
    }
    advance(&mut host, 1)?;
    if read_int(&host, "version")? != 2 {
        return Err("reload did not pick up the edited mod".into());
    }
    Ok(())
}

pub fn lua_missing_args_nil(image: &[u8]) -> Result<(), String> {
    let src = r#"
local function probe(a, b, c, d)
  local n = 0
  if b ~= nil then n = n + 1 end
  if c ~= nil then n = n + 2 end
  if d ~= nil then n = n + 4 end
  return n
end
function later(a, b, c, d)
  return probe(a, b, c, d)
end
function on_tick(state)
  local t = { "s1", "s2", "s3", "s4", "s5", "s6" }
  t = nil
  state.one = probe("x")
  local u = { "s1", "s2", "s3", "s4", "s5", "s6" }
  u = nil
  state.two = probe("x", "y")
  local v = { "s1", "s2", "s3", "s4", "s5", "s6" }
  v = nil
  state.global_two = later("x", "y")
  state.none = probe()
end
"#;
    let (mut host, _mod) = start(image, "lua-missing-args", src)?;
    advance(&mut host, 1)?;
    for (key, want) in [("one", 0), ("two", 1), ("global_two", 1), ("none", 0)] {
        let got = read_int(&host, key)?;
        if got != want {
            return Err(format!("state.{key} = {got}, want {want}: a missing argument was not nil"));
        }
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
    let src = "function on_tick(state)\n  local x = nil\n  state.v = x.field\nend\n";
    let (mut host, _mod) = start(image, "lua-error-location", src)?;
    host.advance().map_err(|e| format!("advance returned the hook error: {e}"))?;
    if !hook_disabled(&host, "lua-error-location", "on_tick") {
        return Err(format!("a runtime error in on_tick did not switch the hook off: {:?}", host.disabled_hooks()));
    }
    let want = "oscraft-gate-lua-error-location:on_tick: runtime error: type error, expected table, found nil";
    if host.hook_errors().iter().any(|e| e == want) {
        Ok(())
    } else {
        Err(format!("error lacks {want}: {:?}", host.hook_errors()))
    }
}

pub fn lua_host_api(image: &[u8]) -> Result<(), String> {
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
    let before = host.units(None).len() as i64;
    advance(&mut host, 1)?;
    let count = read_int(&host, "count")?;
    if count != before + 1 {
        return Err(format!("get_unit_count {count} after spawn_unit, want {}", before + 1));
    }
    Ok(())
}

pub fn edit_terrain(image: &[u8]) -> Result<(), String> {
    let mut s = fresh(image)?;
    let k = kind(&s, "skirmisher")?;
    s.set_cell_blocked(10, 20, true).map_err(|r| format!("set_cell_blocked refused: {}", r.name()))?;
    if s.spawn_for(0, k, 688128, 1343488).is_some() {
        return Err("a unit spawned on a cell set_cell_blocked had blocked".into());
    }
    s.set_cell_blocked(10, 20, false).map_err(|r| format!("set_cell_blocked refused: {}", r.name()))?;
    if s.spawn_for(0, k, 688128, 1343488).is_none() {
        return Err("no unit spawned on the reopened cell".into());
    }
    Ok(())
}

pub fn lua_seat_api(image: &[u8]) -> Result<(), String> {
    let src = r#"
local left_seen
function on_player_left(player) left_seen = player end
function on_tick(state)
  if left_seen then state.left = left_seen end
  if state.done then return end
  state.done = 1
  state.default_name = game.player_name(0) == "Player 1" and 1 or 0
  game.player_name(0, "Anna")
  game.player_colour(0, 0x0042ff)
  game.setting(0, "show_deny", 1)
  state.name = game.player_name(0) == "Anna" and 1 or 0
  state.colour = game.player_colour(0) == 0x0042ff and 1 or 0
  state.setting = game.setting(0, "show_deny")
  state.unset = game.setting(0, "never_set") == nil and 1 or 0
  local mates = 0
  for p = 0, 15 do
    if game.team(p) ~= nil and game.team(p) == game.team(0) then mates = mates + 1 end
  end
  state.mates = mates
  game.message(-1, "ally text", 5, 0)
end
"#;
    let (mut host, _mod) = start(image, "lua-seat-api", src)?;
    advance(&mut host, 2)?;
    for (key, want) in [("default_name", 1), ("name", 1), ("colour", 1), ("setting", 1), ("unset", 1)] {
        let got = read_int(&host, key)?;
        if got != want {
            return Err(format!("state.{key} = {got}, want {want}: {:?}", host.hook_errors()));
        }
    }
    let mates = read_int(&host, "mates")?;
    let ui = host.take_ui();
    let messages = ui.iter().filter(|b| b.contains(r#""kind":"message""#)).count() as i64;
    if mates < 1 || messages != mates {
        return Err(format!("allies-only message reached {messages} seats, want the {mates} on player 0's team: {ui:?}"));
    }
    for kind in ["seat", "setting"] {
        if !ui.iter().any(|b| b.contains(&format!(r#""kind":"{kind}""#))) {
            return Err(format!("no {kind} UI event was queued: {ui:?}"));
        }
    }
    host.dispatch_action("peer_left", &serde_json::json!({"player": 3, "pubkey": "x"})).map_err(|e| format!("dispatch_action: {e}"))?;
    advance(&mut host, 1)?;
    let left = read_int(&host, "left")?;
    if left != 3 {
        return Err(format!("on_player_left got player {left}, want 3: {:?}", host.hook_errors()));
    }
    Ok(())
}

pub fn lua_editor_api(image: &[u8]) -> Result<(), String> {
    let src = r#"
function on_tick(state)
  if state.done then return end
  state.done = 1
  state.before = game.walkable(10 * 65536 + 1, 20 * 65536 + 1) and 1 or 0
  game.edit_terrain(10, 20, true)
  game.set_tile(10, 20, 7)
  state.abilities = #game.list_abilities() > 0 and 1 or 0
  local first = game.list_abilities()[1]
  state.named = (first ~= nil and #game.list_abilities(first.name) >= 1) and 1 or 0
  state.events = #game.list_events()
  local ok = pcall(game.edit_terrain, -1, 0, true)
  state.outside = ok and 0 or 1
end
"#;
    let (mut host, _mod) = start(image, "lua-editor-api", src)?;
    advance(&mut host, 2)?;
    for (key, want) in [("before", 1), ("abilities", 1), ("named", 1), ("outside", 1)] {
        let got = read_int(&host, key)?;
        if got != want {
            return Err(format!("state.{key} = {got}, want {want}: {:?}", host.hook_errors()));
        }
    }
    let events = read_int(&host, "events")?;
    if events != mod_host::EVENTS.len() as i64 {
        return Err(format!("game.list_events returned {events} rows, want {}", mod_host::EVENTS.len()));
    }
    let sim = host.sim();
    if sim.grid().walkable(10, 20) {
        return Err("game.edit_terrain left cell (10,20) walkable".into());
    }
    if sim.grid().tile(10, 20) != Some(7) {
        return Err(format!("game.set_tile left tile {:?}, want 7", sim.grid().tile(10, 20)));
    }
    Ok(())
}

pub fn lua_scaling_api(image: &[u8]) -> Result<(), String> {
    let src = r#"
function on_tick(state)
  if state.done then return end
  state.done = 1
  state.rate_before = game.xp_rate(0)
  state.scale_before = game.unit_scale(0)
  game.set_xp_rate(0, 150)
  game.set_unit_scale(0, 50)
  game.cmd_add_xp(0, 100)
  state.refused = pcall(game.set_unit_scale, 0, 5000) and 0 or 1
end
"#;
    let (mut host, _mod) = start(image, "lua-scaling-api", src)?;
    advance(&mut host, 2)?;
    for (key, want) in [("rate_before", 100), ("scale_before", 100), ("refused", 1)] {
        let got = read_int(&host, key)?;
        if got != want {
            return Err(format!("state.{key} = {got}, want {want}: {:?}", host.hook_errors()));
        }
    }
    let (xp, scale, rate) = {
        let sim = host.sim();
        (sim.xp(0), sim.unit_scale(0), sim.xp_rate(0))
    };
    if (xp, scale, rate) != (150, 50, 150) {
        return Err(format!("xp {xp}, scale {scale}, rate {rate}: want 150, 50, 150 (a 100 xp award at a 150 percent rate)"));
    }
    let hash = host.sim().state_hash();
    let saved = host.sim().save_state();
    host.load_state(&saved).map_err(|e| format!("load_state: {e}"))?;
    let (scale, rate) = {
        let sim = host.sim();
        (sim.unit_scale(0), sim.xp_rate(0))
    };
    if (scale, rate) != (50, 150) || host.sim().state_hash() != hash {
        return Err(format!("save and load kept scale {scale}, rate {rate}, hash equal {}", host.sim().state_hash() == hash));
    }
    Ok(())
}

pub fn lua_host_api_full(image: &[u8]) -> Result<(), String> {
    let src = r#"
function on_tick(state)
  if state.frame == nil then
    state.frame = 1
    state.owner = unit_player(2)
    state.hp0 = unit_hp(2)
    game.cmd_damage(0, 2, 655360)
    game.cmd_buff(2, 1, 5)
  elseif state.frame == 1 then
    state.frame = 2
    state.hp1 = unit_hp(2)
    state.buffed = #game.unit_buffs(2) > 0
  end
end
"#;
    let (mut host, _mod) = start(image, "lua-host-api-full", src)?;
    advance(&mut host, 2)?;
    let owner = read_int(&host, "owner")?;
    if owner != 1 {
        return Err(format!("unit 2 is owned by player {owner}, want 1: map.osd placement order changed"));
    }
    let lost = read_int(&host, "hp0")? - read_int(&host, "hp1")?;
    if lost != 655360 {
        return Err(format!("cmd_damage removed {lost} raw Q16.16 from unit 2, want 655360"));
    }
    if !read_bool(&host, "buffed")? {
        return Err("cmd_buff left no buff on a live unit with free slots".into());
    }
    Ok(())
}

pub fn lua_host_capabilities(image: &[u8]) -> Result<(), String> {
    let src = r#"
function on_tick(state)
  if state.done then return end
  state.done = 1
  state.fmt = string.format("%05d|%-4s|%.2f|%x|%s", 42, "ab", 3.14159, 255, true) == "00042|ab  |3.14|ff|true" and 1 or 0
  local t = {5, 2, 9, 1}
  table.sort(t, function(a, b) return a > b end)
  table.insert(t, 7)
  table.insert(t, 1, 8)
  state.table = table.concat(t, ",") == "8,9,5,2,1,7" and 1 or 0
  state.removed = table.remove(t)
  local r = math.random(1, 6)
  state.random = (r >= 1 and r <= 6) and 1 or 0
  local g = game.random(10)
  state.game_random = (g >= 0 and g < 10) and 1 or 0
  state.bearing = game.bearing(0, 65536)
  local id = game.tree_add(3, 3, 2, 2)
  state.tree_live = game.tree_info(id).alive and 1 or 0
  state.tree_killed = game.tree_kill(id, 0) and 1 or 0
  state.tree_dead = game.tree_info(id).alive and 0 or 1
  state.tree_restored = game.tree_restore(id) and 1 or 0
  state.near = #game.trees_near(4 * 65536, 4 * 65536, 65536)
  state.blocker = game.blocker_add(6, 6, 1, 1, 1) and 1 or 0
  game.set_turn_rate(0, 90)
  state.step = game.turn_step(0)
  game.set_hold_fire(0, true)
  state.hold = game.holds_fire(0) and 1 or 0
  state.controller = type(game.player_controller(0)) == "string" and 1 or 0
end
"#;
    let (mut host, _mod) = start(image, "lua-host-capabilities", src)?;
    advance(&mut host, 1)?;
    for (key, want) in [
        ("fmt", 1),
        ("table", 1),
        ("removed", 7),
        ("random", 1),
        ("game_random", 1),
        ("tree_live", 1),
        ("tree_killed", 1),
        ("tree_dead", 1),
        ("tree_restored", 1),
        ("near", 1),
        ("blocker", 1),
        ("step", 1024),
        ("hold", 1),
        ("controller", 1),
    ] {
        let got = read_int(&host, key)?;
        if got != want {
            return Err(format!("state.{key} = {got}, want {want}: {:?}", host.hook_errors()));
        }
    }
    let bearing = read_int(&host, "bearing")?;
    if !(16384 - 64..=16384).contains(&bearing) {
        return Err(format!("game.bearing(0, 65536) = {bearing}, want a quarter turn (16384)"));
    }
    Ok(())
}

pub fn lua_sandbox_limits(image: &[u8]) -> Result<(), String> {
    let src = "function on_tick(state)\n  local n = 0\n  while true do n = n + 1 end\nend\n";
    let (mut host, _mod) = start(image, "lua-sandbox-limits", src)?;
    host.advance().map_err(|e| format!("stopped with: {e}"))?;
    if !hook_disabled(&host, "lua-sandbox-limits", "on_tick") {
        return Err("an unbounded loop in on_tick was not bounded and switched off".into());
    }
    if host.hook_errors().iter().any(|e| e.contains("instruction_limit")) {
        Ok(())
    } else {
        Err(format!("stopped without instruction_limit: {:?}", host.hook_errors()))
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

const HOST_ITEMS_AND_FACING: &str = r#"
local tick = 0
local wolf, other, member, slot
local function open_spawn(facing)
  for gy = 60, 200, 4 do
    for gx = 60, 200, 4 do
      local id = game.spawn_neutral("neutral_giant_wolf", gx * 65536, gy * 65536, facing)
      if id ~= nil then return id end
    end
  end
end
local function row_of(unit, name)
  for _, row in ipairs(game.unit_items(unit)) do
    if row.name == name then return row end
  end
end
function on_item_use(unit, item_slot, name, kind, target, state)
  state.use_calls = (state.use_calls or 0) + 1
  state.use_by_holder = (unit == wolf and item_slot == slot) and 1 or 0
  state.use_name_len = #name
  if kind == "unit" then state.use_unit_target = target end
  if kind == "prop" then state.use_doodad_target = target end
end
function on_tick(state)
  tick = tick + 1
  if tick == 1 then
    wolf = open_spawn(90)
    other = open_spawn()
    member = open_spawn()
    state.spawned = (wolf ~= nil and other ~= nil and member ~= nil) and 1 or 0
    game.cmd_give_item(wolf, "bottle", 3)
    game.join_spawn_group(member)
  elseif tick == 2 then
    state.spawn_heading = game.unit_heading(wolf)
    local row = row_of(wolf, "bottle")
    slot = row.slot
    state.default_owner = row.owner
    state.holder_player = unit_player(wolf)
    game.cmd_set_item_owner(wolf, slot, 3)
    game.cmd_remove(member)
  elseif tick == 3 then
    game.cmd_replace_item(wolf, slot, "bottle_empty", 1)
    local x, y = unit_position(other)
    game.cmd_set_home(member, x + 65536, y)
    game.cmd_respawn_as(member, "neutral_giant_wolf", -90)
    game.cmd_set_respawn(member, 0)
  elseif tick == 5 then
    local row = row_of(wolf, "bottle_empty")
    state.replaced_slot = row ~= nil and row.slot or -1
    state.expected_slot = slot
    state.replaced_count = row ~= nil and row.count or -1
    state.kept_owner = game.item_owner(wolf, slot) or -1
    state.old_gone = row_of(wolf, "bottle") == nil and 1 or 0
    state.respawn_heading = game.unit_heading(member)
    local doodads = game.props()
    state.doodad_id = doodads[1].id
    game.cmd_use_item_on(wolf, slot, "unit", other)
    game.cmd_use_item_on(wolf, slot, "prop", doodads[1].id)
    game.cmd_use_item_on(wolf, slot, "prop", 4000000000)
    game.cmd_use_item_on(wolf, slot + 1, "unit", other)
    state.other_unit = other
  end
end
"#;

pub fn host_items_and_facing(_image: &[u8]) -> Result<(), String> {
    let image = osdt_data::generate(&crate::checks::pack_dir()).map_err(|e| format!("moba image: {e:#}"))?;
    let (mut host, _mod) = start(&image, "host-items-facing", HOST_ITEMS_AND_FACING)?;
    host.sim_mut().set_fog(false);
    advance(&mut host, 8)?;
    let int = |key: &str| read_int(&host, key);
    let quarter = i64::from(sim::FULL_TURN) / 4;
    let checks: [(&str, i64, i64); 13] = [
        ("units spawned", int("spawned")?, 1),
        ("spawn_neutral facing 90", int("spawn_heading")?, quarter),
        ("cmd_respawn_as facing -90", int("respawn_heading")?, 3 * quarter),
        ("an unowned item reports its holder's player", int("default_owner")?, int("holder_player")?),
        ("the replacement stays in the slot", int("replaced_slot")?, int("expected_slot")?),
        ("the replacement holds its count", int("replaced_count")?, 1),
        ("the old item is gone", int("old_gone")?, 1),
        ("the replacement keeps the owner", int("kept_owner")?, 3),
        ("item use on a unit and on a doodad reach the pack, bad targets and empty slots do not", int("use_calls")?, 2),
        ("the hook names the holder and slot", int("use_by_holder")?, 1),
        ("the hook gets the item name", int("use_name_len")?, "bottle_empty".len() as i64),
        ("the hook gets the unit target", int("use_unit_target")?, int("other_unit")?),
        ("the hook gets the doodad target", int("use_doodad_target")?, int("doodad_id")?),
    ];
    for (what, got, want) in checks {
        if got != want {
            return Err(format!("{what}: got {got}, want {want}"));
        }
    }
    println!("host items and facing ok slot={}", int("expected_slot")?);
    Ok(())
}

pub fn lua_panel_matrix(image: &[u8]) -> Result<(), String> {
    let src = r#"
function on_tick(state)
  if state.done then return end
  state.done = 1
  game.panel("board", { title = "T", player = 2, columns = { "Hero", "K/D" }, rows = {
    { "|cffff0000Axe|r", "1/0" },
    { 'say "hi"\\', "a\nb" },
    {},
  } })
  game.panel("plain", { title = "P", text = "body" })
  state.bad_cell = pcall(game.panel, "x", { rows = { { 5 } } }) and 0 or 1
  state.bad_rows = pcall(game.panel, "x", { rows = "no" }) and 0 or 1
  state.bad_columns = pcall(game.panel, "x", { columns = { "a" } }) and 0 or 1
  local many = {}
  for i = 1, 65 do many[i] = { "r" } end
  state.too_many = pcall(game.panel, "x", { rows = many }) and 0 or 1
  local wide = {}
  for i = 1, 33 do wide[i] = "c" end
  state.too_wide = pcall(game.panel, "x", { rows = { wide } }) and 0 or 1
  state.long_cell = pcall(game.panel, "x", { rows = { { string.rep("z", 257) } } }) and 0 or 1
end
"#;
    let (mut host, _mod) = start(image, "lua-panel-matrix", src)?;
    advance(&mut host, 2)?;
    for key in ["bad_cell", "bad_rows", "bad_columns", "too_many", "too_wide", "long_cell"] {
        let got = read_int(&host, key)?;
        if got != 1 {
            return Err(format!("state.{key} = {got}, want 1 (refused): {:?}", host.hook_errors()));
        }
    }
    let ui = host.take_ui();
    let find = |id: &str| -> Result<serde_json::Value, String> {
        ui.iter()
            .filter_map(|b| serde_json::from_str::<serde_json::Value>(&format!("{{{b}}}")).ok().or_else(|| serde_json::from_str(b).ok()))
            .find(|v| v["kind"] == "panel" && v["id"] == id)
            .ok_or_else(|| format!("no panel event for {id}: {ui:?}"))
    };
    let board = find("board")?;
    let want = serde_json::json!({
        "kind": "panel", "id": "board", "player": 2, "title": "T", "text": "",
        "columns": ["Hero", "K/D"],
        "rows": [["|cffff0000Axe|r", "1/0"], ["say \"hi\"\\", "a\nb"], []],
    });
    if board != want {
        return Err(format!("panel event {board}, want {want}"));
    }
    let plain = find("plain")?;
    if plain.get("rows").is_some() || plain.get("columns").is_some() || plain["text"] != "body" {
        return Err(format!("a panel without rows must keep the old shape: {plain}"));
    }
    Ok(())
}
