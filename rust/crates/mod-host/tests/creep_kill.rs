
#![cfg(feature = "build")]

use std::path::PathBuf;

use mod_host::{Cmd, Death, ModHost};
use sim::fixed;

mod common;

const NEUTRAL: u8 = 255;

fn scenario() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../scenarios/moba")
}

fn show(c: &Cmd) -> String {
    match c {
        Cmd::AddGold { player, amount } => format!("gold player={player} amount={amount}"),
        Cmd::AddUnitGold { unit, amount } => format!("unit_gold unit={unit} amount={amount}"),
        _ => "other".to_owned(),
    }
}

fn death(victim: u32, victim_player: u8, killer: Option<u32>, killer_player: Option<u8>) -> Death {
    Death { victim, victim_player, killer, killer_player, bounty: 0 }
}

fn gold_lines(host: &mut ModHost, d: &Death) -> Vec<String> {
    let fed = host.feed_death(d).expect("fed");
    fed.iter().map(show).filter(|l| l != "other").collect()
}

fn unit_of(host: &ModHost, player: u8, except: usize) -> usize {
    (0..host.sim().count())
        .find(|&u| u != except && host.sim().player(u) == player)
        .unwrap_or_else(|| panic!("the map has a unit of player {player}"))
}

fn match_with_heroes() -> (ModHost, u8) {
    let image = mod_host::build_image(&scenario()).expect("moba scenario builds an image");
    let mut host = ModHost::new(&image).expect("host");
    for p in 0..4u8 {
        host.sim_mut().set_team(p, p).expect("team");
    }
    let rules = std::fs::read_to_string(scenario().join("attr_rules.txt")).expect("rules");
    assert!(host.sim_mut().set_attr_rules_text(&rules), "the pack's rules file loads");
    for unit in [10usize, 20, 21] {
        host.sim_mut().enroll_attributed(unit, 0, 0);
    }
    let victim_player = host.sim().player(10);
    assert_eq!(victim_player, 0, "the victim is a player 0 hero by the map");
    let mods = common::pack_mods(&scenario(), &["heroes.lua".to_owned(), "kill_gold.lua".to_owned()]);
    host.load_mods(&mods).expect("the hero and kill gold rules load");
    (host, victim_player)
}

fn count(lines: &[String], want: &str) -> usize {
    lines.iter().filter(|l| l.as_str() == want).count()
}

const DEATH_LOSS: &str = "gold player=0 amount=-30";

#[test]
fn a_creep_kill_with_no_hero_damage_splits_among_the_enemy_heroes() {
    let (mut host, victim_player) = match_with_heroes();
    let lines = gold_lines(&mut host, &death(10, victim_player, Some(22), Some(1)));
    println!("CREEPKILL no-damage {lines:?}");
    assert_eq!(count(&lines, "gold player=1 amount=54"), 2, "109 split between the two enemy heroes: {lines:?}");
    assert_eq!(count(&lines, DEATH_LOSS), 1);
}

#[test]
fn a_creep_kill_damaged_by_one_hero_pays_that_hero_all_of_it() {
    let (mut host, victim_player) = match_with_heroes();
    host.sim_mut().hit_by(10, fixed::from_int(1), 21);
    let lines = gold_lines(&mut host, &death(10, victim_player, Some(22), Some(1)));
    println!("CREEPKILL one-damager {lines:?}");
    assert_eq!(count(&lines, "gold player=1 amount=109"), 1, "{lines:?}");
    assert_eq!(count(&lines, "gold player=1 amount=150"), 1, "first blood goes to the damager: {lines:?}");
}

#[test]
fn a_creep_kill_damaged_by_two_heroes_splits_among_them() {
    let (mut host, victim_player) = match_with_heroes();
    host.sim_mut().hit_by(10, fixed::from_int(1), 20);
    host.sim_mut().hit_by(10, fixed::from_int(1), 21);
    let lines = gold_lines(&mut host, &death(10, victim_player, Some(22), Some(1)));
    println!("CREEPKILL two-damagers {lines:?}");
    assert_eq!(count(&lines, "gold player=1 amount=54"), 2, "{lines:?}");
    assert_eq!(count(&lines, "gold player=1 amount=75"), 2, "first blood splits 150: {lines:?}");
}

#[test]
fn a_neutral_kill_pays_no_hero_gold_and_still_costs_the_victim() {
    let (mut host, victim_player) = match_with_heroes();
    let camp = (0..host.sim().count())
        .find(|&u| host.sim().player(u) == NEUTRAL)
        .expect("the map has a neutral camp creep");
    let lines = gold_lines(&mut host, &death(10, victim_player, Some(camp as u32), Some(NEUTRAL)));
    println!("NEUTRAL {lines:?}");
    assert_eq!(lines, [DEATH_LOSS]);
}

#[test]
fn a_denied_creep_kill_pays_no_hero_gold_and_still_costs_the_victim() {
    let (mut host, victim_player) = match_with_heroes();
    let denier = unit_of(&host, victim_player, 10);
    let lines = gold_lines(&mut host, &death(10, victim_player, Some(denier as u32), Some(victim_player)));
    println!("DENIAL {lines:?}");
    assert_eq!(lines, [DEATH_LOSS]);
}

#[test]
fn a_hero_kill_pays_the_killer_all_of_it() {
    let (mut host, victim_player) = match_with_heroes();
    let lines = gold_lines(&mut host, &death(10, victim_player, Some(20), Some(1)));
    println!("HEROKILL {lines:?}");
    assert_eq!(count(&lines, "gold player=1 amount=200"), 1, "{lines:?}");
    assert_eq!(count(&lines, "gold player=1 amount=109"), 1, "{lines:?}");
}

#[test]
fn an_illusions_kill_pays_its_owner_hero_all_of_it() {
    let (mut host, victim_player) = match_with_heroes();
    let (x, y) = host.sim().position_fixed(20);
    let image = host.sim_mut().spawn_image(20, x, y, 1000, 35, 300).expect("an image of hero 20");
    assert!(host.sim().is_summoned(image), "an image is a summon");
    let lines = gold_lines(&mut host, &death(10, victim_player, Some(image as u32), Some(1)));
    println!("ILLUSIONKILL {lines:?}");
    assert_eq!(count(&lines, "gold player=1 amount=200"), 1, "{lines:?}");
    assert_eq!(count(&lines, "gold player=1 amount=109"), 1, "{lines:?}");
}
