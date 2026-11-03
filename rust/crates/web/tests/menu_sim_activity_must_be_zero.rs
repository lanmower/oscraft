use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use web::Game;

const OWNER: &str = "probe-owner";
const THIRTY_SECONDS_OF_FRAMES: usize = 1875;
const FRAME_MS: u32 = 16;
const DRAFT_CLOCK_TICKS: u32 = 16;
const START_ATTEMPTS: usize = 400;
const FIRST_WAVE_TICK_LIMIT: usize = 4;

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
    call(game, "ui", json!({ "call": command, "args": args, "player": 1, "pubkey": OWNER }))
}

fn advance(game: &mut Game, ticks: usize) {
    for _ in 0..ticks {
        let outcome = game.tick();
        assert!(outcome.is_empty(), "tick: {outcome}");
    }
}

fn unit_set(game: &Game) -> Vec<(u32, u8)> {
    let ids = game.unit_ids();
    let spots = game.units();
    let mut units: Vec<(u32, u8)> = ids.iter().enumerate().map(|(i, &id)| (id, spots[i * 3 + 2] as u8)).collect();
    units.sort();
    units
}

fn lobby_error(lines: &[Value]) -> String {
    lines
        .iter()
        .rev()
        .find(|line| line["kind"] == "lobby")
        .and_then(|line| line["data"]["error"].as_str())
        .unwrap_or_default()
        .to_owned()
}

fn lobby_widget(lines: &[Value]) -> Option<&Value> {
    lines.iter().rev().find(|line| line["kind"] == "widget" && line["data"]["id"] == "lobby")
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

fn lobby_widget_texts(lines: &[Value]) -> Vec<String> {
    lobby_widget(lines)
        .map(|widget| {
            let mut texts = Vec::new();
            label_texts(&widget["data"]["tree"], &mut texts);
            texts
        })
        .unwrap_or_default()
}

fn started_lobby(lines: &[Value]) -> Option<&Value> {
    lines.iter().rev().find(|line| line["kind"] == "lobby" && line["data"]["started"] == true).map(|line| &line["data"])
}

fn allowed_heroes(lines: &[Value]) -> Vec<String> {
    lines
        .iter()
        .rev()
        .find(|line| line["kind"] == "lobby")
        .map(|line| {
            line["data"]["heroes"]
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

fn lobby_with_local_seat(dir: &Path, image: &[u8], announce_world: bool) -> (Game, Vec<String>, Vec<(u32, u8)>) {
    let mut game = fresh_game(dir, image);
    let setup = call(&mut game, "setup", json!({ "ui": read_ui(dir), "player": 1, "pubkey": OWNER }));
    assert!(lobby_widget(&setup).is_some_and(|w| w["data"]["remove"] != true), "setup mounts the lobby widget");
    let baseline = unit_set(&game);

    let claimed = ui(&mut game, "claim", json!({ "slot": 0 }));
    assert!(lobby_widget(&claimed).is_some(), "a lobby call refreshes the lobby widget");
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
    if announce_world {
        ui(&mut game, "world_ready", json!({}));
    }
    ui(&mut game, "ready", json!({ "slot": 0, "ready": true }));
    (game, heroes, baseline)
}

fn press_start_until_started(game: &mut Game) -> Vec<Value> {
    for _ in 0..START_ATTEMPTS {
        let lines = ui(game, "start", json!({}));
        if started_lobby(&lines).is_some() {
            return lines;
        }
        ui(game, "draft_clock", json!({ "ticks": DRAFT_CLOCK_TICKS }));
    }
    panic!("Start never began the match");
}

#[test]
fn menu_sim_stays_idle_for_thirty_seconds_before_start() {
    let dir = scenario_dir();
    let image = image_for(&dir);
    let (mut game, _heroes, baseline) = lobby_with_local_seat(&dir, &image, true);
    assert_eq!(game.ticks(), 0, "the lobby actions do not tick the sim");

    for _ in 0..THIRTY_SECONDS_OF_FRAMES {
        let refused = game.frame(FRAME_MS);
        assert!(refused.is_empty(), "frame: {refused}");
    }

    assert_eq!(game.ticks(), 0, "no tick in the menu over 30 s of frames");
    assert_eq!(unit_set(&game), baseline, "no unit or wave spawns in the menu");
    println!("menu_sim ticks={} units={} frames={}", game.ticks(), baseline.len(), THIRTY_SECONDS_OF_FRAMES);
}

#[test]
fn start_leaves_the_menu_then_the_first_ticks_spawn_waves() {
    let dir = scenario_dir();
    let image = image_for(&dir);
    let (mut game, heroes, baseline) = lobby_with_local_seat(&dir, &image, true);

    let started = press_start_until_started(&mut game);
    assert_eq!(game.ticks(), 0, "Start is answered without a sim tick");
    assert_eq!(unit_set(&game), baseline, "Start spawns no unit before the first tick");
    assert_eq!(lobby_widget(&started).map(|w| w["data"]["remove"] == true), Some(true), "Start removes the lobby widget");

    for player in 0..10usize {
        let hero = heroes[(20 + player) % heroes.len()].clone();
        call(&mut game, "seat_pick", json!({ "player": player, "hero": hero, "bot": player != 1 }));
    }
    let with_heroes = unit_set(&game);
    assert_eq!(game.ticks(), 0, "seating the heroes does not tick");

    let mut wave_units = Vec::new();
    for _ in 0..FIRST_WAVE_TICK_LIMIT {
        advance(&mut game, 1);
        wave_units = unit_set(&game).into_iter().filter(|unit| !with_heroes.contains(unit) && (unit.1 == 0 || unit.1 == 1)).collect();
        if !wave_units.is_empty() {
            break;
        }
    }
    assert!(!wave_units.is_empty(), "the first ticks after Start spawn wave units");
    println!("start_sim first_wave_tick={} wave_units={} baseline_units={}", game.ticks(), wave_units.len(), baseline.len());
}

#[test]
fn start_is_refused_until_world_ready_then_begins() {
    let dir = scenario_dir();
    let image = image_for(&dir);
    let (mut game, _heroes, _baseline) = lobby_with_local_seat(&dir, &image, false);

    let expected = read_ui(&dir)["text"]["lobby"]["refusals"]["world-loading"].as_str().expect("world-loading refusal text").to_owned();
    let refused = ui(&mut game, "start", json!({}));
    let texts = lobby_widget_texts(&refused);
    assert!(texts.iter().any(|text| text.contains(expected.as_str())), "Start is refused with the world-loading text: {texts:?}");
    assert!(started_lobby(&refused).is_none(), "a refused Start does not begin the match");
    assert_eq!(game.ticks(), 0, "the refused Start does not tick");

    ui(&mut game, "world_ready", json!({}));
    let started = press_start_until_started(&mut game);
    assert!(started_lobby(&started).is_some(), "Start begins once world_ready arrives");
    println!("start_gate refused_with=\"{expected}\" after_world_ready=started");
}
