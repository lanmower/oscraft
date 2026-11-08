use std::path::Path;

use mod_host::{build_image, ModHost};
use serde_json::json;
use sim::fixed::{self, HALF};

const UPGRADES: &str = include_str!("../../../../mods/upgrades.lua");
const FORGED_ARMOR_COST: i32 = 100;
const WAR_BANNER_COST: i32 = 150;

fn host() -> ModHost {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../scenarios/moba");
    let image = build_image(&dir).expect("build the moba image");
    let mut host = ModHost::new(&image).expect("load the moba image");
    host.load_source("upgrades.lua", UPGRADES).expect("load mods/upgrades.lua");
    host
}

fn barracks(host: &mut ModHost, player: u8, cell_x: i32, cell_y: i32) -> usize {
    let mut sim = host.sim_mut();
    let kind = sim.catalog().unit_index("barracks").expect("the upgrades mod defines barracks");
    let x = fixed::from_int(cell_x) + HALF;
    let y = fixed::from_int(cell_y) + HALF;
    sim.spawn_for(player, kind as u32, x, y).expect("the barracks fits on the map")
}

fn set_gold(host: &mut ModHost, player: u8, target: i32) {
    let delta = target - host.sim().resource_a(player);
    host.sim_mut().add_resources(player, delta, 0);
}

fn request(host: &mut ModHost, player: u8, unit: usize, group: &str) {
    host.dispatch_action("upgrade", &json!({ "player": player, "unit": unit, "group": group }))
        .expect("dispatch the upgrade action");
}

#[test]
fn two_requests_in_one_tick_do_not_both_spend_the_same_gold() {
    let mut host = host();
    let armory = barracks(&mut host, 0, 92, 240);
    let hall = barracks(&mut host, 0, 100, 240);
    set_gold(&mut host, 0, FORGED_ARMOR_COST + WAR_BANNER_COST - 50);
    request(&mut host, 0, armory, "forged_armor");
    request(&mut host, 0, hall, "war_banner");
    host.advance().expect("advance one tick");
    assert_eq!(host.sim().resource_a(0), WAR_BANNER_COST - 50, "only the first request may spend");
    assert_eq!(host.sim().counter(armory, "upg:forged_armor"), 1);
    assert_eq!(host.sim().counter(hall, "upg:war_banner"), 0);
}

#[test]
fn a_request_in_the_next_tick_sees_the_settled_gold() {
    let mut host = host();
    let armory = barracks(&mut host, 0, 92, 240);
    let hall = barracks(&mut host, 0, 100, 240);
    set_gold(&mut host, 0, FORGED_ARMOR_COST + WAR_BANNER_COST);
    request(&mut host, 0, armory, "forged_armor");
    host.advance().expect("advance one tick");
    assert_eq!(host.sim().resource_a(0), WAR_BANNER_COST);
    request(&mut host, 0, hall, "war_banner");
    host.advance().expect("advance one tick");
    assert_eq!(host.sim().resource_a(0), 0);
    assert_eq!(host.sim().counter(hall, "upg:war_banner"), 1);
}
