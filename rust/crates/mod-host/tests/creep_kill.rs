//! A hero killed by an enemy creep: the kill gold goes to the enemy heroes (heroes.lua creep_kill_gold, the Dota rule). The
//! heroes that damaged it within kill_window_s share it, one taking all and several splitting it equally; with none, the
//! living enemy heroes split it. A neutral or a denied kill pays nothing, and a hero's kill keeps its team share.

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
    // Display effects (the gold text) show as "other"; these tests compare gold only.
    fed.iter().map(show).filter(|l| l != "other").collect()
}

// The first unit of `player` that is not `except`, by the map's own owners (the sim's player array).
fn unit_of(host: &ModHost, player: u8, except: usize) -> usize {
    (0..host.sim().count())
        .find(|&u| u != except && host.sim().player(u) == player)
        .unwrap_or_else(|| panic!("the map has a unit of player {player}"))
}

// Unit 10 is the victim, a hero of player 0 (the map's owner). Units 20 and 21 are heroes of player 1, the enemy side, and
// unit 22 is a player 1 creep. Each player is its own team, so every other player is an enemy.
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

// Hero gold is 125 plus 8 a level; the victim is level 0 at no xp, so 125. Two living enemy heroes split it 63 and 62, the
// first taking the remainder.
#[test]
fn a_creep_kill_with_no_hero_damage_splits_among_the_living_enemy_heroes() {
    let (mut host, victim_player) = match_with_heroes();
    let lines = gold_lines(&mut host, &death(10, victim_player, Some(22), Some(1)));
    println!("CREEPKILL no-damage {lines:?}");
    assert_eq!(lines, ["unit_gold unit=20 amount=63", "unit_gold unit=21 amount=62"]);
}

#[test]
fn a_creep_kill_damaged_by_one_hero_pays_that_hero_all_of_it() {
    let (mut host, victim_player) = match_with_heroes();
    host.sim_mut().hit_by(10, fixed::from_int(1), 21);
    let lines = gold_lines(&mut host, &death(10, victim_player, Some(22), Some(1)));
    println!("CREEPKILL one-damager {lines:?}");
    assert_eq!(lines, ["unit_gold unit=21 amount=125"]);
}

#[test]
fn a_creep_kill_damaged_by_two_heroes_splits_among_them() {
    let (mut host, victim_player) = match_with_heroes();
    host.sim_mut().hit_by(10, fixed::from_int(1), 20);
    host.sim_mut().hit_by(10, fixed::from_int(1), 21);
    let lines = gold_lines(&mut host, &death(10, victim_player, Some(22), Some(1)));
    println!("CREEPKILL two-damagers {lines:?}");
    assert_eq!(lines, ["unit_gold unit=20 amount=63", "unit_gold unit=21 amount=62"]);
}

#[test]
fn a_neutral_kill_pays_no_hero_gold() {
    let (mut host, victim_player) = match_with_heroes();
    let camp = (0..host.sim().count())
        .find(|&u| host.sim().player(u) == NEUTRAL)
        .expect("the map has a neutral camp creep");
    let lines = gold_lines(&mut host, &death(10, victim_player, Some(camp as u32), Some(NEUTRAL)));
    println!("NEUTRAL {lines:?}");
    assert_eq!(lines, Vec::<String>::new());
}

#[test]
fn a_denied_creep_kill_pays_no_hero_gold() {
    let (mut host, victim_player) = match_with_heroes();
    // A player 0 creep (not the victim): the victim's own side, so a denial.
    let denier = unit_of(&host, victim_player, 10);
    let lines = gold_lines(&mut host, &death(10, victim_player, Some(denier as u32), Some(victim_player)));
    println!("DENIAL {lines:?}");
    assert_eq!(lines, Vec::<String>::new());
}

#[test]
fn a_hero_kill_keeps_the_team_share() {
    let (mut host, victim_player) = match_with_heroes();
    // Hero 20 (player 1) kills hero 10: the killer keeps 125 less the 20 percent share (25), so 100, and player 1's side gets 25.
    let lines = gold_lines(&mut host, &death(10, victim_player, Some(20), Some(1)));
    println!("HEROKILL {lines:?}");
    assert_eq!(lines, ["unit_gold unit=20 amount=100", "gold player=1 amount=25"]);
}
