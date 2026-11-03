use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use web::Game;

const OWNER: &str = "probe-owner";
const LOCAL: u64 = 1;
const GROUND_AT: (f64, f64) = (620.0, 480.0);
const TEAM_SIZE: usize = 5;

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

fn fresh_game(dir: &Path, image: &[u8]) -> Game {
    let mut game = Game::new(image).expect("game");
    game.set_fog(false);
    load_pack(&mut game, dir);
    game
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
    call(game, "ui", json!({ "call": command, "args": args, "player": LOCAL, "pubkey": OWNER }))
}

fn advance(game: &mut Game, ticks: usize) {
    for _ in 0..ticks {
        let outcome = game.tick();
        assert!(outcome.is_empty(), "tick: {outcome}");
    }
}

fn lobby_error(lines: &[Value]) -> String {
    lines
        .iter()
        .find(|line| line["kind"] == "lobby")
        .and_then(|line| line["data"]["error"].as_str())
        .unwrap_or_default()
        .to_owned()
}

fn last_lobby(lines: &[Value]) -> Option<&Value> {
    lines.iter().rev().find(|line| line["kind"] == "lobby").map(|line| &line["data"])
}

fn started_lobby(lines: &[Value]) -> Option<&Value> {
    lines.iter().rev().find(|line| line["kind"] == "lobby" && line["data"]["started"] == true).map(|line| &line["data"])
}

fn allowed_heroes(lines: &[Value]) -> Vec<String> {
    last_lobby(lines)
        .map(|data| {
            data["heroes"]
                .as_array()
                .expect("heroes")
                .iter()
                .filter(|hero| hero["allowed"] == true)
                .map(|hero| hero["unit"].as_str().expect("unit").to_owned())
                .collect()
        })
        .unwrap_or_default()
}

fn read_ui(dir: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(dir.join("moba_ui.json")).expect("read moba_ui.json")).expect("moba_ui.json")
}

fn position_of(game: &Game, unit: u32) -> (f64, f64) {
    let ids = game.unit_ids();
    let spots = game.units();
    let i = ids.iter().position(|&id| id == unit).expect("the unit is alive");
    (f64::from(spots[i * 3]), f64::from(spots[i * 3 + 1]))
}

fn same_place(a: (f64, f64), b: (f64, f64)) -> bool {
    (a.0 - b.0).abs() < 0.01 && (a.1 - b.1).abs() < 0.01
}

fn start_match(dir: &Path, image: &[u8]) -> (Game, u32) {
    let mut game = fresh_game(dir, image);
    call(&mut game, "setup", json!({ "ui": read_ui(dir), "player": LOCAL, "pubkey": OWNER }));
    advance(&mut game, 60);
    ui(&mut game, "claim", json!({ "slot": 0 }));
    ui(&mut game, "mode", json!({ "code": "em" }));
    let fill = ui(&mut game, "fill", json!({}));
    let heroes = allowed_heroes(&fill);
    let mut picked = false;
    for hero in &heroes {
        if lobby_error(&ui(&mut game, "pick", json!({ "slot": 0, "hero": hero }))).is_empty() {
            picked = true;
            break;
        }
    }
    assert!(picked, "the local seat picks a hero");
    ui(&mut game, "world_ready", json!({}));
    ui(&mut game, "ready", json!({ "slot": 0, "ready": true }));
    let mut started = None;
    for step in 0..60 {
        if step % 3 == 0 {
            started = started_lobby(&ui(&mut game, "start", json!({}))).cloned();
        }
        if started.is_some() {
            break;
        }
        advance(&mut game, 100);
    }
    started.expect("the match starts");
    let mut local_hero = None;
    for player in 0..10usize {
        let hero = heroes[(20 + player) % heroes.len()].clone();
        let lines = call(&mut game, "seat_pick", json!({ "player": player, "hero": hero, "bot": player != LOCAL as usize }));
        if player == LOCAL as usize {
            local_hero = lines
                .iter()
                .rev()
                .find(|line| line["kind"] == "feedback" && line["data"]["kind"] == "selection" && line["data"]["player"] == LOCAL)
                .and_then(|line| line["data"]["ids"].as_array()?.first()?.as_u64());
        }
    }
    advance(&mut game, 200);
    let hero = local_hero.expect("the local seat's hero is selected at match start") as u32;
    (game, hero)
}

fn item_index(game: &Game, name: &str) -> u64 {
    let catalog: Value = serde_json::from_str(&game.item_catalog_json()).expect("catalog json");
    catalog
        .as_array()
        .expect("catalog rows")
        .iter()
        .find(|row| row["name"] == name)
        .and_then(|row| row["id"].as_u64())
        .expect("the catalog has the item")
}

fn slot_of_kind(game: &Game, hero: u32, kind: u64) -> u64 {
    let inventory: Value = serde_json::from_str(&game.inventory_json(hero)).expect("inventory json");
    inventory["slots"]
        .as_array()
        .expect("slots")
        .iter()
        .position(|cell| cell[0] == kind + 1)
        .expect("the item is in a slot") as u64
}

fn pings(lines: &[Value]) -> Vec<&Value> {
    lines.iter().filter(|line| line["kind"] == "ping").map(|line| &line["data"]).collect()
}

#[test]
fn alt_click_on_an_item_pings_its_state_for_usable_and_cooling_items() {
    let dir = scenario_dir();
    let image = image_for(&dir);
    let (mut game, hero) = start_match(&dir, &image);
    let cheese = item_index(&game, "cheese");
    assert!(game.give_item(hero, cheese as u32, 2), "the hero takes two cheese");
    let slot = slot_of_kind(&game, hero, cheese);
    call(&mut game, "select", json!({ "player": LOCAL, "ids": [hero] }));
    call(&mut game, "selection", json!({ "player": LOCAL, "ids": [hero] }));

    let usable = ui(&mut game, "sell", json!({ "slot": slot, "alt": true }));
    let usable_pings = pings(&usable);
    println!("ping_kinds usable pings={:?}", usable_pings);
    assert_eq!(usable_pings.len(), TEAM_SIZE, "one item ping per ally");
    assert!(usable_pings.iter().all(|p| p["kind"] == "item" && p["colour"] == "#ffd54f"), "item pings use the item kind");
    assert!(usable_pings.iter().all(|p| p["text"] == "cheese: ready"), "a usable item says ready");
    assert!(usable_pings.iter().all(|p| p["player"].as_u64().is_some_and(|id| id % 2 == LOCAL % 2)), "only allies are pinged");

    let used = call(&mut game, "pointer", json!({ "kind": "down", "x": GROUND_AT.0, "y": GROUND_AT.1, "button": 0, "player": LOCAL, "pubkey": OWNER }));
    assert!(used.iter().all(|line| line["kind"] != "ping"), "a plain down makes no ping");
    assert!(game.use_item(hero, slot as u32, -1, 0.0, 0.0), "the hero uses the cheese");
    advance(&mut game, 2);
    drain(&mut game);

    let cooling = ui(&mut game, "sell", json!({ "slot": slot, "alt": true }));
    let cooling_pings = pings(&cooling);
    println!("ping_kinds cooling pings={:?}", cooling_pings.first());
    assert_eq!(cooling_pings.len(), TEAM_SIZE, "one item ping per ally while cooling");
    let text = cooling_pings.first().and_then(|p| p["text"].as_str()).unwrap_or_default().to_owned();
    assert!(text.starts_with("cheese: ") && text.ends_with("s cooldown"), "a cooling item shows its remaining cooldown, got {text:?}");
}

#[test]
fn ground_ping_kinds_reach_allies_with_their_route() {
    let dir = scenario_dir();
    let image = image_for(&dir);
    let (mut game, hero) = start_match(&dir, &image);
    let at_hero = position_of(&game, hero);

    let expected = [("ping", "#ffffff"), ("danger", "#ff4d4d"), ("on_my_way", "#7cc7ff")];
    for (kind, colour) in expected {
        let lines = call(
            &mut game,
            "pointer",
            json!({ "kind": kind, "x": GROUND_AT.0, "y": GROUND_AT.1, "button": 0, "player": LOCAL, "pubkey": OWNER }),
        );
        let sent = pings(&lines);
        println!("ping_kinds {kind} recipients={} sample={:?}", sent.len(), sent.first());
        assert_eq!(sent.len(), TEAM_SIZE, "{kind} reaches each ally once");
        assert!(sent.iter().all(|p| p["kind"] == kind && p["colour"] == colour), "{kind} carries its kind and colour");
        assert!(sent.iter().all(|p| p["text"].is_null()), "{kind} carries no text");
        assert!(sent.iter().all(|p| p["x"].as_f64() == Some(GROUND_AT.0) && p["z"].as_f64() == Some(GROUND_AT.1)), "{kind} lands on the point");
        assert!(sent.iter().all(|p| p["player"].as_u64().is_some_and(|id| id % 2 == LOCAL % 2)), "{kind} stays with allies");
        if kind == "on_my_way" {
            let from = (
                sent[0]["from"]["x"].as_f64().expect("route start x"),
                sent[0]["from"]["z"].as_f64().expect("route start z"),
            );
            assert!(same_place(from, at_hero), "the route starts at the sender's hero {at_hero:?}, got {from:?}");
        } else {
            assert!(sent.iter().all(|p| p["from"].is_null()), "{kind} draws no route");
        }
    }
}
