use std::path::{Path, PathBuf};

use mod_host::{build_image, ModHost};
use sim::fixed::{self, HALF};

const PLAYER: u8 = 0;
const HERO: &str = "antimage";
const COURIER: &str = "courier";
const GROUND: &str = "sentinel_creep";
const ORDER_STOP: u8 = 1;
const ORDER_FOLLOW: u8 = 5;
const HERO_CELL: (i32, i32) = (88, 232);
const COURIER_CELL: (i32, i32) = (92, 232);
const CROWD_ORIGIN: (i32, i32) = (94, 222);
const CROWD_COLUMNS: i32 = 8;
const CROWD_ROWS: i32 = 4;
const CROWD_MIN: usize = 9;
const SETTLE_TICKS: u32 = 120;
const REGROW_TICK: u32 = 300;
const REGROW_ROUNDS: u32 = 6;
const REGROW_KEYS: u32 = 300;
const REGROW_MOD: &str = "courier_regrow.lua";

fn pack_dir() -> PathBuf {
    match std::env::var_os("PROBE_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../scenarios/moba"),
    }
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

fn pack_host_with(extra: Vec<(String, String)>) -> ModHost {
    let dir = pack_dir();
    let manifest: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("manifest.json")).expect("the pack manifest"))
        .expect("a valid pack manifest");
    let rules = std::fs::read_to_string(dir.join(manifest["rules"].as_str().expect("the manifest rules file")))
        .expect("the pack rules file");
    let mut files = Vec::new();
    collect(&dir, "", &mut files);
    files.sort();
    let mut mods = Vec::new();
    let modules = manifest["modules"].as_object().expect("the manifest modules map").values();
    let roots = modules.map(|file| file.as_str().expect("a module file name"));
    let mods_list = manifest["mods"].as_array().expect("the manifest mods list").iter();
    for root in roots.chain(mods_list.map(|file| file.as_str().expect("a mod file name"))) {
        visit(&files, stem(root), &mut mods);
    }
    mods.extend(extra);
    let image = build_image(&dir).expect("build the moba image");
    let mut host = ModHost::new(&image).expect("load the moba image");
    assert!(host.sim_mut().set_attr_rules_text(&rules), "the pack rules are accepted");
    host.load_mods(&mods).expect("load the moba mods");
    host.sim_mut().set_fog(false);
    host
}

fn try_spawn(host: &mut ModHost, player: u8, kind: &str, hero: bool, cell: (i32, i32)) -> Option<usize> {
    let mut sim = host.sim_mut();
    let index = sim.catalog().unit_index(kind).expect("the unit kind is in the moba catalog") as u32;
    let (x, y) = (fixed::from_int(cell.0) + HALF, fixed::from_int(cell.1) + HALF);
    if hero { sim.spawn_attributed(player, index, x, y) } else { sim.spawn_for(player, index, x, y) }
}

fn spawn(host: &mut ModHost, player: u8, kind: &str, hero: bool, cell: (i32, i32)) -> usize {
    try_spawn(host, player, kind, hero, cell).expect("the unit fits on the map")
}

fn advance(host: &mut ModHost, ticks: u32) {
    for _ in 0..ticks {
        host.advance().expect("advance the match");
    }
}

fn order_kind(host: &ModHost, unit: usize) -> u8 {
    host.sim().order_of(unit).0
}

fn stop(host: &mut ModHost, unit: usize) {
    host.sim_mut().issue_order_kind(unit, ORDER_STOP, 0, 0).expect("the stop order is accepted");
}

fn regrow_mod() -> String {
    format!(
        r#"
local courier = require("courier")
local regrown = false
return {{
    on_tick = function(state)
        if regrown or game.ticks() < {REGROW_TICK} then return end
        regrown = true
        for round = 1, {REGROW_ROUNDS} do
            for _, id in ipairs(game.units(nil)) do
                courier.stalled(id)
            end
            for i = 1, {REGROW_KEYS} do
                courier.errand[round * 1000 + i] = true
            end
            for i = 1, {REGROW_KEYS} do
                courier.errand[round * 1000 + i] = false
            end
        end
        for i = 1, {REGROW_KEYS} do
            if courier.errand[{REGROW_ROUNDS} * 1000 + i] ~= false then
                error("a cleared errand key is not false")
            end
        end
    end,
}}
"#
    )
}

#[test]
fn a_courier_still_follows_its_hero_after_its_tables_are_cleared_and_regrown() {
    let mut host = pack_host_with(vec![(REGROW_MOD.to_owned(), regrow_mod())]);
    spawn(&mut host, PLAYER, HERO, true, HERO_CELL);
    let courier = spawn(&mut host, PLAYER, COURIER, false, COURIER_CELL);
    let mut crowd = 0;
    for column in 0..CROWD_COLUMNS {
        for row in 0..CROWD_ROWS {
            if try_spawn(&mut host, PLAYER, GROUND, false, (CROWD_ORIGIN.0 + 2 * column, CROWD_ORIGIN.1 + 2 * row)).is_some() {
                crowd += 1;
            }
        }
    }
    assert!(crowd >= CROWD_MIN, "the crowd places at least {CROWD_MIN} units so the courier tables grow (placed {crowd})");

    advance(&mut host, SETTLE_TICKS);
    stop(&mut host, courier);
    advance(&mut host, SETTLE_TICKS);
    let follows_before = order_kind(&host, courier) == ORDER_FOLLOW;

    advance(&mut host, REGROW_TICK - 2 * SETTLE_TICKS + 1);
    stop(&mut host, courier);
    advance(&mut host, SETTLE_TICKS);
    let follows_after = order_kind(&host, courier) == ORDER_FOLLOW;
    let hook_errors = host.hook_errors().to_vec();
    println!(
        "probe courier_regrow crowd={crowd} follows_before={follows_before} follows_after={follows_after} hook_errors={} errors={hook_errors:?} order_after={}",
        hook_errors.len(),
        order_kind(&host, courier)
    );
    assert!(follows_before, "the courier follows its hero before the regrow");
    assert!(follows_after, "the courier follows its hero after its tables are cleared and regrown");
    assert!(hook_errors.is_empty(), "no mod hook failed during the regrow: {hook_errors:?}");
}
