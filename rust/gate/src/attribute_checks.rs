use crate::checks::{fresh, kind};
use crate::checks::{role};
use sim::{fixed, Sim};

fn scenario_with_rules() -> Result<Sim, String> {
    let scenario = crate::checks::pack_dir();
    let image = osdt_data::generate(&scenario).map_err(|e| format!("moba scenario image: {e:#}"))?;
    let mut s = fresh(&image)?;
    let rules = std::fs::read_to_string(scenario.join(crate::checks::RULES_FILE)).map_err(|e| e.to_string())?;
    if !s.set_attr_rules_text(&rules) {
        return Err("the pack rules file has a bad line".into());
    }
    Ok(s)
}

fn spawn_attributed(s: &mut Sim, name: &str) -> Result<usize, String> {
    let k = kind(s, name)?;
    let base = kind(s, role("unit.base")?)?;
    let anchor = (0..s.count())
        .find(|&u| s.alive(u) && s.player(u) == 0 && s.unit_type(u) == base as i32)
        .ok_or("moba scenario has no base for player 0")?;
    let (x, y) = s.position_fixed(anchor);
    for dx in 8..60 {
        if let Some(h) = s.spawn_attributed(0, k, x + fixed::from_int(dx), y) {
            return Ok(h);
        }
    }
    Err(format!("spawn_attributed refused {name} beside the base"))
}

pub fn attribute_roster(_image: &[u8]) -> Result<(), String> {
    let mut s = scenario_with_rules()?;
    let mut with_def = 0;
    for name in [role("hero.l")?, role("hero.a")?, role("hero.m")?, role("hero.n")?, role("hero.j")?, role("hero.e")?, role("hero.p")?, role("hero.t")?] {
        let h = spawn_attributed(&mut s, name)?;
        if s.attr_def(h).is_none() {
            return Err(format!("{name} has no hero line"));
        }
        with_def += 1;
    }
    if with_def != 8 || s.catalog().unit_index("npc_dota_hero_base").is_some() {
        return Err("roster check miscounted".into());
    }
    Ok(())
}

/// Every player-controlled unit kind with a model (the page's heroes) has a HERO_PRIMARY row in the moba panels file,
/// and every such row is 1, 2 or 3. The page groups heroes by that row, so a missing row, or a 0 or other value, puts a
/// hero in the wrong column or in none.
pub fn hero_primary_rows(_image: &[u8]) -> Result<(), String> {
    let s = scenario_with_rules()?;
    let path = crate::checks::pack_dir().join("panels.lua");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let rows = hero_primary_table(&text)?;
    let cat = s.catalog();
    let mut heroes = 0;
    let mut missing = Vec::new();
    let mut outside = Vec::new();
    for (k, unit) in cat.units.iter().enumerate() {
        let player = cat.unit_ext.get(k).is_some_and(|e| e.player_controlled);
        if !player || cat.name(unit.model).is_empty() {
            continue;
        }
        heroes += 1;
        let name = cat.name(unit.name);
        match rows.get(name) {
            None => missing.push(name.to_string()),
            Some(v) if !(1..=3).contains(v) => outside.push(format!("{name}={v}")),
            Some(_) => {}
        }
    }
    if heroes == 0 {
        return Err("no player-controlled unit kind with a model, so the check would pass on nothing".into());
    }
    if missing.is_empty() && outside.is_empty() {
        return Ok(());
    }
    Err(format!(
        "{} hero kind(s) without a HERO_PRIMARY row: [{}]; HERO_PRIMARY rows not 1, 2 or 3: [{}]",
        missing.len(),
        missing.join(", "),
        outside.join(", ")
    ))
}

/// The `name = N,` rows of `local HERO_PRIMARY = { ... }` in the panels text, with `--` comments removed. A line of the
/// table that is not such a row is an error, so a typo cannot quietly drop a hero's group.
fn hero_primary_table(text: &str) -> Result<std::collections::BTreeMap<String, i64>, String> {
    let mut rows = std::collections::BTreeMap::new();
    let mut inside = false;
    for raw in text.lines() {
        let line = raw.split("--").next().unwrap_or("").trim();
        if !inside {
            inside = line == "local HERO_PRIMARY = {";
            continue;
        }
        if line == "}" {
            return Ok(rows);
        }
        if line.is_empty() {
            continue;
        }
        let malformed = || format!("HERO_PRIMARY line is not `name = N,`: {raw}");
        let (name, value) = line.trim_end_matches(',').split_once('=').ok_or_else(|| malformed())?;
        let name = name.trim();
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(malformed());
        }
        let value: i64 = value.trim().parse().map_err(|_| malformed())?;
        rows.insert(name.to_string(), value);
    }
    Err(if inside {
        "the HERO_PRIMARY table is not closed".to_string()
    } else {
        "the panels file has no local HERO_PRIMARY table".to_string()
    })
}
