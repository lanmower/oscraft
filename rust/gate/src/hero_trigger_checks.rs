use mod_host::ModHost;
use sim::fixed;

use crate::checks::{kind, pack_run, role};

const DISARM: usize = 7;
const S_MAGIC_RESIST: usize = 11;
const S_ATTACK_SPEED: usize = 12;
const S_ATTACK_BONUS: usize = 31;
const STUN: usize = 0;
const ROOT: usize = 3;

struct Arena {
    host: ModHost,
    x: i32,
    y: i32,
}

fn arena() -> Result<Arena, String> {
    let host = pack_run::start()?;
    let (x, y) = {
        let sim = host.sim();
        let (w, h) = (sim.grid().w as i32, sim.grid().h as i32);
        let reach = 24;
        let standing: Vec<(i32, i32)> = (0..sim.count())
            .filter(|&u| sim.alive(u))
            .map(|u| {
                let (x, y) = sim.position_fixed(u);
                (x >> 16, y >> 16)
            })
            .collect();
        let mut best: Option<(i32, i32, i32)> = None;
        for cy in (reach..h - reach).step_by(4) {
            for cx in (reach..w - reach).step_by(4) {
                if !(-reach / 2..=reach / 2).all(|d| sim.grid().walkable(cx + d, cy) && sim.grid().walkable(cx, cy + d)) {
                    continue;
                }
                let nearest = standing.iter().map(|&(ux, uy)| (ux - cx).pow(2) + (uy - cy).pow(2)).min().unwrap_or(i32::MAX);
                if best.is_none_or(|b| nearest > b.0) {
                    best = Some((nearest, cx, cy));
                }
            }
        }
        let (_, cx, cy) = best.ok_or("no open arena on the map")?;
        (fixed::from_int(cx), fixed::from_int(cy))
    };
    Ok(Arena { host, x, y })
}

impl Arena {
    fn spot(&self, dx: i32, dy: i32) -> (i32, i32) {
        (self.x + fixed::from_int(dx), self.y + fixed::from_int(dy))
    }

    fn hero(&mut self, name: &str, level: u32, dx: i32, dy: i32) -> Result<usize, String> {
        let (x, y) = self.spot(dx, dy);
        let mut sim = self.host.sim_mut();
        let k = kind(&sim, name)?;
        let h = sim.spawn_attributed(0, k, x, y).ok_or_else(|| format!("spawn_attributed refused {name}"))?;
        let need = level.checked_sub(2).and_then(|n| sim.xp_curve().get(n as usize).copied()).unwrap_or(0);
        let gap = need.saturating_sub(sim.xp(h));
        sim.add_xp(h, gap);
        sim.apply_status(h, DISARM, 1_000_000, fixed::ONE);
        drop(sim);
        self.run(8)?;
        Ok(h)
    }

    fn foe(&mut self, player: u8, name: &str, dx: i32, dy: i32) -> Result<usize, String> {
        let (x, y) = self.spot(dx, dy);
        let mut sim = self.host.sim_mut();
        let k = kind(&sim, name)?;
        let u = sim.spawn_for(player, k, x, y).ok_or_else(|| format!("spawn_for refused {name}"))?;
        sim.apply_status(u, DISARM, 1_000_000, fixed::ONE);
        sim.apply_status(u, ROOT, 1_000_000, fixed::ONE);
        sim.set_health(u, fixed::from_int(30_000), fixed::from_int(30_000));
        Ok(u)
    }

    fn learn(&mut self, h: usize, ability: &str, ranks: usize) -> Result<usize, String> {
        let mut sim = self.host.sim_mut();
        let slot = sim.ability_slot(h, ability).ok_or_else(|| format!("hero holds no {ability}"))?;
        for _ in 0..ranks {
            sim.rank_up(h, slot).map_err(|r| format!("skill_up {ability}: {}", r.name()))?;
        }
        Ok(slot)
    }

    fn cast(&mut self, h: usize, slot: usize, target: i32, x: i32, y: i32) -> Result<(), String> {
        let took = self.host.sim_mut().cast_ability(h, slot, target, x, y);
        if !took {
            return Err(format!("cast refused: {}", self.host.sim().stats.last_refusal.name()));
        }
        Ok(())
    }

    fn run(&mut self, ticks: u32) -> Result<(), String> {
        pack_run::advance(&mut self.host, ticks)?;
        let broken = self.host.disabled_hooks();
        if !broken.is_empty() {
            return Err(format!("pack hooks switched off: {broken:?}"));
        }
        Ok(())
    }

    fn hp(&self, u: usize) -> f64 {
        self.host.sim().hp(u) as f64 / fixed::ONE as f64
    }
}

fn close(what: &str, got: f64, want: f64, tol: f64) -> Result<(), String> {
    if (got - want).abs() > tol {
        return Err(format!("{what} {got:.2} want {want:.2} +-{tol}"));
    }
    Ok(())
}

pub fn huskar_berserkers_blood(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let h = a.hero("huskar", 1, 0, 0)?;
    a.run(8)?;
    let (speed0, resist0) = (a.host.sim().stat(h, S_ATTACK_SPEED), a.host.sim().stat(h, S_MAGIC_RESIST));
    a.learn(h, "huskar_berserker_s_blood", 1)?;
    a.run(8)?;
    let (speed1, resist1) = (a.host.sim().stat(h, S_ATTACK_SPEED), a.host.sim().stat(h, S_MAGIC_RESIST));
    if speed1 - speed0 != 14 || resist1 - resist0 != 0 {
        return Err(format!("full life: attack speed +{} want 14, magic resistance +{} want 0", speed1 - speed0, resist1 - resist0));
    }
    let most = a.host.sim().hp_max(h);
    pack_run::hurt_to_life(&mut a.host, h, most / 2);
    a.run(8)?;
    let pct = (a.host.sim().hp(h) as i64 * 100 / most as i64) as i32;
    let steps = ((100 - pct) / 7).clamp(1, 14);
    let (speed2, resist2) = (a.host.sim().stat(h, S_ATTACK_SPEED) - speed0, a.host.sim().stat(h, S_MAGIC_RESIST) - resist0);
    if speed2 != 14 * steps || resist2 != 4 * (3 * steps / 4) {
        return Err(format!("{steps} steps at {pct}%: attack speed +{speed2} want {}, magic resistance +{resist2} want {}", 14 * steps, 4 * (3 * steps / 4)));
    }
    Ok(())
}

pub fn huskar_life_break(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let h = a.hero("huskar", 6, 0, 0)?;
    let foe = a.foe(1, role("unit.creep")?, 4, 0)?;
    let slot = a.learn(h, "huskar_life_break", 1)?;
    let (mine, theirs) = (a.hp(h), a.hp(foe));
    a.cast(h, slot, foe as i32, 0, 0)?;
    a.run(6)?;
    close("caster life paid", mine - a.hp(h), mine * 0.35, mine * 0.02)?;
    close("target life taken", theirs - a.hp(foe), theirs * 0.35, theirs * 0.02)
}

pub fn huskar_burning_spear(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let h = a.hero("huskar", 1, 0, 0)?;
    let foe = a.foe(1, role("unit.creep")?, 1, 0)?;
    a.learn(h, "huskar_burning_spear", 1)?;
    let before = a.hp(h);
    a.host.sim_mut().clear_status(h, DISARM);
    a.host.sim_mut().order_attack(h, foe).map_err(|r| format!("attack refused: {}", r.name()))?;
    let mut lowest = before;
    for _ in 0..90 {
        a.run(1)?;
        lowest = lowest.min(a.hp(h));
    }
    let stacks: i64 = (1..=8).map(|n| a.host.sim().counter(foe, &format!("spear_{n}"))).sum();
    if stacks == 0 {
        return Err("no burning spear stack on the target after 90 ticks of attacks".into());
    }
    if before - lowest < 14.0 {
        return Err(format!("caster paid {:.1} life at most, want at least 15 for one attack", before - lowest));
    }
    Ok(())
}

pub fn huskar_inner_vitality(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let h = a.hero("huskar", 1, 0, 0)?;
    let slot = a.learn(h, "huskar_inner_vitality", 1)?;
    let most = a.host.sim().hp_max(h);
    pack_run::hurt_to_life(&mut a.host, h, most / 2);
    a.run(2)?;
    let before = a.hp(h);
    a.cast(h, slot, h as i32, 0, 0)?;
    a.run(32)?;
    if a.host.sim().counter(h, "vitality_due") == 0 {
        return Err("the cast left no vitality window on the target".into());
    }
    if a.hp(h) - before < 21.0 {
        return Err(format!("healed {:.1} in 2 s, want at least the 20 of the flat regeneration and the attribute part", a.hp(h) - before));
    }
    Ok(())
}

pub fn earthshaker_echo_slam(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let h = a.hero("earthshaker", 6, 0, 0)?;
    let first = a.foe(1, role("unit.creep")?, 3, 0)?;
    let _second = a.foe(1, role("unit.creep")?, 5, 0)?;
    let _third = a.foe(1, role("unit.creep")?, 0, 4)?;
    let slot = a.learn(h, "earthshaker_echo_slam", 1)?;
    let before = a.hp(first);
    a.cast(h, slot, -1, 0, 0)?;
    a.run(6)?;
    close("slam plus three echoes on a unit with two others near", before - a.hp(first), 160.0 + 3.0 * 40.0, 3.0)
}

fn strike_hits(a: &mut Arena, h: usize, foe: usize, want: usize, from: f64) -> Result<Vec<f64>, String> {
    a.host.sim_mut().order_attack(h, foe).map_err(|r| format!("attack refused: {}", r.name()))?;
    let mut hits = Vec::new();
    let mut last = from;
    for _ in 0..400 {
        a.run(1)?;
        let now = a.hp(foe);
        if now < last {
            hits.push(last - now);
            last = now;
        }
        if hits.len() == want {
            break;
        }
    }
    if hits.len() < want {
        return Err(format!("{} hits landed in 400 ticks, want {want}", hits.len()));
    }
    Ok(hits)
}

pub fn earthshaker_enchant_totem(_image: &[u8]) -> Result<(), String> {
    let mut plain = arena()?;
    let h = plain.hero("earthshaker", 1, 0, 0)?;
    plain.host.sim_mut().clear_status(h, DISARM);
    let foe = plain.foe(1, role("unit.creep")?, 1, 0)?;
    let from = plain.hp(foe);
    let base = strike_hits(&mut plain, h, foe, 1, from)?[0];
    let mut a = arena()?;
    let h = a.hero("earthshaker", 1, 0, 0)?;
    let foe = a.foe(1, role("unit.creep")?, 1, 0)?;
    let slot = a.learn(h, "earthshaker_enchant_totem", 1)?;
    let from = a.hp(foe);
    a.cast(h, slot, -1, 0, 0)?;
    a.run(2)?;
    let carried = a.host.sim().stat(h, S_ATTACK_BONUS);
    a.host.sim_mut().clear_status(h, DISARM);
    let hits = strike_hits(&mut a, h, foe, 2, from)?;
    if carried != 100 {
        return Err(format!("attack bonus stat {carried} want 100"));
    }
    close(&format!("first hit against twice a plain hit {base:.1}, hits {hits:?}"), hits[0], 2.0 * base, 0.02 * base)?;
    close("second hit against a plain hit", hits[1], base, 0.02 * base)?;
    if a.host.sim().stat(h, S_ATTACK_BONUS) != 0 {
        return Err("the attack bonus survived the attack that carried it".into());
    }
    Ok(())
}

pub fn jakiro_ice_path(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let h = a.hero("jakiro", 1, 0, 0)?;
    let foe = a.foe(1, role("unit.creep")?, 6, 0)?;
    let slot = a.learn(h, "jakiro_ice_path", 1)?;
    let (tx, ty) = a.spot(12, 0);
    let before = a.hp(foe);
    a.cast(h, slot, -1, tx, ty)?;
    a.run(6)?;
    if before - a.hp(foe) > 5.0 || a.host.sim().status_left(foe, STUN) > 0 {
        return Err(format!("the path hit its target before it formed: lost {:.1}, stun {}", before - a.hp(foe), a.host.sim().status_left(foe, STUN)));
    }
    a.run(14)?;
    close("ice path damage", before - a.hp(foe), 50.0, 4.0)?;
    if a.host.sim().status_left(foe, STUN) <= 0 {
        return Err("the formed path did not stun its target".into());
    }
    Ok(())
}

pub fn jakiro_macropyre(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let h = a.hero("jakiro", 6, 0, 0)?;
    let foe = a.foe(1, role("unit.creep")?, 3, 0)?;
    let slot = a.learn(h, "jakiro_macropyre", 1)?;
    let (tx, ty) = a.spot(12, 0);
    let before = a.hp(foe);
    a.cast(h, slot, -1, tx, ty)?;
    a.run(100)?;
    let lost = before - a.hp(foe);
    if lost < 50.0 * 8.0 {
        return Err(format!("macropyre dealt {lost:.1} in 100 ticks, want at least eight pulses of 50"));
    }
    Ok(())
}

pub fn jakiro_dual_breath(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let h = a.hero("jakiro", 1, 0, 0)?;
    let foe = a.foe(1, role("unit.creep")?, 5, 0)?;
    let slot = a.learn(h, "jakiro_dual_breath", 1)?;
    let (tx, ty) = a.spot(10, 0);
    let before = a.hp(foe);
    a.cast(h, slot, -1, tx, ty)?;
    a.run(60)?;
    if before - a.hp(foe) < 40.0 {
        return Err(format!("dual breath dealt {:.1} in 60 ticks, want at least two ticks of 20", before - a.hp(foe)));
    }
    if a.host.sim().stat(foe, S_ATTACK_SPEED) >= 0 {
        return Err("the breath left no attack speed slow".into());
    }
    Ok(())
}

pub fn lycan_auras_reach_units(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let h = a.hero("lycan", 1, 0, 0)?;
    let wolf = {
        let (x, y) = a.spot(3, 0);
        let mut sim = a.host.sim_mut();
        let k = kind(&sim, "lycan_wolf_4")?;
        sim.spawn_for(0, k, x, y).ok_or("spawn_for refused a wolf")?
    };
    a.learn(h, "lycan_feral_impulse", 1)?;
    let howl = a.learn(h, "lycan_howl", 1)?;
    a.run(24)?;
    if a.host.sim().stat(wolf, S_ATTACK_SPEED) != 15 {
        return Err(format!("feral impulse gave the wolf attack speed {} want 15", a.host.sim().stat(wolf, S_ATTACK_SPEED)));
    }
    a.cast(h, howl, -1, 0, 0)?;
    a.run(4)?;
    let (hero_bonus, wolf_bonus) = (a.host.sim().stat(h, 0), a.host.sim().stat(wolf, 0));
    let want = 22 * 15 / 100 + 22 * 5 / 100;
    if wolf_bonus != want || hero_bonus <= 0 {
        return Err(format!("howl and feral impulse: wolf damage bonus {wolf_bonus} want {want}, hero bonus {hero_bonus} want a positive one"));
    }
    Ok(())
}

pub fn lycan_shapeshift_speed(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let h = a.hero("lycan", 6, 0, 0)?;
    let wolf = {
        let (x, y) = a.spot(3, 0);
        let mut sim = a.host.sim_mut();
        let k = kind(&sim, "lycan_wolf_4")?;
        sim.spawn_for(0, k, x, y).ok_or("spawn_for refused a wolf")?
    };
    let slot = a.learn(h, "lycan_shapeshift", 1)?;
    a.cast(h, slot, -1, 0, 0)?;
    a.run(12)?;
    for (who, unit) in [("hero", h), ("wolf", wolf)] {
        let until = a.host.sim().counter(unit, "speed_lock_until");
        let speed = a.host.sim().counter(unit, "speed_lock_value");
        if until <= 12 || speed != 650 {
            return Err(format!("{who}: speed lock until {until} at {speed}, want 650 held for the form"));
        }
    }
    Ok(())
}
