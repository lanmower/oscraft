use std::path::{Path, PathBuf};

use mod_host::{build_image, ModHost};
use serde_json::{json, Value};
use sim::fixed::{self, HALF};

const BUFF_KIND: i32 = 5;
const BUFF_TICKS: i32 = 100_000;
const MATCH_TICKS: u32 = 3 * 60 * 16 + 32;

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
    let manifest: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("manifest.json")).expect("the pack manifest"))
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

fn find_lines(value: &Value, out: &mut Vec<Value>) {
    match value {
        Value::Object(map) => {
            if map.get("t") == Some(&json!("line")) {
                out.push(value.clone());
            }
            for child in map.values() {
                find_lines(child, out);
            }
        }
        Value::Array(items) => {
            for child in items {
                find_lines(child, out);
            }
        }
        _ => {}
    }
}

fn series_values(line: &Value) -> Vec<(String, Vec<i64>)> {
    line["series"]
        .as_array()
        .expect("a line node has series")
        .iter()
        .map(|s| {
            let values = s["values"].as_array().expect("a series has values").iter().map(|v| v.as_i64().expect("an integer sample")).collect();
            (s["label"].as_str().expect("a series label").to_string(), values)
        })
        .collect()
}

#[test]
fn post_match_graphs_sample_gold_and_buffs_per_minute() {
    let mut host = pack_host();
    host.dispatch_action("setup", &json!({ "ui": serde_json::from_str::<Value>(&std::fs::read_to_string(pack_dir().join("moba_ui.json")).expect("the moba ui file")).expect("a valid moba ui file"), "player": 0, "pubkey": "owner" }))
        .expect("the panels accept the setup");
    let buffed = spawn_hero(&mut host, "antimage", 0, 92, 240);
    spawn_hero(&mut host, "antimage", 5, 120, 240);
    assert!(host.sim_mut().apply_buff(buffed, BUFF_KIND, BUFF_TICKS), "the buff fits on the hero");
    for _ in 0..MATCH_TICKS {
        host.advance().expect("advance the match");
    }
    let _ = host.take_ui();
    for player in [0, 2, 4, 6, 8] {
        host.dispatch_action("peer_left", &json!({ "player": player })).expect("the leave action is accepted");
    }
    for _ in 0..10 {
        host.advance().expect("advance after the forfeit");
    }
    assert_eq!(host.sim().winner(), Some(1), "the forfeit of every seat on side 0 hands the match to side 1");
    host.dispatch_action("ui", &json!({ "call": "post_tab", "args": { "tab": "graphs" }, "player": 0 })).expect("the graphs tab is accepted");
    host.advance().expect("advance after the tab change");
    let mut found = Vec::new();
    for line in host.take_ui() {
        let text = if line.starts_with('{') { line.clone() } else { format!("{{{line}}}") };
        if let Ok(value) = serde_json::from_str::<Value>(&text) {
            find_lines(&value, &mut found);
        }
    }
    assert!(found.len() >= 4, "the graphs tab draws one line per metric, got {}", found.len());
    let buffs = series_values(&found[1]);
    assert_eq!(buffs.len(), 2, "one series per hero");
    assert_eq!(buffs[0].1.len(), 4, "one sample per full minute of the 3:02 match");
    assert!(buffs[0].1.iter().all(|&n| n == 1), "the buffed hero shows one active buff each minute: {:?}", buffs[0].1);
    assert!(buffs[1].1.iter().all(|&n| n == 0), "the unbuffed hero shows no buffs: {:?}", buffs[1].1);
    let gold = series_values(&found[0]);
    assert!(gold[0].1.windows(2).all(|w| w[0] <= w[1]), "cumulative gold never falls: {:?}", gold[0].1);
}
