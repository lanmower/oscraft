use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use web::Game;

const OWNER: &str = "probe-owner";
const LOCAL: u64 = 1;
const TPS: usize = 16;
const REVIVE_TICKS: usize = 140 * TPS;
const FOLLOW_CELLS: f64 = 46.875;
const ORDER_MOVE: i32 = 0;
const ORDER_FOLLOW: i32 = 5;

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

fn selection_of(lines: &[Value]) -> Option<Vec<u64>> {
    lines
        .iter()
        .rev()
        .find(|line| line["kind"] == "feedback" && line["data"]["kind"] == "selection" && line["data"]["player"] == LOCAL)
        .map(|line| line["data"]["ids"].as_array().expect("selection ids").iter().map(|id| id.as_u64().expect("unit id")).collect())
}

fn start_match(dir: &Path, image: &[u8]) -> (Game, u32, Vec<String>) {
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
    (game, hero, heroes)
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

#[test]
fn courier_follows_returns_delivers_and_revives_without_commands() {
    let dir = scenario_dir();
    let image = image_for(&dir);
    let (mut game, hero, _) = start_match(&dir, &image);
    let owner = 1u8;

    let units = units_of(&game);
    let courier = *couriers_of(&units, owner).first().expect("the owner's courier spawns at match start");
    let hero_at = place_of(&units, hero);
    let courier_at = place_of(&units, courier);
    println!("probe start courier={courier} at=({:.2},{:.2}) hero={hero} at=({:.2},{:.2})", courier_at.0, courier_at.1, hero_at.0, hero_at.1);

    let map = game.map_size();
    let (width, height) = (f64::from(map[0]), f64::from(map[1]));
    let mut candidates = [(60.0, 0.0), (-60.0, 0.0), (0.0, 60.0), (0.0, -60.0), (42.0, 42.0), (-42.0, -42.0), (42.0, -42.0), (-42.0, 42.0)]
        .iter()
        .map(|(dx, dy)| (courier_at.0 + dx, courier_at.1 + dy))
        .filter(|&(x, y)| x > 0.0 && y > 0.0 && x < width && y < height && distance((x, y), hero_at) > FOLLOW_CELLS + 5.0)
        .collect::<Vec<_>>()
        .into_iter();
    let mut send_far = |game: &mut Game| loop {
        let (x, y) = candidates.next().expect("a point 60 cells from base is accepted as a move");
        if game.order_move(courier, x as f32, y as f32) {
            break (x, y);
        }
    };
    let mut far = send_far(&mut game);
    let mut sent_tick = 0;
    println!("probe setup far_point=({:.2},{:.2}) order=move-once", far.0, far.1);

    let mut arrived = false;
    let mut home_tick = None;
    let mut follow_tick = None;
    let mut previous = (i32::MIN, (0.0, 0.0));
    for tick in 1..=1200 {
        advance(&mut game, 1);
        let now = units_of(&game);
        let (kind, target) = order_of(&game, courier);
        let at = place_of(&now, courier);
        if kind != previous.0 || (kind == ORDER_MOVE && target != previous.1) {
            println!(
                "probe courier tick={tick} order={kind} target=({:.2},{:.2}) pos=({:.2},{:.2}) hero_dist={:.2}",
                target.0,
                target.1,
                at.0,
                at.1,
                distance(at, place_of(&now, hero))
            );
            previous = (kind, target);
        }
        if !arrived && kind == ORDER_MOVE && distance(at, far) <= 1.0 {
            arrived = true;
        }
        if !arrived && tick - sent_tick >= 300 {
            far = send_far(&mut game);
            sent_tick = tick;
            println!("probe setup retry tick={tick} far_point=({:.2},{:.2})", far.0, far.1);
        }
        if arrived && home_tick.is_none() && kind == ORDER_MOVE && distance(target, far) > 1.0 {
            home_tick = Some(tick);
        }
        if home_tick.is_some() && kind == ORDER_FOLLOW {
            follow_tick = Some(tick);
            break;
        }
    }
    println!("probe return_home tick={home_tick:?} follow tick={follow_tick:?}");
    assert!(home_tick.is_some(), "an idle courier far from its owner returns to base");
    assert!(follow_tick.is_some_and(|t| home_tick.is_some_and(|h| t > h)), "an idle courier near its owner follows it");

    let lifesteal = lifesteal_item(&game);
    assert!(game.give_item(courier, lifesteal, 1), "the courier takes the item");
    let mut delivered = None;
    for tick in 1..=60 {
        advance(&mut game, 1);
        if holds(&game, hero, lifesteal) {
            delivered = Some(tick);
            break;
        }
    }
    println!(
        "probe deliver tick={delivered:?} hero_holds={} courier_holds={}",
        holds(&game, hero, lifesteal),
        holds(&game, courier, lifesteal)
    );
    assert!(delivered.is_some(), "an item the owner needs is carried to the owner when the owner is near");
    assert!(!holds(&game, courier, lifesteal), "the courier gives the item up");

    let death_at = place_of(&units_of(&game), courier);
    game.hurt(courier, 1_000_000);
    let mut death_tick = None;
    let mut revived = None;
    for tick in 1..=REVIVE_TICKS + 40 {
        advance(&mut game, 1);
        let now = units_of(&game);
        let alive = couriers_of(&now, owner);
        if death_tick.is_none() && alive.is_empty() {
            death_tick = Some(tick);
        }
        if death_tick.is_some() && !alive.is_empty() {
            revived = Some((tick, place_of(&now, alive[0])));
            break;
        }
    }
    let (revive_tick, revive_at) = revived.expect("the courier revives on its own");
    let death_tick = death_tick.expect("the courier dies");
    let delta = revive_tick - death_tick;
    println!(
        "probe revive death_tick={death_tick} revive_tick={revive_tick} delta={delta} expected={REVIVE_TICKS} at=({:.2},{:.2}) death_at=({:.2},{:.2})",
        revive_at.0, revive_at.1, death_at.0, death_at.1
    );
    assert!(delta.abs_diff(REVIVE_TICKS) <= 2, "the revive follows the 140 s timer");
    assert!(distance(revive_at, death_at) <= 0.5, "the revived courier appears at the death point");
}

struct Step {
    kind: i32,
    target: (f64, f64),
}

fn chat(game: &mut Game, line: &str) -> Vec<Value> {
    call(game, "chat", json!({ "cmd": line, "text": line, "player": LOCAL, "pubkey": OWNER }))
}

fn carry_trace(game: &mut Game, courier: u32, hero: u32, item: u32, window: usize, label: &str) -> (Option<usize>, Vec<Step>) {
    let before = held(game, hero, item);
    assert!(game.give_item(courier, item, 1), "the courier takes the item");
    let mut trace: Vec<Step> = Vec::new();
    let mut delivered = None;
    for tick in 1..=window {
        advance(game, 1);
        let now = units_of(game);
        let (kind, target) = order_of(game, courier);
        let at = place_of(&now, courier);
        if trace.last().is_none_or(|step| step.kind != kind || (kind == ORDER_MOVE && step.target != target)) {
            println!("probe route {label} tick={tick} order={kind} target=({:.2},{:.2}) courier=({:.2},{:.2})", target.0, target.1, at.0, at.1);
            trace.push(Step { kind, target });
        }
        if held(game, hero, item) > before {
            delivered = Some(tick);
            break;
        }
    }
    println!(
        "probe route {label} delivered={delivered:?} courier_holds={} hero_gained={}",
        held(game, courier, item),
        held(game, hero, item) - before
    );
    (delivered, trace)
}

#[test]
fn courier_route_visits_secret_depot_before_owner_only_when_stop_is_set() {
    let dir = scenario_dir();
    let image = image_for(&dir);
    let (mut game, hero, _) = start_match(&dir, &image);
    let owner = 1u8;
    let courier = *couriers_of(&units_of(&game), owner).first().expect("the owner's courier");
    let lifesteal = lifesteal_item(&game);

    let (direct_at, direct) = carry_trace(&mut game, courier, hero, lifesteal, 400, "stop_off");
    assert!(direct_at.is_some(), "with the stop off the courier hands the item to its owner");
    assert!(!direct.iter().any(|step| step.kind == ORDER_MOVE), "with the stop off the route has no move order");

    chat(&mut game, "-secretstop");
    let (shop_at, shop) = carry_trace(&mut game, courier, hero, lifesteal, 800, "stop_on");
    let first_move = shop.iter().position(|step| step.kind == ORDER_MOVE);
    let last_follow = shop.iter().rposition(|step| step.kind == ORDER_FOLLOW);
    assert!(shop_at.is_some(), "with the stop on the item still reaches the owner");
    assert!(first_move.is_some(), "with the stop on the courier moves to the Secret Shop first");
    assert!(last_follow > first_move, "with the stop on the courier goes to the owner after the Secret Shop");

    let before = held(&game, hero, lifesteal);
    chat(&mut game, "-noautodeliver");
    assert!(game.give_item(courier, lifesteal, 1), "the courier takes the item");
    advance(&mut game, 200);
    println!(
        "probe autodeliver_off after_200 courier_holds={} hero_gained={}",
        held(&game, courier, lifesteal),
        held(&game, hero, lifesteal) - before
    );
    assert!(held(&game, courier, lifesteal) > 0 && held(&game, hero, lifesteal) == before, "with auto-delivery off the courier keeps the item");

    chat(&mut game, "-autodeliver");
    let mut resumed = None;
    for tick in 1..=800 {
        advance(&mut game, 1);
        if held(&game, hero, lifesteal) > before {
            resumed = Some(tick);
            break;
        }
    }
    println!("probe autodeliver_on resumed={resumed:?} hero_gained={}", held(&game, hero, lifesteal) - before);
    assert!(resumed.is_some(), "with auto-delivery back on the item reaches the owner");
}

fn seat_hero(game: &Game, heroes: &[String], player: usize) -> u32 {
    let name = &heroes[(20 + player) % heroes.len()];
    units_of(game)
        .iter()
        .find(|unit| usize::from(unit.player) == player && &unit.kind == name)
        .map(|unit| unit.id)
        .expect("the seat's hero is on the map")
}

fn walk_to(game: &mut Game, unit: u32, target: (f64, f64), ticks: usize) -> bool {
    if !game.order_move(unit, target.0 as f32, target.1 as f32) {
        return false;
    }
    for _ in 0..ticks {
        advance(game, 1);
        if distance(place_of(&units_of(game), unit), target) <= 1.0 {
            return true;
        }
    }
    false
}

fn follows_still_hero_from_a_point_thirty_cells_away(owner: u8) {
    let dir = scenario_dir();
    let image = image_for(&dir);
    let (mut game, _, heroes) = start_match(&dir, &image);
    let courier = *couriers_of(&units_of(&game), owner).first().expect("the owner's courier spawns at match start");
    let hero = seat_hero(&game, &heroes, owner as usize);
    let hero_start = place_of(&units_of(&game), hero);
    assert!(game.order_move(hero, hero_start.0 as f32, hero_start.1 as f32), "the hero holds still");
    advance(&mut game, 4);
    let hero_at = place_of(&units_of(&game), hero);

    let map = game.map_size();
    let (width, height) = (f64::from(map[0]), f64::from(map[1]));
    let spots: Vec<(f64, f64)> = (0..16)
        .map(|k| {
            let angle = k as f64 * std::f64::consts::TAU / 16.0;
            (hero_at.0 + 28.0 * angle.cos(), hero_at.1 + 28.0 * angle.sin())
        })
        .filter(|&(x, y)| x > 0.0 && y > 0.0 && x < width && y < height)
        .collect();
    let arrived_at = spots.into_iter().find(|&spot| walk_to(&mut game, courier, spot, 600));
    let spot = arrived_at.expect("the courier reaches a point 28 cells from its hero");
    advance(&mut game, 16);
    let courier_at = place_of(&units_of(&game), courier);
    let gap = distance(courier_at, hero_at);
    let (kind, _) = order_of(&game, courier);
    println!(
        "probe still_hero owner={owner} courier={courier} hero={hero} spot=({:.2},{:.2}) courier=({:.2},{:.2}) gap={gap:.2} order_after_arrival={kind}",
        spot.0, spot.1, courier_at.0, courier_at.1
    );

    let mut follow_tick = None;
    for tick in 1..=128 {
        advance(&mut game, 1);
        if order_of(&game, courier).0 == ORDER_FOLLOW {
            follow_tick = Some(tick);
            break;
        }
    }
    println!("probe still_hero follow_tick={follow_tick:?}");
    assert!(follow_tick.is_some(), "an idle courier follows its still hero when the hero is 28 cells away");
}

#[test]
fn courier_of_team0_follows_still_hero_from_thirty_cells() {
    follows_still_hero_from_a_point_thirty_cells_away(0);
}

#[test]
fn courier_of_team1_follows_still_hero_from_thirty_cells() {
    follows_still_hero_from_a_point_thirty_cells_away(1);
}
