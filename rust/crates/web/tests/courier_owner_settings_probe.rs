use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use web::Game;

const OWNER: &str = "probe-owner";
const LOCAL: u64 = 1;
const HELD_OWNER: u8 = 1;
const OTHER_OWNER: u8 = 0;
const ORDER_MOVE: i32 = 0;
const AWAY_CELLS: f64 = 4.0;
const ROUTE_TICKS: usize = 800;
const HOLD_TICKS: usize = 400;
const COURIER_WALK_TICKS: usize = 1600;
const DELIVERY_REACH_CELLS: f64 = 10.0;
const DIRECT_TICKS: usize = 40;

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

fn start_match(dir: &Path, image: &[u8]) -> (Game, Vec<String>) {
    let mut game = Game::new(image).expect("game");
    game.set_fog(false);
    load_pack(&mut game, dir);
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
    for player in 0..10usize {
        let hero = heroes[(20 + player) % heroes.len()].clone();
        call(&mut game, "seat_pick", json!({ "player": player, "hero": hero, "bot": player != LOCAL as usize }));
    }
    advance(&mut game, 200);
    (game, heroes)
}

struct Unit {
    id: u32,
    player: u8,
    kind: String,
    x: f64,
    y: f64,
}

fn units_of(game: &Game) -> Vec<Unit> {
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
            x: f64::from(spots[i * 3]),
            y: f64::from(spots[i * 3 + 1]),
        })
        .collect()
}

fn couriers_of(units: &[Unit], player: u8) -> Vec<u32> {
    units.iter().filter(|unit| unit.player == player && unit.kind.starts_with("courier")).map(|unit| unit.id).collect()
}

fn place_of(units: &[Unit], id: u32) -> (f64, f64) {
    units.iter().find(|unit| unit.id == id).map(|unit| (unit.x, unit.y)).expect("the unit is alive")
}

fn distance(a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

fn order_of(game: &Game, unit: u32) -> (i32, (f64, f64)) {
    let order = game.current_order(unit);
    (order[0], (f64::from(order[1]) / 65536.0, f64::from(order[2]) / 65536.0))
}

fn lifesteal_item(game: &Game) -> u32 {
    let catalog: Value = serde_json::from_str(&game.item_catalog_json()).expect("item catalog json");
    catalog
        .as_array()
        .expect("item catalog array")
        .iter()
        .find(|item| item["name"] == "lifesteal")
        .and_then(|item| item["id"].as_u64())
        .expect("lifesteal item") as u32
}

fn held(game: &Game, unit: u32, item: u32) -> u64 {
    let inventory: Value = serde_json::from_str(&game.inventory_json(unit)).expect("inventory json");
    inventory["slots"]
        .as_array()
        .expect("slots")
        .iter()
        .filter(|slot| slot[0].as_u64() == Some(u64::from(item) + 1))
        .map(|slot| slot[1].as_u64().unwrap_or(0))
        .sum()
}

fn holds(game: &Game, unit: u32, item: u32) -> bool {
    held(game, unit, item) > 0
}

fn seat_hero(game: &Game, heroes: &[String], player: usize) -> u32 {
    let name = &heroes[(20 + player) % heroes.len()];
    units_of(game)
        .iter()
        .find(|unit| usize::from(unit.player) == player && &unit.kind == name)
        .map(|unit| unit.id)
        .expect("the seat's hero is on the map")
}

fn chat_as(game: &mut Game, player: u8, line: &str) -> Vec<Value> {
    call(game, "chat", json!({ "cmd": line, "text": line, "player": player, "pubkey": OWNER }))
}

struct Seat {
    courier: u32,
    hero: u32,
    home: (f64, f64),
}

fn seat_of(game: &Game, heroes: &[String], owner: u8) -> Seat {
    let units = units_of(game);
    let courier = *couriers_of(&units, owner).first().expect("the owner's courier spawns at match start");
    Seat {
        courier,
        hero: seat_hero(game, heroes, owner as usize),
        home: place_of(&units, courier),
    }
}

fn bring_courier_to_hero(game: &mut Game, seat: &Seat) {
    for _ in 0..COURIER_WALK_TICKS {
        let hero_at = place_of(&units_of(game), seat.hero);
        if distance(place_of(&units_of(game), seat.courier), hero_at) <= DELIVERY_REACH_CELLS {
            return;
        }
        assert!(game.order_move(seat.courier, hero_at.0 as f32, hero_at.1 as f32), "the owner's courier takes a move order");
        advance(game, 1);
    }
    panic!("the owner's courier reaches its hero within {COURIER_WALK_TICKS} ticks");
}

#[test]
fn autodeliver_setting_is_read_per_owner() {
    let dir = scenario_dir();
    let image = image_for(&dir);
    let (mut game, heroes) = start_match(&dir, &image);
    let held_back = seat_of(&game, &heroes, HELD_OWNER);
    let delivered = seat_of(&game, &heroes, OTHER_OWNER);
    let lifesteal = lifesteal_item(&game);

    bring_courier_to_hero(&mut game, &held_back);
    bring_courier_to_hero(&mut game, &delivered);
    chat_as(&mut game, HELD_OWNER, "-noautodeliver");
    chat_as(&mut game, OTHER_OWNER, "-autodeliver");
    assert!(game.give_item(held_back.courier, lifesteal, 1), "the held-back courier takes the item");
    assert!(game.give_item(delivered.courier, lifesteal, 1), "the delivering courier takes the item");
    let held_before = held(&game, held_back.hero, lifesteal);
    let delivered_before = held(&game, delivered.hero, lifesteal);

    let mut delivered_at = None;
    for tick in 1..=HOLD_TICKS {
        advance(&mut game, 1);
        if delivered_at.is_none() && held(&game, delivered.hero, lifesteal) > delivered_before {
            delivered_at = Some(tick);
        }
    }
    println!(
        "probe owner_autodeliver held_back_courier={} held_back_hero_gained={} delivered_at={delivered_at:?} delivered_hero_gained={}",
        held(&game, held_back.courier, lifesteal),
        held(&game, held_back.hero, lifesteal) - held_before,
        held(&game, delivered.hero, lifesteal) - delivered_before
    );
    assert!(holds(&game, held_back.courier, lifesteal), "with auto-delivery off for its owner the courier keeps the item");
    assert_eq!(held(&game, held_back.hero, lifesteal), held_before, "with auto-delivery off for its owner the item never reaches that owner's hero");
    assert!(delivered_at.is_some(), "with auto-delivery on for its owner the item reaches that owner's hero");
    assert!(!holds(&game, delivered.courier, lifesteal), "the delivering courier gives the item up");
}

#[test]
fn secret_stop_setting_is_read_per_owner() {
    let dir = scenario_dir();
    let image = image_for(&dir);
    let (mut game, heroes) = start_match(&dir, &image);
    let stopping = seat_of(&game, &heroes, HELD_OWNER);
    let direct = seat_of(&game, &heroes, OTHER_OWNER);
    let lifesteal = lifesteal_item(&game);

    bring_courier_to_hero(&mut game, &stopping);
    bring_courier_to_hero(&mut game, &direct);
    chat_as(&mut game, HELD_OWNER, "-secretstop");
    chat_as(&mut game, OTHER_OWNER, "-nosecretstop");
    assert!(game.give_item(stopping.courier, lifesteal, 1), "the stopping courier takes the item");
    assert!(game.give_item(direct.courier, lifesteal, 1), "the direct courier takes the item");
    let stopping_before = held(&game, stopping.hero, lifesteal);
    let direct_before = held(&game, direct.hero, lifesteal);

    let mut stop_walk = None;
    let mut stopping_delivered = None;
    let mut direct_delivered = None;
    for tick in 1..=ROUTE_TICKS {
        advance(&mut game, 1);
        let (kind, target) = order_of(&game, stopping.courier);
        if stop_walk.is_none() && kind == ORDER_MOVE && distance(target, stopping.home) > AWAY_CELLS {
            stop_walk = Some(tick);
        }
        if stopping_delivered.is_none() && held(&game, stopping.hero, lifesteal) > stopping_before {
            stopping_delivered = Some(tick);
        }
        if direct_delivered.is_none() && held(&game, direct.hero, lifesteal) > direct_before {
            direct_delivered = Some(tick);
        }
    }
    println!("probe owner_secretstop stop_walk={stop_walk:?} stopping_delivered={stopping_delivered:?} direct_delivered={direct_delivered:?}");
    assert!(stop_walk.is_some(), "with secret stop set for its owner the courier walks to the Secret Shop first");
    assert!(stopping_delivered.is_some_and(|at| stop_walk.is_some_and(|walk| walk < at)), "that courier reaches its owner after the Secret Shop");
    assert!(direct_delivered.is_some_and(|at| at <= DIRECT_TICKS), "with secret stop off for its owner the item is handed over at once");
    assert!(stopping_delivered.is_some_and(|at| direct_delivered.is_some_and(|direct| at > direct)), "the Secret Shop detour delays that owner's item past the direct one");
}
