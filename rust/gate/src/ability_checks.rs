//! Gate checks for the hero ability mechanics on the moba scenario: damage types, damage over time, areas,
//! chains, zones, channels with pulses, procs, passives, auras, statuses, charges, talents and casting from
//! a distance. Each check casts a real Dota ability from scenarios/moba/abilities_dota.osd and asserts a
//! number computed by hand from npc_dota_hero_*.txt @282fad29f0f74504fad465c52d197fb61f4d612b.

use crate::checks::{fresh, kind};
use sim::{fixed, Sim};

const STUN: usize = 0;
const SLOW: usize = 1;
const ROOT: usize = 3;
const TAUNT: usize = 5;
const HEX: usize = 6;
const DISARM: usize = 7;
const SPELL_IMMUNITY: usize = 8;
const INVULNERABLE: usize = 9;
const PHYSICAL_IMMUNITY: usize = 12;
const S_DAMAGE: usize = 0;
const S_ARMOR: usize = 1;
const S_RANGE: usize = 4;
const S_ATTACK_SPEED: usize = 12;
const S_MANA_REGEN: usize = 13;
const S_HEALTH_REGEN: usize = 14;
const S_MAGIC_RESIST: usize = 11;
const S_SPEED_PCT: usize = 19;
const S_CRIT_CHANCE: usize = 6;
const S_CRIT_MULT: usize = 7;
const S_CLEAVE: usize = 8;

/// The moba scenario with its hero rules, a high level cap and an open arena, centre in whole world units.
struct World {
    s: Sim,
    cx: i32,
    cy: i32,
}

fn world() -> Result<World, String> {
    let scenario = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("scenarios").join("moba");
    let image = osdt_data::generate(&scenario).map_err(|e| format!("moba scenario image: {e:#}"))?;
    let mut s = fresh(&image)?;
    let rules = std::fs::read_to_string(scenario.join("hero_rules.txt")).map_err(|e| e.to_string())?;
    if !s.set_hero_rules_text(&rules) {
        return Err("hero_rules.txt has a bad line".into());
    }
    for u in 0..s.count() {
        if s.alive(u) {
            s.apply_status(u, STUN, 1_000_000, fixed::ONE);
        }
    }
    let (w, h) = (s.grid().w as i32, s.grid().h as i32);
    let reach = 14;
    let open = |cx: i32, cy: i32| (-reach..=reach).all(|dy| (-reach..=reach).all(|dx| s.grid().walkable(cx + dx, cy + dy)));
    for cy in (reach..h - reach).step_by(2) {
        for cx in (reach..w - reach).step_by(2) {
            if open(cx, cy) {
                return Ok(World { s, cx, cy });
            }
        }
    }
    Err(format!("no open {}x{} arena on the {w}x{h} map", 2 * reach + 1, 2 * reach + 1))
}

impl World {
    fn at(&self, dx: i32, dy: i32) -> (i32, i32) {
        (fixed::from_int(self.cx + dx), fixed::from_int(self.cy + dy))
    }

    /// A hero of `player` at an offset from the arena centre, leveled to `level` and holding `ranks` of each named ability.
    fn hero(&mut self, name: &str, player: u8, dx: i32, dy: i32) -> Result<usize, String> {
        let k = kind(&self.s, name)?;
        let (x, y) = self.at(dx, dy);
        let h = self.s.spawn_hero(player, k, x, y).ok_or_else(|| format!("spawn_hero refused {name} at ({dx},{dy})"))?;
        self.s.apply_status(h, DISARM, 1_000_000, fixed::ONE);
        Ok(h)
    }

    fn creep(&mut self, player: u8, dx: i32, dy: i32) -> Result<usize, String> {
        let k = kind(&self.s, "creep")?;
        let (x, y) = self.at(dx, dy);
        let u = self.s.spawn_for(player, k, x, y).ok_or_else(|| format!("spawn_for refused a creep at ({dx},{dy})"))?;
        self.s.apply_status(u, DISARM, 1_000_000, fixed::ONE);
        Ok(u)
    }

    fn level_up(&mut self, h: usize, level: u32) {
        while self.s.character_level(h) < level {
            self.s.add_xp(h, 25);
        }
    }

    fn learn(&mut self, h: usize, ability: &str, ranks: usize) -> Result<usize, String> {
        let slot = self.s.ability_slot(h, ability).ok_or_else(|| format!("hero holds no {ability}"))?;
        for _ in 0..ranks {
            self.s.skill_up(h, slot).map_err(|r| format!("skill_up {ability}: {}", r.name()))?;
        }
        Ok(slot)
    }

    fn cast(&mut self, h: usize, ability: &str, target: i32, x: i32, y: i32) -> Result<(), String> {
        let slot = self.s.ability_slot(h, ability).ok_or_else(|| format!("hero holds no {ability}"))?;
        if self.s.cast_ability(h, slot, target, x, y) {
            let channelled = matches!(ability, "crystal_maiden_freezing_field" | "lion_mana_drain");
            for _ in 0..80 {
                if channelled || self.s.channel_left(h) <= 0 {
                    break;
                }
                self.s.tick();
            }
            Ok(())
        } else {
            Err(format!("cast {ability} refused: {}", self.s.stats.last_refusal.name()))
        }
    }

    /// One weapon attack of `a` on `b`, waiting at most 200 ticks for the weapon.
    fn strike(&mut self, a: usize, b: usize) -> Result<(), String> {
        for _ in 0..200 {
            if self.s.order_attack(a, b).is_ok() {
                return Ok(());
            }
            self.run(1);
        }
        Err(format!("unit {a} could not strike unit {b} in 200 ticks: {}", self.s.stats.last_refusal.name()))
    }

    fn run(&mut self, ticks: u32) {
        for _ in 0..ticks {
            self.s.tick();
        }
    }

    /// Hit points of `u`, whole.
    fn hp(&self, u: usize) -> f64 {
        self.s.hp(u) as f64 / fixed::ONE as f64
    }
}

fn near(what: &str, got: f64, want: f64, tol: f64) -> Result<(), String> {
    if (got - want).abs() > tol {
        return Err(format!("{what} {got:.3} want {want:.3} +-{tol}"));
    }
    Ok(())
}

fn rank_one_hero(w: &mut World, name: &str, level: u32, abilities: &[&str]) -> Result<usize, String> {
    let h = w.hero(name, 0, 0, 0)?;
    w.level_up(h, level);
    for a in abilities {
        w.learn(h, a, 1)?;
    }
    Ok(h)
}

/// Berserker's Call: enemies in 315 units are taunted for 2.1 s and the caster gains 12 armour for as long.
pub fn axe_berserkers_call(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let axe = rank_one_hero(&mut w, "axe", 6, &["axe_berserkers_call"])?;
    let near_enemy = w.creep(1, 3, 0)?;
    let far_enemy = w.creep(1, 9, 0)?;
    w.cast(axe, "axe_berserkers_call", -1, 0, 0)?;
    if w.s.status_left(near_enemy, TAUNT) != 34 {
        return Err(format!("taunt ticks {} want 34 (2.1 s)", w.s.status_left(near_enemy, TAUNT)));
    }
    if w.s.status_left(far_enemy, TAUNT) != 0 {
        return Err("an enemy outside 315 units was taunted".into());
    }
    if w.s.stat_value(axe, S_ARMOR) != 12 {
        return Err(format!("caster armour bonus {} want 12", w.s.stat_value(axe, S_ARMOR)));
    }
    w.run(40);
    if w.s.status_left(near_enemy, TAUNT) != 0 || w.s.stat_value(axe, S_ARMOR) != 0 {
        return Err("taunt or armour bonus outlived 2.1 s".into());
    }
    Ok(())
}

/// Battle Hunger: 12 pure damage a second for 12 s and an 18 percent slow; pure damage ignores magic resistance.
pub fn axe_battle_hunger(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let axe = rank_one_hero(&mut w, "axe", 1, &["axe_battle_hunger"])?;
    let target = w.creep(1, 4, 0)?;
    let before = w.hp(target);
    w.cast(axe, "axe_battle_hunger", target as i32, 0, 0)?;
    w.run(17);
    near("battle hunger hp after one pulse", before - w.hp(target), 12.0, 1.0)?;
    let slow = w.s.slow_multiplier(target);
    if (slow - 53740).abs() > 2 {
        return Err(format!("slow multiplier {slow} want 53740 (18 percent)"));
    }
    w.run(16 * 12);
    if w.s.status_left(target, SLOW) != 0 {
        return Err("slow outlived 12 s".into());
    }
    Ok(())
}

/// Counter Helix: every 7th attack taken spins for 100 pure damage in 275 units.
pub fn axe_counter_helix(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let axe = rank_one_hero(&mut w, "axe", 1, &["axe_counter_helix"])?;
    let a = w.creep(1, 1, 0)?;
    let b = w.creep(1, 0, 3)?;
    let before = w.hp(b);
    for hit in 1..=7 {
        w.s.clear_status(a, DISARM);
        w.strike(a, axe).map_err(|e| format!("hit {hit}: {e}"))?;
        w.s.apply_status(a, DISARM, 1_000_000, fixed::ONE);
        if hit < 7 && (before - w.hp(b)).abs() > 1.0 {
            return Err(format!("the helix spun on hit {hit} of 7"));
        }
    }
    near("helix damage to the bystander", before - w.hp(b), 100.0, 1.5)?;
    Ok(())
}

/// Culling Blade: 275 pure damage kills a target below it and grants the caster and allies speed and armour; a
/// healthy target survives and grants nothing.
pub fn axe_culling_blade(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let axe = rank_one_hero(&mut w, "axe", 6, &["axe_culling_blade"])?;
    let weak = w.creep(1, 2, 0)?;
    w.s.hurt(weak, fixed::from_int(400));
    w.cast(axe, "axe_culling_blade", weak as i32, 0, 0)?;
    if w.s.alive(weak) {
        return Err("a creep at 150 hit points survived a 275 damage blade".into());
    }
    if w.s.stat_value(axe, S_SPEED_PCT) != 20 || w.s.stat_value(axe, S_ARMOR) != 10 {
        return Err(format!("kill bonus speed {} armour {} want 20 and 10", w.s.stat_value(axe, S_SPEED_PCT), w.s.stat_value(axe, S_ARMOR)));
    }
    let mut w = world()?;
    let axe = rank_one_hero(&mut w, "axe", 6, &["axe_culling_blade"])?;
    let healthy = w.creep(1, 2, 0)?;
    w.cast(axe, "axe_culling_blade", healthy as i32, 0, 0)?;
    near("healthy creep after the blade", w.hp(healthy), 550.0 - 275.0, 1.0)?;
    if w.s.stat_value(axe, S_SPEED_PCT) != 0 {
        return Err("a blade that did not kill granted speed".into());
    }
    Ok(())
}

/// Crystal Nova: 110 magical damage, reduced by the target's 25 percent magic resistance, a 20 percent slow and
/// 30 attack speed slow; a unit without resistance takes all 110.
pub fn crystal_nova(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let cm = rank_one_hero(&mut w, "crystal_maiden", 1, &["crystal_maiden_crystal_nova"])?;
    let hero = w.hero("antimage", 1, 5, 0)?;
    let creep = w.creep(1, 5, 2)?;
    let (h0, c0) = (w.hp(hero), w.hp(creep));
    let (x, y) = w.at(5, 1);
    w.cast(cm, "crystal_maiden_crystal_nova", -1, x, y)?;
    near("hero hp loss", h0 - w.hp(hero), 110.0 * 0.75, 0.6)?;
    near("creep hp loss", c0 - w.hp(creep), 110.0, 0.6)?;
    let slow = w.s.slow_multiplier(hero);
    if (slow - 52429).abs() > 2 || w.s.stat_value(hero, S_ATTACK_SPEED) != -30 {
        return Err(format!("nova slow {slow} attack speed {} want 52429 and -30", w.s.stat_value(hero, S_ATTACK_SPEED)));
    }
    Ok(())
}

/// Frostbite: roots and disarms for 1.5 s and deals 100 magical damage a second in 0.25 s steps.
pub fn crystal_frostbite(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let cm = rank_one_hero(&mut w, "crystal_maiden", 1, &["crystal_maiden_frostbite"])?;
    let target = w.creep(1, 4, 0)?;
    let before = w.hp(target);
    w.cast(cm, "crystal_maiden_frostbite", target as i32, 0, 0)?;
    w.run(5);
    if w.s.status_left(target, ROOT) == 0 || w.s.status_left(target, DISARM) == 0 {
        return Err("frostbite left the target free to move or attack".into());
    }
    near("frostbite damage after one step", before - w.hp(target), 25.0, 1.0)?;
    w.run(40);
    near("frostbite damage over 1.5 s", before - w.hp(target), 150.0, 1.5)?;
    Ok(())
}

/// Brilliance Aura: allies gain mana regeneration, the caster and those near her three times as much.
pub fn crystal_brilliance_aura(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let cm = rank_one_hero(&mut w, "crystal_maiden", 1, &["crystal_maiden_brilliance_aura"])?;
    let ally = w.hero("axe", 0, 6, 0)?;
    w.run(4);
    if w.s.stat_value(cm, S_MANA_REGEN) != 120 || w.s.stat_value(ally, S_MANA_REGEN) != 120 {
        return Err(format!("aura regen caster {} ally {} want 120 (0.4 x 3 per second)", w.s.stat_value(cm, S_MANA_REGEN), w.s.stat_value(ally, S_MANA_REGEN)));
    }
    Ok(())
}

/// Freezing Field: a 10 s channel of pulses on random spots of a ring around the caster; a stun ends it early.
pub fn crystal_freezing_field(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let cm = rank_one_hero(&mut w, "crystal_maiden", 6, &["crystal_maiden_freezing_field"])?;
    let mut ring = Vec::new();
    for k in 0..12 {
        let (dx, dy) = [(7, 0), (5, 5), (0, 7), (-5, 5), (-7, 0), (-5, -5), (0, -7), (5, -5), (8, 2), (-8, -2), (2, 8), (-2, -8)][k];
        ring.push(w.creep(1, dx, dy)?);
    }
    let mana = w.s.mana_of(cm);
    w.cast(cm, "crystal_maiden_freezing_field", -1, 0, 0)?;
    if w.s.mana_of(cm) != mana - 200 {
        return Err(format!("mana {} want {}", w.s.mana_of(cm), mana - 200));
    }
    w.run(20);
    if w.s.channel_left(cm) <= 0 {
        return Err("the channel ended early".into());
    }
    w.run(150);
    let total: f64 = ring.iter().map(|&u| 550.0 - w.hp(u)).sum();
    if total < 138.0 * 10.0 {
        return Err(format!("the ring took only {total:.0} damage in 10 s"));
    }
    if w.s.channel_left(cm) != 0 {
        return Err("the channel outlived 10 s".into());
    }
    let mut w = world()?;
    let cm = rank_one_hero(&mut w, "crystal_maiden", 6, &["crystal_maiden_freezing_field"])?;
    w.cast(cm, "crystal_maiden_freezing_field", -1, 0, 0)?;
    w.run(10);
    w.s.apply_status(cm, STUN, 20, fixed::ONE);
    if w.s.channel_left(cm) != 0 {
        return Err("a stun did not end the channel".into());
    }
    Ok(())
}

/// Storm Bolt: 80 magical damage and a 1 s stun in 250 units of the target.
pub fn sven_storm_bolt(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let sven = rank_one_hero(&mut w, "sven", 1, &["sven_storm_bolt"])?;
    let a = w.creep(1, 5, 0)?;
    let b = w.creep(1, 5, 3)?;
    let far = w.creep(1, 5, 8)?;
    w.cast(sven, "sven_storm_bolt", a as i32, 0, 0)?;
    w.run(40);
    for (name, u) in [("target", a), ("neighbour", b)] {
        near(&format!("{name} hp loss"), 550.0 - w.hp(u), 80.0, 1.5)?;
    }
    if w.hp(far) < 549.0 {
        return Err("a creep outside the bolt radius took damage".into());
    }
    Ok(())
}

/// Great Cleave: 60 percent of the hit to enemies in a cone behind the victim and none to those beside or behind the attacker.
pub fn sven_great_cleave(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let sven = rank_one_hero(&mut w, "sven", 1, &["sven_great_cleave"])?;
    w.s.clear_status(sven, DISARM);
    if w.s.stat_value(sven, S_CLEAVE) != 60 {
        return Err(format!("cleave percent {} want 60", w.s.stat_value(sven, S_CLEAVE)));
    }
    let target = w.creep(1, 1, 0)?;
    let behind = w.creep(1, 3, 0)?;
    let aside = w.creep(1, 1, 5)?;
    let rear = w.creep(1, -2, 0)?;
    let raw = w.s.damage_of(sven);
    w.s.order_attack(sven, target).map_err(|r| format!("order_attack: {}", r.name()))?;
    let want = fixed::to_float(sim::armour_reduced(raw * 60 / 100, w.s.armor_of(behind)));
    near("cone victim", 550.0 - w.hp(behind), want, 0.3)?;
    if w.hp(aside) < 549.9 || w.hp(rear) < 549.9 {
        return Err("a unit outside the cone took cleave damage".into());
    }
    Ok(())
}

/// Warcry and God's Strength: allies in 700 units gain 5 armour and 6 percent speed; God's Strength adds 110 percent of base damage.
pub fn sven_buffs(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let sven = rank_one_hero(&mut w, "sven", 6, &["sven_warcry", "sven_gods_strength"])?;
    let ally = w.hero("axe", 0, 8, 0)?;
    let base = fixed::to_int(w.s.damage_of(sven));
    w.cast(sven, "sven_warcry", -1, 0, 0)?;
    w.cast(sven, "sven_gods_strength", -1, 0, 0)?;
    if w.s.stat_value(ally, S_ARMOR) != 5 || w.s.stat_value(ally, S_SPEED_PCT) != 6 {
        return Err(format!("ally armour {} speed {} want 5 and 6", w.s.stat_value(ally, S_ARMOR), w.s.stat_value(ally, S_SPEED_PCT)));
    }
    let bonus = w.s.stat_value(sven, S_DAMAGE);
    if bonus != base * 110 / 100 {
        return Err(format!("God's Strength damage {bonus} want {}", base * 110 / 100));
    }
    w.run(16 * 30 + 4);
    if w.s.stat_value(sven, S_DAMAGE) != 0 {
        return Err("God's Strength outlived 30 s".into());
    }
    Ok(())
}

/// Blade Fury: 5 s of spell immunity and 85 magical damage a second to enemies in 260 units, in 3 tick pulses.
pub fn juggernaut_blade_fury(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let jug = rank_one_hero(&mut w, "juggernaut", 1, &["juggernaut_blade_fury"])?;
    let near_enemy = w.creep(1, 2, 0)?;
    let far_enemy = w.creep(1, 8, 0)?;
    w.cast(jug, "juggernaut_blade_fury", -1, 0, 0)?;
    if w.s.status_left(jug, SPELL_IMMUNITY) != 80 {
        return Err(format!("spell immunity {} ticks want 80", w.s.status_left(jug, SPELL_IMMUNITY)));
    }
    w.run(16);
    near("blade fury damage in 16 ticks", 550.0 - w.hp(near_enemy), 5.0 * 16.0, 1.0)?;
    if w.hp(far_enemy) < 549.9 {
        return Err("a creep outside 260 units took spin damage".into());
    }
    w.run(80);
    let spent = 550.0 - w.hp(near_enemy);
    near("blade fury damage over 5 s", spent, 85.0 * 5.0, 12.0)?;
    Ok(())
}

/// Healing Ward: a ward for 18 s that heals allies in 400 units 2 percent of their health a second.
pub fn juggernaut_healing_ward(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let jug = rank_one_hero(&mut w, "juggernaut", 1, &["juggernaut_healing_ward"])?;
    let ally = w.hero("axe", 0, 3, 0)?;
    w.s.hurt(ally, fixed::from_int(200));
    let before = w.hp(ally);
    let (x, y) = w.at(2, 0);
    w.cast(jug, "juggernaut_healing_ward", -1, x, y)?;
    w.run(0);
    let ward = (0..w.s.count()).find(|&u| w.s.alive(u) && w.s.ward_left(u) > 0).ok_or("no ward was placed")?;
    if w.s.ward_left(ward) < 287 {
        return Err(format!("ward lifetime {} ticks want 288 (18 s)", w.s.ward_left(ward)));
    }
    w.run(2);
    let healed = w.hp(ally) - before;
    let want = w.s.hp_max(ally) as f64 / fixed::ONE as f64 * 0.02;
    near("ward heal after one second", healed, want, want * 0.2 + 1.5)?;
    Ok(())
}

/// Blade Dance: 35 percent chance of a 140 percent critical strike.
pub fn juggernaut_blade_dance(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let jug = rank_one_hero(&mut w, "juggernaut", 1, &["juggernaut_blade_dance"])?;
    w.s.clear_status(jug, DISARM);
    if w.s.stat_value(jug, S_CRIT_CHANCE) != 35 || w.s.stat_value(jug, S_CRIT_MULT) != 140 {
        return Err(format!("crit {} percent x{} want 35 and 140", w.s.stat_value(jug, S_CRIT_CHANCE), w.s.stat_value(jug, S_CRIT_MULT)));
    }
    let target = w.creep(1, 1, 0)?;
    w.s.set_health(target, fixed::from_int(30_000), fixed::from_int(30_000));
    let mut hits = Vec::new();
    for _ in 0..300 {
        let before = w.s.hp(target);
        w.strike(jug, target)?;
        hits.push((before - w.s.hp(target)) as f64);
    }
    let low = hits.iter().cloned().fold(f64::MAX, f64::min);
    let crits = hits.iter().filter(|&&d| d > low * 1.3).count();
    if !(60..=150).contains(&crits) {
        return Err(format!("{crits} critical strikes in 300 attacks, want about 105"));
    }
    let top = hits.iter().cloned().fold(0.0, f64::max);
    near("critical multiplier", top / low, 1.4, 0.03)?;
    Ok(())
}

/// Omnislash: a blink to the target, invulnerability for 3 s and a strike on a nearby enemy every 9 ticks.
pub fn juggernaut_omni_slash(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let jug = rank_one_hero(&mut w, "juggernaut", 6, &["juggernaut_omni_slash"])?;
    let a = w.creep(1, 6, 0)?;
    let b = w.creep(1, 8, 2)?;
    w.cast(jug, "juggernaut_omni_slash", a as i32, 0, 0)?;
    w.run(5 + 1);
    let (jx, _) = w.s.position_fixed(jug);
    let (ax, _) = w.s.position_fixed(a);
    if (fixed::to_float(ax - jx)).abs() > 3.0 {
        return Err("the blink did not reach the target".into());
    }
    if w.s.status_left(jug, INVULNERABLE) == 0 {
        return Err("no invulnerability during the slash".into());
    }
    w.run(60);
    if 550.0 - w.hp(a) + 550.0 - w.hp(b) < 150.0 {
        return Err("the slash dealt under 150 damage in 66 ticks".into());
    }
    Ok(())
}

/// Arc Lightning: 105 magical damage to the target, then bouncing to the nearest unit every 0.25 s, 5 more times.
pub fn zeus_arc_lightning(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let zeus = rank_one_hero(&mut w, "zuus", 1, &["zuus_arc_lightning"])?;
    let line: Vec<usize> = (0..4).map(|k| w.creep(1, 3 + 4 * k, 0)).collect::<Result<_, _>>()?;
    w.cast(zeus, "zuus_arc_lightning", line[0] as i32, 0, 0)?;
    near("first target at once", 550.0 - w.hp(line[0]), 105.0, 0.6)?;
    if 550.0 - w.hp(line[1]) > 1.0 {
        return Err("the second target was hit before the 4 tick jump delay".into());
    }
    w.run(5 + 4 * 6);
    let total: f64 = line.iter().map(|&u| 550.0 - w.hp(u)).sum();
    near("total over 6 hits", total, 6.0 * 105.0, 3.0)?;
    Ok(())
}

/// Lightning Bolt: 140 magical damage and a 0.35 s stun.
pub fn zeus_lightning_bolt(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let zeus = rank_one_hero(&mut w, "zuus", 1, &["zuus_lightning_bolt"])?;
    let target = w.creep(1, 5, 0)?;
    w.cast(zeus, "zuus_lightning_bolt", target as i32, 0, 0)?;
    w.run(5);
    near("bolt damage", 550.0 - w.hp(target), 140.0, 0.6)?;
    if w.s.status_left(target, STUN) < 1 {
        return Err("no ministun".into());
    }
    Ok(())
}

/// Thundergod's Wrath: 275 magical damage to every enemy hero in the world and to nothing else.
pub fn zeus_thundergods_wrath(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let zeus = rank_one_hero(&mut w, "zuus", 6, &["zuus_thundergods_wrath"])?;
    let hero = w.hero("antimage", 1, 10, 10)?;
    let creep = w.creep(1, 10, -10)?;
    let ally = w.hero("axe", 0, -10, -10)?;
    let (h0, a0) = (w.hp(hero), w.hp(ally));
    w.cast(zeus, "zuus_thundergods_wrath", -1, 0, 0)?;
    w.run(8);
    near("enemy hero hp loss", h0 - w.hp(hero), 275.0 * 0.75, 2.5)?;
    if w.hp(creep) < 549.9 || a0 - w.hp(ally) > 0.5 {
        return Err("the wrath hit a creep or an ally".into());
    }
    Ok(())
}

/// Shrapnel: three charges, a zone after 1.2 s that deals 30 magical damage a second and slows, a charge back every 35 s.
pub fn sniper_shrapnel(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let sniper = rank_one_hero(&mut w, "sniper", 1, &["sniper_shrapnel"])?;
    let target = w.creep(1, 10, 0)?;
    let (x, y) = w.at(10, 0);
    w.s.grant_energy(sniper, 1000).map_err(|r| r.name().to_string())?;
    for _ in 0..3 {
        w.cast(sniper, "sniper_shrapnel", -1, x, y).map_err(|e| format!("charge: {e}"))?;
    }
    if w.cast(sniper, "sniper_shrapnel", -1, x, y).is_ok() {
        return Err("a fourth cast went through with three charges".into());
    }
    w.run(15);
    if w.hp(target) < 549.9 {
        return Err("the zone hit before its 1.2 s delay".into());
    }
    w.run(30);
    if 550.0 - w.hp(target) < 30.0 {
        return Err("the zone dealt no damage".into());
    }
    if w.s.slow_multiplier(target) >= fixed::ONE {
        return Err("no slow in the zone".into());
    }
    w.run(560);
    w.s.grant_energy(sniper, 1000).map_err(|r| r.name().to_string())?;
    w.cast(sniper, "sniper_shrapnel", -1, x, y).map_err(|e| format!("restored charge: {e}"))?;
    Ok(())
}

/// Headshot: 40 percent of attacks add 20 physical damage.
pub fn sniper_headshot(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let sniper = rank_one_hero(&mut w, "sniper", 1, &["sniper_headshot"])?;
    w.s.clear_status(sniper, DISARM);
    let target = w.creep(1, 4, 0)?;
    w.s.set_health(target, fixed::from_int(30_000), fixed::from_int(30_000));
    let mut hits = Vec::new();
    for _ in 0..300 {
        let before = w.s.hp(target);
        w.strike(sniper, target)?;
        hits.push((before - w.s.hp(target)) as f64);
    }
    let low = hits.iter().cloned().fold(f64::MAX, f64::min);
    let procs = hits.iter().filter(|&&d| d > low + 5.0 * fixed::ONE as f64).count();
    if !(75..=165).contains(&procs) {
        return Err(format!("{procs} headshots in 300 attacks, want about 120"));
    }
    Ok(())
}

/// Take Aim: 160 units of attack range at once, 75 more while active.
pub fn sniper_take_aim(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let sniper = rank_one_hero(&mut w, "sniper", 1, &["sniper_take_aim"])?;
    if w.s.stat_value(sniper, S_RANGE) != 2500 {
        return Err(format!("passive range {} want 2500 milli (160 units)", w.s.stat_value(sniper, S_RANGE)));
    }
    w.cast(sniper, "sniper_take_aim", -1, 0, 0)?;
    if w.s.stat_value(sniper, S_RANGE) != 2500 + 1172 {
        return Err(format!("active range {} want 3672", w.s.stat_value(sniper, S_RANGE)));
    }
    w.run(60);
    if w.s.stat_value(sniper, S_RANGE) != 2500 {
        return Err("the active range outlived 3 s".into());
    }
    Ok(())
}

/// Assassinate: a 2 s cast point, then 300 magical damage.
pub fn sniper_assassinate(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let sniper = rank_one_hero(&mut w, "sniper", 6, &["sniper_assassinate"])?;
    let target = w.hero("antimage", 1, 9, 0)?;
    let before = w.hp(target);
    let slot = w.s.ability_slot(sniper, "sniper_assassinate").ok_or("no assassinate")?;
    if !w.s.cast_ability(sniper, slot, target as i32, 0, 0) {
        return Err(format!("cast refused: {}", w.s.stats.last_refusal.name()));
    }
    w.run(30);
    if before - w.hp(target) > 0.5 || w.s.channel_left(sniper) <= 0 {
        return Err("the cast point ended before 2 s or damage landed early".into());
    }
    w.run(20);
    near("assassinate damage", before - w.hp(target), 300.0 * 0.75, 3.5)
}

/// Impale: a line 140 units wide; units on it take 105 magical damage and are stunned, units off it are not.
pub fn lion_impale(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let lion = rank_one_hero(&mut w, "lion", 1, &["lion_impale"])?;
    let on_near = w.creep(1, 4, 0)?;
    let on_far = w.creep(1, 11, 0)?;
    let off = w.creep(1, 6, 3)?;
    let beyond = w.creep(1, 12, 5)?;
    let (x, y) = w.at(10, 0);
    w.cast(lion, "lion_impale", -1, x, y)?;
    for (name, u) in [("near", on_near), ("far", on_far)] {
        near(&format!("{name} unit on the line"), 550.0 - w.hp(u), 105.0, 0.6)?;
        if w.s.status_left(u, STUN) < 20 {
            return Err(format!("{name} unit on the line is not stunned"));
        }
    }
    if w.hp(off) < 549.9 || w.hp(beyond) < 549.9 {
        return Err("a unit off the line was hit".into());
    }
    Ok(())
}

/// Hex: the target cannot attack or cast and crawls at 120 for 2 s.
pub fn lion_voodoo(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let lion = rank_one_hero(&mut w, "lion", 1, &["lion_voodoo"])?;
    let target = w.hero("axe", 1, 5, 0)?;
    w.level_up(target, 1);
    w.learn(target, "axe_battle_hunger", 1)?;
    w.cast(lion, "lion_voodoo", target as i32, 0, 0)?;
    if w.s.status_left(target, HEX) != 32 {
        return Err(format!("hex ticks {} want 32", w.s.status_left(target, HEX)));
    }
    let slot = w.s.ability_slot(target, "axe_battle_hunger").ok_or("axe holds no battle hunger")?;
    if w.s.cast_ability(target, slot, lion as i32, 0, 0) || w.s.stats.last_refusal.name() != "silenced" {
        return Err("a hexed unit cast a spell".into());
    }
    Ok(())
}

/// Mana Drain: mana moves from the target to the caster every 2 ticks until the target leaves 1100 units.
pub fn lion_mana_drain(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let lion = rank_one_hero(&mut w, "lion", 1, &["lion_mana_drain"])?;
    let target = w.hero("crystal_maiden", 1, 6, 0)?;
    w.s.spend_energy(lion, w.s.mana_of(lion) - 10).map_err(|r| r.name().to_string())?;
    let (t0, l0) = (w.s.mana_of(target), w.s.mana_of(lion));
    w.cast(lion, "lion_mana_drain", target as i32, 0, 0)?;
    w.run(5 + 32);
    let moved = t0 - w.s.mana_of(target);
    if moved < 25 || w.s.mana_of(lion) - l0 < moved - 2 {
        return Err(format!("drained {moved}, caster gained {}", w.s.mana_of(lion) - l0));
    }
    let (fx, fy) = w.at(30, 0);
    w.s.set_position(target, fx, fy);
    w.run(4);
    if w.s.channel_left(lion) != 0 {
        return Err("the drain did not break at 1100 units".into());
    }
    Ok(())
}

/// Finger of Death: 600 magical damage.
pub fn lion_finger_of_death(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let lion = rank_one_hero(&mut w, "lion", 6, &["lion_finger_of_death"])?;
    let target = w.hero("antimage", 1, 6, 0)?;
    let before = w.hp(target);
    w.cast(lion, "lion_finger_of_death", target as i32, 0, 0)?;
    w.run(6);
    near("finger damage", before - w.hp(target), 600.0 * 0.75, 1.0)
}

/// Purification: heals the target 90 and deals 90 pure damage to enemies within 260 units of it.
pub fn omniknight_purification(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let omni = rank_one_hero(&mut w, "omniknight", 1, &["omniknight_purification"])?;
    let ally = w.hero("axe", 0, 3, 0)?;
    let enemy = w.creep(1, 4, 0)?;
    let far = w.creep(1, 9, 0)?;
    w.s.hurt(ally, fixed::from_int(200));
    let before = w.hp(ally);
    w.cast(omni, "omniknight_purification", ally as i32, 0, 0)?;
    w.run(5);
    near("heal", w.hp(ally) - before, 90.0, 1.5)?;
    near("pure damage", 550.0 - w.hp(enemy), 90.0, 0.6)?;
    if w.hp(far) < 549.9 {
        return Err("a distant creep took purification damage".into());
    }
    Ok(())
}

/// Martyr: 8 health regeneration and 60 magic resistance for 5 s on an ally.
pub fn omniknight_martyr(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let omni = rank_one_hero(&mut w, "omniknight", 1, &["omniknight_martyr"])?;
    let ally = w.hero("axe", 0, 3, 0)?;
    w.cast(omni, "omniknight_martyr", ally as i32, 0, 0)?;
    w.run(5);
    if w.s.stat_value(ally, S_HEALTH_REGEN) != 800 || w.s.stat_value(ally, S_MAGIC_RESIST) != 60 {
        return Err(format!("martyr regen {} resist {} want 800 and 60", w.s.stat_value(ally, S_HEALTH_REGEN), w.s.stat_value(ally, S_MAGIC_RESIST)));
    }
    w.run(16 * 5);
    if w.s.stat_value(ally, S_MAGIC_RESIST) != 0 {
        return Err("martyr outlived 5 s".into());
    }
    Ok(())
}

/// Guardian Angel: allies in 700 units take no physical damage for 4 s, spells still hurt.
pub fn omniknight_guardian_angel(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let omni = rank_one_hero(&mut w, "omniknight", 6, &["omniknight_guardian_angel"])?;
    let ally = w.hero("axe", 0, 4, 0)?;
    w.cast(omni, "omniknight_guardian_angel", -1, 0, 0)?;
    if w.s.status_left(ally, PHYSICAL_IMMUNITY) != 64 {
        return Err(format!("immunity ticks {} want 64", w.s.status_left(ally, PHYSICAL_IMMUNITY)));
    }
    let foe = w.creep(1, 5, 0)?;
    w.s.clear_status(foe, DISARM);
    let before = w.hp(ally);
    w.strike(foe, ally)?;
    if before - w.hp(ally) > 0.5 {
        return Err("a weapon hit hurt an ally under Guardian Angel".into());
    }
    Ok(())
}

/// Talents: the level 10, 15, 20 and 25 picks of Axe change what they name, one pick per tier.
pub fn talent_picks(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let axe = rank_one_hero(&mut w, "axe", 9, &["axe_battle_hunger"])?;
    if w.s.choose_talent(axe, 0, 0).is_ok() {
        return Err("a level 10 talent was chosen at level 9".into());
    }
    w.level_up(axe, 15);
    w.s.choose_talent(axe, 0, 0).map_err(|r| r.name().to_string())?;
    if w.s.choose_talent(axe, 0, 1).is_ok() {
        return Err("both talents of a tier were chosen".into());
    }
    w.s.choose_talent(axe, 1, 0).map_err(|r| r.name().to_string())?;
    let target = w.creep(1, 4, 0)?;
    let before = w.hp(target);
    w.cast(axe, "axe_battle_hunger", target as i32, 0, 0)?;
    w.run(5 + 17);
    near("battle hunger with the +8 talent", before - w.hp(target), 20.0, 1.0)?;
    w.level_up(axe, 20);
    let strength = w.s.hero_attributes(axe)[0];
    w.s.choose_talent(axe, 2, 0).map_err(|r| r.name().to_string())?;
    let gained = w.s.hero_attributes(axe)[0] - strength;
    if gained != 15 {
        return Err(format!("the +15 strength talent added {gained}"));
    }
    Ok(())
}

/// Casting out of range walks the caster into range and casts there.
pub fn cast_walks_into_range(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let axe = rank_one_hero(&mut w, "axe", 1, &["axe_battle_hunger"])?;
    w.s.clear_status(axe, DISARM);
    let target = w.creep(1, 13, 0)?;
    let mana = w.s.mana_of(axe);
    w.cast(axe, "axe_battle_hunger", target as i32, 0, 0)?;
    if !w.s.cast_pending(axe) || w.s.mana_of(axe) != mana {
        return Err("an out of range cast did not wait for the walk".into());
    }
    w.run(200);
    if w.s.cast_pending(axe) || w.s.mana_of(axe) >= mana {
        return Err("the walk never ended in a cast".into());
    }
    Ok(())
}

/// Skill ranks come at levels 1, 3, 5 and 7 for a basic ability and 6, 12 and 18 for an ultimate.
pub fn skill_level_gates(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let axe = w.hero("axe", 0, 0, 0)?;
    w.level_up(axe, 2);
    let slot = w.s.ability_slot(axe, "axe_battle_hunger").ok_or("axe holds no battle hunger")?;
    w.s.skill_up(axe, slot).map_err(|r| r.name().to_string())?;
    if w.s.skill_up(axe, slot).is_ok() {
        return Err("rank 2 of a basic ability was learned at level 2".into());
    }
    w.level_up(axe, 3);
    w.s.skill_up(axe, slot).map_err(|r| r.name().to_string())?;
    let ult = w.s.ability_slot(axe, "axe_culling_blade").ok_or("axe holds no culling blade")?;
    if w.s.skill_up(axe, ult).is_ok() {
        return Err("the ultimate was learned before level 6".into());
    }
    Ok(())
}
