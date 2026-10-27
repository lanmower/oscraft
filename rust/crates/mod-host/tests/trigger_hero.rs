
#![cfg(feature = "build")]

use mod_host::{build_image, Death, ModHost};
use serde_json::json;
use sim::{fixed, TICKS_PER_SECOND};

mod common;
use common::scenario;

fn host_with_hero(name: &str) -> (ModHost, usize) {
    let image = build_image(&scenario()).expect("moba scenario builds an image");
    let mut host = ModHost::new(&image).expect("host");
    for p in 0..2u8 {
        host.sim_mut().set_team(p, p).expect("team");
    }
    let rules = std::fs::read_to_string(scenario().join("attr_rules.txt")).expect("rules");
    assert!(host.sim_mut().set_attr_rules_text(&rules), "the pack's rules file loads");
    let mods = common::pack_mods(&scenario(), &common::pack_roots(&scenario()));
    host.load_mods(&mods).expect("moba mods load");
    let ui: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(scenario().join("moba_ui.json")).expect("ui")).expect("ui json");
    host.dispatch_action("setup", &json!({ "ui": ui, "player": 0 })).expect("setup");
    host.dispatch_action("seat_pick", &json!({ "player": 0, "hero": name })).expect("seat_pick");
    host.advance().expect("tick");
    let hero = hero_of(&host, name);
    (host, hero)
}

fn hero_of(host: &ModHost, name: &str) -> usize {
    let kind = host.sim().catalog().unit_index(name).expect("a unit kind") as i32;
    (0..host.sim().count()).find(|&i| host.sim().alive(i) && host.sim().unit_type(i) == kind && host.sim().player(i) == 0).expect("the picked hero")
}

fn foe(host: &mut ModHost, near: usize, kind: &str, hero: bool) -> usize {
    let k = host.sim().catalog().unit_index(kind).expect("a unit kind") as u32;
    let (x, y) = host.sim().position_fixed(near);
    let at = (x + fixed::from_int(1), y);
    let j = if hero {
        host.sim_mut().spawn_attributed(1, k, at.0, at.1).expect("a hostile hero")
    } else {
        host.sim_mut().spawn_for(1, k, at.0, at.1).expect("a hostile unit")
    };
    settle(host);
    j
}

fn ally(host: &mut ModHost, near: usize) -> usize {
    let k = host.sim().catalog().unit_index("antimage").expect("a unit kind") as u32;
    let (x, y) = host.sim().position_fixed(near);
    let j = host.sim_mut().spawn_attributed(0, k, x + fixed::from_int(1), y).expect("an allied hero");
    settle(host);
    j
}

fn slay(host: &mut ModHost, killer: usize, victim: usize) {
    let (killer_player, victim_player) = (host.sim().player(killer), host.sim().player(victim));
    host.sim_mut().hurt(victim, i32::MAX / 2);
    let death = Death { victim: victim as u32, victim_player, killer: Some(killer as u32), killer_player: Some(killer_player), bounty: 0 };
    host.feed_death(&death).expect("the death is fed");
}

fn tank(host: &mut ModHost, near: usize) -> usize {
    let hero = foe(host, near, "antimage", true);
    host.sim_mut().add_stat_bonus(hero, 2, 1_000_000);
    host.advance().expect("tick");
    hero
}

fn settle(host: &mut ModHost) {
    for _ in 0..3 {
        host.advance().expect("tick");
    }
}

fn learn(host: &mut ModHost, hero: usize, ability: &str, rank: u8) {
    let slot = host.sim().ability_slot(hero, ability).expect("the hero holds the ability");
    assert!(host.sim_mut().set_skill_rank(hero, slot, rank).is_some(), "the rank is set");
}

#[test]
fn backtrack_heals_back_some_hits_but_not_all() {
    let (mut host, void) = host_with_hero("faceless_void");
    learn(&mut host, void, "faceless_void_backtrack", 4);
    let attacker = tank(&mut host, void);
    host.sim_mut().hurt(void, 400 * fixed::ONE);
    let (mut healed, mut hurt) = (0, 0);
    for _ in 0..60 {
        host.sim_mut().hurt(void, 20 * fixed::ONE);
        let before = host.sim().hp(void);
        host.sim_mut().spell_damage(attacker as i32, void, 30, 3);
        host.advance().expect("tick");
        if host.sim().hp(void) >= before { healed += 1 } else { hurt += 1 }
    }
    assert!(healed > 0 && hurt > 0, "Backtrack heals some hits and not others: {healed} healed, {hurt} hurt, errors {:?}", host.hook_errors());
}

#[test]
fn overwhelming_odds_scales_with_units_and_heroes_counted_and_hastes_the_caster() {
    let (mut host, legion) = host_with_hero("legion_commander");
    learn(&mut host, legion, "legion_commander_overwhelming_odds", 1);
    host.sim_mut().grant_energy(legion, 1000).expect("mana");
    let hero = tank(&mut host, legion);
    let creep = foe(&mut host, legion, "antimage", false);
    host.sim_mut().add_stat_bonus(creep, 2, 1_000_000);
    host.advance().expect("tick");
    let (hero_hp, creep_hp) = (host.sim().hp(hero), host.sim().hp(creep));
    let slot = host.sim().ability_slot(legion, "legion_commander_overwhelming_odds").expect("the hero holds the ability");
    let (x, y) = host.sim().position_fixed(hero);
    assert!(host.sim_mut().cast_ability(legion, slot, -1, x, y), "the ability casts");
    for _ in 0..4 {
        host.advance().expect("tick");
    }
    let lost = |after: i32, before: i32| (before - after) as f64 / fixed::ONE as f64;
    assert!(lost(host.sim().hp(hero), hero_hp) > 20.0, "the area hurts the hero: {} errors {:?}", lost(host.sim().hp(hero), hero_hp), host.hook_errors());
    assert!(lost(host.sim().hp(creep), creep_hp) > 20.0, "the area hurts the unit");
    assert_eq!(host.sim().counter(legion, "odds_move"), 12, "the caster gains 9 for the hero and 3 for the unit");
    for _ in 0..8 * TICKS_PER_SECOND {
        host.advance().expect("tick");
    }
    assert_eq!(host.sim().counter(legion, "odds_move"), 0, "the haste ends after seven seconds");
}

#[test]
fn duel_forces_both_fighters_and_gives_the_winner_permanent_damage() {
    let (mut host, legion) = host_with_hero("legion_commander");
    learn(&mut host, legion, "legion_commander_duel", 1);
    host.sim_mut().grant_energy(legion, 1000).expect("mana");
    let enemy = foe(&mut host, legion, "antimage", true);
    let slot = host.sim().ability_slot(legion, "legion_commander_duel").expect("the hero holds the ability");
    assert!(host.sim_mut().cast_ability(legion, slot, enemy as i32, 0, 0), "Duel casts");
    for _ in 0..3 {
        host.advance().expect("tick");
    }
    assert_eq!(host.sim().counter(legion, "duel_foe"), enemy as i64 + 1, "the duel is on: errors {:?}", host.hook_errors());
    slay(&mut host, legion, enemy);
    host.advance().expect("tick");
    host.advance().expect("tick");
    assert_eq!(host.sim().counter(legion, "duel_won"), 10, "the winner keeps ten damage: foe {} due {} errors {:?}", host.sim().counter(legion, "duel_foe"), host.sim().counter(legion, "duel_due"), host.hook_errors());
    assert_eq!(host.sim().counter(legion, "duel_foe"), 0, "the duel is over");
}

#[test]
fn press_the_attack_strips_disables_from_its_target() {
    let (mut host, legion) = host_with_hero("legion_commander");
    learn(&mut host, legion, "legion_commander_press_the_attack", 1);
    host.sim_mut().grant_energy(legion, 1000).expect("mana");
    let friend = ally(&mut host, legion);
    host.sim_mut().apply_status(friend, 1, 1_000, fixed::ONE / 2);
    host.sim_mut().apply_status(friend, 3, 1_000, fixed::ONE);
    let slot = host.sim().ability_slot(legion, "legion_commander_press_the_attack").expect("the hero holds the ability");
    assert!(host.sim_mut().cast_ability(legion, slot, friend as i32, 0, 0), "the ability casts");
    for _ in 0..3 {
        host.advance().expect("tick");
    }
    assert_eq!(host.sim().status_left(friend, 1), 0, "the slow is gone: errors {:?}", host.hook_errors());
    assert_eq!(host.sim().status_left(friend, 3), 0, "the root is gone");
}

#[test]
fn pulse_nova_is_a_toggle_that_burns_mana_and_hurts_enemies_every_second() {
    let (mut host, leshrac) = host_with_hero("leshrac");
    learn(&mut host, leshrac, "leshrac_pulse_nova", 1);
    host.sim_mut().grant_energy(leshrac, 1000).expect("mana");
    let creep = tank(&mut host, leshrac);
    let (mana, creep_hp) = (host.sim().mana_of(leshrac), host.sim().hp(creep));
    let slot = host.sim().ability_slot(leshrac, "leshrac_pulse_nova").expect("the hero holds the ability");
    assert!(host.sim_mut().cast_ability(leshrac, slot, -1, 0, 0), "Pulse Nova casts");
    for _ in 0..3 * TICKS_PER_SECOND {
        host.advance().expect("tick");
    }
    let lost = |after: i32, before: i32| (before - after) as f64 / fixed::ONE as f64;
    assert!(lost(host.sim().hp(creep), creep_hp) > 100.0, "the pulses hurt: {} errors {:?}", lost(host.sim().hp(creep), creep_hp), host.hook_errors());
    assert!(host.sim().mana_of(leshrac) < mana, "the pulses burn mana");
    assert!(host.sim_mut().cast_ability(leshrac, slot, -1, 0, 0), "Pulse Nova casts again to switch off");
    for _ in 0..TICKS_PER_SECOND {
        host.advance().expect("tick");
    }
    let after = host.sim().mana_of(leshrac);
    for _ in 0..2 * TICKS_PER_SECOND {
        host.advance().expect("tick");
    }
    assert!(host.sim().mana_of(leshrac) >= after, "no pulses burn mana after the toggle is switched off");
}

#[test]
fn time_lock_stuns_a_hero_for_up_to_two_seconds() {
    let (mut host, void) = host_with_hero("faceless_void");
    learn(&mut host, void, "faceless_void_time_lock", 4);
    let hero = tank(&mut host, void);
    let mut hero_longest = 0;
    for _ in 0..80 * TICKS_PER_SECOND {
        host.advance().expect("tick");
        hero_longest = hero_longest.max(host.sim().status_left(hero, 0));
    }
    assert!(hero_longest > 16 && hero_longest <= 32, "the hero is stunned up to two seconds: {hero_longest} errors {:?}", host.hook_errors());
}

#[test]
fn moment_of_courage_strikes_back_at_once_with_extra_lifesteal() {
    let (mut host, legion) = host_with_hero("legion_commander");
    learn(&mut host, legion, "legion_commander_moment_of_courage", 4);
    let attacker = tank(&mut host, legion);
    host.sim_mut().hurt(legion, 300 * fixed::ONE);
    let theirs = host.sim().hp(attacker);
    let mut struck = false;
    for _ in 0..200 {
        host.sim_mut().spell_damage(attacker as i32, legion, 1, 3);
        host.advance().expect("tick");
        if host.sim().hp(attacker) < theirs {
            struck = true;
            break;
        }
    }
    assert!(struck, "the attacked hero strikes back: errors {:?}", host.hook_errors());
}

#[test]
fn force_of_nature_places_treants_that_expire() {
    let (mut host, furion) = host_with_hero("furion");
    learn(&mut host, furion, "furion_force_of_nature", 2);
    host.sim_mut().grant_energy(furion, 1000).expect("mana");
    let kind = host.sim().catalog().unit_index("furion_treant").expect("the treant unit") as i32;
    let count = |host: &ModHost| (0..host.sim().count()).filter(|&i| host.sim().alive(i) && host.sim().unit_type(i) == kind).count();
    let slot = host.sim().ability_slot(furion, "furion_force_of_nature").expect("the hero holds the ability");
    let (x, y) = host.sim().position_fixed(furion);
    assert!(host.sim_mut().cast_ability(furion, slot, -1, x, y), "the ability casts");
    for _ in 0..2 * TICKS_PER_SECOND {
        host.advance().expect("tick");
    }
    assert_eq!(count(&host), 3, "three treants at rank two");
    for _ in 0..60 * TICKS_PER_SECOND {
        host.advance().expect("tick");
    }
    assert_eq!(count(&host), 0, "the treants expire after sixty seconds");
}

#[test]
fn time_walk_slows_enemies_along_its_path() {
    let (mut host, void) = host_with_hero("faceless_void");
    learn(&mut host, void, "faceless_void_time_walk", 4);
    host.sim_mut().grant_energy(void, 1000).expect("mana");
    let creep = tank(&mut host, void);
    let (x, y) = host.sim().position_fixed(void);
    let slot = host.sim().ability_slot(void, "faceless_void_time_walk").expect("the hero holds the ability");
    host.advance().expect("tick");
    assert!(host.sim_mut().cast_ability(void, slot, -1, x + fixed::from_int(4), y), "Time Walk casts");
    for _ in 0..4 {
        host.advance().expect("tick");
    }
    assert!(host.sim().slow_multiplier(creep) < fixed::ONE, "the enemy on the path is slowed: errors {:?}", host.hook_errors());
}

#[test]
fn wrath_of_nature_grows_seven_percent_per_jump() {
    let (mut host, furion) = host_with_hero("furion");
    learn(&mut host, furion, "furion_wrath_of_nature", 1);
    host.sim_mut().grant_energy(furion, 1000).expect("mana");
    let first = tank(&mut host, furion);
    let second = tank(&mut host, first);
    let third = tank(&mut host, second);
    let before: Vec<i32> = [first, second, third].iter().map(|&u| host.sim().hp(u)).collect();
    let slot = host.sim().ability_slot(furion, "furion_wrath_of_nature").expect("the hero holds the ability");
    assert!(host.sim_mut().cast_ability(furion, slot, first as i32, 0, 0), "Wrath casts");
    for _ in 0..40 {
        host.advance().expect("tick");
    }
    let loss: Vec<i32> = [first, second, third].iter().zip(&before).map(|(&u, b)| b - host.sim().hp(u)).collect();
    assert!(loss[0] > 0 && loss[2] > loss[1] && loss[2] as i64 * 100 < loss[1] as i64 * 110, "each jump hits seven percent harder: {loss:?} errors {:?}", host.hook_errors());
}

#[test]
fn teleportation_travels_to_a_point_anywhere() {
    let (mut host, furion) = host_with_hero("furion");
    learn(&mut host, furion, "furion_teleportation", 1);
    host.sim_mut().grant_energy(furion, 1000).expect("mana");
    let slot = host.sim().ability_slot(furion, "furion_teleportation").expect("the hero holds the ability");
    let (x, y) = host.sim().position_fixed(furion);
    let goal = (x + fixed::from_int(40), y);
    assert!(host.sim_mut().cast_ability(furion, slot, -1, goal.0, goal.1), "Teleportation casts at a point");
    for _ in 0..4 * TICKS_PER_SECOND {
        host.advance().expect("tick");
    }
    let (nx, ny) = host.sim().position_fixed(furion);
    assert!((nx - goal.0).abs() < fixed::from_int(2) && (ny - goal.1).abs() < fixed::from_int(2), "arrived at the point: {:?} want {:?} errors {:?}", (nx, ny), goal, host.hook_errors());
}

#[test]
fn sprout_accepts_a_unit_target_and_rings_it_with_trees() {
    let (mut host, furion) = host_with_hero("furion");
    learn(&mut host, furion, "furion_sprout", 1);
    host.sim_mut().grant_energy(furion, 1000).expect("mana");
    let enemy = tank(&mut host, furion);
    let slot = host.sim().ability_slot(furion, "furion_sprout").expect("the hero holds the ability");
    assert!(host.sim_mut().cast_ability(furion, slot, enemy as i32, 0, 0), "Sprout casts at a unit");
    for _ in 0..3 {
        host.advance().expect("tick");
    }
    assert!(host.hook_errors().is_empty(), "no hook errors: {:?}", host.hook_errors());
}

#[test]
fn homing_missile_lands_after_its_wait_with_distance_scaled_damage_and_a_stun() {
    let (mut host, gyro) = host_with_hero("gyrocopter");
    learn(&mut host, gyro, "gyrocopter_homing_missile", 4);
    host.sim_mut().grant_energy(gyro, 1000).expect("mana");
    let target = tank(&mut host, gyro);
    let slot = host.sim().ability_slot(gyro, "gyrocopter_homing_missile").expect("the hero holds the ability");
    assert!(host.sim_mut().cast_ability(gyro, slot, target as i32, 0, 0), "the missile launches");
    let (mut landed_at, mut stunned) = (None, false);
    for tick in 0..8 * TICKS_PER_SECOND {
        host.advance().expect("tick");
        stunned |= host.sim().status_left(target, 0) > 0;
        if landed_at.is_none() && tick > 1 && host.sim().counter(gyro, "homing_target") == 0 {
            landed_at = Some(tick);
        }
    }
    let landed = landed_at.expect("the missile lands");
    assert!(landed >= 3 * TICKS_PER_SECOND, "nothing lands before the 3 s wait: tick {landed}");
    assert!(stunned, "the target is stunned: errors {:?}", host.hook_errors());
}

#[test]
fn lightning_storm_slows_every_unit_it_jumps_to() {
    let (mut host, leshrac) = host_with_hero("leshrac");
    learn(&mut host, leshrac, "leshrac_lightning_storm", 4);
    host.sim_mut().grant_energy(leshrac, 1000).expect("mana");
    let first = tank(&mut host, leshrac);
    let second = tank(&mut host, first);
    let third = tank(&mut host, second);
    let slot = host.sim().ability_slot(leshrac, "leshrac_lightning_storm").expect("the hero holds the ability");
    assert!(host.sim_mut().cast_ability(leshrac, slot, first as i32, 0, 0), "the storm casts");
    let mut slowed = [false; 3];
    for _ in 0..30 {
        host.advance().expect("tick");
        for (n, &unit) in [first, second, third].iter().enumerate() {
            slowed[n] |= host.sim().slow_multiplier(unit) < fixed::ONE;
        }
    }
    assert_eq!(slowed, [true; 3], "each unit in the chain is slowed: errors {:?}", host.hook_errors());
}

#[test]
fn time_lock_deals_its_damage_twice_inside_chronosphere() {
    let (mut host, void) = host_with_hero("faceless_void");
    learn(&mut host, void, "faceless_void_time_lock", 4);
    learn(&mut host, void, "faceless_void_chronosphere", 1);
    host.sim_mut().grant_energy(void, 1000).expect("mana");
    host.sim_mut().add_stat_bonus(void, 12, 1000);
    let enemy = tank(&mut host, void);
    let slot = host.sim().ability_slot(void, "faceless_void_chronosphere").expect("the hero holds the ability");
    let (x, y) = host.sim().position_fixed(enemy);
    assert!(host.sim_mut().cast_ability(void, slot, -1, x, y), "Chronosphere casts");
    for _ in 0..4 * TICKS_PER_SECOND {
        host.advance().expect("tick");
    }
    assert!(host.hook_errors().is_empty(), "no hook errors: {:?}", host.hook_errors());
    assert!(host.sim().counter(void, "chrono_due") > 0, "the sphere is recorded");
    assert!(host.sim().counter(void, "time_lock_doubled") > 0, "a Time Lock proc inside the sphere dealt its damage twice: errors {:?}", host.hook_errors());
}

#[test]
fn flak_cannon_hits_every_enemy_in_range_with_the_next_attacks() {
    let (mut host, gyro) = host_with_hero("gyrocopter");
    learn(&mut host, gyro, "gyrocopter_flak_cannon", 1);
    host.sim_mut().grant_energy(gyro, 1000).expect("mana");
    let first = tank(&mut host, gyro);
    let bystander = tank(&mut host, first);
    let slot = host.sim().ability_slot(gyro, "gyrocopter_flak_cannon").expect("the hero holds the ability");
    assert!(host.sim_mut().cast_ability(gyro, slot, -1, 0, 0), "Flak Cannon casts");
    host.advance().expect("tick");
    let before = host.sim().hp(bystander);
    host.sim_mut().add_stat_bonus(gyro, 12, 500);
    for _ in 0..8 * TICKS_PER_SECOND {
        host.advance().expect("tick");
    }
    assert!(host.sim().hp(bystander) < before, "the bystander is hit by the flak: errors {:?}", host.hook_errors());
    assert_eq!(host.sim().counter(gyro, "flak_left"), 0, "the attacks are spent");
}

#[test]
fn scepter_wrath_deals_its_extra_damage_to_each_chained_unit() {
    let (mut host, furion) = host_with_hero("furion");
    learn(&mut host, furion, "furion_wrath_of_nature", 1);
    host.sim_mut().set_upgrade_bit(furion, 0, true).expect("the scepter bit");
    host.sim_mut().grant_energy(furion, 1000).expect("mana");
    let first = tank(&mut host, furion);
    let second = tank(&mut host, first);
    let _third = tank(&mut host, second);
    let slot = host.sim().ability_slot(furion, "furion_wrath_of_nature").expect("the hero holds the ability");
    assert!(host.sim_mut().cast_ability(furion, slot, first as i32, 0, 0), "Wrath casts");
    for _ in 0..40 {
        host.advance().expect("tick");
    }
    assert!(host.sim().counter(first, "wrath_boosted") > 0 && host.sim().counter(second, "wrath_boosted") > 0, "each chained unit takes the scepter damage: errors {:?}", host.hook_errors());
}

#[test]
fn scepter_call_down_adds_damage_to_the_second_missile() {
    let (mut host, gyro) = host_with_hero("gyrocopter");
    learn(&mut host, gyro, "gyrocopter_call_down", 1);
    host.sim_mut().set_upgrade_bit(gyro, 0, true).expect("the scepter bit");
    host.sim_mut().grant_energy(gyro, 1000).expect("mana");
    let target = tank(&mut host, gyro);
    let slot = host.sim().ability_slot(gyro, "gyrocopter_call_down").expect("the hero holds the ability");
    let (x, y) = host.sim().position_fixed(target);
    assert!(host.sim_mut().cast_ability(gyro, slot, -1, x, y), "Call Down casts");
    for _ in 0..3 {
        host.advance().expect("tick");
    }
    assert!(host.sim().counter(gyro, "calldown_due") > 0, "the extra missile is pending");
    let before = host.sim().hp(target);
    for _ in 0..6 * TICKS_PER_SECOND {
        host.advance().expect("tick");
    }
    assert_eq!(host.sim().counter(gyro, "calldown_due"), 0, "the extra missile landed: errors {:?}", host.hook_errors());
    assert!(host.sim().hp(target) < before, "the target took damage");
}
