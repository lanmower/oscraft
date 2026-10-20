//! What the mlua and piccolo mod hosts share: argument checks, the client UI event bodies, effect
//! and ability building, and the Lua source of game.var, game.region, game.timer and the score
//! tally. Errors are plain `name: detail` strings each backend wraps in its own error type.

#![allow(dead_code)]

use sim::{effect_kind, stat_code, target_code, Effect, Sim, MAX_PLAYERS, STAT_NAMES};

pub(crate) fn fixed(v: i64) -> Result<i32, String> {
    i32::try_from(v).map_err(|_| format!("Q16.16 value {v} out of i32 range"))
}

pub(crate) fn amount_arg(amount: i64) -> Result<i32, String> {
    i32::try_from(amount).map_err(|_| format!("invalid_amount: {amount} out of i32 range"))
}

pub(crate) fn player_arg(p: i64) -> Result<u8, String> {
    usize::try_from(p)
        .ok()
        .filter(|&p| p < MAX_PLAYERS)
        .map(|p| p as u8)
        .ok_or_else(|| format!("invalid_player: {p} is outside 0..{MAX_PLAYERS}"))
}

// A UI event's player: -1 addresses every client, otherwise one of the match players.
pub(crate) fn ui_player(p: i64) -> Result<i64, String> {
    if p == -1 {
        return Ok(-1);
    }
    player_arg(p).map(i64::from)
}

fn ui_seconds(seconds: Option<f64>, default: f64) -> Result<f64, String> {
    let s = seconds.unwrap_or(default);
    if s.is_finite() && s >= 0.0 {
        Ok(s)
    } else {
        Err(format!("invalid_seconds: {s} must be finite and not negative"))
    }
}

fn ui_json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

pub(crate) fn ui_message(player: i64, text: &str, seconds: Option<f64>) -> Result<String, String> {
    Ok(format!(r#""kind":"message","player":{},"text":{},"seconds":{}"#, ui_player(player)?, ui_json_string(text), ui_seconds(seconds, 5.0)?))
}

pub(crate) fn ui_banner(text: &str, seconds: Option<f64>) -> Result<String, String> {
    Ok(format!(r#""kind":"banner","text":{},"seconds":{}"#, ui_json_string(text), ui_seconds(seconds, 4.0)?))
}

// A panel spec is (title, text, player); None removes the panel.
pub(crate) fn ui_panel(id: &str, spec: Option<(Option<String>, Option<String>, Option<i64>)>) -> Result<String, String> {
    Ok(match spec {
        None => format!(r#""kind":"panel","id":{},"remove":true"#, ui_json_string(id)),
        Some((title, text, player)) => {
            let player = match player {
                Some(p) => i64::from(player_arg(p)?),
                None => -1,
            };
            format!(
                r#""kind":"panel","id":{},"player":{},"title":{},"text":{}"#,
                ui_json_string(id),
                player,
                ui_json_string(&title.unwrap_or_default()),
                ui_json_string(&text.unwrap_or_default())
            )
        }
    })
}

pub(crate) fn ui_camera(player: i64, x: f64, y: f64, zoom: Option<f64>, seconds: Option<f64>) -> Result<String, String> {
    let zoom = zoom.unwrap_or(1.0);
    if !(zoom.is_finite() && zoom > 0.0) || !x.is_finite() || !y.is_finite() {
        return Err("invalid_camera: x, y and zoom must be finite and zoom positive".into());
    }
    Ok(format!(
        r#""kind":"camera","player":{},"x":{},"y":{},"zoom":{},"seconds":{}"#,
        ui_player(player)?,
        x / 65536.0,
        y / 65536.0,
        zoom,
        ui_seconds(seconds, 1.0)?
    ))
}

pub(crate) fn ui_sound(name: &str) -> String {
    format!(r#""kind":"sound","name":{}"#, ui_json_string(name))
}

// One entry of game.ability's effects list. `field` reads an integer parameter (0 when absent) and
// `text` a string one. The named fields fill the catalog effect parameters each kind reads: damage
// and heal take amount and radius; move takes range; buff takes buff and ticks; shield takes
// amount; stat takes stat (a STAT_NAMES entry) and amount; spawn takes unit (a catalog name) and dx, dy.
pub(crate) fn effect_from(
    sim: &Sim,
    kind: &str,
    field: &dyn Fn(&str) -> Result<i32, String>,
    text: &dyn Fn(&str) -> Result<String, String>,
) -> Result<Effect, String> {
    let code = effect_kind(kind).ok_or_else(|| format!("unknown_effect: {kind} (known: {})", sim::MOD_EFFECTS.join(", ")))?;
    let (p0, p1, p2) = match kind {
        "damage" | "heal" => (field("amount")?, field("radius")?, 0),
        "move" => (field("range")?, 0, 0),
        "shield" => (field("amount")?, 0, 0),
        "stat" => {
            let stat = text("stat")?;
            let code = stat_code(&stat).ok_or_else(|| format!("unknown_stat: {stat} (known: {})", STAT_NAMES.join(", ")))?;
            (code, field("amount")?, 0)
        }
        "buff" => (field("buff")?, field("ticks")?, 0),
        _ => {
            let unit = text("unit")?;
            let index = sim.catalog().unit_index(&unit).ok_or_else(|| format!("unknown_unit: {unit}"))?;
            (index as i32, field("dx")?, field("dy")?)
        }
    };
    Ok(Effect { kind: code, p0, p1, p2, p3: 0 })
}

// The scalar parts of a game.ability spec: target code, cooldown, energy and filter.
pub(crate) fn ability_args(target: Option<&str>, cooldown: i32, energy: i32, filter: i32) -> Result<(u8, i32, i32, u16), String> {
    let target = match target {
        None => 0,
        Some(t) => target_code(t).ok_or_else(|| format!("invalid_ability: target {t} is not none, unit or point"))?,
    };
    let filter = u16::try_from(filter).map_err(|_| "invalid_ability: filter is outside 0..=65535".to_string())?;
    Ok((target, cooldown, energy, filter))
}

pub(crate) const PRELUDE_SRC: &str = r##"
return function(game, vars, host, max_players)
  local function check_key(key)
    if type(key) ~= "string" then error("invalid_key: a variable name is a string, got " .. type(key), 3) end
  end
  local function check_value(v)
    local t = type(v)
    if v ~= nil and t ~= "number" and t ~= "string" and t ~= "boolean" then
      error("invalid_value: a variable holds a number, string or boolean, got " .. t, 3)
    end
  end
  local function bucket(scope, id)
    local all = vars[scope]
    local row = all[id]
    if not row then row = {}; all[id] = row end
    return row
  end
  local function access(scope, id, key, ...)
    check_key(key)
    local row = bucket(scope, id)
    if select("#", ...) == 0 then return row[key] end
    local v = ...
    check_value(v)
    row[key] = v
  end
  local function player_id(p)
    if type(p) ~= "number" or p % 1 ~= 0 or p < 0 or p >= max_players then
      error("invalid_player: " .. tostring(p) .. " is outside 0.." .. max_players, 3)
    end
    return p
  end
  function game.var(key, ...) return access("global", 0, key, ...) end
  function game.player_var(player, key, ...) return access("player", player_id(player), key, ...) end
  function game.unit_var(unit, key, ...)
    game.unit_kind(unit)
    return access("unit", unit, key, ...)
  end
  function game.add_score(player, amount)
    if type(amount) ~= "number" then error("invalid_amount: " .. tostring(amount), 2) end
    local row = bucket("score", player_id(player))
    row.total = (row.total or 0) + amount
    return row.total
  end
  function game.score(player) return bucket("score", player_id(player)).total or 0 end

  function game.region(name, x0, y0, x1, y1, handlers)
    host.define_region(name, x0, y0, x1, y1)
    local function named(region) return region == name end
    if handlers and handlers.enter then
      local fn = handlers.enter
      game.trigger("on_enter", { conditions = { named }, action = function(_, unit) fn(unit) end })
    end
    if handlers and handlers.leave then
      local fn = handlers.leave
      game.trigger("on_leave", { conditions = { named }, action = function(_, unit) fn(unit) end })
    end
    return name
  end
  function game.region_bounds(name)
    return host.region_bounds(name)
  end
  function game.map_size()
    return host.map_size()
  end
  function game.region_center(name)
    local x0, y0, x1, y1 = host.region_bounds(name)
    return (x0 + x1) // 2, (y0 + y1) // 2
  end
  function game.region_units(name)
    local inside = {}
    for _, unit in ipairs(game.units()) do
      local x, y = host.unit_position(unit)
      if host.in_region(name, x, y) then inside[#inside + 1] = unit end
    end
    return inside
  end

  local live = {}
  function game.timer(name, delay, action, period)
    if type(action) ~= "function" then error("invalid_timer: " .. tostring(name) .. " needs an action function", 2) end
    if period ~= nil and (type(period) ~= "number" or period < 1) then error("invalid_timer: period must be 1 or more ticks", 2) end
    local function fire()
      action()
      if period and live[name] == fire then host.schedule(name, period, fire) end
    end
    live[name] = fire
    host.schedule(name, delay, fire)
    return name
  end
  function game.cancel_timer(name)
    live[name] = nil
    return host.cancel(name)
  end
end
"##;
