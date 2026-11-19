use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use web::Game;

const HOST: &str = "preview-host";
const JOINER: &str = "preview-joiner";
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

fn setup_lobby(game: &mut Game, ui_file: &Value) {
    call(game, "setup", json!({ "ui": ui_file.clone(), "player": 1, "pubkey": HOST }));
    advance(game, 60);
}

fn lobby_data(lines: &[Value]) -> Value {
    lines.iter().rev().find(|line| line["kind"] == "lobby").map(|line| line["data"].clone()).expect("a lobby event")
}

fn host_of(lines: &[Value]) -> String {
    lobby_data(lines)["host"].as_str().unwrap_or_default().to_owned()
}

fn seat_owner(lines: &[Value], seat: usize) -> String {
    lobby_data(lines)["slots"][seat]["owner"].as_str().unwrap_or_default().to_owned()
}

#[test]
fn a_preview_claim_seats_the_joiner_without_labelling_it_host() {
    let dir = scenario_dir();
    let mut game = fresh_game(&dir);
    setup_lobby(&mut game, &read_ui(&dir));

    let preview = ui_as(&mut game, JOINER, "claim", json!({ "slot": SEAT, "preview": true }));
    println!("probe preview_claim host=\"{}\" seat{SEAT}=\"{}\"", host_of(&preview), seat_owner(&preview, SEAT));
    assert_eq!(host_of(&preview), "", "a preview claim set the host label");
    assert_eq!(seat_owner(&preview, SEAT), JOINER, "a preview claim did not seat the joiner");
}

#[test]
fn a_plain_claim_on_an_empty_lobby_still_sets_the_host() {
    let dir = scenario_dir();
    let mut game = fresh_game(&dir);
    setup_lobby(&mut game, &read_ui(&dir));

    let plain = ui_as(&mut game, JOINER, "claim", json!({ "slot": SEAT }));
    println!("probe plain_claim host=\"{}\"", host_of(&plain));
    assert_eq!(host_of(&plain), JOINER, "a plain claim on an empty lobby did not set the host");
}

#[test]
fn a_local_preview_dispatched_the_way_the_page_sends_it_leaves_host_empty() {
    let dir = scenario_dir();
    let mut game = fresh_game(&dir);
    setup_lobby(&mut game, &read_ui(&dir));

    let local = call(&mut game, "ui", json!({ "call": "claim", "args": { "slot": SEAT, "preview": true } }));
    println!("probe local_preview host=\"{}\" seat{SEAT}=\"{}\"", host_of(&local), seat_owner(&local, SEAT));
    assert_eq!(host_of(&local), "", "a page-shaped preview claim set the host label");
}

#[test]
fn a_preview_claim_followed_by_world_ready_still_leaves_host_empty() {
    let dir = scenario_dir();
    let mut game = fresh_game(&dir);
    setup_lobby(&mut game, &read_ui(&dir));

    ui_as(&mut game, JOINER, "claim", json!({ "slot": SEAT, "preview": true }));
    let ready = call(&mut game, "ui", json!({ "call": "world_ready", "args": {} }));
    println!("probe world_ready host=\"{}\" seat{SEAT}=\"{}\"", host_of(&ready), seat_owner(&ready, SEAT));
    assert_eq!(host_of(&ready), "", "world_ready set the host label after a preview claim");
}

#[test]
fn play_alone_after_a_preview_claim_makes_the_page_host_and_fills_the_seats() {
    let dir = scenario_dir();
    let mut game = fresh_game(&dir);
    setup_lobby(&mut game, &read_ui(&dir));

    ui_as(&mut game, JOINER, "claim", json!({ "slot": SEAT, "preview": true }));
    let alone = ui_as(&mut game, JOINER, "alone", json!({}));
    println!("probe alone host=\"{}\"", host_of(&alone));
    assert_eq!(host_of(&alone), JOINER, "play alone did not make the page the host");
    let filled = ui_as(&mut game, JOINER, "fill", json!({}));
    assert!(seat_owner(&filled, 0).starts_with("bot:"), "fill after play alone left seat 0 open");
}

#[test]
fn play_alone_is_refused_once_a_host_holds_the_lobby() {
    let dir = scenario_dir();
    let mut game = fresh_game(&dir);
    setup_lobby(&mut game, &read_ui(&dir));

    ui_as(&mut game, HOST, "claim", json!({ "slot": 0 }));
    let refused = ui_as(&mut game, JOINER, "alone", json!({}));
    println!("probe alone_refused host=\"{}\"", host_of(&refused));
    assert_eq!(host_of(&refused), HOST, "play alone replaced a host that already held the lobby");
}
