use std::path::{Path, PathBuf};

use mod_host::{build_image, ModHost};
use sim::fixed::{self, HALF};

const NEUTRAL_SLOT: usize = 9;

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

fn spawn_hero(host: &mut ModHost, kind_name: &str, player: u8, cell_x: i32, cell_y: i32) -> usize {
    let mut sim = host.sim_mut();
    let kind = sim.catalog().unit_index(kind_name).expect("the hero kind is in the moba catalog");
    let x = fixed::from_int(cell_x) + HALF;
    let y = fixed::from_int(cell_y) + HALF;
    sim.spawn_attributed(player, kind as u32, x, y).expect("the hero fits on the map")
}

#[test]
fn a_neutral_item_in_the_neutral_slot_moves_to_the_stash() {
    let mut host = pack_host();
    let hero = spawn_hero(&mut host, "antimage", 0, 92, 240);
    let kind = host.sim().catalog().item_index("essence_ring").expect("the neutral item is in the moba catalog") as u32 + 1;
    assert!(host.sim_mut().inventory_add(hero, kind, 1), "the neutral item fits the neutral slot");
    assert_eq!(host.sim().inventory_slot(hero, NEUTRAL_SLOT), (kind, 1));
    assert!(host.sim_mut().stash_item(hero, NEUTRAL_SLOT), "a neutral item held in the neutral slot is stashed");
    assert_eq!(host.sim().inventory_slot(hero, NEUTRAL_SLOT), (0, 0), "the neutral slot is empty after the stash");
    let stashed = (0..NEUTRAL_SLOT).any(|s| host.sim().stash_slot(hero, s) == (kind, 1));
    assert!(stashed, "the neutral item is in a home stash slot");
}
