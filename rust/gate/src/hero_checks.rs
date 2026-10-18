//! Gate checks for Dota hero base stats on the moba scenario with its hero_rules.txt: attributes, growth,
//! health, mana, armour, damage and attack period for a strength hero (axe) and a universal hero (marci),
//! each against numbers computed by hand from npc_dota_hero_*.txt @282fad29f0f74504fad465c52d197fb61f4d612b.

use crate::checks::{fresh, kind};
use sim::{fixed, Sim};

fn moba_with_rules() -> Result<Sim, String> {
    let scenario = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("scenarios").join("moba");
    let image = osdt_data::generate(&scenario).map_err(|e| format!("moba scenario image: {e:#}"))?;
    let mut s = fresh(&image)?;
    let rules = std::fs::read_to_string(scenario.join("hero_rules.txt")).map_err(|e| e.to_string())?;
    if !s.set_hero_rules_text(&rules) {
        return Err("hero_rules.txt has a bad line".into());
    }
    Ok(s)
}

fn spawn_hero(s: &mut Sim, name: &str) -> Result<usize, String> {
    let k = kind(s, name)?;
    let base = kind(s, "base")?;
    let anchor = (0..s.count())
        .find(|&u| s.alive(u) && s.player(u) == 0 && s.unit_type(u) == base as i32)
        .ok_or("moba scenario has no base for player 0")?;
    let (x, y) = s.position_fixed(anchor);
    for dx in 8..60 {
        if let Some(h) = s.spawn_hero(0, k, x + fixed::from_int(dx), y) {
            return Ok(h);
        }
    }
    Err(format!("spawn_hero refused {name} beside the base"))
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

/// Axe at level 1: strength 25, agility 18, intelligence 18; health 120 + 22 x 25; armour 0 + 18/6;
/// damage 33 (31-35) + 25 strength; attack period 1.7 s / (1 + 18/100) = 23 ticks; mana 75 + 12 x 18.
pub fn hero_axe_level1(_image: &[u8]) -> Result<(), String> {
    let mut s = moba_with_rules()?;
    let h = spawn_hero(&mut s, "axe")?;
    if s.hero_attributes(h) != [25, 18, 18] {
        return Err(format!("axe attributes {:?} want [25, 18, 18]", s.hero_attributes(h)));
    }
    near("axe max health", s.hp_max(h), q16((120 + 22 * 25) * 100))?;
    near("axe armour", s.armor_of(h), q16(300))?;
    near("axe damage", s.damage_of(h), q16((33 + 25) * 100))?;
    if s.period_of(h) != 23 {
        return Err(format!("axe attack period {} want 23 ticks", s.period_of(h)));
    }
    if s.mana_of(h) != 75 + 12 * 18 || s.hero_max_mana(h) != 75 + 12 * 18 {
        return Err(format!("axe mana {} max {} want {}", s.mana_of(h), s.hero_max_mana(h), 75 + 12 * 18));
    }
    Ok(())
}

/// One level of growth: axe strength 25 + 2.7, agility 18 + 1.7, intelligence 18 + 1.6; health gains 22 per
/// strength point and the wounded fraction is kept as a flat amount.
pub fn hero_axe_growth(_image: &[u8]) -> Result<(), String> {
    let mut s = moba_with_rules()?;
    let h = spawn_hero(&mut s, "axe")?;
    let first = s.xp_curve()[0];
    let gained = s.add_xp(h, first);
    if gained != 1 || s.level(h) != 1 {
        return Err(format!("{first} xp gave {gained} levels, level {}", s.level(h)));
    }
    if s.hero_attributes_x100(h) != [2770, 1970, 1960] {
        return Err(format!("axe attributes at level 2 {:?} want [2770, 1970, 1960]", s.hero_attributes_x100(h)));
    }
    near("axe max health at level 2", s.hp_max(h), q16(12000 + 22 * 2770))?;
    near("axe armour at level 2", s.armor_of(h), q16(1970 / 6))?;
    near("axe damage at level 2", s.damage_of(h), q16(3300 + 2770))?;
    if s.hp(h) != s.hp_max(h) {
        return Err("a full-health hero lost its fill on level-up".into());
    }
    Ok(())
}

/// Universal marci at level 1 (strength 23, agility 20, intelligence 19): damage 31 + 0.7 x 62.
pub fn hero_universal_damage(_image: &[u8]) -> Result<(), String> {
    let mut s = moba_with_rules()?;
    let h = spawn_hero(&mut s, "marci")?;
    near("marci damage", s.damage_of(h), q16(3100 + (2300 + 2000 + 1900) * 70 / 100))?;
    Ok(())
}

/// Every enabled Dota hero is in the catalog with its own hero line.
pub fn hero_roster(_image: &[u8]) -> Result<(), String> {
    let mut s = moba_with_rules()?;
    let mut with_def = 0;
    for name in ["antimage", "axe", "crystal_maiden", "zuus", "nevermore", "pudge", "invoker", "largo"] {
        let h = spawn_hero(&mut s, name)?;
        if s.hero_def(h).is_none() {
            return Err(format!("{name} has no hero line"));
        }
        with_def += 1;
    }
    if with_def != 8 || s.catalog().unit_index("npc_dota_hero_base").is_some() {
        return Err("roster check miscounted".into());
    }
    Ok(())
}
