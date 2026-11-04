use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use web::Game;

const OWNER: &str = "probe-owner";

struct Unit {
    id: u32,
    player: u8,
    kind: String,
}

fn scenario_dir() -> PathBuf {
    match std::env::var_os("PROBE_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../scenarios/moba"),
    }
}

fn image_for(dir: &Path) -> Vec<u8> {
    match std::env::var_os("PROBE_IMAGE") {
        Some(path) => std::fs::read(path).expect("read PROBE_IMAGE"),
        None => mod_host::build_image(dir).expect("build image"),
    }
}

fn load_pack(game: &mut Game, dir: &Path) {
    let manifest: Value = serde_json::from_slice(&std::fs::read(dir.join("manifest.json")).expect("read manifest")).expect("manifest json");
    let mut files: Vec<String> = manifest["modules"]
        .as_object()
        .expect("modules")
        .values()
        .map(|file| file.as_str().expect("module file").to_owned())
        .collect();
    files.extend(manifest["mods"].as_array().expect("mods").iter().map(|file| file.as_str().expect("mod file").to_owned()));
    let rules = std::fs::read_to_string(dir.join(manifest["rules"].as_str().expect("rules file"))).expect("read rules");
    let refused = game.set_attr_rules(&rules);
    assert!(refused.is_empty(), "rules refused: {refused}");
    files.sort();
    files.dedup();
    let mods: Vec<Value> = files
        .into_iter()
        .map(|file| {
            let text = std::fs::read_to_string(dir.join(&file)).expect("read module");
            json!({ "name": file, "text": text })
        })
        .collect();
    let refused = game.load_mods(&Value::Array(mods).to_string());
    assert!(refused.is_empty(), "load_mods: {refused}");
}

fn drain(game: &mut Game) -> Vec<Value> {
    serde_json::from_str(&game.drain_ui()).expect("drain_ui JSON array")
}

fn call(game: &mut Game, name: &str, args: Value) -> Vec<Value> {
    let refused = game.dispatch_action(name, &args.to_string());
    assert!(refused.is_empty(), "dispatch_action {name}: {refused}");
    drain(game)
}

fn ui(game: &mut Game, command: &str, args: Value) -> Vec<Value> {
    call(game, "ui", json!({ "call": command, "args": args, "player": 1, "pubkey": OWNER }))
}

fn advance(game: &mut Game, ticks: usize) {
    for _ in 0..ticks {
        let outcome = game.tick();
        assert!(outcome.is_empty(), "tick: {outcome}");
    }
}

fn alive_units(game: &Game) -> Vec<Unit> {
    let ids = game.unit_ids();
    let spots = game.units();
    let types = game.unit_types();
    let names = game.unit_type_names();
    ids.iter()
        .enumerate()
        .map(|(i, &id)| Unit {
            id,
            player: spots[i * 3 + 2] as u8,
            kind: names.get(types[i] as usize).cloned().unwrap_or_default(),
        })
        .collect()
}

fn lobby_error(lines: &[Value]) -> String {
    lines
        .iter()
        .find(|line| line["kind"] == "lobby")
        .and_then(|line| line["data"]["error"].as_str())
        .unwrap_or_default()
        .to_owned()
}

fn lobby_data(lines: &[Value]) -> Option<&Value> {
    lines.iter().rev().find(|line| line["kind"] == "lobby").map(|line| &line["data"])
}

fn lobby_heroes(lines: &[Value]) -> Vec<(String, bool)> {
    lobby_data(lines)
        .map(|data| {
            data["heroes"]
                .as_array()
                .expect("heroes")
                .iter()
                .map(|hero| (hero["unit"].as_str().expect("unit").to_owned(), hero["allowed"] == true))
                .collect()
        })
        .unwrap_or_default()
}

fn allowed_heroes(lines: &[Value]) -> Vec<String> {
    lobby_heroes(lines).into_iter().filter(|hero| hero.1).map(|hero| hero.0).collect()
}

fn widget_texts(lines: &[Value], id: &str) -> Vec<String> {
    lines
        .iter()
        .rev()
        .find(|value| value["kind"] == "widget" && value["data"]["id"] == id)
        .map(|value| {
            let mut texts = Vec::new();
            label_texts(&value["data"]["tree"], &mut texts);
            texts
        })
        .unwrap_or_default()
}

fn label_texts(node: &Value, out: &mut Vec<String>) {
    if let Some(text) = node.get("text").and_then(Value::as_str) {
        out.push(text.to_owned());
    }
    if let Some(kids) = node.get("kids").and_then(Value::as_array) {
        for kid in kids {
            label_texts(kid, out);
        }
    }
}

fn hero_of(units: &[Unit], heroes: &[String], player: u8) -> u32 {
    units
        .iter()
        .find(|unit| unit.player == player && heroes.contains(&unit.kind))
        .map(|unit| unit.id)
        .expect("hero of player")
}

fn start_match(dir: &Path, image: &[u8]) -> (Game, Vec<String>) {
    let mut game = Game::new(image).expect("game");
    game.set_fog(false);
    load_pack(&mut game, dir);
    let ui_file: Value = serde_json::from_slice(&std::fs::read(dir.join("moba_ui.json")).expect("read moba_ui.json")).expect("moba_ui.json");
    call(&mut game, "setup", json!({ "ui": ui_file, "player": 1, "pubkey": OWNER }));
    advance(&mut game, 60);
    ui(&mut game, "claim", json!({ "slot": 0 }));
    ui(&mut game, "mode", json!({ "code": "ai" }));
    ui(&mut game, "mode", json!({ "code": "ai" }));
    let fill = ui(&mut game, "fill", json!({}));
    let heroes = allowed_heroes(&fill);
    for hero in &heroes {
        let lines = ui(&mut game, "pick", json!({ "slot": 0, "hero": hero }));
        if lobby_error(&lines).is_empty() {
            break;
        }
    }
    ui(&mut game, "world_ready", json!({}));
    for seat in 0..10 {
        ui(&mut game, "ready", json!({ "slot": seat, "ready": true }));
    }
    for step in 0..60 {
        if step % 3 == 0 {
            ui(&mut game, "start", json!({}));
        }
        advance(&mut game, 100);
        if alive_units(&game).iter().any(|unit| heroes.contains(&unit.kind)) {
            break;
        }
    }
    for player in 0..10usize {
        let hero = heroes[(20 + player) % heroes.len()].clone();
        call(&mut game, "seat_pick", json!({ "player": player, "hero": hero }));
    }
    advance(&mut game, 200);
    (game, heroes)
}

fn nodes_of_kind<'a>(node: &'a Value, kind: &str, out: &mut Vec<&'a Value>) {
    if node.get("t").and_then(Value::as_str) == Some(kind) {
        out.push(node);
    }
    if let Some(kids) = node.get("kids").and_then(Value::as_array) {
        for kid in kids {
            nodes_of_kind(kid, kind, out);
        }
    }
}

fn widget_tree_of(lines: &[Value], id: &str) -> Value {
    lines
        .iter()
        .rev()
        .find(|value| value["kind"] == "widget" && value["data"]["id"] == id)
        .map(|value| value["data"]["tree"].clone())
        .unwrap_or(Value::Null)
}

fn ability_cells(lines: &[Value]) -> Vec<Value> {
    let tree = widget_tree_of(lines, "select");
    let mut cells = Vec::new();
    nodes_of_kind(&tree, "cell", &mut cells);
    cells.into_iter().filter(|cell| cell.get("unit").is_some() && cell.get("slot").is_some()).cloned().collect()
}

fn button_slot(button: &Value) -> Option<i64> {
    match &button["args"] {
        Value::String(text) => {
            let parsed: Value = serde_json::from_str(text).ok()?;
            parsed["slot"].as_i64()
        }
        other => other["slot"].as_i64(),
    }
}

fn skill_up_slots(lines: &[Value]) -> Vec<i64> {
    let tree = widget_tree_of(lines, "skills");
    let mut buttons = Vec::new();
    nodes_of_kind(&tree, "button", &mut buttons);
    buttons.into_iter().filter(|button| button["call"] == "skill_up").filter_map(button_slot).collect()
}

fn rank_of(tip: &str) -> Option<(u32, u32)> {
    let rest = tip.split("Rank ").nth(1)?;
    let mut words = rest.split_whitespace();
    let rank = words.next()?.parse().ok()?;
    if words.next()? != "of" {
        return None;
    }
    let max = words.next()?.parse().ok()?;
    Some((rank, max))
}

fn slot0_rank(lines: &[Value]) -> Option<(u32, u32)> {
    let cells = ability_cells(lines);
    let cell = cells.iter().find(|cell| cell["slot"] == 0)?;
    rank_of(cell["tip"].as_str()?)
}

#[test]
fn probe_skill_max_rank() {
    let dir = scenario_dir();
    let image = image_for(&dir);
    let (mut game, heroes) = start_match(&dir, &image);
    let units = alive_units(&game);
    let hero = hero_of(&units, &heroes, 1);
    let hero_kind = units.iter().find(|unit| unit.id == hero).map(|unit| unit.kind.clone()).unwrap_or_default();
    println!("probe max_rank hero={hero_kind}");

    let mut lines = call(&mut game, "select", json!({ "ids": [hero], "player": 1, "pubkey": OWNER }));
    lines.extend(call(&mut game, "selection", json!({ "ids": [hero], "player": 1, "pubkey": OWNER })));
    advance(&mut game, 8);
    lines.extend(drain(&mut game));
    let with_point = skill_up_slots(&lines);
    println!("probe max_rank level1 with_point skill_up_slots={with_point:?}");
    assert!(with_point.contains(&0), "level one with a point: slot 0 offers an upgrade");

    let mut current = call(&mut game, "skill_up", json!({ "slot": 0, "player": 1, "pubkey": OWNER }));
    advance(&mut game, 8);
    current.extend(drain(&mut game));
    let no_point = skill_up_slots(&current);
    println!("probe max_rank level1 no_point skill_up_slots={no_point:?} slot0_rank={:?}", slot0_rank(&current));
    assert!(no_point.is_empty(), "no point left: no upgrade button shows");

    let granted = game.xp_curve().iter().fold(0u32, |sum, step| sum.saturating_add(*step));
    game.grant_xp_for_test(hero, granted);
    advance(&mut game, 8);
    current = drain(&mut game);
    let level_line = widget_texts(&current, "skills").into_iter().find(|text| text.starts_with("Level ")).unwrap_or_default();
    println!("probe max_rank granted_xp={granted} skills_line=\"{level_line}\"");

    let mut reached = None;
    for _ in 0..8 {
        let (rank, max) = slot0_rank(&current).expect("slot 0 shows its rank once learned");
        let slots = skill_up_slots(&current);
        println!("probe max_rank slot0 rank={rank} of={max} skill_up_slot0={} skill_up_other={}", slots.contains(&0), slots.iter().filter(|slot| **slot != 0).count());
        if rank >= max {
            reached = Some((rank, max, slots));
            break;
        }
        assert!(slots.contains(&0), "a skill below its max rank with a point shows its upgrade button");
        let mut batch = call(&mut game, "skill_up", json!({ "slot": 0, "player": 1, "pubkey": OWNER }));
        advance(&mut game, 8);
        batch.extend(drain(&mut game));
        current = batch;
    }
    let (rank, max, slots) = reached.expect("slot 0 reaches its maximum rank");
    assert!(!slots.contains(&0), "a skill at its max rank shows no upgrade button");
    assert!(slots.iter().any(|slot| *slot != 0), "other skills still offer upgrades while points remain");
    println!("probe max_rank final slot0 rank={rank} of={max} skill_up_slot0=false skill_up_other={} skills_line=\"{level_line}\"", slots.len());
}
