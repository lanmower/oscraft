use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use web::Game;

const OWNER: &str = "probe-owner";

#[derive(Debug, PartialEq)]
struct Seat {
    owner: String,
    hero: String,
    ready: bool,
}

struct Unit {
    id: u32,
    player: u8,
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

fn alive_units(game: &Game) -> Vec<Unit> {
    let ids = game.unit_ids();
    let spots = game.units();
    ids.iter()
        .enumerate()
        .map(|(i, &id)| Unit {
            id,
            player: spots[i * 3 + 2] as u8,
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

fn seats_of(data: &Value) -> Vec<Seat> {
    data["slots"]
        .as_array()
        .expect("slots")
        .iter()
        .map(|slot| Seat {
            owner: slot["owner"].as_str().unwrap_or_default().to_owned(),
            hero: slot["hero"].as_str().unwrap_or_default().to_owned(),
            ready: slot["ready"] == true,
        })
        .collect()
}

fn enabled_modes(data: &Value) -> Vec<String> {
    data["modes"]
        .as_array()
        .expect("modes")
        .iter()
        .filter(|mode| mode["on"] == true)
        .map(|mode| mode["code"].as_str().expect("code").to_owned())
        .collect()
}

fn read_ui(dir: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(dir.join("moba_ui.json")).expect("read moba_ui.json")).expect("moba_ui.json")
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

fn win_widget_texts(lines: &[Value]) -> Vec<String> {
    lines
        .iter()
        .rev()
        .find(|value| value["kind"] == "widget" && value["data"]["id"] == "win" && value["data"]["remove"] != true)
        .map(|value| {
            let mut texts = Vec::new();
            label_texts(&value["data"]["tree"], &mut texts);
            texts
        })
        .unwrap_or_default()
}

fn win_action(lines: &[Value], action: &str) -> Option<Value> {
    lines
        .iter()
        .rev()
        .find(|value| value["kind"] == "win" && value["data"]["action"] == action)
        .map(|value| value["data"].clone())
}

fn start_match(dir: &Path, image: &[u8]) -> (Game, Vec<String>, Value) {
    let mut game = fresh_game(dir, image);
    let ui_file = read_ui(dir);
    call(&mut game, "setup", json!({ "ui": ui_file, "player": 1, "pubkey": OWNER }));
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
    let started = started.expect("the match starts");
    for player in 0..10usize {
        let hero = heroes[(20 + player) % heroes.len()].clone();
        call(&mut game, "seat_pick", json!({ "player": player, "hero": hero, "bot": player != 1 }));
    }
    advance(&mut game, 200);
    (game, heroes, started)
}

fn end_by_base(game: &mut Game) -> Vec<Value> {
    advance(game, 200);
    let player0: Vec<u32> = alive_units(game).iter().filter(|u| u.player == 0).map(|u| u.id).collect();
    for unit in &player0 {
        game.remove_unit(*unit);
    }
    advance(game, 30);
    let lines = drain(game);
    assert!(game.winner() >= 0, "the base kill ends the match");
    lines
}

fn assert_win_buttons(lines: &[Value]) {
    let texts = win_widget_texts(lines);
    println!("win_screen widget_texts={}", texts.join(" | "));
    assert!(texts.iter().any(|text| text == "Play Again"), "win widget offers Play Again: {texts:?}");
    assert!(texts.iter().any(|text| text == "New Game"), "win widget offers New Game: {texts:?}");
}

#[test]
fn win_screen_offers_both_actions_after_a_base_kill() {
    let dir = scenario_dir();
    let image = image_for(&dir);
    let (mut game, _heroes, _started) = start_match(&dir, &image);
    let lines = end_by_base(&mut game);
    assert_win_buttons(&lines);
}

#[test]
fn play_again_restores_roster_and_mode() {
    let dir = scenario_dir();
    let image = image_for(&dir);
    let (mut game, _heroes, started) = start_match(&dir, &image);
    let before = seats_of(&started);
    let occupied: Vec<&Seat> = before.iter().filter(|seat| !seat.owner.is_empty()).collect();
    println!("win_screen roster_before={} ready={}", occupied.len(), occupied.iter().filter(|seat| seat.ready).count());
    assert_eq!(occupied.len(), 10, "the started lobby seats ten players");
    assert!(occupied.iter().all(|seat| seat.ready), "every started seat is ready: {before:?}");

    let lines = end_by_base(&mut game);
    assert_win_buttons(&lines);
    let action = win_action(&ui(&mut game, "win_play_again", json!({})), "win_play_again").expect("play again event");
    let modes = action["modes"].as_str().expect("modes text").to_owned();
    println!("win_screen play_again modes=\"{modes}\"");
    assert_eq!(modes.split_whitespace().collect::<Vec<_>>(), vec!["em"], "the win event carries the mode");

    let mut rematch = fresh_game(&dir, &image);
    call(&mut rematch, "setup", json!({ "ui": read_ui(&dir), "player": 1, "pubkey": OWNER }));
    let mut ordered: Vec<(usize, &Seat)> = before.iter().enumerate().filter(|(_, seat)| !seat.owner.is_empty()).collect();
    ordered.sort_by_key(|(_, seat)| seat.owner != OWNER);
    let mut restored = Vec::new();
    for (slot, seat) in &ordered {
        let args = json!({
            "slot": slot,
            "seat_owner": seat.owner,
            "bot": seat.owner.starts_with("bot:"),
            "hero": seat.hero,
            "ready": seat.ready,
        });
        restored = ui(&mut rematch, "seat", args);
    }
    for code in modes.split_whitespace() {
        restored = ui(&mut rematch, "mode", json!({ "code": code }));
    }
    let after = seats_of(last_lobby(&restored).expect("lobby after the rematch"));
    println!("win_screen roster_after={} modes_after={:?}", after.iter().filter(|seat| !seat.owner.is_empty()).count(), enabled_modes(last_lobby(&restored).expect("lobby")));
    assert_eq!(after, before, "Play Again restores the roster");
    assert_eq!(enabled_modes(last_lobby(&restored).expect("lobby")), vec!["em".to_owned()], "Play Again restores the mode");
}

#[test]
fn peer_left_victory_shows_win_buttons() {
    let dir = scenario_dir();
    let image = image_for(&dir);
    let (mut game, _heroes, _started) = start_match(&dir, &image);
    call(&mut game, "peer_left", json!({ "player": 1, "pubkey": OWNER }));
    let mut lines = Vec::new();
    for _ in 0..50 {
        if game.winner() >= 0 {
            break;
        }
        let outcome = game.tick();
        assert!(outcome.is_empty(), "tick: {outcome}");
        lines.extend(drain(&mut game));
    }
    assert!(game.winner() >= 0, "the last human leaving ends the match");
    println!("win_screen peer_left decision_lines={}", lines.len());
    lines.extend(ui(&mut game, "win_refresh", json!({})));
    assert_win_buttons(&lines);
}

#[test]
fn new_game_empties_the_roster() {
    let dir = scenario_dir();
    let image = image_for(&dir);
    let (mut game, _heroes, _started) = start_match(&dir, &image);
    let lines = end_by_base(&mut game);
    assert_win_buttons(&lines);
    let closing = ui(&mut game, "win_new_game", json!({}));
    assert!(
        closing.iter().any(|value| value["kind"] == "widget" && value["data"]["id"] == "win" && value["data"]["remove"] == true),
        "New Game removes the win widget"
    );
    assert!(win_action(&closing, "win_new_game").is_some(), "New Game reports its action");

    let mut empty = fresh_game(&dir, &image);
    let setup = call(&mut empty, "setup", json!({ "ui": read_ui(&dir), "player": 1, "pubkey": OWNER }));
    let lobby = last_lobby(&setup).expect("lobby after New Game");
    let seats = seats_of(lobby);
    println!("win_screen new_game_seated={} modes_on={:?}", seats.iter().filter(|seat| !seat.owner.is_empty()).count(), enabled_modes(lobby));
    assert!(seats.iter().all(|seat| seat.owner.is_empty() && seat.hero.is_empty()), "New Game seats nobody: {seats:?}");
    assert!(enabled_modes(lobby).is_empty(), "New Game starts with no mode");
    assert!(lobby["started"] != true, "New Game shows the lobby, not a match");
}
