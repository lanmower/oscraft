use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use web::Game;

const OWNER: &str = "probe-owner";
const SHOP_HOME: (f32, f32) = (262.0, 66.0);
const SHOP_REACH: f32 = 20.312;
const FAR_HERO: (f32, f32) = (257.0, 100.0);
const FAR_COURIER: (f32, f32) = (257.0, 102.0);
const RULES: [&str; 5] = ["SHOP-REACH", "SHOP-COURIER", "SHOP-NO-COURIER", "SHOP-DELIVER", "SHOP-TREE"];

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

fn kids_of(node: &Value) -> Vec<Value> {
    node.get("kids").and_then(Value::as_array).cloned().unwrap_or_default()
}

fn tip_of(shop: &mut Shop, name: &str) -> String {
    let item = nodes_of(&shop.tree)
        .into_iter()
        .find(|node| node["t"] == "cell" && node["call"] == "shop_pick" && kids_of(node).first().and_then(|kid| kid["text"].as_str()) == Some(name))
        .and_then(|node| node["args"]["item"].as_u64())
        .unwrap_or_else(|| panic!("the shop lists {name}"));
    shop.act_ui("shop_pick", json!({ "item": item }));
    texts_of(&shop.tree)
        .into_iter()
        .find(|text| text.contains("Buy from here:"))
        .unwrap_or_else(|| panic!("the selected {name} shows its tip"))
}

fn texts_of(tree: &Value) -> Vec<String> {
    nodes_of(tree).iter().filter_map(|node| node.get("text").and_then(Value::as_str).map(str::to_owned)).collect()
}

fn first_sentence(tip: &str) -> String {
    match tip.find(". ") {
        Some(end) => tip[..=end].to_owned(),
        None => tip.to_owned(),
    }
}

fn after(tip: &str, lead: &str) -> Option<String> {
    let start = tip.find(lead)? + lead.len();
    let rest = &tip[start..];
    let end = rest.find(". ").map(|at| at + 1).unwrap_or_else(|| rest.trim_end_matches('.').len());
    Some(rest[..end].trim_end_matches('.').to_owned())
}

fn name_and_cost(text: &str) -> (String, u32) {
    let (name, cost) = text.rsplit_once(' ').expect("name and cost");
    (name.to_owned(), cost.trim_end_matches('g').parse().expect("cost digits"))
}

fn tree_parts(sentence: &str) -> Vec<(String, u32, Vec<String>)> {
    sentence
        .split(" + ")
        .map(|part| {
            let (head, subs) = match part.find(" (") {
                Some(at) => (&part[..at], part[at + 2..part.len() - 1].split(", ").map(|sub| name_and_cost(sub).0).collect()),
                None => (part, Vec::new()),
            };
            let (name, cost) = name_and_cost(head);
            (name, cost, subs)
        })
        .collect()
}

struct Row {
    name: String,
    cost: u32,
    parts: Vec<String>,
    status: String,
    keep: bool,
}

fn tsv_rows(dir: &Path) -> Vec<Row> {
    let text = std::fs::read_to_string(dir.join("items_688g.tsv")).expect("read items_688g.tsv");
    text.lines()
        .skip(1)
        .filter_map(|line| {
            let f: Vec<&str> = line.split('\t').collect();
            if f.len() < 7 {
                return None;
            }
            let keep = f[6] == "KEEP";
            let parts = if f[6].is_empty() || keep { Vec::new() } else { f[6].split(',').map(str::to_owned).collect() };
            Some(Row { name: f[0].to_owned(), cost: f[5].parse().unwrap_or(0), parts, status: f[4].to_owned(), keep })
        })
        .collect()
}

fn tsv_cost(rows: &[Row], name: &str) -> u32 {
    rows.iter().find(|row| row.name == name).map(|row| row.cost).unwrap_or_else(|| panic!("tsv row {name}"))
}

fn tsv_parents(rows: &[Row], name: &str) -> Vec<String> {
    let mut out: Vec<String> = rows.iter().filter(|row| row.parts.iter().any(|part| part == name)).map(|row| row.name.clone()).collect();
    out.sort();
    out
}

fn check_tree(rows: &[Row], tip: &str, name: &str) -> String {
    let built = after(tip, "Builds from ");
    let owned = after(tip, "Part of ");
    let parts = match &built {
        Some(sentence) => tree_parts(sentence),
        None => {
            assert!(tip.contains("No build tree."), "{name} states no build tree: {tip}");
            Vec::new()
        }
    };
    let row = rows.iter().find(|row| row.name == name).unwrap_or_else(|| panic!("tsv row {name}"));
    let shown: Vec<String> = parts.iter().map(|part| part.0.clone()).collect();
    if !row.keep {
        assert_eq!(shown, row.parts, "{name}: tooltip components equal the tsv recipe");
    }
    for (part, _, subs) in &parts {
        let tsv_part = rows.iter().find(|r| &r.name == part).unwrap_or_else(|| panic!("tsv row {part}"));
        if !tsv_part.keep {
            assert_eq!(subs, &tsv_part.parts, "{name}: sub-tree of {part} equals the tsv recipe");
        }
        let cost = parts.iter().find(|p| &p.0 == part).map(|p| p.1).unwrap();
        assert_eq!(cost, tsv_cost(rows, part), "{name}: cost of {part} equals the tsv cost");
    }
    let want = tsv_parents(rows, name);
    let mut got: Vec<String> = owned
        .map(|sentence| sentence.split(", ").map(|item| name_and_cost(item).0).collect())
        .unwrap_or_default();
    got.sort();
    let missing: Vec<&String> = want.iter().filter(|w| !got.contains(w)).collect();
    assert!(missing.is_empty(), "{name}: every tsv recipe that uses it is listed as a parent: {missing:?}");
    let mut web_only = Vec::new();
    for extra in got.iter().filter(|g| !want.contains(g)) {
        let extra_row = rows.iter().find(|r| &r.name == extra).unwrap_or_else(|| panic!("tsv row {extra}"));
        assert!(
            extra_row.status == "not_in_688g" || extra_row.keep,
            "{name}: parent {extra} names a recipe no 6.88g row gives, so it must be a kept or web recipe"
        );
        web_only.push(format!("{extra}(web)"));
    }
    let mut shown_parents = want.clone();
    shown_parents.extend(web_only);
    let kept = if row.keep && !shown.is_empty() { " kept-reference" } else { "" };
    format!("components=[{}] parents=[{}]{kept}", shown.join(", "), shown_parents.join(", "))
}

fn held(game: &Game, unit: u32, item: u64) -> u32 {
    let inventory: Value = serde_json::from_str(&game.inventory_json(unit)).expect("inventory json");
    let kind = item + 1;
    inventory["slots"]
        .as_array()
        .into_iter()
        .chain(inventory["stash"].as_array())
        .flatten()
        .filter(|cell| cell[0] == kind)
        .map(|cell| cell[1].as_u64().unwrap_or(0) as u32)
        .sum()
}

fn position(game: &Game, unit: u32) -> (f32, f32) {
    let ids = game.unit_ids();
    let spots = game.units();
    let at = ids.iter().position(|&id| id == unit).expect("alive unit");
    (spots[at * 3], spots[at * 3 + 1])
}

fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

fn feedback_lines(lines: &[Value]) -> Vec<String> {
    lines.iter().map(|line| line.to_string()).filter(|text| text.contains("noCourier")).map(|text| text.chars().take(200).collect()).collect()
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

    fn act_ui(&mut self, command: &str, args: Value) -> Vec<Value> {
        let lines = ui(&mut self.game, command, args);
        self.absorb(&lines);
        self.pump(8);
        lines
    }

    fn open(&mut self, dir: &Path) {
        let key = read_ui(dir)["keys"]["shop"][0].as_str().expect("shop key").to_owned();
        let lines = call(&mut self.game, "key", json!({ "key": key, "phase": "down", "player": 1, "pubkey": OWNER }));
        self.absorb(&lines);
        self.pump(8);
        assert!(self.tree != Value::Null, "pressing the shop key opens the shop widget");
    }
}

fn walk_to(shop: &mut Shop, unit: u32, target: (f32, f32), ticks: usize, done: impl Fn(f32) -> bool) -> usize {
    assert!(shop.game.order_move(unit, target.0, target.1), "order_move accepted");
    for tick in 0..ticks {
        shop.pump(1);
        if !shop.game.unit_ids().contains(&unit) {
            panic!("unit {unit} is gone {tick} ticks into the walk to {target:?}");
        }
        if done(distance(position(&shop.game, unit), target)) {
            return tick;
        }
    }
    ticks
}

#[test]
fn off_hand_buy_routes_by_reach_and_courier() {
    let dir = scenario_dir();
    let image = image_for(&dir);
    let (mut game, heroes) = start_match(&dir, &image);
    let hero = own_hero(&game, &heroes, 1);
    call(&mut game, "select", json!({ "ids": [hero], "player": 1, "pubkey": OWNER }));
    call(&mut game, "selection", json!({ "ids": [hero], "player": 1, "pubkey": OWNER }));
    let catalog: Vec<Value> = serde_json::from_str(&game.item_catalog_json()).expect("catalog json");
    let index = |name: &str| catalog.iter().find(|item| item["name"] == name).expect(name)["id"].as_u64().expect("id");
    let tango = index("tango");
    let branches = index("branches");
    let clarity = index("clarity");
    let mut shop = Shop { game, tree: Value::Null };
    shop.open(&dir);

    let rows = tsv_rows(&dir);
    for name in ["branches", "magic_wand", "tango"] {
        let tip = tip_of(&mut shop, name);
        println!("shop_tree first_line item={name} \"{}\"", first_sentence(&tip));
        println!("shop_tree tree item={name} {}", check_tree(&rows, &tip, name));
    }
    let mut listed: Vec<String> = nodes_of(&shop.tree)
        .iter()
        .filter(|node| node["t"] == "cell" && node["call"] == "shop_pick")
        .filter_map(|node| kids_of(node).first().and_then(|kid| kid["text"].as_str()).map(str::to_owned))
        .collect();
    listed.sort();
    listed.dedup();
    let (mut web_only, mut no_tree, mut kept) = (0, 0, 0);
    for name in &listed {
        let line = check_tree(&rows, &tip_of(&mut shop, name), name);
        if line.starts_with("components=[] ") {
            no_tree += 1;
        }
        if line.contains("(web)") {
            web_only += 1;
        }
        if line.contains("kept-reference") {
            kept += 1;
        }
    }
    println!("shop_tree all checked={} web_only={web_only} kept_reference={kept} no_tree={no_tree}", listed.len());
    assert!(!listed.is_empty(), "the shop lists items to check");

    assert!(walk_to(&mut shop, hero, SHOP_HOME, 2000, |d| d < SHOP_REACH - 2.0) < 2000, "the hero reaches the home shop");
    shop.pump(8);
    let tip = tip_of(&mut shop, "tango");
    println!("shop_offhand case=1 hero=in-reach courier=present tip_reach=\"{}\"", after(&tip, "Buy from here: ").unwrap_or_default());
    let (gold0, held0) = (shop.game.gold(1), held(&shop.game, hero, tango));
    shop.act_ui("buy", json!({ "item": tango }));
    let (gold1, held1) = (shop.game.gold(1), held(&shop.game, hero, tango));
    println!("shop_offhand case=1 direct buy gold {gold0} -> {gold1}, hero holds tango {held0} -> {held1}");
    assert!(gold1 < gold0 && held1 > held0, "a buy with the hero in reach is direct");

    let courier = alive_units(&shop.game).iter().find(|unit| unit.player == 1 && unit.kind.starts_with("courier")).map(|unit| unit.id).expect("the player's courier");
    let hero_stop = walk_to(&mut shop, hero, FAR_HERO, 1500, |d| d < 0.5);
    let courier_stop = walk_to(&mut shop, courier, FAR_COURIER, 1500, |d| d < 0.5);
    println!("shop_offhand case=2 walk hero_ticks={hero_stop} courier_ticks={courier_stop}");
    shop.pump(8);
    let tip = tip_of(&mut shop, "branches");
    println!("shop_offhand case=2 hero=out-of-reach courier=present tip_reach=\"{}\"", after(&tip, "Buy from here: ").unwrap_or_default());
    assert!(after(&tip, "Buy from here: ").is_some_and(|reach| reach.contains("courier")), "out of reach, the tooltip routes the buy through the courier");
    let (gold0, hero0, courier0) = (shop.game.gold(1), held(&shop.game, hero, branches), held(&shop.game, courier, branches));
    ui(&mut shop.game, "buy", json!({ "item": branches }));
    let mut courier_at_shop = None;
    let mut delivered = None;
    for tick in 0..4000 {
        shop.pump(1);
        assert!(shop.game.unit_ids().contains(&courier), "the courier is alive at tick {tick}");
        if courier_at_shop.is_none() && held(&shop.game, courier, branches) > courier0 {
            courier_at_shop = Some(tick);
        }
        if delivered.is_none() && held(&shop.game, hero, branches) > hero0 {
            delivered = Some(tick);
            break;
        }
    }
    let (gold1, hero1) = (shop.game.gold(1), held(&shop.game, hero, branches));
    println!(
        "shop_offhand case=2 off-hand buy gold {gold0} -> {gold1}, courier bought at tick {:?}, hero holds branches {hero0} -> {hero1} at tick {:?}",
        courier_at_shop, delivered
    );
    assert!(delivered.is_some() && hero1 > hero0, "the courier fetches and delivers the item");
    assert!(courier_at_shop.is_some_and(|t| t > 0), "the courier walked to the shop before it bought");
    assert!(gold1 < gold0, "the purchase is paid from the player's gold");

    shop.game.remove_unit(courier);
    shop.pump(8);
    let (gold0, hero0) = (shop.game.gold(1), held(&shop.game, hero, clarity));
    let lines = shop.act_ui("buy", json!({ "item": clarity }));
    let refusal = feedback_lines(&lines);
    let (gold1, hero1) = (shop.game.gold(1), held(&shop.game, hero, clarity));
    println!("shop_offhand case=3 hero=out-of-reach courier=absent gold {gold0} -> {gold1}, hero holds clarity {hero0} -> {hero1}");
    println!("shop_offhand case=3 feedback {:?}", refusal);
    assert!(gold1 + 5 >= gold0 && hero1 == hero0, "without a courier the off-hand buy is refused (passive income is under 5 gold here)");
    assert!(!refusal.is_empty(), "the refusal is announced as noCourier");

    walk_to(&mut shop, hero, SHOP_HOME, 2000, |d| d < SHOP_REACH - 2.0);
    shop.pump(8);
    let (gold0, hero0) = (shop.game.gold(1), held(&shop.game, hero, tango));
    shop.act_ui("buy", json!({ "item": tango }));
    let (gold1, hero1) = (shop.game.gold(1), held(&shop.game, hero, tango));
    println!("shop_offhand case=4 hero=in-reach courier=absent direct buy gold {gold0} -> {gold1}, hero holds tango {hero0} -> {hero1}");
    assert!(gold1 < gold0 && hero1 > hero0, "a buy with the hero in reach needs no courier");

    let expensive = index("ultimate_scepter");
    let gem = index("gem");
    shop.act_ui("shop_pick", json!({ "item": expensive, "alt": true }));
    let note = texts_of(&shop.tree).into_iter().find(|text| text.contains("more for this item")).expect("the alt-click shortfall note");
    println!("shop_alt case=5 alt-click item=ultimate_scepter gold={} note=\"{note}\"", shop.game.gold(1));
    assert!(note.starts_with("Needs ") && note.ends_with("more for this item."), "alt-click names the shortfall: {note}");
    shop.act_ui("shop_pick", json!({ "item": expensive, "alt": true, "ctrl": true }));
    let with_buyback = texts_of(&shop.tree).into_iter().find(|text| text.contains("buyback (")).expect("the ctrl+alt buyback note");
    println!("shop_alt case=6 ctrl+alt item=ultimate_scepter note=\"{with_buyback}\"");
    shop.act_ui("build_toggle", json!({ "item": gem }));
    let out_of_stock = texts_of(&shop.tree).into_iter().find(|text| text.starts_with("gem ") && text.contains("Restocks in")).expect("the quick buy entry for gem");
    println!("shop_stock case=7 quick buy entry=\"{out_of_stock}\"");
    let left = tip_of(&mut shop, "blood_grenade");

    println!("shop_stock case=8 blood_grenade tooltip has \"{}\"", after(&left, "Stock left:").map(|rest| format!("Stock left:{rest}")).unwrap_or_default());
    assert!(left.contains("Stock left: 2."), "limited stock shows the remaining quantity: {left}");

    let doc = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../docs/SHOP.md")).expect("read docs/SHOP.md");
    for rule in RULES {
        assert!(doc.contains(rule), "docs/SHOP.md names {rule}");
        println!("shop_doc rule={rule} named");
    }
}
