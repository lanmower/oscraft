use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use web::Game;

const OWNER: &str = "probe-owner";

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

fn read_ui(dir: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(dir.join("moba_ui.json")).expect("read moba_ui.json")).expect("moba_ui.json")
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

fn fresh_game(dir: &Path, image: &[u8]) -> Game {
    let mut game = Game::new(image).expect("game");
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

fn ui(game: &mut Game, command: &str, args: Value) -> Vec<Value> {
    call(game, "ui", json!({ "call": command, "args": args, "player": 1, "pubkey": OWNER }))
}

fn advance(game: &mut Game, ticks: usize) {
    for _ in 0..ticks {
        let outcome = game.tick();
        assert!(outcome.is_empty(), "tick: {outcome}");
    }
}

fn last_lobby(lines: &[Value]) -> Option<&Value> {
    lines.iter().rev().find(|line| line["kind"] == "lobby").map(|line| &line["data"])
}

fn started_lobby(lines: &[Value]) -> Option<&Value> {
    lines.iter().rev().find(|line| line["kind"] == "lobby" && line["data"]["started"] == true).map(|line| &line["data"])
}

fn lobby_error(lines: &[Value]) -> String {
    lines
        .iter()
        .find(|line| line["kind"] == "lobby")
        .and_then(|line| line["data"]["error"].as_str())
        .unwrap_or_default()
        .to_owned()
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

struct Unit {
    id: u32,
    player: u8,
    kind: String,
}

fn alive_units(game: &Game) -> Vec<Unit> {
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
        })
        .collect()
}

fn own_hero(game: &Game, heroes: &[String], player: u8) -> u32 {
    alive_units(game)
        .iter()
        .find(|unit| unit.player == player && heroes.contains(&unit.kind))
        .map(|unit| unit.id)
        .expect("the player's hero")
}

fn start_match(dir: &Path, image: &[u8]) -> (Game, Vec<String>) {
    let mut game = fresh_game(dir, image);
    call(&mut game, "setup", json!({ "ui": read_ui(dir), "player": 1, "pubkey": OWNER }));
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
    let mut started = false;
    for step in 0..60 {
        if step % 3 == 0 && started_lobby(&ui(&mut game, "start", json!({}))).is_some() {
            started = true;
        }
        if started {
            break;
        }
        advance(&mut game, 100);
    }
    assert!(started, "the match starts");
    for player in 0..10usize {
        let hero = heroes[(20 + player) % heroes.len()].clone();
        call(&mut game, "seat_pick", json!({ "player": player, "hero": hero, "bot": player != 1 }));
    }
    advance(&mut game, 200);
    (game, heroes)
}

fn shop_widget(lines: &[Value]) -> Option<Value> {
    lines
        .iter()
        .rev()
        .find(|value| value["kind"] == "widget" && value["data"]["id"] == "shop" && value["data"]["remove"] != true)
        .map(|value| value["data"]["tree"].clone())
}

fn all_nodes(node: &Value, out: &mut Vec<Value>) {
    out.push(node.clone());
    if let Some(kids) = node.get("kids").and_then(Value::as_array) {
        for kid in kids {
            all_nodes(kid, out);
        }
    }
}

fn nodes_of(tree: &Value) -> Vec<Value> {
    let mut out = Vec::new();
    all_nodes(tree, &mut out);
    out
}

fn texts_of(tree: &Value) -> Vec<String> {
    nodes_of(tree).iter().filter_map(|node| node.get("text").and_then(Value::as_str).map(str::to_owned)).collect()
}

fn kind_is(node: &Value, kind: &str) -> bool {
    node["t"] == kind
}

fn kids_of(node: &Value) -> Vec<Value> {
    node.get("kids").and_then(Value::as_array).cloned().unwrap_or_default()
}

fn heading_of(node: &Value) -> Option<String> {
    if kind_is(node, "label") {
        return node["text"].as_str().map(str::to_owned);
    }
    kids_of(node).first().and_then(|first| first["text"].as_str()).map(str::to_owned)
}

fn grid_under(tree: &Value, heading: &str) -> Option<Value> {
    nodes_of(tree)
        .into_iter()
        .filter(|node| kind_is(node, "box"))
        .find(|node| kids_of(node).first().and_then(heading_of).as_deref() == Some(heading))
        .and_then(|node| kids_of(&node).get(1).cloned())
}

fn first_shop_cell(tree: &Value) -> (u64, String) {
    let cell = nodes_of(tree)
        .into_iter()
        .find(|node| kind_is(node, "cell") && node["call"] == "shop_pick")
        .expect("the shop lists an item cell");
    let name = kids_of(&cell)[0]["text"].as_str().expect("item name").to_owned();
    (cell["args"]["item"].as_u64().expect("item index"), name)
}

struct Shop {
    game: Game,
    tree: Value,
}

impl Shop {
    fn absorb(&mut self, lines: &[Value]) {
        if let Some(tree) = shop_widget(lines) {
            self.tree = tree;
        }
    }

    fn pump(&mut self, ticks: usize) {
        for _ in 0..ticks {
            advance(&mut self.game, 1);
            let lines = drain(&mut self.game);
            self.absorb(&lines);
        }
    }

    fn act_ui(&mut self, command: &str, args: Value) {
        let lines = ui(&mut self.game, command, args);
        self.absorb(&lines);
        self.pump(8);
    }

    fn open(&mut self, dir: &Path) {
        let key = read_ui(dir)["keys"]["shop"][0].as_str().expect("shop key").to_owned();
        let lines = call(&mut self.game, "key", json!({ "key": key, "phase": "down", "player": 1, "pubkey": OWNER }));
        self.absorb(&lines);
        self.pump(8);
        assert!(self.tree != Value::Null, "pressing the shop key opens the shop widget");
    }
}

#[test]
fn shop_widget_has_categories_tiers_grid_pinned_and_stash() {
    let dir = scenario_dir();
    let image = image_for(&dir);
    let (mut game, heroes) = start_match(&dir, &image);
    let hero = own_hero(&game, &heroes, 1);
    call(&mut game, "select", json!({ "ids": [hero], "player": 1, "pubkey": OWNER }));
    call(&mut game, "selection", json!({ "ids": [hero], "player": 1, "pubkey": OWNER }));
    let mut shop = Shop { game, tree: Value::Null };
    shop.open(&dir);

    let title = shop.tree["title"].as_str().expect("shop panel title").to_owned();
    println!("shop_layout title=\"{title}\"");
    assert!(title.starts_with("Shop"), "the panel is the shop: {title}");

    let texts = texts_of(&shop.tree);
    for category in ["[All]", "Consumables", "Attributes", "Offence", "Defence", "Utility"] {
        assert!(texts.iter().any(|text| text == category), "category tab {category}: {texts:?}");
    }
    println!("shop_layout categories ok: [All] Consumables Attributes Offence Defence Utility");

    let mut headings = vec!["Starting items", "Early", "Mid", "Late"];
    if texts.iter().any(|text| text == "UNVERIFIED: no 6.88g cost") {
        headings.push("UNVERIFIED: no 6.88g cost");
    }
    for heading in headings {
        assert!(texts.iter().any(|text| text == heading), "tier section {heading}: {texts:?}");
        let grid = grid_under(&shop.tree, heading).unwrap_or_else(|| panic!("tier {heading} has a grid"));
        assert_eq!(grid["t"], "grid", "tier {heading} holds a grid");
        assert_eq!(grid["cols"], 6, "tier {heading} lays its items in a six column grid");
        println!("shop_layout tier=\"{heading}\" cells={}", kids_of(&grid).len());
        assert!(!kids_of(&grid).is_empty(), "tier {heading} lists items");
    }

    let grids: Vec<Value> = nodes_of(&shop.tree).into_iter().filter(|node| kind_is(node, "grid")).collect();
    let stash: Vec<&Value> = grids.iter().filter(|grid| grid["cols"] == 9).collect();
    assert_eq!(stash.len(), 1, "exactly one nine column grid, the stash bar");
    assert_eq!(kids_of(stash[0]).len(), 9, "the stash bar has nine slots");

    assert!(texts.iter().any(|text| text == "Pinned items"), "pinned heading: {texts:?}");
    assert!(texts.iter().any(|text| text == "none pinned"), "pinned strip starts empty");
    assert!(texts.iter().any(|text| text == "stash"), "stash label: {texts:?}");
    assert!(texts.iter().any(|text| text == "Grab all"), "stash bar has Grab all: {texts:?}");

    let (index, name) = first_shop_cell(&shop.tree);
    println!("shop_layout first_item index={index} name=\"{name}\"");
    shop.act_ui("shop_pick", json!({ "item": index }));
    let detail = texts_of(&shop.tree);
    assert!(detail.iter().any(|text| text == &name), "the selected item shows its name: {detail:?}");
    let tail: Vec<&String> = detail.iter().rev().take(30).collect();
    assert!(detail.iter().any(|text| text == "Buy"), "the selected item offers Buy: {tail:?}");
    assert!(detail.iter().any(|text| text == "Recipe"), "the selected item shows its recipe tree: {detail:?}");

    shop.act_ui("pin", json!({ "item": index }));
    let pinned = grid_under(&shop.tree, "Pinned items").expect("pinned strip holds a grid after pinning");
    assert_eq!(kids_of(&pinned).len(), 1, "one pinned item");
    assert!(texts_of(&pinned).iter().any(|text| text == &name), "the pinned strip shows the pinned item");
    println!("shop_layout pinned={} after pin", kids_of(&pinned).len());

    let actions = [
        ("buy", json!({ "item": index })),
        ("grab_all", json!({})),
        ("unstash", json!({ "slot": 0 })),
        ("sell", json!({ "slot": 0 })),
        ("stash_item", json!({})),
    ];
    for (command, args) in actions {
        shop.act_ui(command, args);
        println!("shop_layout call={command} accepted");
    }
    shop.act_ui("pin", json!({ "item": index }));
    let unpinned = texts_of(&shop.tree);
    assert!(unpinned.iter().any(|text| text == "none pinned"), "unpin empties the strip: {unpinned:?}");
}
