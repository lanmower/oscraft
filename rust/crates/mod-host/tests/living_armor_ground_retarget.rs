use std::path::{Path, PathBuf};

use mod_host::{build_image, ModHost};
use sim::fixed::{self, HALF};

const TREANT: &str = "treant";
const LIVING_ARMOR: &str = "treant_living_armor";
const TOWER: &str = "tower";
const FOUNTAIN: &str = "fountain";
const CASTER_PLAYER: u8 = 0;
const CASTER_CELL: (i32, i32) = (92, 240);
const TOWER_CELL: (i32, i32) = (100, 240);
const FOUNTAIN_CELL: (i32, i32) = (100, 232);
const NEAR_TOWER_CELL: (i32, i32) = (96, 236);
const HEALTH_REGEN: usize = 14;
const SETTLE_TICKS: u32 = 4;

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

fn spawn_kind(host: &mut ModHost, kind_name: &str, player: u8, cell: (i32, i32)) -> usize {
    let mut sim = host.sim_mut();
    let kind = sim.catalog().unit_index(kind_name).expect("the kind is in the moba catalog");
    let x = fixed::from_int(cell.0) + HALF;
    let y = fixed::from_int(cell.1) + HALF;
    sim.spawn_for(player, kind as u32, x, y).expect("the unit fits on the map")
}

fn treant(host: &mut ModHost, cell: (i32, i32)) -> usize {
    let mut sim = host.sim_mut();
    let kind = sim.catalog().unit_index(TREANT).expect("the treant kind is in the moba catalog");
    let x = fixed::from_int(cell.0) + HALF;
    let y = fixed::from_int(cell.1) + HALF;
    sim.spawn_attributed(CASTER_PLAYER, kind as u32, x, y).expect("the treant fits on the map")
}

fn learn_living_armor(host: &mut ModHost, caster: usize) {
    let mut sim = host.sim_mut();
    assert!(sim.give_ability(caster, 0, LIVING_ARMOR), "Living Armor is granted");
    assert!(sim.rank_up(caster, 0).is_ok(), "Living Armor takes its first rank");
    let max = sim.unit_max_mana(caster);
    assert!(sim.set_mana_within(caster, max), "the treant has the mana to cast");
}

fn settle(host: &mut ModHost) {
    for _ in 0..SETTLE_TICKS {
        host.advance().expect("advance the match");
    }
}

fn cast_living_armor_at(host: &mut ModHost, caster: usize, point: usize) {
    let (x, y) = host.sim().position_fixed(point);
    let direct = host.cast_allowed(caster, 0, -1, x, y, false);
    if direct {
        assert!(host.sim_mut().cast_ability(caster, 0, -1, x, y), "the treant casts Living Armor on the ground");
    }
    settle(host);
}

fn regen(host: &ModHost, unit: usize) -> i32 {
    host.sim().stat(unit, HEALTH_REGEN)
}

#[test]
fn living_armor_cast_on_the_ground_goes_to_the_allied_tower_in_the_xz_list() {
    let mut host = pack_host();
    let caster = treant(&mut host, CASTER_CELL);
    let tower = spawn_kind(&mut host, TOWER, CASTER_PLAYER, TOWER_CELL);
    learn_living_armor(&mut host, caster);
    settle(&mut host);
    let tower_before = regen(&host, tower);
    let caster_before = regen(&host, caster);
    cast_living_armor_at(&mut host, caster, tower);
    assert!(
        regen(&host, tower) > tower_before,
        "6.88g ALE (war3map.j 54120-54200) gives Living Armor to the nearest allied structure in the xZ list, a tower"
    );
    assert_eq!(regen(&host, caster), caster_before, "the nearer tower takes the armour, not the caster");
}

#[test]
fn living_armor_cast_on_the_ground_skips_an_allied_structure_outside_the_xz_list() {
    let mut host = pack_host();
    let caster = treant(&mut host, CASTER_CELL);
    let fountain = spawn_kind(&mut host, FOUNTAIN, CASTER_PLAYER, FOUNTAIN_CELL);
    let tower = spawn_kind(&mut host, TOWER, CASTER_PLAYER, NEAR_TOWER_CELL);
    learn_living_armor(&mut host, caster);
    settle(&mut host);
    let fountain_before = regen(&host, fountain);
    let tower_before = regen(&host, tower);
    cast_living_armor_at(&mut host, caster, fountain);
    assert_eq!(
        regen(&host, fountain),
        fountain_before,
        "6.88g ALE takes only xZ structures (towers and barracks), so a fountain is not a candidate"
    );
    assert!(
        regen(&host, tower) > tower_before,
        "the nearest xZ structure to the cast point is the tower, which takes the armour"
    );
}

