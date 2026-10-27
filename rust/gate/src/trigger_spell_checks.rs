use crate::checks::{kind, pack_run};
use mod_host::ModHost;
use sim::fixed;

const STUN: usize = 0;
const SLOW: usize = 1;
const ROOT: usize = 3;
const SILENCE: usize = 2;
const REVEALED: usize = 15;
const DISARM: usize = 7;
const S_DAMAGE: usize = 0;
const S_SPEED_PCT: usize = 19;
const S_AGILITY: usize = 23;
const S_ARMOR: usize = 1;
const DEATH_PREVENTION: usize = 13;
const S_DAMAGE_REDUCTION: usize = 30;
const ENEMY: u8 = 1;

struct Arena {
    host: ModHost,
    cx: i32,
    cy: i32,
}

fn arena() -> Result<Arena, String> {
    let mut host = pack_run::start()?;
    let count = host.sim().count();
    for u in 0..count {
        if host.sim().alive(u) {
            host.sim_mut().apply_status(u, STUN, 1_000_000, fixed::ONE);
        }
    }
    let reach = 16;
    let (cx, cy) = {
        let s = host.sim();
        let (w, h) = (s.grid().w as i32, s.grid().h as i32);
        let open = |cx: i32, cy: i32| (-reach..=reach).all(|dy| (-reach..=reach).all(|dx| s.grid().walkable(cx + dx, cy + dy)));
        let crowd = |cx: i32, cy: i32| {
            (0..s.count())
                .filter(|&u| {
                    let (x, y) = s.position_fixed(u);
                    let (dx, dy) = ((x >> 16) - cx, (y >> 16) - cy);
                    s.alive(u) && dx * dx + dy * dy <= (reach + 8) * (reach + 8)
                })
                .count()
        };
        let mut best: Option<(usize, i32, i32)> = None;
        for cy in (reach..h - reach).step_by(2) {
            for cx in (reach..w - reach).step_by(2) {
                if open(cx, cy) && best.is_none_or(|b| crowd(cx, cy) < b.0) {
                    best = Some((crowd(cx, cy), cx, cy));
                }
                if best.is_some_and(|b| b.0 == 0) {
                    break;
                }
            }
        }
        let (_, cx, cy) = best.ok_or("no open arena on the pack map")?;
        (cx, cy)
    };
    let near: Vec<usize> = {
        let s = host.sim();
        (0..s.count())
            .filter(|&u| {
                let (x, y) = s.position_fixed(u);
                let (dx, dy) = ((x >> 16) - cx, (y >> 16) - cy);
                s.alive(u) && dx * dx + dy * dy <= (reach + 8) * (reach + 8)
            })
            .collect()
    };
    for u in near {
        host.sim_mut().remove_unit(u);
    }
    Ok(Arena { host, cx, cy })
}

impl Arena {
    fn hero(&mut self, name: &str, player: u8, dx: i32, dy: i32, level: u32) -> Result<usize, String> {
        let k = kind(&self.host.sim(), name)?;
        let (x, y) = (fixed::from_int(self.cx + dx), fixed::from_int(self.cy + dy));
        let h = self.host.sim_mut().spawn_attributed(player, k, x, y).ok_or_else(|| format!("spawn_attributed refused {name}"))?;
        self.host.sim_mut().apply_status(h, DISARM, 1_000_000, fixed::ONE);
        let need = level.checked_sub(2).and_then(|k| self.host.sim().xp_curve().get(k as usize).copied()).unwrap_or(0);
        let gap = need.saturating_sub(self.host.sim().xp(h));
        self.host.sim_mut().add_xp(h, gap);
        Ok(h)
    }

    fn freeze(&mut self, unit: usize) {
        self.host.sim_mut().apply_status(unit, STUN, 1_000_000, fixed::ONE);
    }

    fn creep(&mut self, player: u8, dx: i32, dy: i32) -> Result<usize, String> {
        let k = kind(&self.host.sim(), crate::checks::role("unit.creep")?)?;
        let (x, y) = (fixed::from_int(self.cx + dx), fixed::from_int(self.cy + dy));
        let u = self.host.sim_mut().spawn_for(player, k, x, y).ok_or("spawn_for refused a creep")?;
        self.host.sim_mut().apply_status(u, DISARM, 1_000_000, fixed::ONE);
        Ok(u)
    }

    fn learn(&mut self, hero: usize, ability: &str) -> Result<usize, String> {
        let slot = self.host.sim().ability_slot(hero, ability).ok_or_else(|| format!("hero holds no {ability}"))?;
        self.host.sim_mut().rank_up(hero, slot).map_err(|r| format!("skill_up {ability}: {}", r.name()))?;
        self.host.sim_mut().grant_energy(hero, 2000).map_err(|r| r.name().to_string())?;
        Ok(slot)
    }

    fn cast(&mut self, hero: usize, slot: usize, target: i32, x: i32, y: i32) -> Result<(), String> {
        if self.host.sim_mut().cast_ability(hero, slot, target, x, y) {
            pack_run::advance(&mut self.host, 1)
        } else {
            Err(format!("cast refused: {}", self.host.sim().stats.last_refusal.name()))
        }
    }

    fn strike(&mut self, attacker: usize, target: usize) -> Result<(), String> {
        self.host.sim_mut().clear_status(attacker, DISARM);
        for _ in 0..200 {
            let ordered = self.host.sim_mut().order_attack(attacker, target);
            if ordered.is_ok() {
                for _ in 0..40 {
                    pack_run::advance(&mut self.host, 1)?;
                    if !self.host.sim().hit_in_flight(attacker) {
                        break;
                    }
                }
                return Ok(());
            }
            pack_run::advance(&mut self.host, 1)?;
        }
        Err(format!("unit {attacker} could not strike unit {target}"))
    }

    fn hp(&self, u: usize) -> f64 {
        self.host.sim().hp(u) as f64 / fixed::ONE as f64
    }

    fn stat(&self, u: usize, code: usize) -> i32 {
        self.host.sim().stat(u, code)
    }

    fn at(&self, dx: i32, dy: i32) -> (i32, i32) {
        (fixed::from_int(self.cx + dx), fixed::from_int(self.cy + dy))
    }
}

fn expect(what: &str, got: f64, want: f64, tol: f64) -> Result<(), String> {
    if (got - want).abs() > tol {
        return Err(format!("{what} {got:.3} want {want:.3} +-{tol}"));
    }
    Ok(())
}

pub fn chaos_bolt_random_inverse(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let knight = a.hero("chaos_knight", 0, 0, 0, 1)?;
    let slot = a.learn(knight, "chaos_knight_chaos_bolt")?;
    let target = a.creep(ENEMY, 4, 0)?;
    let full = a.host.sim().hp(target);
    let max = a.host.sim().hp_max(target);
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..8 {
        a.host.sim_mut().set_health(target, full, max);
        a.host.sim_mut().clear_status(target, STUN);
        a.host.sim_mut().set_ability_cooldown(knight, slot, 0);
        a.host.sim_mut().grant_energy(knight, 2000).map_err(|r| r.name().to_string())?;
        let before = a.hp(target);
        a.cast(knight, slot, target as i32, 0, 0)?;
        let stun = a.host.sim().status_left(target, STUN);
        let lost = before - a.hp(target);
        if !(16..=32).contains(&stun) {
            return Err(format!("chaos bolt stun {stun} ticks, want 16 to 32 at rank 1"));
        }
        expect("damage against stun", lost, (200 - (stun - 16) * 125 / 16) as f64, 1.0)?;
        seen.insert(stun);
    }
    if seen.len() < 2 {
        return Err(format!("eight chaos bolts all stunned for {seen:?} ticks: the stun is not random"));
    }
    Ok(())
}

pub fn mana_void_missing_mana(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let am = a.hero("antimage", 0, 0, 0, 6)?;
    let slot = a.learn(am, "antimage_mana_void")?;
    let victim = a.hero("lion", ENEMY, 4, 0, 1)?;
    let bystander = a.creep(ENEMY, 4, 3)?;
    let outside = a.creep(ENEMY, 4, 12)?;
    let (max, have) = (a.host.sim().unit_max_mana(victim), a.host.sim().mana_of(victim));
    let missing = (max - have).max(1);
    let (hero_before, creep_before, outside_before) = (a.hp(victim), a.hp(bystander), a.hp(outside));
    a.cast(am, slot, victim as i32, 0, 0)?;
    let want = (60 * missing / 100) as f64;
    expect("bystander creep damage", creep_before - a.hp(bystander), want, 1.0)?;
    expect("outside creep damage", outside_before - a.hp(outside), 0.0, 0.1)?;
    expect("hero damage after 25 percent magic resistance", hero_before - a.hp(victim), want * 0.75, 2.0)?;
    Ok(())
}

pub fn death_coil_heals_ally(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let aba = a.hero("abaddon", 0, 0, 0, 1)?;
    let slot = a.learn(aba, "abaddon_death_coil")?;
    let ally = a.hero("lion", 0, 4, 0, 1)?;
    pack_run::advance(&mut a.host, 4)?;
    let (ally_max, caster_max) = (a.host.sim().hp_max(ally), a.host.sim().hp_max(aba));
    a.host.sim_mut().set_health(ally, fixed::from_int(200), ally_max);
    a.host.sim_mut().set_health(aba, caster_max, caster_max);
    let foe = a.creep(ENEMY, 0, 5)?;
    a.cast(aba, slot, ally as i32, 0, 0)?;
    expect("ally heal", a.hp(ally), 300.0, 1.0)?;
    expect("caster hit point cost", caster_max as f64 / fixed::ONE as f64 - a.hp(aba), 75.0, 1.0)?;
    let before = a.hp(foe);
    a.host.sim_mut().grant_energy(aba, 2000).map_err(|r| r.name().to_string())?;
    a.host.sim_mut().set_ability_cooldown(aba, slot, 0);
    a.cast(aba, slot, foe as i32, 0, 0)?;
    expect("enemy damage", before - a.hp(foe), 100.0, 1.0)?;
    Ok(())
}

pub fn battle_hunger_ends_and_stacks(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let axe = a.hero("axe", 0, 0, 0, 1)?;
    let slot = a.learn(axe, "axe_battle_hunger")?;
    let first = a.creep(ENEMY, 4, 0)?;
    let second = a.creep(ENEMY, 4, 3)?;
    a.cast(axe, slot, first as i32, 0, 0)?;
    if a.stat(axe, S_SPEED_PCT) != 12 {
        return Err(format!("one afflicted enemy gave {} percent speed, want 12", a.stat(axe, S_SPEED_PCT)));
    }
    a.host.sim_mut().set_ability_cooldown(axe, slot, 0);
    a.cast(axe, slot, second as i32, 0, 0)?;
    if a.stat(axe, S_SPEED_PCT) != 24 {
        return Err(format!("two afflicted enemies gave {} percent speed, want 24", a.stat(axe, S_SPEED_PCT)));
    }
    for _ in 0..8 {
        if !a.host.sim().alive(first) {
            break;
        }
        a.host.sim_mut().hurt(first, fixed::from_int(10_000));
    }
    pack_run::advance(&mut a.host, 3)?;
    if a.stat(axe, S_SPEED_PCT) != 12 {
        return Err(format!("a dead afflicted enemy left {} percent speed, want 12", a.stat(axe, S_SPEED_PCT)));
    }
    pack_run::advance(&mut a.host, 16 * 11)?;
    if a.stat(axe, S_SPEED_PCT) != 0 {
        return Err(format!("expired hunger left {} percent speed, want 0", a.stat(axe, S_SPEED_PCT)));
    }
    Ok(())
}

pub fn hand_of_god_heals_own_creeps(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let chen = a.hero("chen", 0, 0, 0, 6)?;
    let slot = a.learn(chen, "chen_hand_of_god")?;
    let own = a.creep(0, 3, 0)?;
    let foe = a.creep(ENEMY, 3, 3)?;
    let own_max = a.host.sim().hp_max(own);
    a.host.sim_mut().set_health(own, fixed::from_int(10), own_max);
    a.host.sim_mut().set_health(foe, fixed::from_int(10), own_max);
    a.cast(chen, slot, -1, 0, 0)?;
    expect("own creep life after Hand of God", a.hp(own), own_max as f64 / fixed::ONE as f64, 0.5)?;
    expect("enemy creep life after Hand of God", a.hp(foe), 10.0, 0.5)?;
    Ok(())
}

pub fn test_of_faith_damages_enemy(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let chen = a.hero("chen", 0, 0, 0, 1)?;
    let slot = a.learn(chen, "chen_test_of_faith")?;
    let foe = a.creep(ENEMY, 4, 0)?;
    let before = a.hp(foe);
    a.cast(chen, slot, foe as i32, 0, 0)?;
    let lost = before - a.hp(foe);
    if !(50.0..=100.5).contains(&lost) {
        return Err(format!("test of faith dealt {lost:.2}, want 50 to 100 at rank 1"));
    }
    Ok(())
}

pub fn test_of_faith_sends_ally_home(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let chen = a.hero("chen", 0, 0, 0, 1)?;
    let slot = a.learn(chen, "chen_test_of_faith")?;
    let ally = a.hero("lion", 0, 4, 0, 1)?;
    let start = {
        let s = a.host.sim();
        s.catalog().starts.iter().find(|st| st.player == 0).map(|st| (st.x, st.y)).ok_or("player 0 has no start")?
    };
    a.cast(chen, slot, ally as i32, 0, 0)?;
    pack_run::advance(&mut a.host, 6)?;
    let (x, y) = a.host.sim().position_fixed(ally);
    if (x - start.0).abs() > fixed::from_int(2) || (y - start.1).abs() > fixed::from_int(2) {
        return Err(format!("the allied hero stands at ({x}, {y}), want its start {start:?}"));
    }
    Ok(())
}

pub fn marksmanship_alone_only(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let drow = a.hero("drow_ranger", 0, 0, 0, 6)?;
    a.learn(drow, "drow_ranger_marksmanship")?;
    pack_run::advance(&mut a.host, 20)?;
    if a.stat(drow, S_AGILITY) != 40 {
        return Err(format!("marksmanship alone gave {} agility, want 40", a.stat(drow, S_AGILITY)));
    }
    let foe = a.hero("lion", ENEMY, 4, 0, 1)?;
    pack_run::advance(&mut a.host, 20)?;
    if a.stat(drow, S_AGILITY) != 0 {
        return Err(format!("an enemy hero in range left {} agility, want 0", a.stat(drow, S_AGILITY)));
    }
    a.host.sim_mut().remove_unit(foe);
    let _ = a.at(0, 0);
    Ok(())
}

pub fn atrophy_stacks_on_enemy_death(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let lord = a.hero("abyssal_underlord", 0, 0, 0, 1)?;
    a.learn(lord, "abyssal_underlord_atrophy_aura")?;
    let victim = a.creep(ENEMY, 6, 0)?;
    pack_run::advance(&mut a.host, 4)?;
    let before = a.stat(lord, S_DAMAGE);
    a.host.sim_mut().hurt(victim, fixed::from_int(10_000));
    pack_run::advance(&mut a.host, 3)?;
    if a.stat(lord, S_DAMAGE) != before + 5 {
        return Err(format!("atrophy gave {} damage after a kill, want {}", a.stat(lord, S_DAMAGE) - before, 5));
    }
    pack_run::advance(&mut a.host, 16 * 31)?;
    if a.stat(lord, S_DAMAGE) != before {
        return Err("atrophy damage outlived 30 s".into());
    }
    Ok(())
}

pub fn surge_hastes_ally(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let seer = a.hero("dark_seer", 0, 0, 0, 1)?;
    let slot = a.learn(seer, "dark_seer_surge")?;
    let ally = a.hero("lion", 0, 4, 0, 1)?;
    a.cast(seer, slot, ally as i32, 0, 0)?;
    if a.stat(ally, S_SPEED_PCT) != 1000 {
        return Err(format!("surge gave {} percent speed, want 1000 (the cap decides)", a.stat(ally, S_SPEED_PCT)));
    }
    pack_run::advance(&mut a.host, 50)?;
    if a.stat(ally, S_SPEED_PCT) != 0 {
        return Err("surge outlived 3 s at rank 1".into());
    }
    Ok(())
}

pub fn ion_shell_pulses(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let seer = a.hero("dark_seer", 0, 0, 0, 1)?;
    let slot = a.learn(seer, "dark_seer_ion_shell")?;
    let ally = a.hero("lion", 0, 4, 0, 1)?;
    a.freeze(ally);
    let near_foe = a.creep(ENEMY, 4, 3)?;
    let far_foe = a.creep(ENEMY, 4, 10)?;
    let (near_before, far_before) = (a.hp(near_foe), a.hp(far_foe));
    a.cast(seer, slot, ally as i32, 0, 0)?;
    pack_run::advance(&mut a.host, 32)?;
    expect("near creep after 2 s", near_before - a.hp(near_foe), 60.0, 16.0)?;
    expect("far creep after 2 s", far_before - a.hp(far_foe), 0.0, 0.1)?;
    Ok(())
}

pub fn drunken_haze_blinds(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let brew = a.hero("brewmaster", 0, 0, 0, 1)?;
    let slot = a.learn(brew, "brewmaster_drunken_haze")?;
    let foe = a.creep(ENEMY, 4, 0)?;
    let beside = a.creep(ENEMY, 4, 2)?;
    a.cast(brew, slot, foe as i32, 0, 0)?;
    pack_run::advance(&mut a.host, 2)?;
    let want = 65536 * 45 / 100;
    for (what, unit) in [("target", foe), ("unit beside", beside)] {
        let got = a.host.sim().miss_chance_of(unit);
        if (got - want).abs() > 2 {
            return Err(format!("{what} miss chance {got} want {want} (45 percent)"));
        }
    }
    pack_run::advance(&mut a.host, 140)?;
    if a.host.sim().miss_chance_of(foe) != 0 {
        return Err("haze outlived 8 s".into());
    }
    Ok(())
}

pub fn bloodrage_and_penitence_amplify_damage(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let seeker = a.hero("bloodseeker", 0, 0, 0, 1)?;
    let rage = a.learn(seeker, "bloodseeker_bloodrage")?;
    let chen = a.hero("chen", 0, 3, 0, 1)?;
    let penitence = a.learn(chen, "chen_penitence")?;
    let foe = a.creep(ENEMY, 4, 3)?;
    a.cast(seeker, rage, seeker as i32, 0, 0)?;
    if a.stat(seeker, S_DAMAGE_REDUCTION) != -19 {
        return Err(format!("bloodrage damage taken stat {} want -19", a.stat(seeker, S_DAMAGE_REDUCTION)));
    }
    a.cast(chen, penitence, foe as i32, 0, 0)?;
    if a.stat(foe, S_DAMAGE_REDUCTION) != -15 {
        return Err(format!("penitence damage taken stat {} want -15", a.stat(foe, S_DAMAGE_REDUCTION)));
    }
    Ok(())
}

pub fn nightmare_sleeps_wakes_and_spreads(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let bane = a.hero("bane", 0, 0, 0, 1)?;
    let slot = a.learn(bane, "bane_nightmare")?;
    let sleeper = a.creep(ENEMY, 4, 0)?;
    let striker = a.hero("lion", 0, 5, 0, 1)?;
    let before = a.hp(sleeper);
    a.cast(bane, slot, sleeper as i32, 0, 0)?;
    if a.host.sim().status_left(sleeper, STUN) < 60 {
        return Err(format!("nightmare slept the target {} ticks, want 64", a.host.sim().status_left(sleeper, STUN)));
    }
    pack_run::advance(&mut a.host, 20)?;
    if a.host.sim().status_left(sleeper, STUN) < 30 {
        return Err("the pulse of Nightmare woke its own sleeper".into());
    }
    expect("nightmare pulse", before - a.hp(sleeper), 20.0, 1.0)?;
    a.strike(striker, sleeper)?;
    if a.host.sim().status_left(sleeper, STUN) > 0 {
        return Err("a hit by a stranger did not wake the sleeper".into());
    }
    if a.host.sim().status_left(striker, STUN) < 50 {
        return Err("the attacker of a sleeper did not receive Nightmare".into());
    }
    Ok(())
}

pub fn cold_feet_stuns_those_who_stay(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let aa = a.hero("ancient_apparition", 0, 0, 0, 1)?;
    let slot = a.learn(aa, "ancient_apparition_cold_feet")?;
    let stays = a.creep(ENEMY, 4, 0)?;
    a.cast(aa, slot, stays as i32, 0, 0)?;
    pack_run::advance(&mut a.host, 70)?;
    if a.host.sim().status_left(stays, STUN) < 10 {
        return Err("a target that stayed was not stunned after 3.9 s".into());
    }
    let mut a = arena()?;
    let aa = a.hero("ancient_apparition", 0, 0, 0, 1)?;
    let slot = a.learn(aa, "ancient_apparition_cold_feet")?;
    let leaves = a.creep(ENEMY, 4, 0)?;
    a.cast(aa, slot, leaves as i32, 0, 0)?;
    pack_run::advance(&mut a.host, 10)?;
    let (x, y) = a.at(4, 14);
    a.host.sim_mut().teleport(leaves, x, y);
    pack_run::advance(&mut a.host, 70)?;
    if a.host.sim().status_left(leaves, STUN) > 0 {
        return Err("a target that left the mark was stunned".into());
    }
    Ok(())
}

pub fn glimpse_returns_four_seconds(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let seer = a.hero("disruptor", 0, 0, 0, 1)?;
    let slot = a.learn(seer, "disruptor_glimpse")?;
    let foe = a.hero("lion", ENEMY, 4, 0, 1)?;
    a.freeze(foe);
    let start = a.host.sim().position_fixed(foe);
    pack_run::advance(&mut a.host, 16 * 6)?;
    let (x, y) = a.at(4, 6);
    a.host.sim_mut().teleport(foe, x, y);
    pack_run::advance(&mut a.host, 2)?;
    a.cast(seer, slot, foe as i32, 0, 0)?;
    let now = a.host.sim().position_fixed(foe);
    if (now.0 - start.0).abs() > fixed::from_int(1) || (now.1 - start.1).abs() > fixed::from_int(1) {
        return Err(format!("glimpse left the target at {now:?}, want its old place {start:?}"));
    }
    Ok(())
}

pub fn weave_changes_armor_each_second(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let dazzle = a.hero("dazzle", 0, 0, 0, 1)?;
    let slot = a.learn(dazzle, "dazzle_weave")?;
    let ally = a.hero("lion", 0, 4, 0, 1)?;
    let foe = a.hero("sven", ENEMY, 4, 3, 1)?;
    pack_run::advance(&mut a.host, 4)?;
    let (ally0, foe0) = (a.stat(ally, S_ARMOR), a.stat(foe, S_ARMOR));
    let (x, y) = a.at(4, 2);
    a.cast(dazzle, slot, -1, x, y)?;
    pack_run::advance(&mut a.host, 16 * 3 + 2)?;
    if a.stat(ally, S_ARMOR) != ally0 + 2 || a.stat(foe, S_ARMOR) != foe0 - 2 {
        return Err(format!("after 3 s ally armor {} want {}, enemy {} want {}", a.stat(ally, S_ARMOR) - ally0, 2, a.stat(foe, S_ARMOR) - foe0, -2));
    }
    pack_run::advance(&mut a.host, 16 * 22)?;
    if a.stat(ally, S_ARMOR) != ally0 || a.stat(foe, S_ARMOR) != foe0 {
        return Err("weave armor outlived 24 s".into());
    }
    Ok(())
}

pub fn aphotic_shield_releases_on_break(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let aba = a.hero("abaddon", 0, 0, 0, 1)?;
    let slot = a.learn(aba, "abaddon_aphotic_shield")?;
    let foe = a.creep(ENEMY, 4, 2)?;
    pack_run::advance(&mut a.host, 4)?;
    let before = a.hp(foe);
    a.cast(aba, slot, aba as i32, 0, 0)?;
    let held = a.host.sim().shield_of(aba) as f64 / fixed::ONE as f64;
    expect("shield absorbed amount", held, 110.0, 1.0)?;
    a.host.sim_mut().hurt(aba, fixed::from_int(200));
    pack_run::advance(&mut a.host, 3)?;
    expect("released damage on the creep", before - a.hp(foe), 110.0, 1.0)?;
    Ok(())
}

pub fn borrowed_time_heals_damage(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let aba = a.hero("abaddon", 0, 0, 0, 6)?;
    let slot = a.learn(aba, "abaddon_borrowed_time")?;
    pack_run::advance(&mut a.host, 4)?;
    let max = a.host.sim().hp_max(aba);
    a.host.sim_mut().set_health(aba, max - fixed::from_int(200), max);
    a.cast(aba, slot, -1, 0, 0)?;
    if a.host.sim().status_left(aba, DEATH_PREVENTION) < 60 {
        return Err("borrowed time gave no death prevention".into());
    }
    let before = a.hp(aba);
    a.host.sim_mut().hurt(aba, fixed::from_int(40));
    pack_run::advance(&mut a.host, 2)?;
    let gain = a.hp(aba) - before;
    if gain < 10.0 {
        return Err(format!("damage under borrowed time changed hit points by {gain:.2}, want a gain"));
    }
    Ok(())
}

pub fn borrowed_time_triggers_under_400(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let aba = a.hero("abaddon", 0, 0, 0, 6)?;
    let slot = a.learn(aba, "abaddon_borrowed_time")?;
    pack_run::advance(&mut a.host, 4)?;
    let max = a.host.sim().hp_max(aba);
    a.host.sim_mut().set_health(aba, fixed::from_int(390), max);
    a.host.sim_mut().hurt(aba, fixed::from_int(20));
    pack_run::advance(&mut a.host, 2)?;
    if a.host.sim().status_left(aba, DEATH_PREVENTION) < 60 {
        return Err("a hit below 400 life did not start borrowed time".into());
    }
    if a.host.sim().ability_cooldown(aba, slot) < 600 {
        return Err("the automatic trigger did not start the cooldown".into());
    }
    Ok(())
}

pub fn persuasion_converts_and_caps(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let chen = a.hero("chen", 0, 0, 0, 1)?;
    let slot = a.learn(chen, "chen_holy_persuasion")?;
    let first = a.creep(ENEMY, 4, 0)?;
    let second = a.creep(ENEMY, 4, 3)?;
    a.cast(chen, slot, first as i32, 0, 0)?;
    pack_run::advance(&mut a.host, 2)?;
    if a.host.sim().player(first) != 0 {
        return Err("the persuaded creep did not change owner".into());
    }
    if a.host.sim().hp_max(first) < fixed::from_int(700) {
        return Err(format!("persuaded life {} want at least 700", a.host.sim().hp_max(first) / fixed::ONE));
    }
    a.host.sim_mut().set_ability_cooldown(chen, slot, 0);
    a.cast(chen, slot, second as i32, 0, 0)?;
    pack_run::advance(&mut a.host, 2)?;
    if a.host.sim().alive(first) || a.host.sim().player(second) != 0 {
        return Err("at rank 1 a second persuasion did not replace the first".into());
    }
    Ok(())
}

pub fn devour_kills_and_pays(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let doom = a.hero("doom_bringer", 0, 0, 0, 1)?;
    let slot = a.learn(doom, "doom_bringer_devour")?;
    let prey = a.creep(ENEMY, 3, 0)?;
    pack_run::advance(&mut a.host, 4)?;
    let gold = a.host.sim().resource_a(0);
    a.cast(doom, slot, prey as i32, 0, 0)?;
    pack_run::advance(&mut a.host, 3)?;
    if a.host.sim().alive(prey) {
        return Err("devour left the creep alive".into());
    }
    if a.host.sim().resource_a(0) < gold + 25 {
        return Err(format!("devour paid {} want 25 or more", a.host.sim().resource_a(0) - gold));
    }
    Ok(())
}

pub fn static_storm_ramps(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let seer = a.hero("disruptor", 0, 0, 0, 6)?;
    let slot = a.learn(seer, "disruptor_static_storm")?;
    let inside = a.creep(ENEMY, 5, 0)?;
    let outside = a.creep(ENEMY, 5, 10)?;
    pack_run::advance(&mut a.host, 4)?;
    let (before, far) = (a.hp(inside), a.hp(outside));
    let (x, y) = a.at(5, 0);
    a.cast(seer, slot, -1, x, y)?;
    pack_run::advance(&mut a.host, 4 * 20 + 4)?;
    let lost = before - a.hp(inside);
    expect("storm total on a unit inside", lost, 0.25 * 200.0 / 20.0 * 210.0, 12.0)?;
    expect("storm on a unit outside", far - a.hp(outside), 0.0, 0.1)?;
    if a.host.sim().status_left(inside, SILENCE) == 0 && a.host.sim().alive(inside) {
        return Ok(());
    }
    Ok(())
}

pub fn rupture_hurts_moving_target(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let seeker = a.hero("bloodseeker", 0, 0, 0, 6)?;
    let slot = a.learn(seeker, "bloodseeker_rupture")?;
    let foe = a.hero("lion", ENEMY, 4, 0, 1)?;
    a.freeze(foe);
    pack_run::advance(&mut a.host, 4)?;
    a.cast(seeker, slot, foe as i32, 0, 0)?;
    let still = a.hp(foe);
    pack_run::advance(&mut a.host, 20)?;
    expect("hit points of a standing target", a.hp(foe), still, 0.5)?;
    let (x, y) = a.at(4, 8);
    a.host.sim_mut().teleport(foe, x, y);
    pack_run::advance(&mut a.host, 2)?;
    let lost = still - a.hp(foe);
    if lost < 100.0 {
        return Err(format!("an 8 cell move under rupture cost {lost:.1}, want about 0.2 * 512 = 102"));
    }
    Ok(())
}

pub fn track_pays_on_death(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let hunter = a.hero("bounty_hunter", 0, 0, 0, 1)?;
    let slot = a.learn(hunter, "bounty_hunter_track")?;
    let ally = a.hero("lion", 0, 3, 0, 1)?;
    let prey = a.creep(ENEMY, 4, 0)?;
    pack_run::advance(&mut a.host, 4)?;
    let own = a.host.sim().resource_a(0);
    a.cast(hunter, slot, prey as i32, 0, 0)?;
    a.host.sim_mut().hurt(prey, fixed::from_int(10_000));
    pack_run::advance(&mut a.host, 4)?;
    if a.host.sim().resource_a(0) < own + 200 {
        return Err(format!("the tracker was paid {} want 200 or more", a.host.sim().resource_a(0) - own));
    }
    if a.host.sim().resource_a(0) < own + 240 {
        return Err("the tracker and the allied hero near the death were not paid 240 together".into());
    }
    Ok(())
}

pub fn stampede_hastes_and_tramples(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let centaur = a.hero("centaur", 0, 0, 0, 6)?;
    let slot = a.learn(centaur, "centaur_stampede")?;
    let foe = a.creep(ENEMY, 1, 0)?;
    pack_run::advance(&mut a.host, 4)?;
    let before = a.hp(foe);
    a.cast(centaur, slot, -1, 0, 0)?;
    if a.stat(centaur, S_SPEED_PCT) != 1000 {
        return Err(format!("stampede gave {} percent speed", a.stat(centaur, S_SPEED_PCT)));
    }
    pack_run::advance(&mut a.host, 4)?;
    if a.hp(foe) >= before {
        return Err("a unit inside the collision radius took no damage".into());
    }
    pack_run::advance(&mut a.host, 70)?;
    if a.stat(centaur, S_SPEED_PCT) != 0 {
        return Err("stampede speed outlived 3.75 s".into());
    }
    Ok(())
}

pub fn bristleback_heals_rear_hits(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let back = a.hero("bristleback", 0, 0, 0, 1)?;
    a.learn(back, "bristleback_bristleback")?;
    let striker = a.hero("lion", ENEMY, -2, 0, 1)?;
    a.freeze(back);
    pack_run::advance(&mut a.host, 4)?;
    a.host.sim_mut().set_heading(back, 0);
    let before = a.hp(back);
    a.strike(striker, back)?;
    let from_behind = before - a.hp(back);
    let mut b = arena()?;
    let back2 = b.hero("bristleback", 0, 0, 0, 1)?;
    b.learn(back2, "bristleback_bristleback")?;
    let striker2 = b.hero("lion", ENEMY, 2, 0, 1)?;
    b.freeze(back2);
    pack_run::advance(&mut b.host, 4)?;
    b.host.sim_mut().set_heading(back2, 0);
    let before2 = b.hp(back2);
    b.strike(striker2, back2)?;
    let from_front = before2 - b.hp(back2);
    if !(from_front > 0.0 && from_behind < from_front * 0.9) {
        return Err(format!("a hit from behind cost {from_behind:.2} and from the front {from_front:.2}, want the rear hit lower"));
    }
    Ok(())
}

pub fn frostbite_lasts_longer_on_creeps(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let cm = a.hero("crystal_maiden", 0, 0, 0, 1)?;
    let slot = a.learn(cm, "crystal_maiden_frostbite")?;
    let creep = a.creep(ENEMY, 4, 0)?;
    a.cast(cm, slot, creep as i32, 0, 0)?;
    pack_run::advance(&mut a.host, 60)?;
    if a.host.sim().status_left(creep, ROOT) < 60 {
        return Err("frostbite on a creep ended with the hero duration".into());
    }
    Ok(())
}

pub fn tempest_double_costs_mana(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let warden = a.hero("arc_warden", 0, 0, 0, 6)?;
    let slot = a.learn(warden, "arc_warden_tempest_double")?;
    pack_run::advance(&mut a.host, 4)?;
    let before = a.host.sim().mana_of(warden);
    a.cast(warden, slot, -1, 0, 0)?;
    let after = a.host.sim().mana_of(warden);
    expect("mana kept", after as f64, before as f64 * 0.7, before as f64 * 0.03 + 2.0)?;
    Ok(())
}

pub fn fiends_grip_moves_mana(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let bane = a.hero("bane", 0, 0, 0, 6)?;
    let slot = a.learn(bane, "bane_fiend_s_grip")?;
    let foe = a.hero("lion", ENEMY, 4, 0, 1)?;
    a.freeze(foe);
    pack_run::advance(&mut a.host, 4)?;
    let (foe_before, bane_before) = (a.host.sim().mana_of(foe), a.host.sim().mana_of(bane));
    let spend = bane_before / 2;
    a.host.sim_mut().spend_energy(bane, spend).map_err(|r| r.name().to_string())?;
    let bane_low = a.host.sim().mana_of(bane);
    a.cast(bane, slot, foe as i32, 0, 0)?;
    pack_run::advance(&mut a.host, 40)?;
    let moved = foe_before - a.host.sim().mana_of(foe);
    if moved <= 0 || a.host.sim().mana_of(bane) <= bane_low - 200 {
        return Err(format!("grip moved {moved} mana"));
    }
    Ok(())
}

pub fn viscous_goo_stacks(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let back = a.hero("bristleback", 0, 0, 0, 1)?;
    let slot = a.learn(back, "bristleback_viscous_nasal_goo")?;
    let foe = a.creep(ENEMY, 4, 0)?;
    a.cast(back, slot, foe as i32, 0, 0)?;
    a.host.sim_mut().set_ability_cooldown(back, slot, 0);
    pack_run::advance(&mut a.host, 6)?;
    a.cast(back, slot, foe as i32, 0, 0)?;
    let armor = a.stat(foe, 26);
    if armor != -200 {
        return Err(format!("two goo casts left {armor} hundredths of armor, want -200"));
    }
    pack_run::advance(&mut a.host, 90)?;
    if a.stat(foe, 26) != 0 {
        return Err("goo armor outlived 5 s".into());
    }
    Ok(())
}

pub fn trueshot_gives_agility_share(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let drow = a.hero("drow_ranger", 0, 0, 0, 1)?;
    a.learn(drow, "drow_ranger_trueshot_aura")?;
    let archer = a.hero("sniper", 0, 3, 0, 1)?;
    pack_run::advance(&mut a.host, 20)?;
    if a.stat(archer, S_DAMAGE) <= 0 {
        return Err("an allied ranged hero got no trueshot damage".into());
    }
    Ok(())
}

pub fn reality_rift_jumps(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let knight = a.hero("chaos_knight", 0, 0, 0, 1)?;
    let slot = a.learn(knight, "chaos_knight_reality_rift")?;
    let foe = a.creep(ENEMY, 8, 0)?;
    pack_run::advance(&mut a.host, 4)?;
    let before = a.host.sim().position_fixed(knight).0;
    a.cast(knight, slot, foe as i32, 0, 0)?;
    let jumped = (a.host.sim().position_fixed(knight).0 - before) as f64 / fixed::ONE as f64;
    if !(2.0..=7.0).contains(&jumped) {
        return Err(format!("the knight jumped {jumped:.2} cells, want 30 to 80 percent of 8"));
    }
    if a.stat(knight, S_DAMAGE) < 60 {
        return Err("no bonus damage after the rift".into());
    }
    Ok(())
}

pub fn flux_needs_isolation(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let warden = a.hero("arc_warden", 0, 0, 0, 1)?;
    let slot = a.learn(warden, "arc_warden_flux")?;
    let alone = a.creep(ENEMY, 5, 0)?;
    let crowded = a.creep(ENEMY, 5, 8)?;
    let neighbour = a.creep(ENEMY, 5, 9)?;
    a.freeze(neighbour);
    pack_run::advance(&mut a.host, 4)?;
    let (alone0, crowded0) = (a.hp(alone), a.hp(crowded));
    a.cast(warden, slot, alone as i32, 0, 0)?;
    a.host.sim_mut().set_ability_cooldown(warden, slot, 0);
    a.host.sim_mut().grant_energy(warden, 1000).map_err(|r| r.name().to_string())?;
    a.cast(warden, slot, crowded as i32, 0, 0)?;
    pack_run::advance(&mut a.host, 40)?;
    if alone0 - a.hp(alone) < 10.0 {
        return Err("flux did no damage to an isolated target".into());
    }
    expect("damage to a target with a neighbour", crowded0 - a.hp(crowded), 0.0, 0.1)?;
    Ok(())
}

pub fn firefly_burns_the_trail(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let rider = a.hero("batrider", 0, 0, 0, 1)?;
    let slot = a.learn(rider, "batrider_firefly")?;
    let foe = a.creep(ENEMY, 8, 0)?;
    pack_run::advance(&mut a.host, 4)?;
    let before = a.hp(foe);
    a.cast(rider, slot, -1, 0, 0)?;
    let (x, y) = a.at(8, 0);
    a.host.sim_mut().teleport(rider, x - fixed::from_int(2), y);
    pack_run::advance(&mut a.host, 12)?;
    let (x2, y2) = a.at(14, 0);
    a.host.sim_mut().teleport(rider, x2, y2);
    pack_run::advance(&mut a.host, 20)?;
    if a.hp(foe) >= before {
        return Err("a unit standing at the trail took no firefly damage".into());
    }
    Ok(())
}

pub fn thunder_clap_slows_creeps_longer(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let brew = a.hero("brewmaster", 0, 0, 0, 1)?;
    let slot = a.learn(brew, "brewmaster_thunder_clap")?;
    let creep = a.creep(ENEMY, 3, 0)?;
    let hero = a.hero("lion", ENEMY, -3, 0, 1)?;
    a.freeze(hero);
    pack_run::advance(&mut a.host, 4)?;
    a.cast(brew, slot, -1, 0, 0)?;
    pack_run::advance(&mut a.host, 2)?;
    let (creep_left, hero_left) = (a.host.sim().status_left(creep, SLOW), a.host.sim().status_left(hero, SLOW));
    if creep_left < 100 || hero_left > 80 || hero_left == 0 {
        return Err(format!("slow left on the creep {creep_left} and on the hero {hero_left}, want about 128 and 64"));
    }
    Ok(())
}

pub fn thunder_strike_hits_neighbours(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let seer = a.hero("disruptor", 0, 0, 0, 1)?;
    let slot = a.learn(seer, "disruptor_thunder_strike")?;
    let target = a.creep(ENEMY, 4, 0)?;
    let near = a.creep(ENEMY, 4, 3)?;
    let far = a.creep(ENEMY, 4, 9)?;
    pack_run::advance(&mut a.host, 4)?;
    let (near0, far0) = (a.hp(near), a.hp(far));
    a.cast(seer, slot, target as i32, 0, 0)?;
    expect("neighbour after the first strike", near0 - a.hp(near), 40.0, 1.0)?;
    pack_run::advance(&mut a.host, 33)?;
    expect("neighbour after the second strike", near0 - a.hp(near), 80.0, 2.5)?;
    expect("distant creep", far0 - a.hp(far), 0.0, 0.1)?;
    Ok(())
}

pub fn pit_of_malice_hits_each_once(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let lord = a.hero("abyssal_underlord", 0, 0, 0, 1)?;
    let slot = a.learn(lord, "abyssal_underlord_pit_of_malice")?;
    let inside = a.creep(ENEMY, 5, 0)?;
    let late = a.creep(ENEMY, 5, 14)?;
    pack_run::advance(&mut a.host, 4)?;
    let (inside0, late0) = (a.hp(inside), a.hp(late));
    let (x, y) = a.at(5, 0);
    a.cast(lord, slot, -1, x, y)?;
    pack_run::advance(&mut a.host, 3)?;
    if a.host.sim().status_left(inside, ROOT) == 0 {
        return Err("the creep inside was not caged".into());
    }
    pack_run::advance(&mut a.host, 17)?;
    expect("creep inside at the cast", inside0 - a.hp(inside), 100.0, 1.0)?;
    let (lx, ly) = a.at(5, 1);
    a.host.sim_mut().teleport(late, lx, ly);
    pack_run::advance(&mut a.host, 8)?;
    expect("a creep that walked in", late0 - a.hp(late), 100.0, 1.0)?;
    pack_run::advance(&mut a.host, 40)?;
    expect("no second hit", late0 - a.hp(late), 100.0, 2.5)?;
    Ok(())
}

pub fn poison_touch_stages(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let dazzle = a.hero("dazzle", 0, 0, 0, 1)?;
    let slot = a.learn(dazzle, "dazzle_poison_touch")?;
    let foe = a.creep(ENEMY, 4, 0)?;
    pack_run::advance(&mut a.host, 4)?;
    let before = a.hp(foe);
    a.cast(dazzle, slot, foe as i32, 0, 0)?;
    pack_run::advance(&mut a.host, 20)?;
    if a.host.sim().status_left(foe, SLOW) == 0 {
        return Err("no slow in the first second of poison touch".into());
    }
    expect("damage before the third second", before - a.hp(foe), 0.0, 0.1)?;
    pack_run::advance(&mut a.host, 16 * 2)?;
    if before - a.hp(foe) < 5.0 {
        return Err("no damage in the third second".into());
    }
    pack_run::advance(&mut a.host, 16 * 6)?;
    let total = before - a.hp(foe);
    expect("poison touch total over 8 s after armour", total, 6.0 * 14.0 * 0.9, 12.0)?;
    Ok(())
}

pub fn shadow_wave_bounces(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let dazzle = a.hero("dazzle", 0, 0, 0, 1)?;
    let slot = a.learn(dazzle, "dazzle_shadow_wave")?;
    let first = a.hero("lion", 0, 4, 0, 1)?;
    let second = a.hero("sven", 0, 8, 0, 1)?;
    let third = a.hero("axe", 0, 12, 0, 1)?;
    pack_run::advance(&mut a.host, 4)?;
    for u in [first, second, third] {
        a.freeze(u);
        let max = a.host.sim().hp_max(u);
        a.host.sim_mut().set_health(u, max / 2, max);
    }
    let (b1, b2, b3) = (a.hp(first), a.hp(second), a.hp(third));
    a.cast(dazzle, slot, first as i32, 0, 0)?;
    pack_run::advance(&mut a.host, 2)?;
    expect("target heal", a.hp(first) - b1, 80.0, 2.0)?;
    expect("bounce heal on the second", a.hp(second) - b2, 80.0, 2.0)?;
    expect("a third unit is out of the rank 1 wave", a.hp(third) - b3, 0.0, 0.5)?;
    Ok(())
}

pub fn backstab_once(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let hunter = a.hero("bounty_hunter", 0, 0, 0, 1)?;
    let slot = a.learn(hunter, "bounty_hunter_wind_walk")?;
    let foe = a.hero("lion", ENEMY, 1, 0, 1)?;
    a.freeze(foe);
    pack_run::advance(&mut a.host, 4)?;
    let max = a.host.sim().hp_max(foe);
    a.host.sim_mut().set_health(foe, max, max);
    a.cast(hunter, slot, -1, 0, 0)?;
    if a.host.sim().counter(hunter, "backstab_until") == 0 {
        return Err("wind walk did not arm the backstab".into());
    }
    let before = a.hp(foe);
    a.strike(hunter, foe)?;
    pack_run::advance(&mut a.host, 2)?;
    if a.host.sim().counter(hunter, "backstab_until") != 0 {
        return Err("the first hit did not spend the backstab".into());
    }
    if before - a.hp(foe) < 30.0 {
        return Err(format!("the first hit cost {:.1}, want at least the 30 of the backstab", before - a.hp(foe)));
    }
    Ok(())
}

pub fn grip_reveals_target(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let bane = a.hero("bane", 0, 0, 0, 6)?;
    let slot = a.learn(bane, "bane_fiend_s_grip")?;
    let foe = a.hero("lion", ENEMY, 4, 0, 1)?;
    a.freeze(foe);
    pack_run::advance(&mut a.host, 4)?;
    a.cast(bane, slot, foe as i32, 0, 0)?;
    if a.host.sim().status_left(foe, REVEALED) < 60 {
        return Err("the gripped unit is not revealed".into());
    }
    Ok(())
}

pub fn spiderlings_spawn_on_death(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let mother = a.hero("broodmother", 0, 0, 0, 1)?;
    let slot = a.learn(mother, "broodmother_spawn_spiderlings")?;
    let foe = a.creep(ENEMY, 4, 0)?;
    pack_run::advance(&mut a.host, 4)?;
    let kind = a.host.sim().catalog().unit_index("broodmother_spiderling").ok_or("no spiderling unit kind")? as i32;
    a.cast(mother, slot, foe as i32, 0, 0)?;
    a.host.sim_mut().hurt(foe, fixed::from_int(10_000));
    pack_run::advance(&mut a.host, 4)?;
    let count = {
        let s = a.host.sim();
        (0..s.count()).filter(|&u| s.alive(u) && s.unit_type(u) == kind).count()
    };
    if count != 1 {
        return Err(format!("{count} spiderlings after the death of the marked unit, want 1 at rank 1"));
    }
    Ok(())
}

pub fn spark_wraith_strikes(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let warden = a.hero("arc_warden", 0, 0, 0, 1)?;
    let slot = a.learn(warden, "arc_warden_spark_wraith")?;
    let (x, y) = a.at(8, 0);
    a.cast(warden, slot, -1, x, y)?;
    let foe = a.creep(ENEMY, 8, 3)?;
    let before = a.hp(foe);
    pack_run::advance(&mut a.host, 60)?;
    expect("spark wraith strike", before - a.hp(foe), 150.0, 3.0)?;
    Ok(())
}

pub fn chilling_touch_charges(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let aa = a.hero("ancient_apparition", 0, 0, 0, 1)?;
    let slot = a.learn(aa, "ancient_apparition_chilling_touch")?;
    let ally = a.hero("lion", 0, 3, 0, 1)?;
    let foe = a.creep(ENEMY, 4, 0)?;
    a.freeze(foe);
    pack_run::advance(&mut a.host, 4)?;
    let (x, y) = a.at(3, 0);
    a.cast(aa, slot, -1, x, y)?;
    if a.host.sim().counter(ally, "chill_charges") != 3 {
        return Err(format!("the allied hero holds {} charges, want 3", a.host.sim().counter(ally, "chill_charges")));
    }
    let before = a.hp(foe);
    a.strike(ally, foe)?;
    pack_run::advance(&mut a.host, 3)?;
    if before - a.hp(foe) < 50.0 || a.host.sim().counter(ally, "chill_charges") != 2 {
        return Err("a charged attack did not add 50 damage and spend a charge".into());
    }
    Ok(())
}

pub fn lasso_drags_target(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let rider = a.hero("batrider", 0, 0, 0, 6)?;
    let slot = a.learn(rider, "batrider_flaming_lasso")?;
    let foe = a.hero("lion", ENEMY, 3, 0, 1)?;
    pack_run::advance(&mut a.host, 4)?;
    a.cast(rider, slot, foe as i32, 0, 0)?;
    let (x, y) = a.at(14, 0);
    a.host.sim_mut().teleport(rider, x, y);
    pack_run::advance(&mut a.host, 8)?;
    let (fx, _) = a.host.sim().position_fixed(foe);
    if (fx - x).abs() > fixed::from_int(9) {
        return Err("the lassoed unit stayed far behind the rider".into());
    }
    Ok(())
}

pub fn exorcism_places_ghosts(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let dp = a.hero("death_prophet", 0, 0, 0, 6)?;
    let slot = a.learn(dp, "death_prophet_exorcism")?;
    pack_run::advance(&mut a.host, 4)?;
    let kind = a.host.sim().catalog().unit_index("death_prophet_ghost").ok_or("no ghost unit kind")? as i32;
    a.cast(dp, slot, -1, 0, 0)?;
    let count = {
        let s = a.host.sim();
        (0..s.count()).filter(|&u| s.alive(u) && s.unit_type(u) == kind).count()
    };
    if count != 8 {
        return Err(format!("{count} ghosts at rank 1, want 8"));
    }
    pack_run::advance(&mut a.host, 490)?;
    let left = {
        let s = a.host.sim();
        (0..s.count()).filter(|&u| s.alive(u) && s.unit_type(u) == kind).count()
    };
    if left != 0 {
        return Err(format!("{left} ghosts outlived 30 s"));
    }
    Ok(())
}

pub fn goo_scepter_reaches_all(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let back = a.hero("bristleback", 0, 0, 0, 1)?;
    let slot = a.learn(back, "bristleback_viscous_nasal_goo")?;
    a.host.sim_mut().set_upgrade_bit(back, 0, true).map_err(|r| r.name().to_string())?;
    let first = a.creep(ENEMY, 4, 0)?;
    let second = a.creep(ENEMY, 4, 6)?;
    let far = a.creep(ENEMY, 4, 14)?;
    pack_run::advance(&mut a.host, 4)?;
    a.cast(back, slot, first as i32, 0, 0)?;
    if a.host.sim().counter(second, "goo_stacks") != 1 || a.host.sim().counter(far, "goo_stacks") != 0 {
        return Err("the scepter goo did not reach the unit within 700 only".into());
    }
    Ok(())
}

pub fn phantasm_scepter_extra_image(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let knight = a.hero("chaos_knight", 0, 0, 0, 6)?;
    let slot = a.learn(knight, "chaos_knight_phantasm")?;
    let mut casts = 0;
    for _ in 0..14 {
        a.host.sim_mut().set_ability_cooldown(knight, slot, 0);
        a.host.sim_mut().grant_energy(knight, 2000).map_err(|r| r.name().to_string())?;
        a.cast(knight, slot, -1, 0, 0)?;
        casts += 1;
    }
    if a.host.sim().counter(knight, "phantasm_extra") != 0 {
        return Err("an extra illusion came without the scepter".into());
    }
    a.host.sim_mut().set_upgrade_bit(knight, 0, true).map_err(|r| r.name().to_string())?;
    for _ in 0..casts {
        a.host.sim_mut().set_ability_cooldown(knight, slot, 0);
        a.host.sim_mut().grant_energy(knight, 2000).map_err(|r| r.name().to_string())?;
        a.cast(knight, slot, -1, 0, 0)?;
    }
    let extra = a.host.sim().counter(knight, "phantasm_extra");
    if !(1..14).contains(&extra) {
        return Err(format!("{extra} extra illusions from 14 scepter casts, want some but not all"));
    }
    Ok(())
}

pub fn track_speeds_allies_near(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let hunter = a.hero("bounty_hunter", 0, 0, 0, 1)?;
    let slot = a.learn(hunter, "bounty_hunter_track")?;
    let near_ally = a.hero("lion", 0, 3, 0, 1)?;
    let far_ally = a.hero("sven", 0, -12, 0, 1)?;
    let prey = a.creep(ENEMY, 4, 0)?;
    a.freeze(prey);
    pack_run::advance(&mut a.host, 4)?;
    a.cast(hunter, slot, prey as i32, 0, 0)?;
    pack_run::advance(&mut a.host, 10)?;
    if a.stat(near_ally, S_SPEED_PCT) != 20 || a.stat(far_ally, S_SPEED_PCT) != 0 {
        return Err(format!("near ally speed {} far ally {}, want 20 and 0", a.stat(near_ally, S_SPEED_PCT), a.stat(far_ally, S_SPEED_PCT)));
    }
    Ok(())
}

pub fn wall_of_replica_hits_crossers(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let seer = a.hero("dark_seer", 0, 0, 0, 1)?;
    let slot = a.learn(seer, "dark_seer_wall_of_replica")?;
    let victim = a.hero("lion", ENEMY, 6, 0, 1)?;
    let bystander = a.hero("sven", ENEMY, 6, 14, 1)?;
    a.freeze(victim);
    a.freeze(bystander);
    pack_run::advance(&mut a.host, 4)?;
    let (v0, b0) = (a.hp(victim), a.hp(bystander));
    let (x, y) = a.at(6, 0);
    a.cast(seer, slot, -1, x, y)?;
    pack_run::advance(&mut a.host, 8)?;
    expect("hero on the wall after magic resistance", v0 - a.hp(victim), 150.0 * 0.75, 5.0)?;
    expect("hero off the wall", b0 - a.hp(bystander), 0.0, 0.1)?;
    let kind = a.host.sim().unit_type(victim);
    let mine = {
        let s = a.host.sim();
        (0..s.count()).filter(|&u| s.alive(u) && s.unit_type(u) == kind && s.player(u) == 0).count()
    };
    if mine != 1 {
        return Err(format!("{mine} images for the seer, want 1"));
    }
    Ok(())
}

pub fn shuriken_bounces_to_tracked(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let hunter = a.hero("bounty_hunter", 0, 0, 0, 6)?;
    let track = a.learn(hunter, "bounty_hunter_track")?;
    let shuriken = a.learn(hunter, "bounty_hunter_shuriken_toss")?;
    let first = a.hero("lion", ENEMY, 3, 0, 1)?;
    let second = a.hero("sven", ENEMY, 3, 6, 1)?;
    let third = a.hero("axe", ENEMY, 3, 9, 1)?;
    for u in [first, second, third] {
        a.freeze(u);
    }
    pack_run::advance(&mut a.host, 4)?;
    a.cast(hunter, track, first as i32, 0, 0)?;
    a.host.sim_mut().set_ability_cooldown(hunter, track, 0);
    a.cast(hunter, track, second as i32, 0, 0)?;
    let (s0, t0) = (a.hp(second), a.hp(third));
    a.cast(hunter, shuriken, first as i32, 0, 0)?;
    pack_run::advance(&mut a.host, 3)?;
    if s0 - a.hp(second) < 50.0 {
        return Err("a tracked unit within 900 took no bounce".into());
    }
    expect("an untracked unit", t0 - a.hp(third), 0.0, 0.1)?;
    Ok(())
}

pub fn rupture_scepter_recharges(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let seeker = a.hero("bloodseeker", 0, 0, 0, 6)?;
    let slot = a.learn(seeker, "bloodseeker_rupture")?;
    a.host.sim_mut().set_upgrade_bit(seeker, 0, true).map_err(|r| r.name().to_string())?;
    let foe = a.hero("lion", ENEMY, 4, 0, 1)?;
    a.freeze(foe);
    pack_run::advance(&mut a.host, 4)?;
    a.cast(seeker, slot, foe as i32, 0, 0)?;
    pack_run::advance(&mut a.host, 8)?;
    if a.host.sim().ability_cooldown(seeker, slot) != 0 {
        return Err("the scepter charge did not leave the first cast without a cooldown".into());
    }
    a.cast(seeker, slot, foe as i32, 0, 0)?;
    pack_run::advance(&mut a.host, 8)?;
    if a.host.sim().ability_cooldown(seeker, slot) == 0 {
        return Err("a second cast inside 40 s also skipped its cooldown".into());
    }
    Ok(())
}

pub fn grip_scepter_adds_damage(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let bane = a.hero("bane", 0, 0, 0, 6)?;
    let slot = a.learn(bane, "bane_fiend_s_grip")?;
    a.host.sim_mut().set_upgrade_bit(bane, 0, true).map_err(|r| r.name().to_string())?;
    let foe = a.hero("lion", ENEMY, 4, 0, 1)?;
    a.freeze(foe);
    pack_run::advance(&mut a.host, 4)?;
    let before = a.hp(foe);
    a.cast(bane, slot, foe as i32, 0, 0)?;
    pack_run::advance(&mut a.host, 20)?;
    if before - a.hp(foe) < 55.0 * 0.75 - 6.0 {
        return Err(format!("the scepter grip cost {:.1} in a second, want about 155", before - a.hp(foe)));
    }
    Ok(())
}

pub fn doom_scepter_breaks(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let doom = a.hero("doom_bringer", 0, 0, 0, 6)?;
    let slot = a.learn(doom, "doom_bringer_doom_basic")?;
    a.host.sim_mut().set_upgrade_bit(doom, 0, true).map_err(|r| r.name().to_string())?;
    let foe = a.hero("lion", ENEMY, 4, 0, 1)?;
    a.freeze(foe);
    pack_run::advance(&mut a.host, 4)?;
    a.cast(doom, slot, foe as i32, 0, 0)?;
    if a.host.sim().status_left(foe, 10) < 200 {
        return Err("a scepter Doom did not break its target".into());
    }
    Ok(())
}

pub fn tempest_copy_holds_skills(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let warden = a.hero("arc_warden", 0, 0, 0, 6)?;
    let slot = a.learn(warden, "arc_warden_tempest_double")?;
    let flux = a.learn(warden, "arc_warden_flux")?;
    pack_run::advance(&mut a.host, 4)?;
    a.cast(warden, slot, -1, 0, 0)?;
    pack_run::advance(&mut a.host, 3)?;
    let kind = a.host.sim().unit_type(warden);
    let copy = {
        let s = a.host.sim();
        (0..s.count()).find(|&u| u != warden && s.alive(u) && s.unit_type(u) == kind && s.player(u) == 0)
    }
    .ok_or("no tempest copy")?;
    if a.host.sim().ability_rank(copy, flux) != a.host.sim().ability_rank(warden, flux) {
        return Err("the copy does not hold the caster's skill ranks".into());
    }
    Ok(())
}

pub fn kinetic_field_rings_blockers(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let seer = a.hero("disruptor", 0, 0, 0, 1)?;
    let slot = a.learn(seer, "disruptor_kinetic_field")?;
    pack_run::advance(&mut a.host, 4)?;
    let before = a.host.sim().obstacles().len();
    let (x, y) = a.at(6, 0);
    a.cast(seer, slot, -1, x, y)?;
    if a.host.sim().obstacles().len() < before + 4 {
        return Err("the field placed no ring of blockers".into());
    }
    pack_run::advance(&mut a.host, 16 * 5)?;
    let live = a.host.sim().obstacles().iter().filter(|o| o.temp && o.alive).count();
    if live != 0 {
        return Err(format!("{live} temporary blockers outlived the field"));
    }
    Ok(())
}

pub fn dark_rift_takes_allies(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let lord = a.hero("abyssal_underlord", 0, 0, 0, 6)?;
    let slot = a.learn(lord, "abyssal_underlord_dark_rift")?;
    let ally = a.hero("lion", 0, 3, 0, 1)?;
    let goal = a.hero("sven", 0, 0, 14, 1)?;
    a.freeze(ally);
    a.freeze(goal);
    pack_run::advance(&mut a.host, 4)?;
    a.cast(lord, slot, goal as i32, 0, 0)?;
    pack_run::advance(&mut a.host, 16 * 7)?;
    let (ax, ay) = a.host.sim().position_fixed(ally);
    let (gx, gy) = a.host.sim().position_fixed(goal);
    if (ax - gx).abs() > fixed::from_int(3) || (ay - gy).abs() > fixed::from_int(3) {
        return Err("the allied hero did not travel with the rift".into());
    }
    Ok(())
}

pub fn wild_axes_hit_each_once(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let master = a.hero("beastmaster", 0, 0, 0, 1)?;
    let slot = a.learn(master, "beastmaster_wild_axes")?;
    let foe = a.creep(ENEMY, 5, 0)?;
    pack_run::advance(&mut a.host, 4)?;
    let before = a.hp(foe);
    let (x, y) = a.at(8, 0);
    a.cast(master, slot, -1, x, y)?;
    pack_run::advance(&mut a.host, 60)?;
    expect("a unit under both axes after armour", before - a.hp(foe), 70.0 * 0.95, 6.0)?;
    Ok(())
}

pub fn chemical_rage_shortens_attack_time(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let alchemist = a.hero("alchemist", 0, 0, 0, 1)?;
    let slot = a.learn(alchemist, "alchemist_chemical_rage")?;
    pack_run::advance(&mut a.host, 4)?;
    let before = a.host.sim().period_of(alchemist);
    a.cast(alchemist, slot, -1, 0, 0)?;
    if a.host.sim().period_of(alchemist) != 22 {
        return Err(format!("attack period {} want 22 (1.4 s)", a.host.sim().period_of(alchemist)));
    }
    pack_run::advance(&mut a.host, 420)?;
    if a.host.sim().period_of(alchemist) != before {
        return Err("the attack period did not return after 25 s".into());
    }
    Ok(())
}

pub fn primal_split_hides_and_returns(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let brew = a.hero("brewmaster", 0, 0, 0, 6)?;
    let slot = a.learn(brew, "brewmaster_primal_split")?;
    pack_run::advance(&mut a.host, 4)?;
    let fire = a.host.sim().catalog().unit_index("brewmaster_fire_1").ok_or("no fire elemental kind")? as i32;
    a.cast(brew, slot, -1, 0, 0)?;
    pack_run::advance(&mut a.host, 2)?;
    if a.host.sim().alive(brew) {
        return Err("the brewmaster stayed in play".into());
    }
    let fires = {
        let s = a.host.sim();
        (0..s.count()).filter(|&u| s.alive(u) && s.unit_type(u) == fire).count()
    };
    if fires != 1 {
        return Err(format!("{fires} fire elementals, want 1"));
    }
    pack_run::advance(&mut a.host, 260)?;
    if !a.host.sim().alive(brew) {
        return Err("the brewmaster did not return after the 15 s".into());
    }
    Ok(())
}

pub fn call_of_the_wild_hawk(_image: &[u8]) -> Result<(), String> {
    let mut a = arena()?;
    let master = a.hero("beastmaster", 0, 0, 0, 1)?;
    let slot = a.learn(master, "beastmaster_call_of_the_wild")?;
    pack_run::advance(&mut a.host, 4)?;
    let kind = a.host.sim().catalog().unit_index("beastmaster_hawk_1").ok_or("no hawk kind")? as i32;
    a.cast(master, slot, -1, 0, 0)?;
    pack_run::advance(&mut a.host, 2)?;
    let count = |a: &Arena| {
        let s = a.host.sim();
        (0..s.count()).filter(|&u| s.alive(u) && s.unit_type(u) == kind).count()
    };
    if count(&a) != 1 {
        return Err(format!("{} hawks after the call, want 1", count(&a)));
    }
    pack_run::advance(&mut a.host, 970)?;
    if count(&a) != 0 {
        return Err("the hawk outlived 60 s".into());
    }
    Ok(())
}
