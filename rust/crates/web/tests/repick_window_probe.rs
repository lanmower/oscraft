use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use web::Game;

const OWNER: &str = "probe-owner";
const TICKS_PER_SECOND: usize = 16;
const REPICK_WINDOW_SECONDS: usize = 20;
const DRAFT_TICK_CAP: usize = 200_000;
const DRAFT_CLOCK_CHUNK: usize = 64;
const REPICK_ACCEPTED: &str = "You can now pick a new hero";
const REPICK_LATE: &str = "Too late to repick";
const REPICK_OFF: &str = "Repicking has been disabled by the host";

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

fn chat(game: &mut Game, line: &str) -> Vec<Value> {
    call(game, "chat", json!({ "cmd": line, "text": line, "player": 1, "pubkey": OWNER }))
}

fn advance(game: &mut Game, ticks: usize) -> Vec<Value> {
    let mut lines = Vec::new();
    for _ in 0..ticks {
        let outcome = game.tick();
        assert!(outcome.is_empty(), "tick: {outcome}");
    }
    lines.extend(drain(game));
    lines
}

fn read_ui(dir: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(dir.join("moba_ui.json")).expect("read moba_ui.json")).expect("moba_ui.json")
}

fn lobby_error(lines: &[Value]) -> String {
    lines
        .iter()
        .find(|line| line["kind"] == "lobby")
        .and_then(|line| line["data"]["error"].as_str())
        .unwrap_or_default()
        .to_owned()
}

fn said(lines: &[Value], text: &str) -> bool {
    lines.iter().any(|line| line.to_string().contains(text))
}

fn lobby_seat_hero(lines: &[Value], seat: usize) -> Option<String> {
    lines
        .iter()
        .rev()
        .find(|line| line["kind"] == "lobby")
        .and_then(|line| line["data"]["slots"][seat]["hero"].as_str())
        .map(str::to_owned)
}

fn start_cm_draft(game: &mut Game, dir: &Path) {
    let ui_file = read_ui(dir);
    call(game, "setup", json!({ "ui": ui_file, "player": 1, "pubkey": OWNER }));
    advance(game, 60);
    ui(game, "claim", json!({ "slot": 0 }));
    let mode = ui(game, "mode", json!({ "code": "cm" }));
    assert!(lobby_error(&mode).is_empty(), "the host selects captains mode");
    ui(game, "fill", json!({}));
    ui(game, "world_ready", json!({}));
    for seat in 0..10 {
        ui(game, "ready", json!({ "slot": seat, "ready": true }));
    }
    let started = ui(game, "start", json!({}));
    assert!(lobby_error(&started).is_empty(), "start begins the draft: {}", lobby_error(&started));
}

fn run_draft_to_finish(game: &mut Game) -> usize {
    let mut ticks = 0;
    while ticks < DRAFT_TICK_CAP {
        ui(game, "draft_clock", json!({ "ticks": DRAFT_CLOCK_CHUNK }));
        ticks += DRAFT_CLOCK_CHUNK;
        let probe = ui(game, "ready", json!({ "slot": 0, "ready": true }));
        if lobby_error(&probe).is_empty() {
            return ticks;
        }
    }
    panic!("the draft did not finish within {DRAFT_TICK_CAP} draft ticks");
}

#[test]
fn probe_repick_accepted_at_draft_finish() {
    let dir = scenario_dir();
    let image = image_for(&dir);
    let mut game = Game::new(&image).expect("game");
    game.set_fog(false);
    load_pack(&mut game, &dir);
    start_cm_draft(&mut game, &dir);
    let finished_after = run_draft_to_finish(&mut game);
    println!("probe repick_at_finish draft_finished_after_ticks={finished_after}");

    let lines = chat(&mut game, "-repick");
    println!("probe repick_at_finish accepted={} refused_late={}", said(&lines, REPICK_ACCEPTED), said(&lines, REPICK_LATE));
    assert!(said(&lines, REPICK_ACCEPTED), "a repick at the draft finish is accepted");
}

#[test]
fn probe_repick_refused_after_twenty_second_window() {
    let dir = scenario_dir();
    let image = image_for(&dir);
    let mut game = Game::new(&image).expect("game");
    game.set_fog(false);
    load_pack(&mut game, &dir);
    start_cm_draft(&mut game, &dir);
    run_draft_to_finish(&mut game);
    let holds_before = lobby_seat_hero(&ui(&mut game, "ready", json!({ "slot": 0, "ready": true })), 0);
    println!("probe repick_after_window seat0_hero_before={holds_before:?}");

    advance(&mut game, (REPICK_WINDOW_SECONDS + 1) * TICKS_PER_SECOND);
    let lines = chat(&mut game, "-repick");
    println!(
        "probe repick_after_window seconds={} accepted={} refused_late={}",
        REPICK_WINDOW_SECONDS + 1,
        said(&lines, REPICK_ACCEPTED),
        said(&lines, REPICK_LATE)
    );
    assert!(said(&lines, REPICK_LATE), "a repick more than {REPICK_WINDOW_SECONDS} s after the draft finish is refused");
    assert!(!said(&lines, REPICK_ACCEPTED), "the late repick must not clear the hero");
}

#[test]
fn probe_repick_refused_while_draft_runs() {
    let dir = scenario_dir();
    let image = image_for(&dir);
    let mut game = Game::new(&image).expect("game");
    game.set_fog(false);
    load_pack(&mut game, &dir);
    start_cm_draft(&mut game, &dir);
    advance(&mut game, TICKS_PER_SECOND);

    let lines = chat(&mut game, "-repick");
    println!("probe repick_during_draft refused_off={} accepted={}", said(&lines, REPICK_OFF), said(&lines, REPICK_ACCEPTED));
    assert!(said(&lines, REPICK_OFF), "a repick while the draft runs is refused");
    assert!(!said(&lines, REPICK_ACCEPTED), "a repick while the draft runs clears nothing");
}
