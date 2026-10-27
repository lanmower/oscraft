use super::pack_run;
use crate::checks::{kind, role};
use mod_host::ModHost;
use sim::fixed;

fn quiet_spot(host: &ModHost) -> Result<(i32, i32), String> {
    let s = host.sim();
    let (w, h) = (s.grid().w as i32, s.grid().h as i32);
    let reach = 6;
    for cy in (reach..h - reach).step_by(3) {
        for cx in (reach..w - reach).step_by(3) {
            let open = (-reach..=reach).all(|dy| (-reach..=reach).all(|dx| s.grid().walkable(cx + dx, cy + dy)));
            if !open {
                continue;
            }
            let crowded = (0..s.count()).any(|u| {
                if !s.alive(u) {
                    return false;
                }
                let (x, y) = s.position_fixed(u);
                let (dx, dy) = ((x >> 16) - cx, (y >> 16) - cy);
                dx * dx + dy * dy <= (reach + 12) * (reach + 12)
            });
            if !crowded {
                return Ok((cx, cy));
            }
        }
    }
    Err("no quiet open arena on the pack map".into())
}

fn hero(host: &mut ModHost, name: &str, player: u8, at: (i32, i32), dx: i32, level: u32) -> Result<usize, String> {
    let k = kind(&host.sim(), name)?;
    let u = host
        .sim_mut()
        .spawn_attributed(player, k, fixed::from_int(at.0 + dx), fixed::from_int(at.1))
        .ok_or_else(|| format!("spawn_attributed refused {name}"))?;
    let need = level.checked_sub(2).and_then(|n| host.sim().xp_curve().get(n as usize).copied()).unwrap_or(0);
    let gap = need.saturating_sub(host.sim().xp(u));
    host.sim_mut().add_xp(u, gap);
    Ok(u)
}

fn learn(host: &mut ModHost, unit: usize, ability: &str) -> Result<usize, String> {
    let slot = host.sim().ability_slot(unit, ability).ok_or_else(|| format!("hero holds no {ability}"))?;
    host.sim_mut().rank_up(unit, slot).map_err(|r| format!("rank_up {ability}: {}", r.name()))?;
    Ok(slot)
}

fn strike(host: &mut ModHost, a: usize, b: usize) -> Result<(), String> {
    for _ in 0..200 {
        if host.sim_mut().order_attack(a, b).is_ok() {
            while host.sim().hit_in_flight(a) {
                pack_run::advance(host, 1)?;
            }
            pack_run::advance(host, 2)?;
            return Ok(());
        }
        pack_run::advance(host, 1)?;
    }
    Err(format!("unit {a} could not strike unit {b}"))
}

pub fn attack_spells(_image: &[u8]) -> Result<(), String> {
    let mut host = pack_run::start()?;
    pack_run::advance(&mut host, 2)?;
    let at = quiet_spot(&host)?;

    let spectre = hero(&mut host, role("hero.spectre")?, 0, at, 0, 1)?;
    learn(&mut host, spectre, role("hero.spectre.desolate")?)?;
    let alone = hero(&mut host, role("hero.a")?, 1, at, 2, 1)?;
    strike(&mut host, spectre, alone)?;
    if host.sim().counter(spectre, "desolate_ready") == 0 {
        return Err("Desolate did not fire on an isolated target".into());
    }
    host.sim_mut().set_counter(spectre, "desolate_ready", 0);
    let ally = hero(&mut host, role("hero.b")?, 1, at, 3, 1)?;
    strike(&mut host, spectre, alone)?;
    if host.sim().counter(spectre, "desolate_ready") != 0 {
        return Err(format!("Desolate fired with an allied unit {ally} beside the target"));
    }

    let ursa = hero(&mut host, role("hero.ursa")?, 0, at, -3, 1)?;
    learn(&mut host, ursa, role("hero.ursa.fury")?)?;
    let victim = hero(&mut host, role("hero.d")?, 1, at, -5, 1)?;
    for _ in 0..3 {
        strike(&mut host, ursa, victim)?;
    }
    let total = host.sim().counter(victim, "fury_total_0");
    if total != 45 {
        return Err(format!("Fury Swipes total {total} after three hits, want 45"));
    }
    Ok(())
}

pub fn overload_and_dispersion(_image: &[u8]) -> Result<(), String> {
    let mut host = pack_run::start()?;
    pack_run::advance(&mut host, 2)?;
    let at = quiet_spot(&host)?;
    let storm = hero(&mut host, role("hero.storm")?, 0, at, 0, 1)?;
    learn(&mut host, storm, role("hero.storm.overload")?)?;
    let vortex = learn(&mut host, storm, role("hero.storm.vortex")?)?;
    host.sim_mut().grant_energy(storm, 1000).map_err(|r| r.name().to_string())?;
    let victim = hero(&mut host, role("hero.a")?, 1, at, 4, 1)?;
    if !host.sim_mut().cast_ability(storm, vortex, victim as i32, 0, 0) {
        return Err("Electric Vortex cast refused".into());
    }
    pack_run::advance(&mut host, 4)?;
    if host.sim().counter(storm, "overload_charged") != 1 {
        return Err("a cast did not charge Overload".into());
    }
    let before = host.sim().hp(victim);
    let mut spent = false;
    for _ in 0..120 {
        pack_run::advance(&mut host, 1)?;
        if host.sim().counter(storm, "overload_charged") == 0 {
            spent = true;
            break;
        }
    }
    if !spent {
        return Err("no attack spent the Overload charge in 120 ticks".into());
    }
    if host.sim().status_left(victim, 1) == 0 {
        return Err("Overload did not slow the unit it burst on".into());
    }
    if host.sim().hp(victim) >= before {
        return Err("the Overload attack did no damage".into());
    }

    let mut host = pack_run::start()?;
    pack_run::advance(&mut host, 2)?;
    let at = quiet_spot(&host)?;
    let spectre = hero(&mut host, role("hero.spectre")?, 0, at, 0, 1)?;
    learn(&mut host, spectre, role("hero.spectre.dispersion")?)?;
    let attacker = hero(&mut host, role("hero.a")?, 1, at, 2, 1)?;
    pack_run::advance(&mut host, 4)?;
    let full = host.sim().hp(attacker);
    strike(&mut host, attacker, spectre)?;
    if host.sim().hp(attacker) >= full {
        return Err(format!("Dispersion returned no damage to the attacker: hp {} -> {}", full, host.sim().hp(attacker)));
    }
    Ok(())
}

fn live(host: &ModHost, name: &str) -> Vec<usize> {
    let s = host.sim();
    (0..s.count()).filter(|&u| s.alive(u) && s.kind_name(u) == Some(name)).collect()
}

fn plant(host: &mut ModHost, techies: usize, ability: &str, x: i32, y: i32) -> Result<(), String> {
    let slot = learn(host, techies, ability)?;
    host.sim_mut().grant_energy(techies, 1000).map_err(|r| r.name().to_string())?;
    if !host.sim_mut().cast_ability(techies, slot, -1, fixed::from_int(x), fixed::from_int(y)) {
        return Err(format!("cast {ability} refused"));
    }
    pack_run::advance(host, 30)
}

pub fn techies_mines(_image: &[u8]) -> Result<(), String> {
    let mut host = pack_run::start()?;
    pack_run::advance(&mut host, 2)?;
    let at = quiet_spot(&host)?;
    let techies = hero(&mut host, role("hero.techies")?, 0, at, 0, 1)?;
    plant(&mut host, techies, role("hero.techies.mines")?, at.0 + 4, at.1)?;
    if live(&host, role("unit.land_mine")?).len() != 1 {
        return Err("Land Mines planted no mine".into());
    }
    let victim = hero(&mut host, role("hero.a")?, 1, (at.0 + 4, at.1), 2, 1)?;
    pack_run::advance(&mut host, 2)?;
    let before = host.sim().hp(victim);
    pack_run::advance(&mut host, 40)?;
    let lost = (before - host.sim().hp(victim)) >> 16;
    if !(100..=260).contains(&lost) {
        return Err(format!("mine damage {lost} want 150 scaled by armour and the hero attack factor"));
    }
    if !live(&host, role("unit.land_mine")?).is_empty() {
        return Err("the mine stood after it exploded".into());
    }

    let mut host = pack_run::start()?;
    pack_run::advance(&mut host, 2)?;
    let at = quiet_spot(&host)?;
    let techies = hero(&mut host, role("hero.techies")?, 0, at, 0, 1)?;
    plant(&mut host, techies, role("hero.techies.trap")?, at.0 + 4, at.1)?;
    if live(&host, role("unit.stasis_trap")?).len() != 1 {
        return Err("Stasis Trap planted no trap".into());
    }
    let victim = hero(&mut host, role("hero.a")?, 1, (at.0 + 4, at.1), 3, 1)?;
    let mut stunned = false;
    for _ in 0..120 {
        pack_run::advance(&mut host, 1)?;
        if host.sim().status_left(victim, 0) > 0 {
            stunned = true;
            break;
        }
    }
    if !stunned {
        return Err("the trap never stunned the unit beside it".into());
    }
    Ok(())
}

pub fn stacks_cloak_remnant(_image: &[u8]) -> Result<(), String> {
    let mut host = pack_run::start()?;
    pack_run::advance(&mut host, 2)?;
    let at = quiet_spot(&host)?;
    let troll = hero(&mut host, role("hero.troll")?, 0, at, 0, 1)?;
    learn(&mut host, troll, role("hero.troll.fervor")?)?;
    let victim = hero(&mut host, role("hero.d")?, 1, at, -4, 1)?;
    for _ in 0..3 {
        strike(&mut host, troll, victim)?;
        pack_run::advance(&mut host, 8)?;
    }
    let stacks = host.sim().counter(troll, "fervor_stacks");
    if stacks < 2 || host.sim().counter(troll, "fervor_bonus") != 20 * stacks {
        return Err(format!("Fervor stacks {stacks} bonus {} want at least 2 and 20 each; disabled {:?}", host.sim().counter(troll, "fervor_bonus"), host.disabled_hooks()));
    }

    let visage = hero(&mut host, role("hero.visage")?, 0, at, 8, 1)?;
    learn(&mut host, visage, role("hero.visage.cloak")?)?;
    let attacker = hero(&mut host, role("hero.a")?, 1, at, 10, 1)?;
    pack_run::advance(&mut host, 4)?;
    let armor_before = host.sim().stat(visage, 1);
    strike(&mut host, attacker, visage)?;
    let lost = host.sim().counter(visage, "cloak_lost");
    if lost < 1 || host.sim().stat(visage, 1) != armor_before - lost as i32 {
        return Err(format!("a hit left cloak_lost {} armour {} -> {}", host.sim().counter(visage, "cloak_lost"), armor_before, host.sim().stat(visage, 1)));
    }
    host.sim_mut().remove_unit(attacker);
    pack_run::advance(&mut host, 4 * 70)?;
    if host.sim().counter(visage, "cloak_lost") != 0 || host.sim().stat(visage, 1) != armor_before {
        return Err("the cloak layer did not recover after 4 s".into());
    }

    let mut host = pack_run::start()?;
    pack_run::advance(&mut host, 2)?;
    let at = quiet_spot(&host)?;
    let storm = hero(&mut host, role("hero.storm")?, 0, at, 0, 1)?;
    let slot = learn(&mut host, storm, role("hero.storm.remnant")?)?;
    host.sim_mut().grant_energy(storm, 1000).map_err(|r| r.name().to_string())?;
    let walker = hero(&mut host, role("hero.b")?, 1, at, 8, 1)?;
    pack_run::advance(&mut host, 4)?;
    if !host.sim_mut().cast_ability(storm, slot, -1, 0, 0) {
        return Err("Static Remnant cast refused".into());
    }
    pack_run::advance(&mut host, 3)?;
    if live(&host, role("unit.remnant")?).len() != 1 {
        return Err("Static Remnant left no remnant".into());
    }
    let before = host.sim().hp(walker);
    host.sim_mut().order(walker, fixed::from_int(at.0), fixed::from_int(at.1));
    pack_run::advance(&mut host, 120)?;
    if host.sim().hp(walker) >= before || !live(&host, role("unit.remnant")?).is_empty() {
        return Err(format!("the remnant did not explode on the unit that walked up: hp {} -> {}", before, host.sim().hp(walker)));
    }
    Ok(())
}

pub fn death_ward_by_rank(_image: &[u8]) -> Result<(), String> {
    let mut host = pack_run::start()?;
    pack_run::advance(&mut host, 2)?;
    let at = quiet_spot(&host)?;
    let witch = hero(&mut host, role("hero.witch")?, 0, at, 0, 11)?;
    let slot = host.sim().ability_slot(witch, role("hero.witch.ward")?).ok_or("no death ward slot")?;
    for _ in 0..3 {
        host.sim_mut().rank_up(witch, slot).map_err(|r| r.name().to_string())?;
    }
    host.sim_mut().grant_energy(witch, 1000).map_err(|r| r.name().to_string())?;
    if !host.sim_mut().cast_ability(witch, slot, -1, fixed::from_int(at.0 + 4), fixed::from_int(at.1)) {
        return Err("Death Ward cast refused".into());
    }
    pack_run::advance(&mut host, 3)?;
    if live(&host, role("unit.death_ward_3")?).len() != 1 {
        return Err("Death Ward rank 3 placed no rank 3 ward".into());
    }
    pack_run::advance(&mut host, 140)?;
    if !live(&host, role("unit.death_ward_3")?).is_empty() {
        return Err("the ward outlived its 8 s".into());
    }
    Ok(())
}

pub fn morph_keeps_fraction(_image: &[u8]) -> Result<(), String> {
    let mut host = pack_run::start()?;
    pack_run::advance(&mut host, 2)?;
    let at = quiet_spot(&host)?;
    let from = kind(&host.sim(), role("unit.creep")?)?;
    let to = kind(&host.sim(), role("unit.creep_c")?)?;
    let u = host
        .sim_mut()
        .spawn_for(1, from, fixed::from_int(at.0), fixed::from_int(at.1))
        .ok_or("spawn_for refused a creep")?;
    let max = host.sim().hp_max(u);
    host.sim_mut().set_hp_within(u, max / 2);
    if !host.sim_mut().morph(u, to) {
        return Err("morph refused".into());
    }
    if host.sim().kind_name(u) != Some(role("unit.creep_c")?) {
        return Err(format!("unit is {:?} after the morph", host.sim().kind_name(u)));
    }
    let (hp, max_after) = (host.sim().hp(u), host.sim().hp_max(u));
    if max_after == max || (i64::from(hp) * 100 / i64::from(max_after.max(1)) - 50).abs() > 2 {
        return Err(format!("hit points {} of {max} became {hp} of {max_after}, want half of the new maximum", max / 2));
    }
    Ok(())
}

fn arena(level_hero: &str, level: u32) -> Result<(ModHost, (i32, i32), usize), String> {
    let mut host = pack_run::start()?;
    pack_run::advance(&mut host, 2)?;
    let at = quiet_spot(&host)?;
    let h = hero(&mut host, level_hero, 0, at, 0, level)?;
    host.sim_mut().grant_energy(h, 5000).map_err(|r| r.name().to_string())?;
    Ok((host, at, h))
}

fn cast_at(host: &mut ModHost, h: usize, ability: &str, x: i32, y: i32) -> Result<(), String> {
    let slot = learn(host, h, ability)?;
    if !host.sim_mut().cast_ability(h, slot, -1, fixed::from_int(x), fixed::from_int(y)) {
        return Err(format!("cast {ability} refused: {}", host.sim().stats.last_refusal.name()));
    }
    Ok(())
}

pub fn summon_behaviours(_image: &[u8]) -> Result<(), String> {
    let (mut host, at, tinker) = arena(role("hero.tinker")?, 1)?;
    let victim = hero(&mut host, role("hero.a")?, 1, at, 4, 1)?;
    pack_run::advance(&mut host, 4)?;
    let before = host.sim().hp(victim);
    cast_at(&mut host, tinker, role("hero.tinker.march")?, at.0 + 4, at.1)?;
    pack_run::advance(&mut host, 40)?;
    if host.sim().hp(victim) >= before {
        return Err("March of the Machines hurt nothing".into());
    }

    let (mut host, at, tusk) = arena(role("hero.tusk")?, 1)?;
    let enemy = hero(&mut host, role("hero.a")?, 1, at, 6, 1)?;
    cast_at(&mut host, tusk, role("hero.tusk.sigil")?, at.0, at.1)?;
    pack_run::advance(&mut host, 12)?;
    if live(&host, role("unit.sigil")?).len() != 1 || host.sim().status_left(enemy, 1) == 0 {
        return Err("Frozen Sigil stood no sigil or slowed nobody".into());
    }
    pack_run::advance(&mut host, 520)?;
    if !live(&host, role("unit.sigil")?).is_empty() {
        return Err("the sigil outlived its 30 s".into());
    }

    let (mut host, at, undying) = arena(role("hero.undying")?, 1)?;
    let _enemy = hero(&mut host, role("hero.a")?, 1, at, 6, 1)?;
    cast_at(&mut host, undying, role("hero.undying.tomb")?, at.0 + 2, at.1)?;
    pack_run::advance(&mut host, 70)?;
    if live(&host, role("unit.tomb")?).len() != 1 || live(&host, role("unit.zombie")?).is_empty() {
        return Err("the tombstone stood no zombie within 4 s".into());
    }

    let (mut host, at, weaver) = arena(role("hero.weaver")?, 3)?;
    let slot = host.sim().ability_slot(weaver, role("hero.weaver.swarm")?).ok_or("no swarm slot")?;
    for _ in 0..2 {
        host.sim_mut().rank_up(weaver, slot).map_err(|r| r.name().to_string())?;
    }
    if !host.sim_mut().cast_ability(weaver, slot, -1, fixed::from_int(at.0 + 3), fixed::from_int(at.1)) {
        return Err("Swarm cast refused".into());
    }
    pack_run::advance(&mut host, 4)?;
    if live(&host, role("unit.beetle_2")?).len() != 1 {
        return Err("Swarm rank 2 placed no rank 2 beetle".into());
    }

    let (mut host, _at, visage) = arena(role("hero.visage")?, 6)?;
    let slot = learn(&mut host, visage, role("hero.visage.familiars")?)?;
    if !host.sim_mut().cast_ability(visage, slot, -1, 0, 0) {
        return Err("Summon Familiars cast refused".into());
    }
    pack_run::advance(&mut host, 4)?;
    let familiars = live(&host, role("unit.familiar_2")?).len() + live(&host, "visage_familiar_1").len();
    if familiars != 2 {
        return Err(format!("{familiars} familiars, want 2"));
    }

    let (mut host, at, techies) = arena(role("hero.techies")?, 6)?;
    let enemy = hero(&mut host, role("hero.a")?, 1, at, 6, 1)?;
    pack_run::advance(&mut host, 4)?;
    cast_at(&mut host, techies, role("hero.techies.remote")?, at.0 + 6, at.1)?;
    pack_run::advance(&mut host, 40)?;
    if live(&host, role("unit.remote_mine")?).len() != 1 {
        return Err("Remote Mines planted no mine".into());
    }
    pack_run::advance(&mut host, 20)?;
    if live(&host, role("unit.remote_mine")?).len() != 1 {
        return Err("a remote mine went off without Detonate".into());
    }
    let before = host.sim().hp(enemy);
    let det = host.sim().ability_slot(techies, role("hero.techies.detonate")?).ok_or("no detonate slot")?;
    if !host.sim_mut().cast_ability(techies, det, -1, 0, 0) {
        return Err(format!("Detonate refused: {}", host.sim().stats.last_refusal.name()));
    }
    pack_run::advance(&mut host, 6)?;
    if host.sim().hp(enemy) >= before || !live(&host, role("unit.remote_mine")?).is_empty() {
        return Err(format!("Detonate did not blow the remote mine: hp {} -> {}", before, host.sim().hp(enemy)));
    }

    let (mut host, at, ta) = arena(role("hero.ta")?, 6)?;
    let enemy = hero(&mut host, role("hero.a")?, 1, at, 6, 1)?;
    pack_run::advance(&mut host, 4)?;
    cast_at(&mut host, ta, role("hero.ta.trap")?, at.0 + 6, at.1)?;
    pack_run::advance(&mut host, 30)?;
    if host.sim().status_left(enemy, 1) == 0 {
        return Err("the psionic trap did not slow the unit beside it".into());
    }

    let (mut host, at, ta) = arena(role("hero.ta")?, 1)?;
    let slot = learn(&mut host, ta, role("hero.ta.refraction")?)?;
    if !host.sim_mut().cast_ability(ta, slot, -1, 0, 0) {
        return Err("Refraction cast refused".into());
    }
    pack_run::advance(&mut host, 4)?;
    if host.sim().counter(ta, "refr_hits") != 3 {
        return Err("Refraction armed no absorbs".into());
    }
    let attacker = hero(&mut host, role("hero.a")?, 1, at, 2, 1)?;
    pack_run::advance(&mut host, 4)?;
    strike(&mut host, attacker, ta)?;
    if host.sim().counter(ta, "refr_hits") >= 3 {
        return Err("Refraction absorbed no hit".into());
    }
    Ok(())
}

pub fn blocks_bonds_flare(_image: &[u8]) -> Result<(), String> {
    let (mut host, at, tide) = arena(role("hero.tide")?, 1)?;
    learn(&mut host, tide, role("hero.tide.kraken")?)?;
    let attacker = hero(&mut host, role("hero.a")?, 1, at, 2, 1)?;
    pack_run::advance(&mut host, 4)?;
    strike(&mut host, attacker, tide)?;
    if host.sim().counter(tide, "kraken_blocked") != 12 * 65536 {
        return Err(format!("Kraken Shell blocked {} want 12 whole", host.sim().counter(tide, "kraken_blocked")));
    }

    let (mut host, at, treant) = arena(role("hero.treant")?, 1)?;
    let slot = learn(&mut host, treant, role("hero.treant.armor")?)?;
    if !host.sim_mut().cast_ability(treant, slot, treant as i32, 0, 0) {
        return Err("Living Armor cast refused".into());
    }
    pack_run::advance(&mut host, 6)?;
    if host.sim().counter(treant, "la_hits") != 4 {
        return Err("Living Armor armed no hits".into());
    }
    let _ = at;

    let (mut host, at, wyvern) = arena(role("hero.wyvern")?, 1)?;
    let slot = learn(&mut host, wyvern, role("hero.wyvern.siege")?)?;
    let enemy = hero(&mut host, role("hero.a")?, 1, at, 2, 1)?;
    pack_run::advance(&mut host, 4)?;
    if !host.sim_mut().cast_ability(wyvern, slot, -1, 0, 0) {
        return Err("Arctic Burn cast refused".into());
    }
    pack_run::advance(&mut host, 4)?;
    strike(&mut host, wyvern, enemy)?;
    if host.sim().status_left(enemy, 1) == 0 || host.sim().counter(enemy, "burn_left") == 0 {
        return Err("an Arctic Burn attack neither slowed nor burned".into());
    }

    let (mut host, at, warlock) = arena(role("hero.warlock")?, 1)?;
    let first = hero(&mut host, role("hero.a")?, 1, at, 3, 1)?;
    let second = hero(&mut host, role("hero.b")?, 1, at, 5, 1)?;
    pack_run::advance(&mut host, 4)?;
    let slot = learn(&mut host, warlock, role("hero.warlock.bonds")?)?;
    if !host.sim_mut().cast_ability(warlock, slot, first as i32, 0, 0) {
        return Err("Fatal Bonds cast refused".into());
    }
    pack_run::advance(&mut host, 6)?;
    if host.sim().counter(second, "bond_id") == 0 {
        return Err("Fatal Bonds linked no second unit".into());
    }
    let before = host.sim().hp(second);
    host.sim_mut().hit_by(first, fixed::from_int(400), warlock as i32);
    pack_run::advance(&mut host, 3)?;
    if host.sim().hp(second) >= before {
        return Err("a hit on one bonded unit did not reach the other".into());
    }

    let (mut host, at, sky) = arena(role("hero.sky")?, 6)?;
    let victim = hero(&mut host, role("hero.a")?, 1, at, 8, 1)?;
    pack_run::advance(&mut host, 4)?;
    host.sim_mut().apply_status(victim, 0, 200, fixed::ONE);
    let before = host.sim().hp(victim);
    cast_at(&mut host, sky, role("hero.sky.flare")?, at.0 + 8, at.1)?;
    pack_run::advance(&mut host, 60)?;
    let lost = (before - host.sim().hp(victim)) >> 16;
    if lost < 350 {
        return Err(format!("Mystic Flare dealt {lost}, want about 600 less magic resistance"));
    }
    Ok(())
}
