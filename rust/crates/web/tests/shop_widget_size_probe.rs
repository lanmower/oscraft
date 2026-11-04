use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use web::Game;

const OWNER: &str = "probe-owner";
const FRAME_LIMIT: usize = 65_535;
const FIRST_PICK: &str = "magic_wand";
const SECOND_PICK: &str = "branches";

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

fn start_match(dir: &Path, image: &[u8]) -> (Game, Vec<String>) {
    let mut game = Game::new(image).expect("game");
    game.set_fog(false);
    load_pack(&mut game, dir);
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

fn tips_of(tree: &Value) -> usize {
    nodes_of(tree).iter().filter(|node| node.get("tip").is_some()).count()
}

fn cell_tips(tree: &Value) -> Vec<String> {
    nodes_of(tree).iter().filter(|node| node["t"] == "cell").filter_map(|node| node.get("tip").and_then(Value::as_str).map(str::to_owned)).collect()
}

struct Shop {
    game: Game,
    tree: Option<Value>,
}

impl Shop {
    fn absorb(&mut self, lines: &[Value]) -> Vec<usize> {
        let mut sizes = Vec::new();
        for value in lines {
            if value["kind"] == "widget" && value["data"]["id"] == "shop" && value["data"]["remove"] != true {
                sizes.push(value.to_string().len());
                self.tree = Some(value["data"]["tree"].clone());
            }
        }
        sizes
    }

    fn pump(&mut self, ticks: usize) -> Vec<usize> {
        let mut sizes = Vec::new();
        for _ in 0..ticks {
            advance(&mut self.game, 1);
            let lines = drain(&mut self.game);
            sizes.extend(self.absorb(&lines));
        }
        sizes
    }

    fn step(&mut self, lines: Vec<Value>) -> Vec<usize> {
        let mut sizes = self.absorb(&lines);
        sizes.extend(self.pump(8));
        sizes
    }

    fn tree(&self) -> &Value {
        self.tree.as_ref().expect("the shop widget is open")
    }
}

fn report(step: &str, sizes: &[usize], tips: usize) {
    let max = sizes.iter().copied().max().unwrap_or(0);
    println!("shop_widget_size step={step} events={sizes:?} max_bytes={max} limit={FRAME_LIMIT} cell_tips={tips}");
}

#[test]
fn shop_widget_stays_under_the_frame_limit_and_the_pick_detail_keeps_the_recipe() {
    let dir = scenario_dir();
    let image = image_for(&dir);
    let ui_text = read_ui(&dir);
    let (mut game, heroes) = start_match(&dir, &image);
    let hero = own_hero(&game, &heroes, 1);
    call(&mut game, "select", json!({ "ids": [hero], "player": 1, "pubkey": OWNER }));
    call(&mut game, "selection", json!({ "ids": [hero], "player": 1, "pubkey": OWNER }));
    let mut shop = Shop { game, tree: None };

    let key = ui_text["keys"]["shop"][0].as_str().expect("shop key").to_owned();
    let lines = call(&mut shop.game, "key", json!({ "key": key, "phase": "down", "player": 1, "pubkey": OWNER }));
    let open = shop.step(lines);
    report("open", &open, tips_of(shop.tree()));

    let catalog: Vec<Value> = serde_json::from_str(&shop.game.item_catalog_json()).expect("catalog json");
    let index = |name: &str| catalog.iter().find(|item| item["name"] == name).expect(name)["id"].as_u64().expect("id");
    let first = index(FIRST_PICK);
    let second = index(SECOND_PICK);

    let lines = ui(&mut shop.game, "shop_pick", json!({ "item": first }));
    let pick_first = shop.step(lines);
    let first_tips = tips_of(shop.tree());
    report(&format!("pick:{FIRST_PICK}"), &pick_first, first_tips);
    let builds = ui_text["text"]["shop"]["tip"]["builds"].as_str().expect("builds text");
    let buy = ui_text["text"]["shop"]["tip"]["buy"].as_str().expect("buy text");
    let first_detail = texts_of(shop.tree()).into_iter().find(|text| text.contains(builds));
    println!("shop_widget_detail pick={FIRST_PICK} detail_has_recipe={} detail_has_buy={}", first_detail.is_some(), texts_of(shop.tree()).iter().any(|text| text.contains(buy)));
    if let Some(text) = &first_detail {
        println!("shop_widget_detail pick={FIRST_PICK} recipe=\"{text}\"");
    }

    let lines = ui(&mut shop.game, "shop_pick", json!({ "item": second }));
    let pick_second = shop.step(lines);
    let second_tips = tips_of(shop.tree());
    report(&format!("pick:{SECOND_PICK}"), &pick_second, second_tips);
    let second_detail = texts_of(shop.tree()).into_iter().any(|text| text.contains(buy));
    println!("shop_widget_detail pick={SECOND_PICK} detail_has_buy={second_detail}");

    for (step, sizes) in [("open", &open), (FIRST_PICK, &pick_first), (SECOND_PICK, &pick_second)] {
        for bytes in sizes.iter() {
            assert!(*bytes <= FRAME_LIMIT, "{step}: widget:shop is {bytes} bytes, over the {FRAME_LIMIT}-byte frame limit");
        }
    }
    let hovers = cell_tips(shop.tree());
    let longest = hovers.iter().map(String::len).max().unwrap_or(0);
    println!("shop_widget_hover cells={} longest={longest} sample=\"{}\"", hovers.len(), hovers.first().cloned().unwrap_or_default());
    assert!(first_tips > 0 && second_tips > 0, "shop cells carry a short hover tip");
    assert!(hovers.iter().all(|tip| !tip.contains(builds)), "a cell hover leaves the build tree to the selected item's detail");
    assert!(first_detail.as_ref().is_some_and(|text| text.contains("magic_stick") && text.contains("circlet")), "the selected item's detail carries its recipe");
    assert!(second_detail, "the second selected item's detail carries the buy sentence");
}
