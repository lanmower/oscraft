use crate::checks::{fresh, kind};
use crate::checks::{role};
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
const BLIND: usize = 14;
const REVEALED: usize = 15;
const INVISIBLE: usize = 4;
const S_DAMAGE: usize = 0;
const S_ARMOR: usize = 1;
const S_RANGE: usize = 4;
const S_ATTACK_SPEED: usize = 12;
const S_SLOT_13: usize = 13;
const S_HEALTH_REGEN: usize = 14;
const S_MAGIC_RESIST: usize = 11;
const S_SPEED_PCT: usize = 19;
const S_CRIT_CHANCE: usize = 6;
const S_CRIT_MULT: usize = 7;
const S_CLEAVE: usize = 8;
const S_STRENGTH_BONUS: usize = 22;

struct World {
    s: Sim,
    cx: i32,
    cy: i32,
}

fn world() -> Result<World, String> {
    let scenario = crate::checks::pack_dir();
    let image = osdt_data::generate(&scenario).map_err(|e| format!("moba scenario image: {e:#}"))?;
    let mut s = fresh(&image)?;
    let rules = std::fs::read_to_string(scenario.join(crate::checks::RULES_FILE)).map_err(|e| e.to_string())?;
    if !s.set_attr_rules_text(&rules) {
        return Err("the pack rules file has a bad line".into());
    }
    for u in 0..s.count() {
        if s.alive(u) {
            s.apply_status(u, STUN, 1_000_000, fixed::ONE);
        }
    }
    let (w, h) = (s.grid().w as i32, s.grid().h as i32);
    let reach = 16;
    let open = |cx: i32, cy: i32| (-reach..=reach).all(|dy| (-reach..=reach).all(|dx| s.grid().walkable(cx + dx, cy + dy)));
    let near = |cx: i32, cy: i32| -> Vec<usize> {
        (0..s.count())
            .filter(|&u| {
                if !s.alive(u) {
                    return false;
                }
                let (x, y) = s.position_fixed(u);
                let (dx, dy) = ((x >> 16) - cx, (y >> 16) - cy);
                dx * dx + dy * dy <= (reach + 8) * (reach + 8)
            })
            .collect()
    };
    let mut best: Option<(usize, i32, i32)> = None;
    'scan: for cy in (reach..h - reach).step_by(2) {
        for cx in (reach..w - reach).step_by(2) {
            if !open(cx, cy) {
                continue;
            }
            let n = near(cx, cy).len();
            if best.is_none_or(|b| n < b.0) {
                best = Some((n, cx, cy));
            }
            if n == 0 {
                break 'scan;
            }
        }
    }
    let Some((_, cx, cy)) = best else {
        return Err(format!("no open {}x{} arena on the {w}x{h} map", 2 * reach + 1, 2 * reach + 1));
    };
    for u in near(cx, cy) {
        s.remove_unit(u);
    }
    Ok(World { s, cx, cy })
}

impl World {
    fn at(&self, dx: i32, dy: i32) -> (i32, i32) {
        (fixed::from_int(self.cx + dx), fixed::from_int(self.cy + dy))
    }

    fn hero(&mut self, name: &str, player: u8, dx: i32, dy: i32) -> Result<usize, String> {
        let k = kind(&self.s, name)?;
        let (x, y) = self.at(dx, dy);
        let h = self.s.spawn_attributed(player, k, x, y).ok_or_else(|| format!("spawn_attributed refused {name} at ({dx},{dy})"))?;
        self.s.apply_status(h, DISARM, 1_000_000, fixed::ONE);
        Ok(h)
    }

    fn creep(&mut self, player: u8, dx: i32, dy: i32) -> Result<usize, String> {
        let k = kind(&self.s, role("unit.creep")?)?;
        let (x, y) = self.at(dx, dy);
        let u = self.s.spawn_for(player, k, x, y).ok_or_else(|| format!("spawn_for refused a creep at ({dx},{dy})"))?;
        self.s.apply_status(u, DISARM, 1_000_000, fixed::ONE);
        Ok(u)
    }

    fn level_up(&mut self, h: usize, level: u32) {
        let need = level.checked_sub(2).and_then(|k| self.s.xp_curve().get(k as usize).copied()).unwrap_or(0);
        let gap = need.saturating_sub(self.s.xp(h));
        self.s.add_xp(h, gap);
    }

    fn learn(&mut self, h: usize, ability: &str, ranks: usize) -> Result<usize, String> {
        let slot = self.s.ability_slot(h, ability).ok_or_else(|| format!("hero holds no {ability}"))?;
        for _ in 0..ranks {
            self.s.rank_up(h, slot).map_err(|r| format!("skill_up {ability}: {}", r.name()))?;
        }
        Ok(slot)
    }

    fn cast(&mut self, h: usize, ability: &str, target: i32, x: i32, y: i32) -> Result<(), String> {
        let slot = self.s.ability_slot(h, ability).ok_or_else(|| format!("hero holds no {ability}"))?;
        if self.s.cast_ability(h, slot, target, x, y) {
            let channelled = ability == role("hero.m.s3")? || ability == role("hero.o.s3")?;
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

    fn strike(&mut self, a: usize, b: usize) -> Result<(), String> {
        for _ in 0..200 {
            if self.s.order_attack(a, b).is_ok() {
                while self.s.hit_in_flight(a) {
                    self.s.tick();
                }
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

fn rank_one_unit(w: &mut World, name: &str, level: u32, abilities: &[&str]) -> Result<usize, String> {
    let h = w.hero(name, 0, 0, 0)?;
    w.level_up(h, level);
    for a in abilities {
        w.learn(h, a, 1)?;
    }
    Ok(h)
}

pub fn spell_a_2(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let axe = rank_one_unit(&mut w, role("hero.a")?, 6, &[role("hero.a.s2")?])?;
    let near_enemy = w.creep(1, 3, 0)?;
    let far_enemy = w.creep(1, 9, 0)?;
    w.cast(axe, role("hero.a.s2")?, -1, 0, 0)?;
    if w.s.status_left(near_enemy, TAUNT) != 32 {
        return Err(format!("taunt ticks {} want 32 (2.0 s)", w.s.status_left(near_enemy, TAUNT)));
    }
    if w.s.status_left(far_enemy, TAUNT) != 0 {
        return Err("an enemy outside 325 units was taunted".into());
    }
    if w.s.stat(axe, S_ARMOR) != 40 {
        return Err(format!("caster armour bonus {} want 40", w.s.stat(axe, S_ARMOR)));
    }
    w.run(40);
    if w.s.status_left(near_enemy, TAUNT) != 0 || w.s.stat(axe, S_ARMOR) != 0 {
        return Err("taunt or armour bonus outlived 2.0 s".into());
    }
    Ok(())
}

pub fn magnataur_skewer(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let m = rank_one_unit(&mut w, "magnataur", 1, &["magnataur_skewer"])?;
    let enemy = w.hero(role("hero.a")?, 1, 6, 1)?;
    let creep = w.creep(1, 6, -1)?;
    let (before, creep_before) = (w.hp(enemy), w.hp(creep));
    w.cast(m, "magnataur_skewer", -1, w.at(14, 0).0, w.at(14, 0).1)?;
    let (ex, _) = w.s.position_fixed(enemy);
    let (mx, _) = w.s.position_fixed(m);
    if ((ex - mx) >> 16).abs() > 1 {
        return Err(format!("the hero on the path ended {} cells from Magnus", (ex - mx) >> 16));
    }
    if w.hp(creep) != creep_before {
        return Err("a non-hero on the path took skewer damage".into());
    }
    if w.s.position_fixed(creep).0 >> 16 != w.cx + 6 {
        return Err("a non-hero on the path was carried".into());
    }
    let lost = before - w.hp(enemy);
    near("skewer damage after the Normal attack factor 0.75", lost, 52.5, 1.0)?;
    if w.s.status_left(enemy, SLOW) <= 0 {
        return Err("the dragged hero was not slowed".into());
    }
    Ok(())
}

pub fn elder_titan_earth_splitter(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let e = rank_one_unit(&mut w, "elder_titan", 6, &["elder_titan_earth_splitter"])?;
    w.s.grant_energy(e, 1000).map_err(|r| r.name().to_string())?;
    let on_line = w.creep(1, 8, 0)?;
    let off_line = w.creep(1, 8, 9)?;
    let (max, other) = (w.hp(on_line), w.hp(off_line));
    w.cast(e, "elder_titan_earth_splitter", -1, w.at(14, 0).0, w.at(14, 0).1)?;
    let lost = max - w.hp(on_line);
    if lost < max * 0.12 || lost > max * 0.36 {
        return Err(format!("earth splitter took {lost:.1} of {max:.1} life, want 12 to 36 percent (35 percent split into physical and magical)"));
    }
    if w.hp(off_line) != other {
        return Err("a unit outside the crack's width took damage".into());
    }
    if w.s.status_left(on_line, SLOW) != 48 {
        return Err(format!("slow ticks {} want 48 (3 s)", w.s.status_left(on_line, SLOW)));
    }
    Ok(())
}

pub fn spell_a_1(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let axe = rank_one_unit(&mut w, role("hero.a")?, 1, &[role("hero.a.s1")?])?;
    let target = w.creep(1, 4, 0)?;
    let before = w.hp(target);
    w.cast(axe, role("hero.a.s1")?, target as i32, 0, 0)?;
    w.run(17);
    near("battle hunger hp after one pulse", before - w.hp(target), 15.0, 1.0)?;
    let slow = w.s.slow_multiplier(target);
    if (slow - 57672).abs() > 2 {
        return Err(format!("slow multiplier {slow} want 57672 (12 percent)"));
    }
    w.run(16 * 12);
    if w.s.status_left(target, SLOW) != 0 {
        return Err("slow outlived 10 s".into());
    }
    Ok(())
}

pub fn spell_a_3(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let axe = rank_one_unit(&mut w, role("hero.a")?, 1, &[role("hero.a.s3")?])?;
    let a = w.creep(1, 1, 0)?;
    let b = w.creep(1, 0, 3)?;
    w.s.set_health(axe, fixed::from_int(30_000), fixed::from_int(30_000));
    w.s.set_health(a, fixed::from_int(30_000), fixed::from_int(30_000));
    w.s.set_health(b, fixed::from_int(30_000), fixed::from_int(30_000));
    let mut spins = 0;
    for hit in 1..=120 {
        let before = w.hp(b);
        w.s.clear_status(a, DISARM);
        w.strike(a, axe).map_err(|e| format!("hit {hit}: {e}"))?;
        w.s.apply_status(a, DISARM, 1_000_000, fixed::ONE);
        let lost = before - w.hp(b);
        if lost > 10.0 {
            near("helix damage to the bystander", lost, 100.0, 20.0)?;
            spins += 1;
        }
    }
    if !(10..=40).contains(&spins) {
        return Err(format!("{spins} helix spins in 120 attacks taken, want about 24 (20 percent)"));
    }
    Ok(())
}

pub fn spell_a_4(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let axe = rank_one_unit(&mut w, role("hero.a")?, 6, &[role("hero.a.s4")?])?;
    let weak = w.creep(1, 2, 0)?;
    w.s.hurt(weak, fixed::from_int(450));
    w.cast(axe, role("hero.a.s4")?, weak as i32, 0, 0)?;
    if w.s.alive(weak) {
        return Err("a creep at 100 hit points survived a 150 damage blade".into());
    }
    if w.s.stat(axe, S_SPEED_PCT) != 30 || w.s.stat(axe, S_ATTACK_SPEED) != 30 {
        return Err(format!("kill bonus speed {} attack speed {} want 30 and 30", w.s.stat(axe, S_SPEED_PCT), w.s.stat(axe, S_ATTACK_SPEED)));
    }
    let mut w = world()?;
    let axe = rank_one_unit(&mut w, role("hero.a")?, 6, &[role("hero.a.s4")?])?;
    let healthy = w.creep(1, 2, 0)?;
    w.cast(axe, role("hero.a.s4")?, healthy as i32, 0, 0)?;
    near("healthy creep after the blade", w.hp(healthy), 550.0 - 150.0, 1.0)?;
    if w.s.stat(axe, S_SPEED_PCT) != 0 {
        return Err("a blade that did not kill granted speed".into());
    }
    Ok(())
}

pub fn crystal_nova(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let cm = rank_one_unit(&mut w, role("hero.m")?, 1, &[role("hero.m.s2")?])?;
    let hero = w.hero(role("hero.l")?, 1, 5, 0)?;
    let creep = w.creep(1, 5, 2)?;
    let (h0, c0) = (w.hp(hero), w.hp(creep));
    let (x, y) = w.at(5, 1);
    w.cast(cm, role("hero.m.s2")?, -1, x, y)?;
    near("hero hp loss", h0 - w.hp(hero), 75.0, 0.6)?;
    near("creep hp loss", c0 - w.hp(creep), 100.0, 0.6)?;
    let slow = w.s.slow_multiplier(hero);
    if (slow - 52429).abs() > 2 || w.s.stat(hero, S_ATTACK_SPEED) != -20 {
        return Err(format!("nova slow {slow} attack speed {} want 52429 and -20", w.s.stat(hero, S_ATTACK_SPEED)));
    }
    Ok(())
}

pub fn crystal_frostbite(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let cm = rank_one_unit(&mut w, role("hero.m")?, 1, &[role("hero.m.s4")?])?;
    w.s.grant_energy(cm, 1000).map_err(|r| r.name().to_string())?;
    let target = w.creep(1, 4, 0)?;
    let before = w.hp(target);
    w.cast(cm, role("hero.m.s4")?, target as i32, 0, 0)?;
    w.run(5);
    if w.s.status_left(target, ROOT) == 0 || w.s.status_left(target, DISARM) == 0 {
        return Err("frostbite left the target free to move or attack".into());
    }
    near("frostbite damage at the cast", before - w.hp(target), 50.0, 1.0)?;
    w.run(40);
    near("frostbite damage over 1.5 s", before - w.hp(target), 150.0, 1.5)?;
    Ok(())
}

pub fn crystal_brilliance_aura(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let cm = rank_one_unit(&mut w, role("hero.m")?, 1, &[role("hero.m.s1")?])?;
    let ally = w.hero(role("hero.a")?, 0, 6, 0)?;
    w.run(4);
    if w.s.stat(cm, S_SLOT_13) != 200 || w.s.stat(ally, S_SLOT_13) != 100 {
        return Err(format!("aura regen caster {} ally {} want 200 and 100 (1.0 a second, doubled for the caster)", w.s.stat(cm, S_SLOT_13), w.s.stat(ally, S_SLOT_13)));
    }
    Ok(())
}

pub fn crystal_freezing_field(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let cm = rank_one_unit(&mut w, role("hero.m")?, 6, &[role("hero.m.s3")?])?;
    w.s.grant_energy(cm, 1000).map_err(|r| r.name().to_string())?;
    let mut ring = Vec::new();
    for k in 0..12 {
        let (dx, dy) = [(7, 0), (5, 5), (0, 7), (-5, 5), (-7, 0), (-5, -5), (0, -7), (5, -5), (8, 2), (-8, -2), (2, 8), (-2, -8)][k];
        ring.push(w.creep(1, dx, dy)?);
    }
    let mana = w.s.mana_of(cm);
    w.cast(cm, role("hero.m.s3")?, -1, 0, 0)?;
    if w.s.mana_of(cm) != mana - 200 {
        return Err(format!("mana {} want {}", w.s.mana_of(cm), mana - 200));
    }
    w.run(20);
    if w.s.channel_left(cm) <= 0 {
        return Err("the channel ended early".into());
    }
    w.run(150);
    let total: f64 = ring.iter().map(|&u| 550.0 - w.hp(u)).sum();
    if total < 131.0 * 10.0 {
        return Err(format!("the ring took only {total:.0} damage in 10 s"));
    }
    if w.s.channel_left(cm) != 0 {
        return Err("the channel outlived 10 s".into());
    }
    let mut w = world()?;
    let cm = rank_one_unit(&mut w, role("hero.m")?, 6, &[role("hero.m.s3")?])?;
    w.s.grant_energy(cm, 1000).map_err(|r| r.name().to_string())?;
    const ENDS_CHANNEL: i32 = 1;
    if !w.s.set_status_flags(STUN, ENDS_CHANNEL) {
        return Err("set_status_flags refused the stun kind".into());
    }
    w.cast(cm, role("hero.m.s3")?, -1, 0, 0)?;
    w.run(10);
    w.s.apply_status(cm, STUN, 20, fixed::ONE);
    if w.s.channel_left(cm) != 0 {
        return Err("a stun did not end the channel".into());
    }
    Ok(())
}

pub fn spell_b_3(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let sven = rank_one_unit(&mut w, role("hero.b")?, 1, &[role("hero.b.s3")?])?;
    w.s.grant_energy(sven, 1000).map_err(|r| r.name().to_string())?;
    let a = w.creep(1, 5, 0)?;
    let b = w.creep(1, 5, 3)?;
    let far = w.creep(1, 5, 8)?;
    w.cast(sven, role("hero.b.s3")?, a as i32, 0, 0)?;
    w.run(40);
    for (name, u) in [("target", a), ("neighbour", b)] {
        near(&format!("{name} hp loss"), 550.0 - w.hp(u), 100.0, 1.5)?;
    }
    if w.hp(far) < 549.0 {
        return Err("a creep outside the bolt radius took damage".into());
    }
    Ok(())
}

pub fn spell_b_2(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let sven = rank_one_unit(&mut w, role("hero.b")?, 1, &[role("hero.b.s2")?])?;
    w.s.clear_status(sven, DISARM);
    if w.s.stat(sven, S_CLEAVE) != 30 {
        return Err(format!("cleave percent {} want 30", w.s.stat(sven, S_CLEAVE)));
    }
    let target = w.creep(1, 1, 0)?;
    let behind = w.creep(1, 3, 0)?;
    let aside = w.creep(1, 1, 5)?;
    let rear = w.creep(1, -2, 0)?;
    let raw = w.s.damage_of(sven);
    w.strike(sven, target)?;
    let scaled = ((raw * 30 / 100) as i64 * w.s.armour_scale_of(behind) as i64 / 65536) as i32;
    let want = fixed::to_float(scaled);
    near("cone victim", 550.0 - w.hp(behind), want, 0.3)?;
    if w.hp(aside) < 549.9 || w.hp(rear) < 549.9 {
        return Err("a unit outside the cone took cleave damage".into());
    }
    Ok(())
}

pub fn spell_b_4(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let sven = rank_one_unit(&mut w, role("hero.b")?, 6, &[role("hero.b.s4")?, role("hero.b.s1")?])?;
    w.s.grant_energy(sven, 1000).map_err(|r| r.name().to_string())?;
    let ally = w.hero(role("hero.a")?, 0, 8, 0)?;
    let base = fixed::to_int(w.s.damage_of(sven));
    w.cast(sven, role("hero.b.s4")?, -1, 0, 0)?;
    w.cast(sven, role("hero.b.s1")?, -1, 0, 0)?;
    if w.s.stat(ally, S_ARMOR) != 5 || w.s.stat(ally, S_SPEED_PCT) != 12 {
        return Err(format!("ally armour {} speed {} want 5 and 12", w.s.stat(ally, S_ARMOR), w.s.stat(ally, S_SPEED_PCT)));
    }
    let bonus = w.s.stat(sven, S_DAMAGE);
    if bonus != base {
        return Err(format!("God's Strength damage {bonus} want {base}"));
    }
    w.run(16 * 25 + 4);
    if w.s.stat(sven, S_DAMAGE) != 0 {
        return Err("God's Strength outlived 25 s".into());
    }
    Ok(())
}

pub fn spell_g_2(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let jug = rank_one_unit(&mut w, role("hero.g")?, 1, &[role("hero.g.s2")?])?;
    w.s.grant_energy(jug, 1000).map_err(|r| r.name().to_string())?;
    let near_enemy = w.creep(1, 2, 0)?;
    let far_enemy = w.creep(1, 8, 0)?;
    w.cast(jug, role("hero.g.s2")?, -1, 0, 0)?;
    if w.s.status_left(jug, SPELL_IMMUNITY) != 80 {
        return Err(format!("spell immunity {} ticks want 80", w.s.status_left(jug, SPELL_IMMUNITY)));
    }
    let mut pulses = 0;
    for tick in 1..=96 {
        let before = w.hp(near_enemy);
        w.run(1);
        let drop = before - w.hp(near_enemy);
        if drop > 10.0 {
            pulses += 1;
            near("blade fury pulse", drop, 20.0, 1.5)?;
        }
        if tick == 16 {
            if pulses != 4 {
                return Err(format!("{pulses} pulses in 16 ticks want 4"));
            }
            if w.hp(far_enemy) < 549.9 {
                return Err("a creep outside 275 units took spin damage".into());
            }
        }
    }
    if pulses != 19 {
        return Err(format!("{pulses} pulses in 96 ticks want 19"));
    }
    Ok(())
}

pub fn spell_g_3(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let jug = rank_one_unit(&mut w, role("hero.g")?, 1, &[role("hero.g.s3")?])?;
    w.s.grant_energy(jug, 1000).map_err(|r| r.name().to_string())?;
    let ally = w.hero(role("hero.a")?, 0, 3, 0)?;
    w.s.set_health(ally, fixed::from_int(3000), fixed::from_int(3000));
    w.s.hurt(ally, fixed::from_int(200));
    let before = w.hp(ally);
    let (x, y) = w.at(2, 0);
    w.cast(jug, role("hero.g.s3")?, -1, x, y)?;
    w.run(0);
    let ward = (0..w.s.count()).find(|&u| w.s.alive(u) && w.s.timed_left(u) > 0).ok_or("no ward was placed")?;
    if w.s.timed_left(ward) < 399 {
        return Err(format!("ward lifetime {} ticks want 400 (25 s)", w.s.timed_left(ward)));
    }
    w.run(2);
    let healed = w.hp(ally) - before;
    let want = w.s.hp_max(ally) as f64 / fixed::ONE as f64 * 0.02;
    near("ward heal after one second", healed, want, want * 0.2 + 1.5)?;
    Ok(())
}

pub fn spell_g_1(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let jug = rank_one_unit(&mut w, role("hero.g")?, 1, &[role("hero.g.s1")?])?;
    w.s.clear_status(jug, DISARM);
    if w.s.stat(jug, S_CRIT_CHANCE) != 20 || w.s.stat(jug, S_CRIT_MULT) != 200 {
        return Err(format!("crit {} percent x{} want 20 and 200", w.s.stat(jug, S_CRIT_CHANCE), w.s.stat(jug, S_CRIT_MULT)));
    }
    let target = w.creep(1, 1, 0)?;
    w.s.set_health(target, fixed::from_int(30_000), fixed::from_int(30_000));
    let mut hits = Vec::new();
    for _ in 0..300 {
        w.s.set_health(target, fixed::from_int(30_000), fixed::from_int(30_000));
        let before = w.s.hp(target);
        w.strike(jug, target)?;
        hits.push((before - w.s.hp(target)) as f64);
    }
    let low = hits.iter().cloned().fold(f64::MAX, f64::min);
    let split = low * 1.5;
    let crits = hits.iter().filter(|&&d| d > split).count();
    if !(35..=90).contains(&crits) {
        return Err(format!("{crits} critical strikes in 300 attacks, want about 60"));
    }
    let plain_top = hits.iter().cloned().filter(|&d| d <= split).fold(0.0, f64::max);
    let crit_low = hits.iter().cloned().filter(|&d| d > split).fold(f64::MAX, f64::min);
    let crit_top = hits.iter().cloned().fold(0.0, f64::max);
    near("plain hit spread", plain_top / low, 1.0, 0.03)?;
    near("critical multiplier of the weakest critical hit", crit_low / low, 2.0, 0.03)?;
    near("critical multiplier of the strongest critical hit", crit_top / low, 2.0, 0.03)?;
    Ok(())
}

pub fn spell_g_4(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let jug = rank_one_unit(&mut w, role("hero.g")?, 6, &[role("hero.g.s4")?])?;
    w.s.grant_energy(jug, 1000).map_err(|r| r.name().to_string())?;
    let a = w.creep(1, 6, 0)?;
    let b = w.creep(1, 8, 2)?;
    w.cast(jug, role("hero.g.s4")?, a as i32, 0, 0)?;
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

pub fn zeus_arc_lightning(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let zeus = rank_one_unit(&mut w, role("hero.n")?, 1, &[role("hero.n.s1")?])?;
    let line: Vec<usize> = (0..4).map(|k| w.creep(1, 3 + 4 * k, 0)).collect::<Result<_, _>>()?;
    w.cast(zeus, role("hero.n.s1")?, line[0] as i32, 0, 0)?;
    near("first target at once", 550.0 - w.hp(line[0]), 85.0, 0.6)?;
    if 550.0 - w.hp(line[1]) > 1.0 {
        return Err("the second target was hit before the 4 tick jump delay".into());
    }
    w.run(5 + 4 * 6);
    let total: f64 = line.iter().map(|&u| 550.0 - w.hp(u)).sum();
    near("total over 5 hits", total, 5.0 * 85.0, 3.0)?;
    Ok(())
}

pub fn zeus_lightning_bolt(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let zeus = rank_one_unit(&mut w, role("hero.n")?, 1, &[role("hero.n.s2")?])?;
    let target = w.creep(1, 5, 0)?;
    w.cast(zeus, role("hero.n.s2")?, target as i32, 0, 0)?;
    if w.s.status_left(target, STUN) < 1 {
        return Err("no ministun".into());
    }
    w.run(5);
    near("bolt damage", 550.0 - w.hp(target), 100.0, 0.6)?;
    Ok(())
}

pub fn zeus_thundergods_wrath(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let zeus = rank_one_unit(&mut w, role("hero.n")?, 6, &[role("hero.n.s3")?])?;
    w.s.grant_energy(zeus, 1000).map_err(|r| r.name().to_string())?;
    let hero = w.hero(role("hero.l")?, 1, 10, 10)?;
    w.s.set_health(hero, fixed::from_int(3000), fixed::from_int(3000));
    let creep = w.creep(1, 10, -10)?;
    let ally = w.hero(role("hero.a")?, 0, -10, -10)?;
    let (h0, a0) = (w.hp(hero), w.hp(ally));
    w.cast(zeus, role("hero.n.s3")?, -1, 0, 0)?;
    w.run(8);
    near("enemy hero hp loss", h0 - w.hp(hero), 168.75, 2.5)?;
    if w.hp(creep) < 549.9 || a0 - w.hp(ally) > 0.5 {
        return Err("the wrath hit a creep or an ally".into());
    }
    Ok(())
}

pub fn spell_h_3(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let sniper = rank_one_unit(&mut w, role("hero.h")?, 1, &[role("hero.h.s3")?])?;
    let target = w.creep(1, 10, 0)?;
    let (x, y) = w.at(10, 0);
    w.s.grant_energy(sniper, 1000).map_err(|r| r.name().to_string())?;
    for _ in 0..3 {
        w.cast(sniper, role("hero.h.s3")?, -1, x, y).map_err(|e| format!("charge: {e}"))?;
    }
    if w.cast(sniper, role("hero.h.s3")?, -1, x, y).is_ok() {
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
    w.run(880);
    w.s.grant_energy(sniper, 1000).map_err(|r| r.name().to_string())?;
    w.cast(sniper, role("hero.h.s3")?, -1, x, y).map_err(|e| format!("restored charge: {e}"))?;
    Ok(())
}

pub fn spell_h_2(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let sniper = rank_one_unit(&mut w, role("hero.h")?, 1, &[role("hero.h.s2")?])?;
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

pub fn spell_h_4(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let sniper = rank_one_unit(&mut w, role("hero.h")?, 1, &[role("hero.h.s4")?])?;
    if w.s.stat(sniper, S_RANGE) != 1563 {
        return Err(format!("passive range {} want 1563 milli (100 units)", w.s.stat(sniper, S_RANGE)));
    }
    Ok(())
}

pub fn spell_h_1(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let sniper = rank_one_unit(&mut w, role("hero.h")?, 6, &[role("hero.h.s1")?])?;
    w.s.grant_energy(sniper, 1000).map_err(|r| r.name().to_string())?;
    let target = w.hero(role("hero.l")?, 1, 9, 0)?;
    w.s.set_health(target, fixed::from_int(5_000), fixed::from_int(5_000));
    let before = w.hp(target);
    let slot = w.s.ability_slot(sniper, role("hero.h.s1")?).ok_or("no assassinate")?;
    if !w.s.cast_ability(sniper, slot, target as i32, 0, 0) {
        return Err(format!("cast refused: {}", w.s.stats.last_refusal.name()));
    }
    w.run(20);
    if before - w.hp(target) > 0.5 || w.s.channel_left(sniper) <= 0 {
        return Err("the cast point ended before 1.7 s or damage landed early".into());
    }
    w.run(30);
    near("assassinate damage", before - w.hp(target), 240.0, 0.6)
}

pub fn spell_o_2(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let lion = rank_one_unit(&mut w, role("hero.o")?, 1, &[role("hero.o.s2")?])?;
    let on_near = w.creep(1, 4, 0)?;
    let on_far = w.creep(1, 10, 0)?;
    let off = w.creep(1, 6, 3)?;
    let beyond = w.creep(1, 12, 5)?;
    let (x, y) = w.at(10, 0);
    w.cast(lion, role("hero.o.s2")?, -1, x, y)?;
    if w.hp(on_near) < 549.9 {
        return Err("the spikes hit before their 0.52 s delay".into());
    }
    w.run(10);
    for (name, u) in [("near", on_near), ("far", on_far)] {
        near(&format!("{name} unit on the line"), 550.0 - w.hp(u), 80.0, 0.6)?;
        if w.s.status_left(u, STUN) < 6 {
            return Err(format!("{name} unit on the line is not stunned"));
        }
    }
    if w.hp(off) < 549.9 || w.hp(beyond) < 549.9 {
        return Err("a unit off the line was hit".into());
    }
    Ok(())
}

pub fn lina_laguna_blade_delay(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let lina = rank_one_unit(&mut w, role("hero.r")?, 6, &[role("hero.r.s3")?])?;
    w.s.grant_energy(lina, 1000).map_err(|r| r.name().to_string())?;
    let target = w.creep(1, 5, 0)?;
    w.cast(lina, role("hero.r.s3")?, target as i32, 0, 0)?;
    if 550.0 - w.hp(target) > 0.5 {
        return Err("the bolt landed before its 0.25 s delay".into());
    }
    w.run(8);
    near("laguna damage after the delay", 550.0 - w.hp(target), 450.0, 2.5)?;
    Ok(())
}

pub fn lina_dragon_slave_taper(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let lina = rank_one_unit(&mut w, role("hero.r")?, 1, &[role("hero.r.s1")?])?;
    let wide = w.creep(1, 1, 4)?;
    let narrow = w.creep(1, 12, 4)?;
    let (x, y) = w.at(12, 0);
    w.cast(lina, role("hero.r.s1")?, -1, x, y)?;
    if 550.0 - w.hp(wide) < 100.0 {
        return Err("the wave missed a unit inside its starting width".into());
    }
    if 550.0 - w.hp(narrow) > 0.5 {
        return Err("the wave hit a unit outside its narrowed end".into());
    }
    Ok(())
}

pub fn lich_dark_ritual_sacrifice(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let lich = rank_one_unit(&mut w, role("hero.q")?, 1, &[role("hero.q.s3")?])?;
    w.s.spend_energy(lich, w.s.mana_of(lich) - 30).map_err(|r| r.name().to_string())?;
    let ally = w.creep(0, 3, 0)?;
    w.s.set_health(ally, fixed::from_int(200), fixed::from_int(200));
    w.cast(lich, role("hero.q.s3")?, ally as i32, 0, 0)?;
    if w.s.alive(ally) {
        return Err("the sacrificed unit survived".into());
    }
    let gained = w.s.mana_of(lich) - 5;
    if gained != 50 {
        return Err(format!("mana gained {gained} want 50 (25 percent of 200)"));
    }
    Ok(())
}

pub fn spell_o_4(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let lion = rank_one_unit(&mut w, role("hero.o")?, 1, &[role("hero.o.s4")?])?;
    let target = w.hero(role("hero.a")?, 1, 5, 0)?;
    w.cast(lion, role("hero.o.s4")?, target as i32, 0, 0)?;
    if w.s.status_left(target, HEX) != 40 {
        return Err(format!("hex ticks {} want 40", w.s.status_left(target, HEX)));
    }
    Ok(())
}

pub fn spell_o_3(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let lion = rank_one_unit(&mut w, role("hero.o")?, 1, &[role("hero.o.s3")?])?;
    let target = w.hero(role("hero.m")?, 1, 6, 0)?;
    w.s.spend_energy(lion, w.s.mana_of(lion) - 10).map_err(|r| r.name().to_string())?;
    let (t0, l0) = (w.s.mana_of(target), w.s.mana_of(lion));
    w.cast(lion, role("hero.o.s3")?, target as i32, 0, 0)?;
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

pub fn spell_o_1(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let lion = rank_one_unit(&mut w, role("hero.o")?, 6, &[role("hero.o.s1")?])?;
    w.s.grant_energy(lion, 1000).map_err(|r| r.name().to_string())?;
    let target = w.hero(role("hero.l")?, 1, 6, 0)?;
    w.s.set_health(target, fixed::from_int(5_000), fixed::from_int(5_000));
    let before = w.hp(target);
    w.cast(lion, role("hero.o.s1")?, target as i32, 0, 0)?;
    w.run(6);
    near("finger damage", before - w.hp(target), 450.0, 1.0)
}

pub fn spell_c_3(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let omni = rank_one_unit(&mut w, role("hero.c")?, 1, &[role("hero.c.s3")?])?;
    w.s.grant_energy(omni, 1000).map_err(|r| r.name().to_string())?;
    let ally = w.hero(role("hero.a")?, 0, 3, 0)?;
    let enemy = w.creep(1, 4, 0)?;
    let far = w.creep(1, 9, 0)?;
    w.s.set_health(ally, fixed::from_int(1_000), fixed::from_int(1_000));
    w.s.hurt(ally, fixed::from_int(200));
    let before = w.hp(ally);
    w.cast(omni, role("hero.c.s3")?, ally as i32, 0, 0)?;
    w.run(5);
    near("heal", w.hp(ally) - before, 90.0, 1.5)?;
    near("purification damage", 550.0 - w.hp(enemy), 90.0, 0.6)?;
    if w.hp(far) < 549.9 {
        return Err("a distant creep took purification damage".into());
    }
    Ok(())
}

pub fn spell_c_2(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let omni = rank_one_unit(&mut w, role("hero.c")?, 1, &[role("hero.c.s2")?])?;
    w.s.grant_energy(omni, 1000).map_err(|r| r.name().to_string())?;
    let ally = w.hero(role("hero.a")?, 0, 3, 0)?;
    w.cast(omni, role("hero.c.s2")?, ally as i32, 0, 0)?;
    if w.s.status_left(ally, SPELL_IMMUNITY) != 64 {
        return Err(format!("repel immunity ticks {} want 64", w.s.status_left(ally, SPELL_IMMUNITY)));
    }
    w.run(16 * 4);
    if w.s.status_left(ally, SPELL_IMMUNITY) != 0 {
        return Err("repel outlived 4 s".into());
    }
    Ok(())
}

pub fn spell_c_1(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let omni = rank_one_unit(&mut w, role("hero.c")?, 6, &[role("hero.c.s1")?])?;
    w.s.grant_energy(omni, 1000).map_err(|r| r.name().to_string())?;
    let ally = w.hero(role("hero.a")?, 0, 4, 0)?;
    w.cast(omni, role("hero.c.s1")?, -1, 0, 0)?;
    if w.s.status_left(ally, PHYSICAL_IMMUNITY) != 96 {
        return Err(format!("immunity ticks {} want 96", w.s.status_left(ally, PHYSICAL_IMMUNITY)));
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

pub fn talent_picks(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let axe = rank_one_unit(&mut w, role("hero.a")?, 9, &[role("hero.a.s1")?])?;
    w.level_up(axe, 20);
    let strength = w.s.stat(axe, S_STRENGTH_BONUS);
    w.s.set_upgrade_bit(axe, 6, true).map_err(|r| r.name().to_string())?;
    let gained = w.s.stat(axe, S_STRENGTH_BONUS) - strength;
    if gained != 15 {
        return Err(format!("the +15 strength talent added {gained}"));
    }
    Ok(())
}

pub fn skill_gate_data(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let axe = w.hero(role("hero.a")?, 0, 0, 0)?;
    let slot = w.s.ability_slot(axe, role("hero.a.s1")?).ok_or("axe holds no battle hunger")?;
    if w.s.next_rank_gate(axe, slot) != Some(1) {
        return Err(format!("first rank of a basic ability gates at {:?}, expected Some(1)", w.s.next_rank_gate(axe, slot)));
    }
    w.s.rank_up(axe, slot).map_err(|r| r.name().to_string())?;
    if w.s.next_rank_gate(axe, slot) != Some(3) {
        return Err("second rank of a basic ability should gate at level 3".into());
    }
    let ult = w.s.ability_slot(axe, role("hero.a.s4")?).ok_or("axe holds no culling blade")?;
    if w.s.next_rank_gate(axe, ult) != Some(6) {
        return Err("first rank of an ultimate should gate at level 6".into());
    }
    w.s.rank_up(axe, ult).map_err(|r| r.name().to_string())?;
    if w.s.next_rank_gate(axe, ult) != Some(11) {
        return Err("second rank of an ultimate should gate at level 11".into());
    }
    Ok(())
}

pub fn spell_illusions(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let hero = w.hero(role("hero.a")?, 0, 0, 0)?;
    let (x, y) = w.at(2, 0);
    let c = w.s.spawn_image(hero, x, y, 64, 50, 300).ok_or("spawn_image made no illusion")?;
    let copies = [c];
    let before = w.hp(c);
    w.s.hurt(c, fixed::from_int(10));
    near("illusion damage taken", before - w.hp(c), 30.0, 0.5)?;
    w.run(70);
    if copies.iter().any(|&u| w.s.alive(u)) {
        return Err("an illusion outlived its 4 s".into());
    }
    Ok(())
}

pub fn spell_i_2(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let pl = rank_one_unit(&mut w, role("hero.i")?, 6, &[role("hero.i.s2")?])?;
    w.s.grant_energy(pl, 1000).map_err(|r| r.name().to_string())?;
    let target = w.hero(role("hero.l")?, 1, 5, 0)?;
    w.s.set_health(target, fixed::from_int(5_000), fixed::from_int(5_000));
    let before = w.hp(target);
    w.cast(pl, role("hero.i.s2")?, target as i32, 0, 0)?;
    w.run(6);
    let lost = before - w.hp(target);
    if !(74.0..=76.0).contains(&lost) {
        return Err(format!("lance damage {lost:.2} want 75"));
    }
    Ok(())
}

pub fn spell_s_1(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let dz = rank_one_unit(&mut w, role("hero.s")?, 1, &[role("hero.s.s1")?])?;
    let ally = w.hero(role("hero.a")?, 0, 4, 0)?;
    w.cast(dz, role("hero.s.s1")?, ally as i32, 0, 0)?;
    w.s.hurt(ally, fixed::from_int(20_000));
    if !w.s.alive(ally) || w.hp(ally) > 1.5 {
        return Err("a lethal blow killed or missed the protected ally".into());
    }
    w.run(16 * 4 + 6);
    w.s.hurt(ally, fixed::from_int(20_000));
    if !w.s.alive(ally) {
        return Err("the grave ended before its 5 s".into());
    }
    w.run(16);
    w.s.hurt(ally, fixed::from_int(20_000));
    if w.s.alive(ally) {
        return Err("the grave outlived its 5 s".into());
    }
    Ok(())
}

pub fn spell_r_1(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let lina = rank_one_unit(&mut w, role("hero.r")?, 3, &[role("hero.r.s1")?, role("hero.r.s2")?])?;
    w.s.grant_energy(lina, 1000).map_err(|r| r.name().to_string())?;
    let on = w.creep(1, 6, 0)?;
    let off = w.creep(1, 6, 8)?;
    let (x, y) = w.at(8, 0);
    w.cast(lina, role("hero.r.s1")?, -1, x, y)?;
    near("line victim", 550.0 - w.hp(on), 110.0, 0.6)?;
    if w.hp(off) < 549.9 {
        return Err("a unit off the line took dragon slave damage".into());
    }
    let (ax, ay) = w.at(6, 8);
    w.s.issue_order_kind(off, sim::ORDER_HOLD, ax, ay).map_err(|r| format!("hold refused: {}", r.name()))?;
    w.cast(lina, role("hero.r.s2")?, -1, ax, ay)?;
    if w.hp(off) < 549.9 {
        return Err("the array hit before its delay".into());
    }
    w.run(12);
    near("array damage", 550.0 - w.hp(off), 80.0, 0.6)?;
    if w.s.status_left(off, STUN) < 15 {
        return Err("no array stun".into());
    }
    Ok(())
}

pub fn spell_q_2(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let lich = rank_one_unit(&mut w, role("hero.q")?, 6, &[role("hero.q.s2")?, role("hero.q.s1")?])?;
    w.s.grant_energy(lich, 1000).map_err(|r| r.name().to_string())?;
    let target = w.creep(1, 5, 0)?;
    let near_one = w.creep(1, 5, 2)?;
    w.cast(lich, role("hero.q.s2")?, target as i32, 0, 0)?;
    near("target damage", 550.0 - w.hp(target), 125.0, 0.6)?;
    near("neighbour damage", 550.0 - w.hp(near_one), 75.0, 0.6)?;
    if w.s.slow_multiplier(target) >= fixed::ONE {
        return Err("no frost nova slow".into());
    }
    let mut w = world()?;
    let lich = rank_one_unit(&mut w, role("hero.q")?, 6, &[role("hero.q.s1")?])?;
    w.s.grant_energy(lich, 1000).map_err(|r| r.name().to_string())?;
    let line: Vec<usize> = (0..3).map(|k| w.creep(1, 3 + 4 * k, 0)).collect::<Result<_, _>>()?;
    for &u in &line {
        w.s.set_health(u, fixed::from_int(9000), fixed::from_int(9000));
    }
    w.cast(lich, role("hero.q.s1")?, line[0] as i32, 0, 0)?;
    w.run(80);
    let hits: f64 = line.iter().map(|&u| 9000.0 - w.hp(u)).sum();
    if hits < 280.0 * 8.0 {
        return Err(format!("chain frost dealt only {hits:.0} over 11 hits among three units"));
    }
    Ok(())
}

pub fn spell_d_2(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let tide = rank_one_unit(&mut w, role("hero.d")?, 6, &[role("hero.d.s2")?])?;
    w.s.grant_energy(tide, 1000).map_err(|r| r.name().to_string())?;
    let foes: Vec<usize> = (0..3).map(|k| w.creep(1, 4 + 3 * k, 0)).collect::<Result<_, _>>()?;
    w.cast(tide, role("hero.d.s2")?, -1, 0, 0)?;
    for &f in &foes {
        near("ravage damage", 550.0 - w.hp(f), 200.0, 0.6)?;
        if w.s.status_left(f, STUN) < 24 {
            return Err("ravage did not stun".into());
        }
    }
    Ok(())
}

pub fn spell_f_1(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let slardar = rank_one_unit(&mut w, role("hero.f")?, 1, &[role("hero.f.s1")?])?;
    w.s.clear_status(slardar, DISARM);
    let target = w.creep(1, 1, 0)?;
    w.s.set_health(target, fixed::from_int(30_000), fixed::from_int(30_000));
    let mut stuns = 0;
    for _ in 0..300 {
        w.strike(slardar, target)?;
        let left = w.s.status_left(target, STUN);
        if left > 0 {
            if !(14..=16).contains(&left) {
                return Err(format!("bash stun {left} ticks want 16"));
            }
            stuns += 1;
            w.s.clear_status(target, STUN);
        }
    }
    if !(12..=52).contains(&stuns) {
        return Err(format!("{stuns} bashes in 300 attacks, want about 30"));
    }
    Ok(())
}

pub fn vengeful_command_aura(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let vs = rank_one_unit(&mut w, role("hero.k")?, 1, &[role("hero.k.s1")?])?;
    let ally = w.hero(role("hero.a")?, 0, 5, 0)?;
    w.run(4);
    if w.s.stat(ally, S_DAMAGE) != 26 * 12 / 100 || w.s.stat(vs, S_DAMAGE) != 19 * 12 / 100 {
        return Err(format!("aura damage ally {} caster {} want 3 and 2", w.s.stat(ally, S_DAMAGE), w.s.stat(vs, S_DAMAGE)));
    }
    Ok(())
}

pub fn spell_l_1(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let am = rank_one_unit(&mut w, role("hero.l")?, 1, &[role("hero.l.s1")?])?;
    let (x0, y0) = w.s.position_fixed(am);
    let (tx, ty) = w.at(-20, 0);
    w.cast(am, role("hero.l.s1")?, -1, tx, ty)?;
    let (x1, _) = w.s.position_fixed(am);
    near("blink distance in cells", fixed::to_float(x0 - x1), 925.0 * 15.625 / 1000.0, 0.3)?;
    let _ = y0;
    let mut w = world()?;
    let am = rank_one_unit(&mut w, role("hero.l")?, 1, &[role("hero.l.s2")?])?;
    w.s.clear_status(am, DISARM);
    let victim = w.hero(role("hero.m")?, 1, 1, 0)?;
    w.s.apply_status(victim, DISARM, 1_000_000, fixed::ONE);
    let (m0, h0) = (w.s.mana_of(victim), w.hp(victim));
    w.strike(am, victim)?;
    if m0 - w.s.mana_of(victim) != 28 {
        return Err(format!("mana burned {} want 28", m0 - w.s.mana_of(victim)));
    }
    let lost = h0 - w.hp(victim);
    if lost < 17.0 {
        return Err(format!("health lost {lost:.1}, want the 17 of mana break plus the attack"));
    }
    Ok(())
}

fn adaptive_strike_loss(agility: i64) -> Result<f64, String> {
    let mut w = world()?;
    let m = rank_one_unit(&mut w, "morphling", 1, &["morphling_adaptive_strike"])?;
    w.s.set_counter(m, "attr_agi", agility);
    let creep = w.creep(1, 3, 0)?;
    let before = w.hp(creep);
    w.cast(m, "morphling_adaptive_strike", creep as i32, 0, 0)?;
    Ok(before - w.hp(creep))
}

pub fn damage_by_agility(_image: &[u8]) -> Result<(), String> {
    let flat = adaptive_strike_loss(0)?;
    let scaled = adaptive_strike_loss(100)?;
    if flat <= 0.0 {
        return Err("adaptive strike did no damage at zero agility".into());
    }
    near("agility part of the damage", (scaled - flat) / flat, 0.37, 0.01)
}

pub fn damage_by_missing_life(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let n = rank_one_unit(&mut w, "necrolyte", 6, &["necrolyte_reaper_s_scythe"])?;
    w.s.grant_energy(n, 1000).map_err(|r| r.name().to_string())?;
    let creep = w.creep(1, 3, 0)?;
    let max = w.hp(creep);
    let missing = max / 2.0;
    w.s.set_hp_within(creep, fixed::from_int((max - missing) as i32));
    let before = w.hp(creep);
    w.cast(n, "necrolyte_reaper_s_scythe", creep as i32, 0, 0)?;
    let loss = before - w.hp(creep);
    if loss <= 0.0 || loss > missing * 0.4 + 0.6 {
        return Err(format!("scythe loss {loss:.2} want at most {:.2} (0.4 per missing life, after magic resistance)", missing * 0.4));
    }
    Ok(())
}

pub fn ward_group_by_rank(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let s = rank_one_unit(&mut w, "shadow_shaman", 6, &["shadow_shaman_mass_serpent_ward"])?;
    w.s.grant_energy(s, 1000).map_err(|r| r.name().to_string())?;
    let rank_two = kind(&w.s, "shadow_shaman_serpent_ward_2")?;
    let slot = w.s.ability_slot(s, "shadow_shaman_mass_serpent_ward").ok_or("no ability slot")?;
    w.s.rank_up(s, slot).map_err(|r| r.name().to_string())?;
    let (x, y) = w.at(4, 0);
    w.cast(s, "shadow_shaman_mass_serpent_ward", -1, x, y)?;
    let wards = (0..w.s.count()).filter(|&u| w.s.alive(u) && w.s.unit_type(u) == rank_two as i32).count();
    if wards != 10 {
        return Err(format!("{wards} rank-two serpent wards want 10"));
    }
    Ok(())
}

pub fn slot_summon_group(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let lycan = rank_one_unit(&mut w, "lycan", 1, &["lycan_summon_wolves"])?;
    w.s.grant_energy(lycan, 1000).map_err(|r| r.name().to_string())?;
    w.cast(lycan, "lycan_summon_wolves", -1, 0, 0)?;
    let wolf = kind(&w.s, "lycan_wolf_1")?;
    let wolves: Vec<usize> = (0..w.s.count()).filter(|&u| w.s.alive(u) && w.s.unit_type(u) == wolf as i32).collect();
    if wolves.len() != 2 {
        return Err(format!("wolves {} want 2 (summoned {}, refusal {}, energy {}, cd {})", wolves.len(), w.s.stats.summoned, w.s.stats.last_refusal.name(), w.s.mana_of(lycan), w.s.stats.casts));
    }
    for &u in &wolves {
        near("wolf hit points", w.hp(u), 200.0, 0.01)?;
        if w.s.timed_left(u) < 870 || w.s.timed_left(u) > 880 {
            return Err(format!("wolf lifetime {} want about 880 ticks", w.s.timed_left(u)));
        }
        if w.s.stat(u, S_MAGIC_RESIST) != 80 {
            return Err(format!("wolf magic resistance {} want 80", w.s.stat(u, S_MAGIC_RESIST)));
        }
    }
    w.run(900);
    if wolves.iter().any(|&u| w.s.alive(u)) {
        return Err("a wolf outlived its 880 ticks".into());
    }
    Ok(())
}

pub fn slot_displace_vacuum(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let seer = rank_one_unit(&mut w, "dark_seer", 1, &["dark_seer_vacuum"])?;
    let enemy = w.creep(1, 9, 0)?;
    let (px, py) = w.at(6, 0);
    let slot = w.s.ability_slot(seer, "dark_seer_vacuum").ok_or("hero holds no dark_seer_vacuum")?;
    if !w.s.cast_ability(seer, slot, -1, px, py) {
        return Err(format!("cast refused: {}", w.s.stats.last_refusal.name()));
    }
    w.run(40);
    let (x, y) = w.s.position_fixed(enemy);
    let miss = (((x - px) as i64).pow(2) + ((y - py) as i64).pow(2)) as f64;
    let miss = miss.sqrt() / fixed::ONE as f64;
    if miss > 1.5 {
        return Err(format!("the pulled unit rests {miss:.2} cells from the centre, want under 1.5"));
    }
    Ok(())
}

pub fn axe_culling_blade_execute(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let axe = rank_one_unit(&mut w, role("hero.a")?, 6, &[role("hero.a.s4")?])?;
    let spared = w.creep(1, 2, 0)?;
    let threshold = w.creep(1, 2, 2)?;
    w.s.hurt(threshold, fixed::from_int(300));
    w.cast(axe, role("hero.a.s4")?, spared as i32, 0, 0)?;
    near("creep above the threshold takes the 150 damage", 550.0 - w.hp(spared), 150.0, 1.0)?;
    let slot = w.s.ability_slot(axe, role("hero.a.s4")?).ok_or("axe holds no blade")?;
    if w.s.ability_cooldown(axe, slot) == 0 {
        return Err("a blade that killed nothing reset its cooldown".into());
    }
    w.s.grant_energy(axe, 1000).map_err(|r| r.name().to_string())?;
    w.s.set_ability_cooldown(axe, slot, 0);
    w.cast(axe, role("hero.a.s4")?, threshold as i32, 0, 0)?;
    if w.s.alive(threshold) {
        return Err("a creep at 250 hit points survived the instant kill".into());
    }
    if w.s.ability_cooldown(axe, slot) == 0 {
        return Err("a creep kill reset the cooldown, only a hero kill does".into());
    }
    let victim = w.hero(role("hero.o")?, 1, 3, 0)?;
    w.s.set_health(victim, fixed::from_int(240), fixed::from_int(600));
    w.s.set_ability_cooldown(axe, slot, 0);
    w.cast(axe, role("hero.a.s4")?, victim as i32, 0, 0)?;
    if w.s.alive(victim) {
        return Err("a hero at 240 hit points survived the instant kill".into());
    }
    if w.s.ability_cooldown(axe, slot) != 0 {
        return Err(format!("a hero kill left {} cooldown ticks on the blade", w.s.ability_cooldown(axe, slot)));
    }
    Ok(())
}

pub fn drow_gust_knockback(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let drow = rank_one_unit(&mut w, "drow_ranger", 1, &["drow_ranger_gust"])?;
    w.s.grant_energy(drow, 1000).map_err(|r| r.name().to_string())?;
    let near_unit = w.creep(1, 3, 0)?;
    let far_unit = w.creep(1, 12, 0)?;
    let (n0, f0) = (w.s.position_fixed(near_unit).0, w.s.position_fixed(far_unit).0);
    let (x, y) = w.at(13, 0);
    w.cast(drow, "drow_ranger_gust", -1, x, y)?;
    w.run(20);
    let near_push = (w.s.position_fixed(near_unit).0 - n0) as f64 / fixed::ONE as f64;
    let far_push = (w.s.position_fixed(far_unit).0 - f0) as f64 / fixed::ONE as f64;
    if !(near_push > far_push && far_push > 0.0) {
        return Err(format!("gust pushed the near unit {near_push:.2} cells and the far unit {far_push:.2}, want near above far above 0"));
    }
    near("near unit push in cells", near_push, 5.469 * (14.063 - 3.0) / 14.063, 0.3)?;
    near("far unit push in cells", far_push, 5.469 * (14.063 - 12.0) / 14.063, 0.3)?;
    Ok(())
}

pub fn antimage_mana_break_follows_burn(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let am = rank_one_unit(&mut w, "antimage", 1, &["antimage_mana_break"])?;
    let rich = w.hero(role("hero.o")?, 1, 2, 0)?;
    let poor = w.hero(role("hero.o")?, 1, -2, 0)?;
    let spend = w.s.mana_of(poor) - 10;
    w.s.spend_energy(poor, spend).map_err(|r| r.name().to_string())?;
    let (rich_mana, poor_mana) = (w.s.mana_of(rich), w.s.mana_of(poor));
    let (rich_hp, poor_hp) = (w.hp(rich), w.hp(poor));
    w.s.clear_status(am, DISARM);
    w.strike(am, rich)?;
    w.strike(am, poor)?;
    if rich_mana - w.s.mana_of(rich) != 28 {
        return Err(format!("mana break burned {} of a full pool, want 28", rich_mana - w.s.mana_of(rich)));
    }
    if poor_mana - w.s.mana_of(poor) != 10 {
        return Err(format!("mana break burned {} of a 10 mana pool, want 10", poor_mana - w.s.mana_of(poor)));
    }
    let gap = (rich_hp - w.hp(rich)) - (poor_hp - w.hp(poor));
    if !(8.0..=11.0).contains(&gap) {
        return Err(format!("the full pool took {gap:.2} more than the 10 mana pool, want about 0.6 * 18 = 10.8 before armour"));
    }
    Ok(())
}

pub fn spell_blind_and_revealed(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let tinker = rank_one_unit(&mut w, role("hero.tinker")?, 1, &[role("hero.tinker.laser")?])?;
    w.s.grant_energy(tinker, 1000).map_err(|r| r.name().to_string())?;
    let target = w.hero(role("hero.a")?, 1, 5, 0)?;
    w.cast(tinker, role("hero.tinker.laser")?, target as i32, 0, 0)?;
    if w.s.status_left(target, BLIND) != 48 {
        return Err(format!("blind ticks {} want 48 (3.0 s)", w.s.status_left(target, BLIND)));
    }
    if !(0..20).all(|_| w.s.misses(target)) {
        return Err("a blinded unit's attack landed".into());
    }
    w.run(50);
    if w.s.status_left(target, BLIND) != 0 || (0..20).any(|_| w.s.misses(target)) {
        return Err("blind outlived 3.0 s".into());
    }
    let mut w = world()?;
    let zuus = rank_one_unit(&mut w, role("hero.zuus")?, 1, &[role("hero.zuus.bolt")?])?;
    w.s.grant_energy(zuus, 1000).map_err(|r| r.name().to_string())?;
    let target = w.hero(role("hero.a")?, 1, 5, 0)?;
    w.s.apply_status(target, INVISIBLE, 1_000_000, fixed::ONE);
    if w.s.detected(0, target) {
        return Err("an invisible unit was detected before it was revealed".into());
    }
    w.cast(zuus, role("hero.zuus.bolt")?, target as i32, 0, 0)?;
    if w.s.status_left(target, REVEALED) != 72 || !w.s.detected(0, target) {
        return Err(format!("revealed ticks {} want 72 and detected", w.s.status_left(target, REVEALED)));
    }
    Ok(())
}

pub fn spell_plague_ward_rank_unit(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let vm = w.hero(role("hero.venomancer")?, 0, 0, 0)?;
    w.level_up(vm, 3);
    w.learn(vm, role("hero.venomancer.ward")?, 2)?;
    w.s.grant_energy(vm, 1000).map_err(|r| r.name().to_string())?;
    let (tx, ty) = w.at(4, 0);
    w.cast(vm, role("hero.venomancer.ward")?, -1, tx, ty)?;
    w.run(3);
    let live = |w: &World, name: &str| -> Vec<usize> { (0..w.s.count()).filter(|&u| w.s.alive(u) && w.s.kind_name(u) == Some(name)).collect() };
    let wards = live(&w, role("unit.plague_ward_2")?);
    if wards.len() != 1 || !live(&w, role("unit.plague_ward_1")?).is_empty() {
        return Err(format!("{} rank 2 wards, want one and no rank 1 ward", wards.len()));
    }
    if (w.hp(wards[0]) - 200.0).abs() > 0.5 {
        return Err(format!("ward health {:.1} want 200", w.hp(wards[0])));
    }
    w.run(640 + 2);
    if w.s.alive(wards[0]) {
        return Err("the ward outlived its 40 s".into());
    }
    Ok(())
}

pub fn spell_hit_marks_attack(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let tinker = w.hero(role("hero.tinker")?, 0, 0, 0)?;
    let target = w.hero(role("hero.a")?, 1, 3, 0)?;
    w.s.clear_status(tinker, DISARM);
    w.s.take_hits();
    w.strike(tinker, target)?;
    let hits = w.s.take_hits();
    if !hits.iter().any(|h| h.attacker == tinker as i32 && h.victim == target as u32 && h.attack) {
        return Err(format!("an attack landing was not marked attack: {hits:?}"));
    }
    w.s.hurt(target, fixed::from_int(10));
    let hurt = w.s.take_hits();
    if hurt.iter().any(|h| h.attack) {
        return Err(format!("a spell hurt was marked attack: {hurt:?}"));
    }
    Ok(())
}

pub fn spell_vortex_pulls(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let storm = rank_one_unit(&mut w, role("hero.storm")?, 1, &[role("hero.storm.vortex")?])?;
    w.s.grant_energy(storm, 1000).map_err(|r| r.name().to_string())?;
    let target = w.hero(role("hero.a")?, 1, 4, 0)?;
    let gap = |w: &World| {
        let (sx, sy) = w.s.position_fixed(storm);
        let (tx, ty) = w.s.position_fixed(target);
        (fixed::to_float(tx - sx).powi(2) + fixed::to_float(ty - sy).powi(2)).sqrt()
    };
    let before = gap(&w);
    w.cast(storm, role("hero.storm.vortex")?, target as i32, 0, 0)?;
    w.run(24);
    let after = gap(&w);
    if before - after < 2.0 {
        return Err(format!("the pulled unit moved from {before:.2} to {after:.2} cells, want at least 2 closer"));
    }
    Ok(())
}

pub fn spell_cast_point_by_rank(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let tinker = w.hero(role("hero.tinker")?, 0, 0, 0)?;
    w.level_up(tinker, 11);
    let slot = w.learn(tinker, role("hero.tinker.rearm")?, 2)?;
    w.s.grant_energy(tinker, 1000).map_err(|r| r.name().to_string())?;
    if !w.s.cast_ability(tinker, slot, -1, 0, 0) {
        return Err(format!("Rearm refused: {}", w.s.stats.last_refusal.name()));
    }
    let mut held = 0;
    while w.s.channel_left(tinker) > 0 && held < 100 {
        w.s.tick();
        held += 1;
    }
    if !(23..=25).contains(&held) {
        return Err(format!("rank 2 Rearm held the caster {held} ticks, want 24 (1.5 s)"));
    }
    Ok(())
}

pub fn sleep_song_and_blind(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let naga = rank_one_unit(&mut w, "naga_siren", 6, &["naga_siren_song_of_the_siren"])?;
    w.s.grant_energy(naga, 1000).map_err(|r| r.name().to_string())?;
    let sleeper = w.creep(1, 8, 0)?;
    w.cast(naga, "naga_siren_song_of_the_siren", -1, 0, 0)?;
    if w.s.status_left(sleeper, 16) != 112 {
        return Err(format!("song sleep ticks {} want 112 (7 s)", w.s.status_left(sleeper, 16)));
    }
    let (x0, _) = w.s.position_fixed(sleeper);
    let y0 = w.s.position_fixed(sleeper).1;
    w.s.order(sleeper, x0 + fixed::from_int(4), y0);
    w.run(8);
    if w.s.position_fixed(sleeper).0 != x0 {
        return Err("a sleeping unit walked".into());
    }
    let mut w = world()?;
    let stalker = rank_one_unit(&mut w, "night_stalker", 1, &["night_stalker_crippling_fear"])?;
    w.s.grant_energy(stalker, 1000).map_err(|r| r.name().to_string())?;
    let target = w.creep(1, 3, 0)?;
    w.cast(stalker, "night_stalker_crippling_fear", target as i32, 0, 0)?;
    if w.s.status_left(target, 14) != 80 {
        return Err(format!("blind ticks {} want 80", w.s.status_left(target, 14)));
    }
    Ok(())
}

pub fn plasma_field_by_distance(_image: &[u8]) -> Result<(), String> {
    let mut w = world()?;
    let razor = rank_one_unit(&mut w, "razor", 1, &["razor_plasma_field"])?;
    w.s.grant_energy(razor, 1000).map_err(|r| r.name().to_string())?;
    let creep = w.creep(1, 6, 0)?;
    let before = w.hp(creep);
    w.cast(razor, "razor_plasma_field", -1, 0, 0)?;
    let lost = before - w.hp(creep);
    if !(100.0..=125.0).contains(&lost) {
        return Err(format!("two plasma hits at 6 cells took {lost:.1} want about 113 (30 + 4.44 a cell, twice)"));
    }
    Ok(())
}

fn scythe_loss(scepter: bool) -> Result<f64, String> {
    let mut w = world()?;
    let n = rank_one_unit(&mut w, "necrolyte", 6, &["necrolyte_reaper_s_scythe"])?;
    w.s.grant_energy(n, 1000).map_err(|r| r.name().to_string())?;
    if scepter {
        w.s.set_upgrade_bit(n, 0, true).map_err(|r| r.name().to_string())?;
    }
    let creep = w.creep(1, 3, 0)?;
    let max = w.hp(creep);
    w.s.set_hp_within(creep, fixed::from_int((max / 2.0) as i32));
    let before = w.hp(creep);
    w.cast(n, "necrolyte_reaper_s_scythe", creep as i32, 0, 0)?;
    Ok(before - w.hp(creep))
}

pub fn scythe_scepter_delta(_image: &[u8]) -> Result<(), String> {
    let plain = scythe_loss(false)?;
    let held = scythe_loss(true)?;
    near("scepter scythe loss over the plain one", held / plain, 70.0 / 40.0, 0.1)
}
