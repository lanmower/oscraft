use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use web::Game;

const HOST: &str = "probe-host";
const JOINER: &str = "probe-joiner";
const JOINER_AGAIN: &str = "probe-joiner-refreshed";
const SEAT: usize = 5;

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

fn fresh_game(dir: &Path) -> Game {
    let image = image_for(dir);
    let mut game = Game::new(&image).expect("game");
    game.set_fog(false);
    load_pack(&mut game, dir);
    game
}

fn drain(game: &mut Game) -> Vec<Value> {
    serde_json::from_str(&game.drain_ui()).expect("drain_ui JSON array")
}

fn call(game: &mut Game, name: &str, args: Value) -> Vec<Value> {
    let refused = game.dispatch_action(name, &args.to_string());
    assert!(refused.is_empty(), "dispatch_action {name}: {refused}");
    drain(game)
}

fn ui_as(game: &mut Game, pubkey: &str, command: &str, args: Value) -> Vec<Value> {
    call(game, "ui", json!({ "call": command, "args": args, "player": 1, "pubkey": pubkey }))
}

fn advance(game: &mut Game, ticks: usize) {
    for _ in 0..ticks {
        let outcome = game.tick();
        assert!(outcome.is_empty(), "tick: {outcome}");
    }
}

fn read_ui(dir: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(dir.join("moba_ui.json")).expect("read moba_ui.json")).expect("moba_ui.json")
}

fn start_host(game: &mut Game, ui_file: &Value) {
    call(game, "setup", json!({ "ui": ui_file.clone(), "player": 1, "pubkey": HOST }));
    advance(game, 60);
    call(game, "lobby_host", json!({ "pubkey": HOST }));
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

fn lobby_events(lines: &[Value]) -> usize {
    lines.iter().filter(|line| line["kind"] == "lobby" && line["data"]["started"] == false).count()
}

fn modal_widgets(lines: &[Value]) -> usize {
    lines.iter().filter(|line| line["kind"] == "widget" && line["data"]["id"] == "lobby" && line["data"]["remove"] != true).count()
}

fn seat_owners(lines: &[Value]) -> Vec<String> {
    lobby_data(lines)
        .map(|data| {
            data["slots"]
                .as_array()
                .expect("slots")
                .iter()
                .map(|slot| slot["owner"].as_str().unwrap_or_default().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

fn negotiated(lines: &[Value]) -> bool {
    lobby_data(lines).map(|data| data["negotiated"] == true).unwrap_or(false)
}

fn seat_owner(lines: &[Value], seat: usize) -> String {
    seat_owners(lines).get(seat).cloned().unwrap_or_default()
}

fn check(label: &str, lines: &[Value], problems: &mut Vec<String>) {
    let error = lobby_error(lines);
    println!("probe {label} error=\"{error}\" negotiated={} seat{SEAT}=\"{}\"", negotiated(lines), seat_owner(lines, SEAT));
    if !error.is_empty() {
        problems.push(format!("{label}: {error}"));
    }
}

#[test]
fn probe_host_without_seat_fills_after_joiner_leaves() {
    let dir = scenario_dir();
    let mut game = fresh_game(&dir);
    let ui_file = read_ui(&dir);
    start_host(&mut game, &ui_file);
    let mut problems = Vec::new();

    let mode = ui_as(&mut game, HOST, "mode", json!({ "code": "ai" }));
    check("host_no_seat mode", &mode, &mut problems);
    let filled = ui_as(&mut game, HOST, "fill", json!({}));
    check("host_no_seat fill_before_join", &filled, &mut problems);
    let joined = ui_as(&mut game, JOINER, "claim", json!({ "slot": SEAT }));
    check("host_no_seat joiner_claim", &joined, &mut problems);
    let left = call(&mut game, "peer_left", json!({ "pubkey": JOINER, "player": SEAT }));
    println!("probe host_no_seat joiner_left seat{SEAT}=\"{}\"", seat_owner(&left, SEAT));
    let refilled = ui_as(&mut game, HOST, "fill", json!({}));
    check("host_no_seat fill_after_joiner_leaves", &refilled, &mut problems);
    if seat_owner(&refilled, SEAT) != "bot:5" {
        problems.push(format!("the host's fill gave seat {SEAT} to \"{}\", not a bot", seat_owner(&refilled, SEAT)));
    }
    assert!(problems.is_empty(), "host fill problems: {problems:#?}");
}

#[test]
fn probe_refreshed_joiner_claims_seat_and_new_peer_gets_snapshot() {
    let dir = scenario_dir();
    let mut game = fresh_game(&dir);
    let ui_file = read_ui(&dir);
    start_host(&mut game, &ui_file);
    let mut problems = Vec::new();

    let creation = ui_as(&mut game, HOST, "claim", json!({ "slot": 0 }));
    check("host_seat claim", &creation, &mut problems);
    let mode = ui_as(&mut game, HOST, "mode", json!({ "code": "ai" }));
    check("host_seat mode", &mode, &mut problems);
    let filled = ui_as(&mut game, HOST, "fill", json!({}));
    check("host_seat fill", &filled, &mut problems);
    let joined = ui_as(&mut game, JOINER, "claim", json!({ "slot": SEAT }));
    check("joiner_claim", &joined, &mut problems);
    let left = call(&mut game, "peer_left", json!({ "pubkey": JOINER, "player": SEAT }));
    println!("probe joiner_left seat{SEAT}=\"{}\" negotiated={}", seat_owner(&left, SEAT), negotiated(&left));
    let again = ui_as(&mut game, JOINER_AGAIN, "claim", json!({ "slot": SEAT }));
    check("refreshed_joiner_claim", &again, &mut problems);

    let snapshot = call(&mut game, "lobby_sync", json!({}));
    let host_is_host = lobby_data(&snapshot).map(|data| data["host"] == HOST).unwrap_or(false);
    println!(
        "probe new_peer_snapshot lobby_events={} modal_widgets={} seat{SEAT}=\"{}\" host_is_host={host_is_host}",
        lobby_events(&snapshot),
        modal_widgets(&snapshot),
        seat_owner(&snapshot, SEAT)
    );
    if lobby_events(&snapshot) == 0 || modal_widgets(&snapshot) == 0 {
        problems.push("a new peer gets no lobby snapshot".to_owned());
    }
    if seat_owner(&snapshot, SEAT) != JOINER_AGAIN {
        problems.push(format!("the snapshot shows seat {SEAT} as \"{}\"", seat_owner(&snapshot, SEAT)));
    }
    assert!(problems.is_empty(), "refresh problems: {problems:#?}");
}
