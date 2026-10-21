//! Gate checks for the reference hero lines on the moba scenario with its rules file. Attribute totals, level
//! growth and the stat step are computed in the pack's Lua, so only the engine's own readouts are checked here.

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

fn near(what: &str, got: i32, want_q16: i64) -> Result<(), String> {
    if (got as i64 - want_q16).abs() > 2 {
        return Err(format!("{what} {got} want {want_q16} (Q16.16)"));
    }
    Ok(())
}

fn q16(x100: i64) -> i64 {
    x100 * fixed::ONE as i64 / 100
}

/// Universal marci at level 1 (strength 23, agility 20, intelligence 19): damage 31 + 0.7 x 62.
pub fn attribute_universal_damage(_image: &[u8]) -> Result<(), String> {
    let mut s = scenario_with_rules()?;
    let h = spawn_attributed(&mut s, role("hero.u")?)?;
    near("marci damage", s.damage_of(h), q16(3100 + (2300 + 2000 + 1900) * 70 / 100))?;
    Ok(())
}

/// Every enabled reference hero is in the catalog with its own hero line.
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
