use std::path::{Path, PathBuf};

use mod_host::{build_image, ModHost};
use serde_json::json;
use sim::fixed::{self, HALF};

const PROBE: &str = r#"
local courier = require("courier")
local probe = {}
probe.on_action = function(name, args)
  if name == "probe_drop" then courier.command(args.who, args.unit, "drop") end
  if name == "probe_pickup" then courier.command(args.who, args.unit, "pickup") end
end
return probe
"#;

const SPOT: (i32, i32) = (92, 240);
const TOLERANCE: i32 = 2;

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

fn probe_host() -> ModHost {
    let dir = pack_dir();
    let rules = std::fs::read_to_string(dir.join("attr_rules.txt")).expect("the pack rules file");
    let mut files = Vec::new();
    collect(&dir, "", &mut files);
    files.sort();
    let mut mods = Vec::new();
    visit(&files, "courier", &mut mods);
    mods.push(("courier_owner_probe.lua".to_owned(), PROBE.to_owned()));
    let image = build_image(&dir).expect("build the moba image");
    let mut host = ModHost::new(&image).expect("load the moba image");
    assert!(host.sim_mut().set_attr_rules_text(&rules), "the pack rules are accepted");
    host.load_mods(&mods).expect("load the courier and the probe");
    host.sim_mut().set_fog(false);
    host
}

fn spawn_unit(host: &mut ModHost, kind_name: &str, player: u8, cell_x: i32, cell_y: i32) -> usize {
    let mut sim = host.sim_mut();
    let kind = sim.catalog().unit_index(kind_name).expect("the unit kind is in the moba catalog");
    let x = fixed::from_int(cell_x) + HALF;
    let y = fixed::from_int(cell_y) + HALF;
    sim.spawn_attributed(player, kind as u32, x, y).expect("the unit fits on the map")
}

fn give(host: &mut ModHost, unit: usize, item_name: &str, owner: usize) {
    let mut sim = host.sim_mut();
    let item = sim.catalog().item_index(item_name).expect("the item is in the moba catalog");
    assert!(sim.grant_item(unit, item, 1), "the unit takes the item");
    let kind = item as u32 + 1;
    let slot = (0..sim.inventory_slots())
        .find(|&s| sim.inventory_slot(unit, s).0 == kind)
        .expect("the item sits in a slot");
    sim.set_item_owner(unit, slot, owner).expect("the item takes an owner");
}

fn holds(host: &ModHost, unit: usize, item_name: &str) -> bool {
    let sim = host.sim();
    let kind = sim.catalog().item_index(item_name).expect("the item is in the moba catalog") as u32 + 1;
    (0..sim.inventory_slots()).any(|s| sim.inventory_slot(unit, s).0 == kind)
}

fn ground_of(host: &ModHost, item_name: &str) -> Vec<(i32, i32)> {
    let sim = host.sim();
    let kind = sim.catalog().item_index(item_name).expect("the item is in the moba catalog") as u32 + 1;
    sim.ground_items().iter().filter(|g| g.kind == kind).map(|g| (g.x, g.y)).collect()
}

fn dispatch(host: &mut ModHost, action: &str, unit: usize, who: u8) {
    host.dispatch_action(action, &json!({ "unit": unit, "who": who }))
        .expect("the probe takes the courier command");
}

fn advance(host: &mut ModHost, ticks: u32) {
    for _ in 0..ticks {
        host.advance().expect("advance the match");
    }
}

fn spot_point() -> (i32, i32) {
    (fixed::from_int(SPOT.0) + HALF, fixed::from_int(SPOT.1) + HALF)
}

fn near(actual: (i32, i32), expected: (i32, i32)) -> bool {
    (actual.0 - expected.0).abs() <= TOLERANCE && (actual.1 - expected.1).abs() <= TOLERANCE
}

fn player_one_home() -> (i32, i32) {
    (fixed::from_milli(273_000), fixed::from_milli(63_000))
}

#[test]
fn a_basic_courier_drops_an_owners_item_at_the_owners_home_point_from_anywhere() {
    let mut host = probe_host();
    let courier = spawn_unit(&mut host, "courier", 1, SPOT.0, SPOT.1);
    give(&mut host, courier, "ring_of_regen", 1);
    dispatch(&mut host, "probe_drop", courier, 1);
    advance(&mut host, 1);
    let drops = ground_of(&host, "ring_of_regen");
    assert_eq!(drops.len(), 1, "the courier drops its item on the ground");
    assert!(
        near(drops[0], player_one_home()),
        "a basic courier drops an owner's item at the owner's courier home point (6.88g SAV/LHV pd/Pd), not where it stands or at the hero start: got {:?}, want {:?}",
        drops[0],
        player_one_home()
    );
}

#[test]
fn a_flying_courier_outside_its_base_drops_an_owners_item_where_it_stands() {
    let mut host = probe_host();
    let courier = spawn_unit(&mut host, "courier_flying", 1, SPOT.0, SPOT.1);
    give(&mut host, courier, "ring_of_regen", 1);
    dispatch(&mut host, "probe_drop", courier, 1);
    advance(&mut host, 1);
    let drops = ground_of(&host, "ring_of_regen");
    assert_eq!(drops.len(), 1, "the flying courier drops its item on the ground");
    assert!(
        near(drops[0], spot_point()),
        "a flying courier outside its base keeps the item where it stands (LHV: pd/Pd only in base or for the ground courier): got {:?}, want {:?}",
        drops[0],
        spot_point()
    );
}

#[test]
fn a_loan_copy_of_a_rune_stays_on_the_courier_when_it_is_dropped() {
    let mut host = probe_host();
    let courier = spawn_unit(&mut host, "courier", 1, SPOT.0, SPOT.1);
    give(&mut host, courier, "bottle_illusion_loan", 1);
    dispatch(&mut host, "probe_drop", courier, 1);
    advance(&mut host, 1);
    assert!(ground_of(&host, "bottle_illusion_loan").is_empty(), "the loan copy of a bottle rune is not dropped (N0 6385 via B0)");
    assert!(holds(&host, courier, "bottle_illusion_loan"), "the courier still holds the loan copy");
}

#[test]
fn a_pickup_takes_only_the_callers_owned_items_from_couriers_in_reach() {
    let mut host = probe_host();
    let hero = spawn_unit(&mut host, "antimage", 1, SPOT.0, SPOT.1);
    let courier = spawn_unit(&mut host, "courier", 1, SPOT.0, SPOT.1);
    give(&mut host, courier, "ring_of_regen", 1);
    give(&mut host, courier, "enchanted_mango", 2);
    dispatch(&mut host, "probe_pickup", courier, 1);
    advance(&mut host, 4);
    assert!(holds(&host, hero, "ring_of_regen"), "the caller's item reaches the hero");
    assert!(!holds(&host, courier, "ring_of_regen"), "the caller's item leaves the courier");
    assert!(holds(&host, courier, "enchanted_mango"), "another player's item stays on the courier");
    assert!(!holds(&host, hero, "enchanted_mango"), "another player's item never reaches the caller's hero");
}
