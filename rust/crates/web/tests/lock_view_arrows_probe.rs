use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use web::Game;

const OWNER: &str = "probe-owner";
const LOCAL: u64 = 1;

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

fn started_lobby(lines: &[Value]) -> Option<&Value> {
    lines.iter().rev().find(|line| line["kind"] == "lobby" && line["data"]["started"] == true).map(|line| &line["data"])
}

fn last_lobby(lines: &[Value]) -> Option<&Value> {
    lines.iter().rev().find(|line| line["kind"] == "lobby").map(|line| &line["data"])
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

fn start_match(dir: &Path, image: &[u8]) -> (Game, u32, Vec<String>) {
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
    let mut seated = Vec::new();
    for player in 0..10usize {
        let hero = heroes[(20 + player) % heroes.len()].clone();
        let lines = call(&mut game, "seat_pick", json!({ "player": player, "hero": hero, "bot": player != LOCAL as usize }));
        if player == LOCAL as usize {
            local_hero = selection_of(&lines).and_then(|ids| ids.first().copied());
        }
        seated.push(hero);
    }
    advance(&mut game, 200);
    let hero = local_hero.expect("the local seat's hero is selected at match start") as u32;
    (game, hero, seated)
}

fn position_of(game: &Game, unit: u32) -> (f64, f64) {
    let ids = game.unit_ids();
    let spots = game.units();
    let i = ids.iter().position(|&id| id == unit).expect("the unit is alive");
    (f64::from(spots[i * 3]), f64::from(spots[i * 3 + 1]))
}

const ORDER_EVERY_TICKS: usize = 4;
const HOLD_TICKS: usize = 48;
const STEP_CELLS: f64 = 4.0;
const MIN_TRAVEL_CELLS: f64 = 8.0;
const CENTRE_CELLS: (f64, f64) = (163.0, 163.0);
const SETTLE_TICKS: usize = 800;
const ARROWS: [(&str, f64, f64); 4] = [
    ("ArrowRight", 1.0, 0.0),
    ("ArrowLeft", -1.0, 0.0),
    ("ArrowUp", 0.0, -1.0),
    ("ArrowDown", 0.0, 1.0),
];

fn lock_rule_hero(game: &Game, player: f32) -> Option<u32> {
    let ids = game.unit_ids();
    let spots = game.units();
    let types = game.unit_types();
    let flags = game.unit_flags();
    ids.iter()
        .enumerate()
        .find(|(i, _)| spots[i * 3 + 2] == player && flags[types[*i] as usize] == 1)
        .map(|(_, id)| *id)
}

fn walk_to(game: &mut Game, unit: u32, target: (f64, f64)) -> (f64, f64) {
    assert!(game.order_move(unit, target.0 as f32, target.1 as f32), "the walk target is accepted: {}", game.last_refusal());
    for _ in 0..SETTLE_TICKS {
        advance(game, 1);
        let at = position_of(game, unit);
        if (at.0 - target.0).hypot(at.1 - target.1) < 1.0 {
            return at;
        }
    }
    panic!("the hero reaches {target:?} within {SETTLE_TICKS} ticks, at {:?}", position_of(game, unit));
}

#[test]
fn locked_view_arrows_move_the_local_hero() {
    let dir = scenario_dir();
    let image = image_for(&dir);
    let (mut game, hero, seated) = start_match(&dir, &image);
    let picked = lock_rule_hero(&game, LOCAL as f32);
    println!("lock_view rule_pick={picked:?} local_hero={hero} hero={} map={:?}", seated[LOCAL as usize], game.map_size());
    assert_eq!(picked, Some(hero), "the lock's hero rule picks the local hero");

    for (key, ax, az) in ARROWS {
        let start = walk_to(&mut game, hero, CENTRE_CELLS);
        let mut refused = 0;
        for tick in 0..HOLD_TICKS {
            if tick % ORDER_EVERY_TICKS == 0 {
                let here = position_of(&game, hero);
                let target = (here.0 + ax * STEP_CELLS, here.1 + az * STEP_CELLS);
                if !game.order_move(hero, target.0 as f32, target.1 as f32) {
                    refused += 1;
                }
            }
            advance(&mut game, 1);
        }
        let end = position_of(&game, hero);
        let (dx, dz) = (end.0 - start.0, end.1 - start.1);
        let travel = dx.hypot(dz);
        let along = (dx * ax + dz * az) / travel.max(1e-9);
        println!(
            "lock_view {key} start=({:.1},{:.1}) end=({:.1},{:.1}) travel={travel:.1} along={along:.3} refused={refused} last_refusal={:?}",
            start.0, start.1, end.0, end.1, game.last_refusal()
        );
        assert!(travel >= MIN_TRAVEL_CELLS && along > 0.9, "{key} moves the hero that way");
    }
}
