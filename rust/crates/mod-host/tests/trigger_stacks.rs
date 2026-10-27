
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
fn flesh_heap_counts_nearby_enemy_hero_deaths_into_strength() {
    let (mut host, pudge) = host_with_hero("pudge");
    learn(&mut host, pudge, "pudge_flesh_heap", 1);
    for _ in 0..2 {
        let victim = foe(&mut host, pudge, "antimage", true);
        host.sim_mut().hurt(victim, i32::MAX / 2);
        host.advance().expect("the death tick");
    }
    assert!(host.hook_errors().is_empty(), "hook errors {:?}", host.hook_errors());
    assert_eq!(host.sim().counter(pudge, "flesh_deaths"), 2, "two hero deaths counted");
    assert_eq!(host.sim().stat(pudge, 22), 2, "floor(2 x (0.5 + 0.5 x 1)) strength");
}

#[test]
fn necromastery_takes_a_soul_per_kill_and_twelve_per_hero_up_to_the_limit() {
    let (mut host, sf) = host_with_hero("nevermore");
    learn(&mut host, sf, "nevermore_necromastery", 1);
    let creep = foe(&mut host, sf, "creep_ranged", false);
    slay(&mut host, sf, creep);
    assert_eq!(host.sim().counter(sf, "souls"), 1, "a soul per creep");
    let hero = foe(&mut host, sf, "antimage", true);
    slay(&mut host, sf, hero);
    assert_eq!(host.sim().counter(sf, "souls"), 13, "twelve more for a hero");
    let hero = foe(&mut host, sf, "antimage", true);
    slay(&mut host, sf, hero);
    assert_eq!(host.sim().counter(sf, "souls"), 15, "held at the rank-one limit");
}

#[test]
fn reactive_armor_stacks_one_armor_per_hit_to_the_rank_limit_and_expires_after_sixteen_seconds() {
    let (mut host, shredder) = host_with_hero("shredder");
    learn(&mut host, shredder, "shredder_reactive_armor", 1);
    let attacker = foe(&mut host, shredder, "antimage", true);
    for _ in 0..7 {
        host.sim_mut().spell_damage(attacker as i32, shredder, 1, 3);
        host.advance().expect("tick");
    }
    assert_eq!(host.sim().stat(shredder, 1), 5, "five stacks at rank one");
    assert_eq!(host.sim().counter(shredder, "reactive_regen"), 500, "five hit points a second of regeneration");
    for _ in 0..17 * TICKS_PER_SECOND {
        host.advance().expect("tick");
    }
    assert_eq!(host.sim().stat(shredder, 1), 0, "the stacks end after sixteen seconds");
}

#[test]
fn caustic_finale_marks_the_first_hit_and_explodes_on_death_or_at_half_damage_after_six_seconds() {
    let (mut host, king) = host_with_hero("sand_king");
    learn(&mut host, king, "sand_king_caustic_finale", 1);
    let marked = foe(&mut host, king, "creep", false);
    let bystander = foe(&mut host, king, "creep", false);
    host.sim_mut().spell_damage(king as i32, marked, 1, 3);
    host.advance().expect("tick");
    let before = host.sim().hp(bystander);
    slay(&mut host, king, marked);
    host.advance().expect("tick");
    let lost = (before - host.sim().hp(bystander)) as f64 / fixed::ONE as f64;
    assert!(lost > 50.0 && lost <= 90.0, "the death explosion hurt the bystander by {lost} (90 before magic resistance)");
    assert!(host.sim().status_left(bystander, 1) > 0, "the explosion slows");

    let timed = foe(&mut host, king, "creep", false);
    let other = tank(&mut host, king);
    for unit in [king, timed, other] {
        host.sim_mut().apply_status(unit, 7, 1_000_000, fixed::ONE);
    }
    host.sim_mut().spell_damage(king as i32, timed, 1, 3);
    host.advance().expect("tick");
    let due = host.sim().counter(timed, "caustic_due") as u32;
    assert!(due > 0, "the first hit marks the unit");
    while host.sim().ticks() < due {
        host.advance().expect("tick");
    }
    host.advance().expect("tick");
    assert!(host.sim().status_left(other, 1) > 0, "the timed explosion slows the bystander");
    assert_eq!(host.sim().counter(timed, "caustic_due"), 0, "the mark is spent");
}

#[test]
fn rot_is_a_toggle_that_pulses_damage_on_enemies_in_reach_and_on_pudge_until_cast_again() {
    let (mut host, pudge) = host_with_hero("pudge");
    learn(&mut host, pudge, "pudge_rot", 1);
    let creep = tank(&mut host, pudge);
    for unit in [pudge, creep] {
        host.sim_mut().apply_status(unit, 7, 1_000_000, fixed::ONE);
    }
    let slot = host.sim().ability_slot(pudge, "pudge_rot").expect("the hero holds Rot");
    let (hp, creep_hp) = (host.sim().hp(pudge), host.sim().hp(creep));
    assert!(host.sim_mut().cast_ability(pudge, slot, -1, 0, 0), "Rot casts");
    for _ in 0..2 * TICKS_PER_SECOND {
        host.advance().expect("tick");
    }
    let lost = |after: i32, before: i32| (before - after) as f64 / fixed::ONE as f64;
    assert!(lost(host.sim().hp(creep), creep_hp) >= 10.0, "Rot hurts the enemy in reach: {}", lost(host.sim().hp(creep), creep_hp));
    assert!(lost(host.sim().hp(pudge), hp) >= 3.0, "Rot hurts Pudge too: {} errors {:?}", lost(host.sim().hp(pudge), hp), host.hook_errors());
    assert!(host.sim().slow_multiplier(creep) < fixed::ONE, "Rot slows the enemy: left {} mul {} creep loss {} on {}", host.sim().status_left(creep, 1), host.sim().slow_multiplier(creep), lost(host.sim().hp(creep), creep_hp), host.sim().counter(pudge, "rot_on"));
    assert!(host.sim_mut().cast_ability(pudge, slot, -1, 0, 0), "Rot casts again to switch off");
    host.advance().expect("tick");
    for _ in 0..2 * TICKS_PER_SECOND {
        host.advance().expect("tick");
    }
    assert_eq!(host.sim().status_left(creep, 1), 0, "no pulses after the toggle is switched off");
}

#[test]
fn spiked_carapace_returns_each_enemys_first_hit_as_magical_damage_and_stuns_it() {
    let (mut host, nyx) = host_with_hero("nyx_assassin");
    learn(&mut host, nyx, "nyx_assassin_spiked_carapace", 1);
    host.sim_mut().grant_energy(nyx, 1000).expect("mana");
    let slot = host.sim().ability_slot(nyx, "nyx_assassin_spiked_carapace").expect("the hero holds the ability");
    assert!(host.sim_mut().cast_ability(nyx, slot, -1, 0, 0), "the carapace casts");
    host.advance().expect("tick");
    let attacker = tank(&mut host, nyx);
    let (hp, theirs) = (host.sim().hp(nyx), host.sim().hp(attacker));
    host.sim_mut().spell_damage(attacker as i32, nyx, 100, 3);
    host.advance().expect("tick");
    assert!(host.sim().status_left(attacker, 0) > 0, "the attacker is stunned");
    assert!(host.sim().hp(attacker) < theirs, "the damage came back");
    assert!(host.sim().hp(nyx) as i64 >= hp as i64 - 10 * fixed::ONE as i64, "the hit was undone");
    let again = host.sim().hp(attacker);
    host.sim_mut().spell_damage(attacker as i32, nyx, 100, 3);
    host.advance().expect("tick");
    assert!(host.sim().hp(attacker) >= again - 20 * fixed::ONE, "a second hit by the same unit is not returned again");
}

#[test]
fn nether_ward_is_placed_for_thirty_seconds_and_dies_on_its_fourth_hit() {
    let (mut host, pugna) = host_with_hero("pugna");
    learn(&mut host, pugna, "pugna_nether_ward", 1);
    host.sim_mut().grant_energy(pugna, 1000).expect("mana");
    let slot = host.sim().ability_slot(pugna, "pugna_nether_ward").expect("the hero holds the ability");
    let (x, y) = host.sim().position_fixed(pugna);
    assert!(host.sim_mut().cast_ability(pugna, slot, -1, x + fixed::from_int(2), y), "the ward casts");
    host.advance().expect("tick");
    let kind = host.sim().catalog().unit_index("pugna_nether_ward_1").expect("the ward unit") as i32;
    let ward = (0..host.sim().count()).find(|&u| host.sim().alive(u) && host.sim().unit_type(u) == kind).expect("a ward stands");
    assert!(host.sim().timed_left(ward) > 400, "the ward lives about 30 s");
    let attacker = tank(&mut host, pugna);
    for hit in 1..=4 {
        assert!(host.sim().alive(ward), "alive before hit {hit}");
        host.sim_mut().spell_damage(attacker as i32, ward, 1, 3);
        host.advance().expect("tick");
    }
    assert!(!host.sim().alive(ward), "the fourth hit kills the ward");
}

#[test]
fn soul_catcher_marks_enemies_in_its_area_so_each_hit_for_twelve_seconds_deals_twenty_percent_more() {
    let (mut host, caster) = host_with_hero("shadow_demon");
    learn(&mut host, caster, "shadow_demon_soul_catcher", 1);
    host.sim_mut().grant_energy(caster, 1000).expect("mana");
    let victim = tank(&mut host, caster);
    let slot = host.sim().ability_slot(caster, "shadow_demon_soul_catcher").expect("the hero holds the ability");
    let (x, y) = host.sim().position_fixed(victim);
    assert!(host.sim_mut().cast_ability(caster, slot, -1, x, y), "the catcher casts");
    host.advance().expect("tick");
    assert_eq!(host.sim().counter(victim, "amp_all_pct"), 20, "the enemy is marked");
    let before = host.sim().hp(victim);
    host.sim_mut().spell_damage(caster as i32, victim, 400, 3);
    host.advance().expect("tick");
    let lost = (before - host.sim().hp(victim)) as f64 / fixed::ONE as f64;
    assert!(lost >= 470.0 && lost <= 520.0, "400 plus 20 percent, lost {lost}");
}

#[test]
fn decrepify_adds_spell_damage_taken_on_its_target() {
    let (mut host, caster) = host_with_hero("pugna");
    learn(&mut host, caster, "pugna_decrepify", 1);
    host.sim_mut().grant_energy(caster, 1000).expect("mana");
    let victim = tank(&mut host, caster);
    let slot = host.sim().ability_slot(caster, "pugna_decrepify").expect("the hero holds the ability");
    assert!(host.sim_mut().cast_ability(caster, slot, victim as i32, 0, 0), "decrepify casts");
    host.advance().expect("tick");
    assert_eq!(host.sim().counter(victim, "amp_spell_pct"), 30, "an enemy takes 30 percent more");
}

#[test]
fn mana_shield_toggles_and_pays_sixty_percent_of_a_hit_from_mana() {
    let (mut host, medusa) = host_with_hero("medusa");
    learn(&mut host, medusa, "medusa_mana_shield", 1);
    host.sim_mut().grant_energy(medusa, 1000).expect("mana");
    let slot = host.sim().ability_slot(medusa, "medusa_mana_shield").expect("the hero holds the ability");
    assert!(host.sim_mut().cast_ability(medusa, slot, -1, 0, 0), "the shield casts");
    host.advance().expect("tick");
    assert_eq!(host.sim().counter(medusa, "mshield_on"), 1, "the shield is on");
    let attacker = tank(&mut host, medusa);
    let (hp, mana) = (host.sim().hp(medusa), host.sim().mana_of(medusa));
    host.sim_mut().spell_damage(attacker as i32, medusa, 100, 3);
    host.advance().expect("tick");
    let lost = (hp - host.sim().hp(medusa)) as f64 / fixed::ONE as f64;
    assert!(lost <= 55.0 && lost >= 30.0, "40 of 100 reaches life, lost {lost}");
    assert!(mana - host.sim().mana_of(medusa) >= 30, "about 38 mana paid");
    assert!(host.sim_mut().cast_ability(medusa, slot, -1, 0, 0), "the shield casts again");
    host.advance().expect("tick");
    assert_eq!(host.sim().counter(medusa, "mshield_on"), 0, "the shield is off");
}

#[test]
fn sadist_gives_mana_and_life_a_second_for_six_seconds_after_a_kill() {
    let (mut host, necro) = host_with_hero("necrolyte");
    learn(&mut host, necro, "necrolyte_sadist", 1);
    assert!(host.sim_mut().set_mana_within(necro, 10), "mana set");
    let creep = foe(&mut host, necro, "creep", false);
    slay(&mut host, necro, creep);
    for _ in 0..7 * TICKS_PER_SECOND {
        host.advance().expect("tick");
    }
    assert!(host.sim().mana_of(necro) >= 16, "2 mana a second for 6 s on top of 10: {}", host.sim().mana_of(necro));
}

#[test]
fn static_link_moves_damage_from_a_hero_to_razor_for_eight_seconds_and_holds_it_eighteen_more() {
    let (mut host, razor) = host_with_hero("razor");
    learn(&mut host, razor, "razor_static_link", 1);
    host.sim_mut().grant_energy(razor, 1000).expect("mana");
    let target = tank(&mut host, razor);
    for unit in [razor, target] {
        host.sim_mut().apply_status(unit, 7, 1_000_000, fixed::ONE);
    }
    let slot = host.sim().ability_slot(razor, "razor_static_link").expect("the hero holds the ability");
    assert!(host.sim_mut().cast_ability(razor, slot, target as i32, 0, 0), "the link casts");
    for _ in 0..9 * TICKS_PER_SECOND {
        host.advance().expect("tick");
    }
    assert_eq!(host.sim().counter(razor, "slink_gain"), 56, "7 x 8 damage stolen at rank one");
    assert_eq!(host.sim().counter(target, "slink_loss"), -56, "taken from the target");
    for _ in 0..19 * TICKS_PER_SECOND {
        host.advance().expect("tick");
    }
    assert_eq!(host.sim().counter(razor, "slink_gain"), 0, "the bonus ends 18 s after the link");
    assert_eq!(host.sim().counter(target, "slink_loss"), 0, "the loss ends with it");
}

#[test]
fn mana_burn_burns_intelligence_times_rank_mana_and_deals_it_as_damage() {
    let (mut host, nyx) = host_with_hero("nyx_assassin");
    learn(&mut host, nyx, "nyx_assassin_mana_burn_nyx", 1);
    host.sim_mut().grant_energy(nyx, 1000).expect("mana");
    host.sim_mut().set_counter(nyx, "attr_int", 20);
    let victim = tank(&mut host, nyx);
    host.advance().expect("tick");
    let (mana, slot) = (host.sim().mana_of(victim), host.sim().ability_slot(nyx, "nyx_assassin_mana_burn_nyx").expect("slot"));
    assert!(host.sim_mut().cast_ability(nyx, slot, victim as i32, 0, 0), "the burn casts");
    host.advance().expect("tick");
    let burned = mana - host.sim().mana_of(victim);
    assert!((60..=75).contains(&burned), "20 intelligence x 3.5 burns about 70 mana, burned {burned} of {mana}");
}

#[test]
fn demonic_purge_dispels_the_target_and_hurts_it_when_the_slow_ends() {
    let (mut host, caster) = host_with_hero("shadow_demon");
    learn(&mut host, caster, "shadow_demon_demonic_purge", 1);
    host.sim_mut().grant_energy(caster, 1000).expect("mana");
    let victim = tank(&mut host, caster);
    let slot = host.sim().ability_slot(caster, "shadow_demon_demonic_purge").expect("slot");
    let before = host.sim().hp(victim);
    assert!(host.sim_mut().cast_ability(caster, slot, victim as i32, 0, 0), "the purge casts");
    for _ in 0..3 * TICKS_PER_SECOND {
        host.advance().expect("tick");
    }
    let lost = (before - host.sim().hp(victim)) as f64 / fixed::ONE as f64;
    assert!(lost >= 120.0, "200 magical damage at the end, lost {lost}");
}

#[test]
fn curse_of_the_silent_burns_eight_mana_a_second_per_rank_for_six_seconds() {
    let (mut host, silencer) = host_with_hero("silencer");
    learn(&mut host, silencer, "silencer_curse_of_the_silent", 1);
    host.sim_mut().grant_energy(silencer, 1000).expect("mana");
    let victim = tank(&mut host, silencer);
    host.advance().expect("tick");
    let slot = host.sim().ability_slot(silencer, "silencer_curse_of_the_silent").expect("slot");
    let (x, y) = host.sim().position_fixed(victim);
    let mana = host.sim().mana_of(victim);
    assert!(host.sim_mut().cast_ability(silencer, slot, -1, x, y), "the curse casts");
    for _ in 0..7 * TICKS_PER_SECOND {
        host.advance().expect("tick");
    }
    let burned = mana - host.sim().mana_of(victim);
    assert!(burned >= 40, "8 mana a second for 6 s burns about 48, burned {burned} of {mana}");
}

#[test]
fn power_cog_rings_eight_cogs_that_shock_enemies_with_mana_and_die_on_the_second_attack() {
    let (mut host, gob) = host_with_hero("rattletrap");
    learn(&mut host, gob, "rattletrap_power_cog", 1);
    host.sim_mut().grant_energy(gob, 1000).expect("mana");
    let slot = host.sim().ability_slot(gob, "rattletrap_power_cog").expect("slot");
    assert!(host.sim_mut().cast_ability(gob, slot, -1, 0, 0), "the cogs cast");
    host.advance().expect("tick");
    let kind = host.sim().catalog().unit_index("rattletrap_power_cog_1").expect("the cog unit") as i32;
    let cogs: Vec<usize> = (0..host.sim().count()).filter(|&u| host.sim().alive(u) && host.sim().unit_type(u) == kind).collect();
    assert_eq!(cogs.len(), 8, "a ring of eight cogs");
    let foe = foe(&mut host, cogs[0], "antimage", true);
    host.advance().expect("tick");
    let full = host.sim().unit_max_mana(foe);
    assert!(host.sim().mana_of(foe) + 60 <= full, "the shock burns 80 mana: {} of {full}", host.sim().mana_of(foe));
    let attacker = tank(&mut host, cogs[3]);
    host.sim_mut().spell_damage(attacker as i32, cogs[3], 1, 3);
    host.advance().expect("tick");
    assert!(host.sim().alive(cogs[3]), "alive after one hit");
    host.sim_mut().spell_damage(attacker as i32, cogs[3], 1, 3);
    host.advance().expect("tick");
    assert!(!host.sim().alive(cogs[3]), "the second hit destroys the cog");
}

#[test]
fn divided_we_stand_keeps_a_clone_per_rank_that_dies_with_the_hero_and_poof_jumps_to_it() {
    let (mut host, meepo) = host_with_hero("meepo");
    learn(&mut host, meepo, "meepo_divided_we_stand", 1);
    learn(&mut host, meepo, "meepo_poof", 1);
    host.sim_mut().grant_energy(meepo, 1000).expect("mana");
    for _ in 0..2 * TICKS_PER_SECOND {
        host.advance().expect("tick");
    }
    let kind = host.sim().catalog().unit_index("meepo").expect("the hero kind") as i32;
    let clone = (0..host.sim().count()).find(|&u| u != meepo && host.sim().alive(u) && host.sim().unit_type(u) == kind).expect("a clone stands");
    let spot = host.sim().position_fixed(clone);
    host.sim_mut().teleport(meepo, spot.0 + fixed::from_int(10), spot.1);
    host.advance().expect("tick");
    let slot = host.sim().ability_slot(meepo, "meepo_poof").expect("slot");
    assert!(host.sim_mut().cast_ability(meepo, slot, clone as i32, 0, 0), "poof casts");
    for _ in 0..2 * TICKS_PER_SECOND {
        host.advance().expect("tick");
    }
    let (mx, _) = host.sim().position_fixed(meepo);
    assert!((mx - host.sim().position_fixed(clone).0).abs() < fixed::from_int(3), "poof moved Meepo to the clone");
    host.sim_mut().hurt(meepo, i32::MAX / 2);
    host.advance().expect("tick");
    host.advance().expect("tick");
    assert!(!host.sim().alive(clone), "the clone dies with the hero");
}
