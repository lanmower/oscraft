use std::path::{Path, PathBuf};

use mod_host::{build_image, ModHost};
use serde_json::json;
use sim::fixed::{self, HALF};

const OWNER: u8 = 0;
const FOE_PLAYER: u8 = 1;
const HERO_KIND: &str = "skeleton_king";
const FOE_KIND: &str = "axe";
const HELLFIRE: &str = "skeleton_king_hellfire_blast";
const HERO_CELL: (i32, i32) = (92, 240);
const FOE_CELL: (i32, i32) = (96, 240);
const FOE_AIM: (f64, f64) = (96.5, 240.5);
const EMPTY_AIM: (f64, f64) = (92.5, 236.5);
const FAR_AIM: (f64, f64) = (120.5, 240.5);
const EMPTY_SLOT: usize = 9;
const NO_TARGET_KIND: &str = "axe";
const NO_TARGET_ABILITY: &str = "axe_berserkers_call";
const CAST_TICKS: u32 = 40;
const NO_TARGET: i32 = -1;

fn pack_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../scenarios/moba")
}

fn collect(dir: &Path, rel: &str, out: &mut Vec<(String, String)>) {
    for entry in std::fs::read_dir(dir).expect("read the pack directory") {
        let path = entry.expect("read a pack entry").path();
        let name = path.file_name().expect("a pack file name").to_string_lossy().into_owned();
        if path.is_dir() {
            if name != "fixtures" {
                collect(&path, &format!("{rel}{name}/"), out);
            }
        } else if path.extension().is_some_and(|e| e == "lua") {
            out.push((format!("{rel}{name}"), std::fs::read_to_string(&path).expect("read a lua module")));
        }
    }
}

fn stem(rel: &str) -> &str {
    let file = rel.rsplit('/').next().unwrap_or(rel);
    file.strip_suffix(".lua").unwrap_or(file)
}

fn visit(files: &[(String, String)], module: &str, out: &mut Vec<(String, String)>) {
    let Some((rel, src)) = files.iter().find(|(rel, _)| stem(rel) == module) else { return };
    if out.iter().any(|(name, _)| name == rel) {
        return;
    }
    out.push((rel.clone(), src.clone()));
    for required in src.split("require(\"").skip(1).filter_map(|rest| rest.split('"').next()) {
        visit(files, required, out);
    }
}

fn pack_host() -> ModHost {
    let dir = pack_dir();
    let manifest: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("manifest.json")).expect("the pack manifest"))
        .expect("a valid pack manifest");
    let rules = std::fs::read_to_string(dir.join(manifest["rules"].as_str().expect("the manifest rules file")))
        .expect("the pack rules file");
    let mut files = Vec::new();
    collect(&dir, "", &mut files);
    files.sort();
    let mut mods = Vec::new();
    for root in manifest["mods"].as_array().expect("the manifest mods list") {
        visit(&files, stem(root.as_str().expect("a mod file name")), &mut mods);
    }
    let image = build_image(&dir).expect("build the moba image");
    let mut host = ModHost::new(&image).expect("load the moba image");
    assert!(host.sim_mut().set_attr_rules_text(&rules), "the pack rules are accepted");
    host.load_mods(&mods).expect("load the moba mods");
    host.sim_mut().set_fog(false);
    host
}

fn spawn(host: &mut ModHost, kind_name: &str, player: u8, cell: (i32, i32)) -> usize {
    let mut sim = host.sim_mut();
    let kind = sim.catalog().unit_index(kind_name).expect("the kind is in the moba catalog");
    let x = fixed::from_int(cell.0) + HALF;
    let y = fixed::from_int(cell.1) + HALF;
    sim.spawn_attributed(player, kind as u32, x, y).expect("the unit fits on the map")
}

fn fill_mana(host: &mut ModHost, hero: usize) {
    let mut sim = host.sim_mut();
    let max = sim.unit_max_mana(hero);
    assert!(sim.set_mana_within(hero, max), "the hero has the mana to cast");
}

fn advance(host: &mut ModHost, ticks: u32) {
    for _ in 0..ticks {
        host.advance().expect("advance the match");
    }
}

fn ui(host: &mut ModHost, call: &str, args: serde_json::Value) {
    host.dispatch_action("ui", &json!({ "call": call, "args": args, "player": OWNER }))
        .expect("the ui call is accepted");
}

fn select(host: &mut ModHost, hero: usize) {
    host.dispatch_action("select", &json!({ "ids": [hero], "player": OWNER })).expect("the selection is accepted");
}

fn one_hand(host: &mut ModHost, on: bool) {
    ui(host, "one_hand", json!({ "on": on }));
}

fn cast_at(host: &mut ModHost, slot: usize, aim: (f64, f64)) {
    ui(host, "cast_at", json!({ "slot": slot, "x": aim.0, "z": aim.1 }));
}

fn learn(host: &mut ModHost, hero: usize, slot: usize) {
    host.sim_mut().rank_up(hero, slot).expect("the ability takes its first rank");
}

fn field(one_hand_on: bool) -> (ModHost, usize, usize) {
    let mut host = pack_host();
    let hero = spawn(&mut host, HERO_KIND, OWNER, HERO_CELL);
    fill_mana(&mut host, hero);
    select(&mut host, hero);
    one_hand(&mut host, one_hand_on);
    let slot = host.sim().ability_slot(hero, HELLFIRE).expect("Hellfire Blast is in a hero slot");
    learn(&mut host, hero, slot);
    (host, hero, slot)
}

fn spawn_foe(host: &mut ModHost) -> usize {
    let foe = spawn(host, FOE_KIND, FOE_PLAYER, FOE_CELL);
    advance(host, 1);
    foe
}

fn refusals(host: &mut ModHost) -> Vec<String> {
    host.take_ui().into_iter().filter(|line| line.contains("\"kind\":\"refusal\"")).collect()
}

#[test]
fn cast_at_with_one_hand_off_casts_the_unit_under_the_aim() {
    let (mut host, hero, slot) = field(false);
    let foe = spawn_foe(&mut host);
    host.take_ui();
    cast_at(&mut host, slot, FOE_AIM);
    advance(&mut host, CAST_TICKS);
    assert_eq!(host.sim().last_cast(hero).0, foe as i32, "the aimed unit is the cast target when one-hand is off");
    assert!(refusals(&mut host).is_empty(), "a cast at a unit under the aim is not refused");
}

#[test]
fn cast_at_with_one_hand_on_ignores_the_aim_and_takes_the_best_unit_in_range() {
    let (mut host, hero, slot) = field(true);
    let foe = spawn_foe(&mut host);
    host.take_ui();
    cast_at(&mut host, slot, EMPTY_AIM);
    advance(&mut host, CAST_TICKS);
    assert_eq!(host.sim().last_cast(hero).0, foe as i32, "one-hand picks the best unit in range, not the empty aim point");
    assert!(refusals(&mut host).is_empty(), "a best-in-range cast under one-hand is not refused");
}

#[test]
fn cast_at_with_one_hand_on_and_no_unit_in_range_refuses_without_casting() {
    let (mut host, hero, slot) = field(true);
    host.take_ui();
    cast_at(&mut host, slot, FOE_AIM);
    advance(&mut host, CAST_TICKS);
    assert_eq!(host.sim().last_cast(hero).0, NO_TARGET, "no unit in range means no cast under one-hand");
    assert!(refusals(&mut host).iter().any(|line| line.contains("\"reason\":\"noTarget\"")), "the refusal is noTarget");
}

#[test]
fn cast_at_with_one_hand_off_and_no_unit_under_the_aim_refuses_without_casting() {
    let (mut host, hero, slot) = field(false);
    spawn_foe(&mut host);
    host.take_ui();
    cast_at(&mut host, slot, EMPTY_AIM);
    advance(&mut host, CAST_TICKS);
    assert_eq!(host.sim().last_cast(hero).0, NO_TARGET, "an empty aim point is not a unit target");
    assert!(refusals(&mut host).iter().any(|line| line.contains("\"reason\":\"noTarget\"")), "the refusal is noTarget");
}

#[test]
fn cast_at_on_a_no_target_ability_casts_at_the_caster_whatever_the_aim() {
    let mut host = pack_host();
    let hero = spawn(&mut host, NO_TARGET_KIND, OWNER, HERO_CELL);
    advance(&mut host, 1);
    fill_mana(&mut host, hero);
    select(&mut host, hero);
    one_hand(&mut host, false);
    let slot = host.sim().ability_slot(hero, NO_TARGET_ABILITY).expect("Berserker's Call is in a hero slot");
    learn(&mut host, hero, slot);
    host.take_ui();
    cast_at(&mut host, slot, FAR_AIM);
    advance(&mut host, CAST_TICKS);
    let (target, x, y) = host.sim().last_cast(hero);
    assert_eq!(target, NO_TARGET, "a no-target ability has no unit target");
    assert_ne!((x, y), (0, 0), "the no-target cast was applied");
    let aim = (fixed::from_int(120) + HALF, fixed::from_int(240) + HALF);
    assert_ne!((x, y), aim, "a no-target ability is not cast at the aim point");
}

#[test]
fn cast_at_on_an_empty_slot_refuses_with_no_ability() {
    let (mut host, hero, _) = field(false);
    host.take_ui();
    cast_at(&mut host, EMPTY_SLOT, FOE_AIM);
    advance(&mut host, CAST_TICKS);
    assert_eq!(host.sim().last_cast(hero).0, NO_TARGET, "an empty slot casts nothing");
    assert!(refusals(&mut host).iter().any(|line| line.contains("\"reason\":\"noAbility\"")), "the refusal is noAbility");
}
