use crate::checks::{kind, pack_run, role};
use mod_host::ModHost;
use sim::{fixed, Sim};

const STUN: usize = 0;
const ROOT: usize = 3;
const DISARM: usize = 7;
const S_ATTACK_SPEED: usize = 12;
const S_SPEED_PCT: usize = 19;
const MAGIC: i32 = 2;
const PHYSICAL: i32 = 1;
const CELL_UNITS: f64 = 64.0;

struct Arena {
    host: ModHost,
    cx: i32,
    cy: i32,
}

fn arena() -> Result<Arena, String> {
    let mut host = pack_run::start()?;
    let (cx, cy) = {
        let s = host.sim();
        let (w, h) = (s.grid().w as i32, s.grid().h as i32);
        let reach = 18;
        let open = |cx: i32, cy: i32| (-reach..=reach).all(|dy| (-reach..=reach).all(|dx| s.grid().walkable(cx + dx, cy + dy)));
        let crowd = |cx: i32, cy: i32| {
            (0..s.count())
                .filter(|&u| {
                    let (x, y) = s.position_fixed(u);
                    let (dx, dy) = ((x >> 16) - cx, (y >> 16) - cy);
                    s.alive(u) && dx * dx + dy * dy <= (reach + 10) * (reach + 10)
                })
                .count()
        };
        let mut best: Option<(usize, i32, i32)> = None;
        for cy in (reach..h - reach).step_by(3) {
            for cx in (reach..w - reach).step_by(3) {
                if open(cx, cy) {
                    let n = crowd(cx, cy);
                    if best.is_none_or(|b| n < b.0) {
                        best = Some((n, cx, cy));
                    }
                }
            }
        }
        let (_, cx, cy) = best.ok_or("no open arena on the map")?;
        (cx, cy)
    };
    let near: Vec<usize> = {
        let s = host.sim();
        (0..s.count())
            .filter(|&u| {
                let (x, y) = s.position_fixed(u);
                let (dx, dy) = ((x >> 16) - cx, (y >> 16) - cy);
                s.alive(u) && dx * dx + dy * dy <= 28 * 28
            })
            .collect()
    };
    for u in near {
        host.sim_mut().remove_unit(u);
    }
    Ok(Arena { host, cx, cy })
}

impl Arena {
    fn sim(&mut self) -> std::cell::RefMut<'_, Sim> {
        self.host.sim_mut()
    }

    fn at(&self, dx: i32, dy: i32) -> (i32, i32) {
        (fixed::from_int(self.cx + dx), fixed::from_int(self.cy + dy))
    }

    fn hero(&mut self, name: &str, player: u8, dx: i32, dy: i32, level: u32) -> Result<usize, String> {
        let k = kind(&self.host.sim(), name)?;
        let (x, y) = self.at(dx, dy);
        let h = self.sim().spawn_attributed(player, k, x, y).ok_or_else(|| format!("spawn_attributed refused {name}"))?;
        let need = level.checked_sub(2).and_then(|i| self.sim().xp_curve().get(i as usize).copied()).unwrap_or(0);
        let gap = need.saturating_sub(self.sim().xp(h));
        self.sim().add_xp(h, gap);
        self.sim().grant_energy(h, 5000).map_err(|r| r.name().to_string())?;
        Ok(h)
    }

    fn creep(&mut self, player: u8, dx: i32, dy: i32) -> Result<usize, String> {
        let k = kind(&self.host.sim(), role("unit.creep")?)?;
        let (x, y) = self.at(dx, dy);
        let u = self.sim().spawn_for(player, k, x, y).ok_or("spawn_for refused a creep")?;
        self.sim().apply_status(u, DISARM, 1_000_000, fixed::ONE);
        self.sim().apply_status(u, ROOT, 1_000_000, fixed::ONE);
        Ok(u)
    }

    fn learn(&mut self, h: usize, ability: &str, ranks: usize) -> Result<usize, String> {
        let slot = self.sim().ability_slot(h, ability).ok_or_else(|| format!("hero holds no {ability}"))?;
        for _ in 0..ranks {
            self.sim().rank_up(h, slot).map_err(|r| format!("skill_up {ability}: {}", r.name()))?;
        }
        Ok(slot)
    }

    fn cast(&mut self, h: usize, ability: &str, target: i32, x: i32, y: i32) -> Result<(), String> {
        let slot = self.sim().ability_slot(h, ability).ok_or_else(|| format!("hero holds no {ability}"))?;
        if self.sim().cast_ability(h, slot, target, x, y) {
            Ok(())
        } else {
            Err(format!("cast {ability} refused: {}", self.sim().stats.last_refusal.name()))
        }
    }

    fn run(&mut self, ticks: u32) -> Result<(), String> {
        pack_run::advance(&mut self.host, ticks)
    }

    fn strike(&mut self, a: usize, b: usize) -> Result<(), String> {
        for _ in 0..200 {
            if self.sim().order_attack(a, b).is_ok() {
                while self.sim().hit_in_flight(a) {
                    self.run(1)?;
                }
                return self.run(2);
            }
            self.run(1)?;
        }
        Err(format!("unit {a} could not strike unit {b}"))
    }

    fn hp(&mut self, u: usize) -> f64 {
        self.sim().hp(u) as f64 / fixed::ONE as f64
    }

    fn cells_between(&mut self, a: usize, b: usize) -> f64 {
        let (ax, ay) = self.sim().position_fixed(a);
        let (bx, by) = self.sim().position_fixed(b);
        fixed::to_float(ax - bx).hypot(fixed::to_float(ay - by))
    }

    fn pin(&mut self, u: usize) {
        self.sim().apply_status(u, DISARM, 1_000_000, fixed::ONE);
        self.sim().apply_status(u, ROOT, 1_000_000, fixed::ONE);
    }
}

pub fn tidebringer_cleave(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let kunkka = a.hero("kunkka", 0, 0, 0, 1)?;
    a.learn(kunkka, "kunkka_tidebringer", 1)?;
    let victim = a.creep(1, 3, 0)?;
    let beside = a.creep(1, 6, 0)?;
    let far = a.creep(1, 3, 14)?;
    let (v0, b0, f0) = (a.hp(victim), a.hp(beside), a.hp(far));
    a.strike(kunkka, victim)?;
    let (lost_victim, lost_beside, lost_far) = (v0 - a.hp(victim), b0 - a.hp(beside), f0 - a.hp(far));
    if lost_victim <= 0.0 || lost_beside <= 0.0 {
        return Err(format!("tidebringer strike lost {lost_victim:.1} on the target and {lost_beside:.1} beside it"));
    }
    if lost_far != 0.0 {
        return Err(format!("the unit 14 cells from the target lost {lost_far:.1}"));
    }
    if lost_victim - lost_beside < 15.0 {
        return Err(format!("target lost {lost_victim:.1}, neighbour {lost_beside:.1}: want the 20 bonus on the target only"));
    }
    Ok(())
}

fn strike_loss(impetus: bool, cells: i32) -> Result<(f64, i32), String> {
    let mut a = arena()?;
    let ench = a.hero("enchantress", 0, 0, 0, 6)?;
    if impetus {
        a.learn(ench, "enchantress_impetus", 1)?;
    }
    a.sim().apply_status(ench, ROOT, 1_000_000, fixed::ONE);
    let creep = a.creep(1, cells, 0)?;
    let (before, mana) = (a.hp(creep), a.sim().mana_of(ench));
    a.strike(ench, creep)?;
    let left = a.sim().mana_of(ench);
    Ok((before - a.hp(creep), mana - left))
}

pub fn impetus_distance(_image: &[u8]) -> Result<(), String> {
    let (plain, _) = strike_loss(false, 7)?;
    let (with, spent) = strike_loss(true, 7)?;
    let want = 0.15 * 7.0 * CELL_UNITS;
    if ((with - plain) - want).abs() > 2.0 {
        return Err(format!("impetus added {:.1}, want {want:.1} (15 percent of 7 cells in reference units)", with - plain));
    }
    if spent < 55 {
        return Err(format!("impetus spent {spent} mana, want its 55"));
    }
    Ok(())
}

pub fn flame_guard_barrier(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let ember = a.hero("ember_spirit", 0, 0, 0, 1)?;
    a.learn(ember, "ember_spirit_flame_guard", 1)?;
    let enemy = a.creep(1, 12, 12)?;
    a.cast(ember, "ember_spirit_flame_guard", -1, 0, 0)?;
    a.run(2)?;
    let shield = fixed::to_float(a.sim().shield_of(ember));
    if (shield - 50.0).abs() > 0.01 {
        return Err(format!("flame guard shield {shield}, want 50"));
    }
    let before = a.hp(ember);
    a.sim().spell_damage(enemy as i32, ember, 30, PHYSICAL);
    if a.hp(ember) >= before {
        return Err("a physical hit was absorbed by the magic barrier".into());
    }
    let before = a.hp(ember);
    a.sim().spell_damage(enemy as i32, ember, 30, MAGIC);
    if a.hp(ember) != before {
        return Err("a magical hit under the barrier cost life".into());
    }
    Ok(())
}

pub fn black_hole_pull(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let enigma = a.hero("enigma", 0, 0, 0, 6)?;
    a.learn(enigma, "enigma_black_hole", 1)?;
    let victim = a.hero("kunkka", 1, 4, 0, 1)?;
    a.pin(victim);
    a.pin(enigma);
    let (px, py) = a.at(0, 0);
    a.run(2)?;
    let before = a.cells_between(victim, enigma);
    a.cast(enigma, "enigma_black_hole", -1, px, py)?;
    a.run(40)?;
    let after = a.cells_between(victim, enigma);
    if before - after < 0.8 {
        return Err(format!("victim moved {:.2} cells toward the centre in 40 ticks, want at least 0.8", before - after));
    }
    if a.sim().status_left(victim, STUN) <= 0 {
        return Err("an enemy hero in the hole is not held".into());
    }
    Ok(())
}

pub fn untouchable_attacker_slow(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let ench = a.hero("enchantress", 0, 0, 0, 1)?;
    a.learn(ench, "enchantress_untouchable", 1)?;
    let creep = a.creep(1, 1, 0)?;
    a.sim().clear_status(creep, DISARM);
    a.strike(creep, ench)?;
    let slowed = a.sim().stat(creep, S_ATTACK_SPEED);
    if slowed != -40 {
        return Err(format!("attacker attack speed bonus {slowed}, want -40"));
    }
    Ok(())
}

pub fn x_marks_return(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let kunkka = a.hero("kunkka", 0, 0, 0, 1)?;
    a.learn(kunkka, "kunkka_x_marks_the_spot", 1)?;
    a.pin(kunkka);
    let victim = a.hero("enigma", 1, 4, 0, 1)?;
    a.pin(victim);
    let marked_at = a.sim().position_fixed(victim);
    a.cast(kunkka, "kunkka_x_marks_the_spot", victim as i32, 0, 0)?;
    a.run(2)?;
    let (x, y) = a.at(12, 6);
    a.sim().set_position(victim, x, y);
    a.run(60)?;
    if a.sim().position_fixed(victim) != (x, y) {
        return Err("the marked unit returned before 4 s".into());
    }
    a.run(10)?;
    if a.sim().position_fixed(victim) != marked_at {
        return Err("the marked unit is not back at its mark after 4 s".into());
    }
    Ok(())
}

pub fn ghost_ship_crash(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let kunkka = a.hero("kunkka", 0, -15, 0, 6)?;
    a.learn(kunkka, "kunkka_ghost_ship", 1)?;
    let ally = a.hero("enigma", 0, -10, 2, 1)?;
    let victim = a.hero("enigma", 1, 1, 0, 1)?;
    a.pin(victim);
    let enemy = a.creep(1, 1, 8)?;
    let (px, py) = a.at(0, 0);
    a.run(2)?;
    let before = a.hp(victim);
    a.cast(kunkka, "kunkka_ghost_ship", -1, px, py)?;
    a.run(30)?;
    if a.sim().stat(ally, S_SPEED_PCT) != 10 {
        return Err(format!("ally on the ship's path has speed bonus {}, want 10", a.sim().stat(ally, S_SPEED_PCT)));
    }
    a.sim().spell_damage(enemy as i32, ally, 100, MAGIC);
    a.run(1)?;
    if a.sim().counter(ally, "gs_owed") <= 0 {
        return Err("the boarded ally deferred no damage".into());
    }
    a.run(20)?;
    if a.sim().status_left(victim, STUN) <= 0 {
        return Err("the crash did not stun the enemy at its landing point".into());
    }
    let lost = before - a.hp(victim);
    if lost < 200.0 {
        return Err(format!("the crash did {lost:.1} to the enemy, want about 300 after magic resistance"));
    }
    Ok(())
}

pub fn enchant_convert(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let ench = a.hero("enchantress", 0, 0, 0, 1)?;
    a.learn(ench, "enchantress_enchant", 1)?;
    let creep = a.creep(1, 3, 0)?;
    a.cast(ench, "enchantress_enchant", creep as i32, 0, 0)?;
    a.run(4)?;
    if a.sim().player(creep) != 0 {
        return Err("the enchanted creep is not the Enchantress's".into());
    }
    a.run(80 * 16)?;
    if a.sim().alive(creep) {
        return Err("the enchanted creep outlived its 80 s".into());
    }
    Ok(())
}

pub fn attendants_heal(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let ench = a.hero("enchantress", 0, 0, 0, 1)?;
    a.learn(ench, "enchantress_nature_s_attendants", 1)?;
    let friend = a.creep(0, 3, 0)?;
    let low = fixed::from_int(1);
    a.sim().set_hp_within(friend, low);
    a.cast(ench, "enchantress_nature_s_attendants", -1, 0, 0)?;
    a.run(2 * 16 + 2)?;
    let healed = a.hp(friend) - 1.0;
    if healed < 10.0 {
        return Err(format!("the wisps healed the wounded ally {healed:.1} in two pulses, want a multiple of 10"));
    }
    Ok(())
}

pub fn malefice_pulses(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let enigma = a.hero("enigma", 0, 0, 0, 1)?;
    a.learn(enigma, "enigma_malefice", 1)?;
    a.pin(enigma);
    let victim = a.hero("kunkka", 1, 4, 0, 1)?;
    a.pin(victim);
    a.run(2)?;
    let before = a.hp(victim);
    a.cast(enigma, "enigma_malefice", victim as i32, 0, 0)?;
    let (mut stunned_late, mut hits) = (false, 0);
    let mut last = before;
    for tick in 0..80 {
        a.run(1)?;
        let now = a.hp(victim);
        if now < last - 5.0 {
            hits += 1;
        }
        last = now;
        if tick > 30 && a.sim().status_left(victim, STUN) > 0 {
            stunned_late = true;
        }
    }
    if hits < 3 {
        return Err(format!("malefice hit {hits} times in 5 s, want 3 pulses"));
    }
    if !stunned_late {
        return Err("malefice did not stun at its later pulses".into());
    }
    Ok(())
}

pub fn demonic_conversion(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let enigma = a.hero("enigma", 0, 0, 0, 1)?;
    a.learn(enigma, "enigma_demonic_conversion", 1)?;
    a.pin(enigma);
    let victim = a.creep(1, 3, 0)?;
    let foe = a.creep(1, 6, 0)?;
    a.cast(enigma, "enigma_demonic_conversion", victim as i32, 0, 0)?;
    a.run(6)?;
    if a.sim().alive(victim) {
        return Err("the converted creep lives".into());
    }
    let eidolons = |a: &mut Arena| -> Vec<usize> {
        let s = a.sim();
        (0..s.count()).filter(|&u| s.alive(u) && s.player(u) == 0 && s.kind_name(u).is_some_and(|n| n.starts_with("enigma_eidolon_"))).collect()
    };
    let first = eidolons(&mut a);
    if first.len() != 3 {
        return Err(format!("{} eidolons after the cast, want 3", first.len()));
    }
    for &u in &first {
        a.sim().set_counter(u, "eidolon_hits", 5);
        a.strike(u, foe)?;
    }
    a.run(100)?;
    let after = eidolons(&mut a).len();
    if after <= 3 {
        return Err(format!("{after} eidolons after three sixth attacks, want more than 3"));
    }
    Ok(())
}

pub fn fire_remnant(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let ember = a.hero("ember_spirit", 0, 0, 0, 6)?;
    a.learn(ember, "ember_spirit_fire_remnant_learning", 1)?;
    a.pin(ember);
    let (px, py) = a.at(8, 0);
    a.cast(ember, "ember_spirit_fire_remnant_learning", -1, px, py)?;
    a.run(40)?;
    let remnants = |a: &mut Arena| -> Vec<usize> {
        let s = a.sim();
        (0..s.count()).filter(|&u| s.alive(u) && s.kind_name(u) == Some("ember_spirit_remnant")).collect()
    };
    let made = remnants(&mut a);
    if made.len() != 1 {
        return Err(format!("{} remnants after the cast, want 1", made.len()));
    }
    let remnant = made[0];
    let foe = a.creep(1, 9, 1)?;
    let before = a.hp(foe);
    if a.sim().ability_slot(ember, "ember_spirit_fire_remnant_activate").is_none() {
        return Err("the activation ability was not granted".into());
    }
    a.cast(ember, "ember_spirit_fire_remnant_activate", remnant as i32, 0, 0)?;
    a.run(4)?;
    if a.sim().alive(remnant) {
        return Err("the remnant survived its detonation".into());
    }
    if before - a.hp(foe) < 50.0 {
        return Err(format!("the detonation did {:.1} to a unit beside the remnant", before - a.hp(foe)));
    }
    if a.cells_between(ember, foe) > 3.0 {
        return Err("Ember did not arrive at the remnant".into());
    }
    Ok(())
}
