use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use web::Game;

const OWNER: &str = "probe-owner";
const LOCAL: u64 = 1;
const RESPAWN_TICK_CAP: usize = 6000;

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

fn selection_of(lines: &[Value]) -> Option<Vec<u64>> {
    lines
        .iter()
        .rev()
        .find(|line| line["kind"] == "feedback" && line["data"]["kind"] == "selection" && line["data"]["player"] == LOCAL)
        .map(|line| line["data"]["ids"].as_array().expect("selection ids").iter().map(|id| id.as_u64().expect("unit id")).collect())
}

fn refusal_of(lines: &[Value]) -> Option<String> {
    lines
        .iter()
        .rev()
        .find(|line| line["kind"] == "feedback" && line["data"]["kind"] == "refusal" && line["data"]["player"] == LOCAL)
        .and_then(|line| line["data"]["reason"].as_str().map(str::to_owned))
}

fn camera_of(lines: &[Value]) -> Option<(f64, f64)> {
    lines
        .iter()
        .rev()
        .find(|line| line["kind"] == "camera" && line["player"] == LOCAL)
        .map(|line| (line["x"].as_f64().expect("camera x"), line["y"].as_f64().expect("camera y")))
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
            local_hero = selection_of(&lines).and_then(|ids| ids.first().copied());
        }
    }
    advance(&mut game, 200);
    let hero = local_hero.expect("the local seat's hero is selected at match start") as u32;
    (game, hero)
}

fn press(game: &mut Game, key: &str) -> Vec<Value> {
    call(game, "key", json!({ "key": key, "phase": "down", "player": LOCAL, "pubkey": OWNER }))
}

fn kill_hero(game: &mut Game, hero: u32) -> Vec<Value> {
    let mut lines = Vec::new();
    for _ in 0..200 {
        if !game.unit_ids().contains(&hero) {
            return lines;
        }
        game.hurt(hero, 1000);
        advance(game, 1);
        lines.extend(drain(game));
    }
    panic!("the local hero dies");
}

fn advance_until_alive(game: &mut Game, hero: u32) -> Vec<Value> {
    let mut lines = Vec::new();
    for _ in 0..RESPAWN_TICK_CAP {
        advance(game, 1);
        lines.extend(drain(game));
        if game.unit_ids().contains(&hero) {
            return lines;
        }
    }
    panic!("the local hero respawns within {RESPAWN_TICK_CAP} ticks");
}

#[test]
fn hotkey_one_selects_and_centres_the_local_hero() {
    let dir = scenario_dir();
    let image = image_for(&dir);
    let (mut game, hero) = start_match(&dir, &image);
    call(&mut game, "select", json!({ "player": LOCAL, "ids": [] }));

    let first = press(&mut game, "1");
    let at_hero = position_of(&game, hero);
    let camera = camera_of(&first).expect("the first 1 centres the camera");
    println!("hero_hotkey first_press selection={:?} camera={camera:?} hero_at={at_hero:?}", selection_of(&first));
    assert_eq!(selection_of(&first), Some(vec![u64::from(hero)]), "the first 1 selects the local hero");
    assert!(same_place(camera, at_hero), "the camera {camera:?} is on the hero at {at_hero:?}");

    let second = press(&mut game, "1");
    println!("hero_hotkey second_press selection={:?}", selection_of(&second));
    assert_eq!(selection_of(&second), Some(vec![u64::from(hero)]), "a second 1 keeps the selection");
    assert!(same_place(camera_of(&second).expect("the second 1 centres the camera"), at_hero));
}

#[test]
fn hotkey_one_is_quiet_when_the_hero_is_dead_or_absent() {
    let dir = scenario_dir();
    let image = image_for(&dir);
    let (mut game, hero) = start_match(&dir, &image);
    call(&mut game, "select", json!({ "player": LOCAL, "ids": [] }));

    kill_hero(&mut game, hero);
    let dead = press(&mut game, "1");
    println!("hero_hotkey dead_press refusal={:?} selection={:?}", refusal_of(&dead), selection_of(&dead));
    assert_eq!(refusal_of(&dead).as_deref(), Some("noHero"), "a dead hero refuses with noHero");
    assert_eq!(selection_of(&dead), None, "a dead hero leaves the selection alone");

    game.remove_unit(hero);
    let absent = press(&mut game, "1");
    println!("hero_hotkey absent_press refusal={:?} selection={:?}", refusal_of(&absent), selection_of(&absent));
    assert_eq!(refusal_of(&absent).as_deref(), Some("noHero"), "an absent hero refuses with noHero");
    advance(&mut game, 20);
}

#[test]
fn respawned_local_hero_is_selected_and_centred() {
    let dir = scenario_dir();
    let image = image_for(&dir);
    let (mut game, hero) = start_match(&dir, &image);
    call(&mut game, "select", json!({ "player": LOCAL, "ids": [] }));

    kill_hero(&mut game, hero);
    let respawn = advance_until_alive(&mut game, hero);
    let at_hero = position_of(&game, hero);
    let camera = camera_of(&respawn).expect("the respawn centres the local camera");
    println!("hero_hotkey respawn selection={:?} camera={camera:?} hero_at={at_hero:?}", selection_of(&respawn));
    assert_eq!(selection_of(&respawn), Some(vec![u64::from(hero)]), "the respawned local hero is selected");
    assert!(same_place(camera, at_hero), "the camera {camera:?} is on the respawned hero at {at_hero:?}");
    assert!(
        respawn.iter().all(|line| line["kind"] != "camera" || line["player"] == LOCAL),
        "only the local seat's camera moves on respawn"
    );
}
