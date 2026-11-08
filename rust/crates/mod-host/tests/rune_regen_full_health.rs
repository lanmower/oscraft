use std::path::{Path, PathBuf};

use mod_host::{build_image, ModHost};
use serde_json::json;
use sim::fixed::{self, HALF};

const S_HEALTH_REGEN: usize = 14;

const PROBE: &str = r#"
local runes = require("runes")
local probe = {}
probe.on_tick = function(state) runes.tick(game.time()) end
probe.on_action = function(name, args)
  if name == "probe_rune_regen" then runes.apply(args.unit, 4, game.time()) end
end
return probe
"#;
const RUNE_WARMUP_TICKS: u32 = 320;
const SETTLE_TICKS: u32 = 20;
const AFTER_FULL_TICKS: u32 = 32;

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
    visit(&files, "runes", &mut mods);
    mods.push(("rune_regen_probe.lua".to_owned(), PROBE.to_owned()));
    let image = build_image(&dir).expect("build the moba image");
    let mut host = ModHost::new(&image).expect("load the moba image");
    assert!(host.sim_mut().set_attr_rules_text(&rules), "the pack rules are accepted");
    host.load_mods(&mods).expect("load the rune and the probe");
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

fn advance(host: &mut ModHost, ticks: u32) {
    for _ in 0..ticks {
        host.advance().expect("advance the match");
    }
}

#[test]
fn a_regeneration_rune_stops_once_health_and_mana_are_full() {
    let mut host = probe_host();
    let hero = spawn_hero(&mut host, "antimage", 0, 92, 240);
    let (max_hp, max_mana, base_regen) = {
        let sim = host.sim();
        (sim.hp_max(hero), sim.unit_max_mana(hero), sim.stat(hero, S_HEALTH_REGEN))
    };
    {
        let mut sim = host.sim_mut();
        assert!(sim.set_hp_within(hero, max_hp / 2), "the hero takes a health setting");
        assert!(sim.set_mana_within(hero, max_mana / 2), "the hero takes a mana setting");
    }
    host.dispatch_action("probe_rune_regen", &json!({ "unit": hero, "player": 0 }))
        .expect("the probe applies the regeneration rune");
    advance(&mut host, RUNE_WARMUP_TICKS);
    assert_eq!(host.sim().hp(hero), max_hp, "the rune refills health");
    assert_eq!(host.sim().mana_of(hero), max_mana, "the rune refills mana");
    advance(&mut host, SETTLE_TICKS);
    assert_eq!(host.sim().stat(hero, S_HEALTH_REGEN), base_regen, "the rune's health regeneration is taken back at full");

    let half_mana = max_mana / 2;
    assert!(host.sim_mut().set_mana_within(hero, half_mana), "the hero spends mana");
    advance(&mut host, AFTER_FULL_TICKS);
    assert_eq!(host.sim().mana_of(hero), half_mana, "a regeneration rune that ended at full mana does not refill the spent mana");
}
