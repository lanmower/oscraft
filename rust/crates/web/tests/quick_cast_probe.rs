use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use web::Game;

const OWNER: &str = "probe-owner";
const LOCAL: u64 = 1;
const PANEL_REFRESH_TICKS: usize = 5;

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

fn refusal_of(lines: &[Value]) -> Option<String> {
    lines
        .iter()
        .rev()
        .find(|line| line["kind"] == "feedback" && line["data"]["kind"] == "refusal" && line["data"]["player"] == LOCAL)
        .and_then(|line| line["data"]["reason"].as_str().map(str::to_owned))
}

fn arm_of(lines: &[Value], slot: u64) -> Option<Value> {
    lines
        .iter()
        .rev()
        .find(|line| line["kind"] == "feedback" && line["data"]["kind"] == "targetArm" && line["data"]["slot"] == slot)
        .map(|line| line["data"].clone())
}

fn ended(lines: &[Value]) -> bool {
    lines.iter().any(|line| line["kind"] == "feedback" && line["data"]["kind"] == "targetEnd")
}

fn body_of(line: &Value) -> &Value {
    if line["data"].is_object() { &line["data"] } else { line }
}

fn setting_of(lines: &[Value], slot: u64) -> Option<i64> {
    let key = format!("quickcast_{slot}");
    lines
        .iter()
        .rev()
        .find(|line| line["kind"] == "setting" && body_of(line)["key"] == key)
        .and_then(|line| body_of(line)["value"].as_i64())
}

fn networth_widget(lines: &[Value]) -> Option<&Value> {
    lines
        .iter()
        .rev()
        .find(|line| line["kind"] == "widget" && line["data"]["id"] == "networth")
        .filter(|line| line["data"]["remove"] != true)
        .map(|line| &line["data"]["tree"])
}

fn networth_removed(lines: &[Value]) -> bool {
    lines.iter().any(|line| line["kind"] == "widget" && line["data"]["id"] == "networth" && line["data"]["remove"] == true)
}

fn bank_value(cells: &Value, costs: &[i64]) -> i64 {
    cells
        .as_array()
        .expect("bank cells")
        .iter()
        .filter(|cell| cell[0].as_u64().unwrap_or(0) > 0)
        .map(|cell| costs[cell[0].as_u64().expect("kind") as usize - 1] * cell[1].as_i64().expect("count"))
        .sum()
}

fn strings_of(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(text) => out.push(text.clone()),
        Value::Array(items) => items.iter().for_each(|item| strings_of(item, out)),
        Value::Object(map) => map.values().for_each(|item| strings_of(item, out)),
        _ => {}
    }
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

fn press(game: &mut Game, key: &str) -> Vec<Value> {
    call(game, "key", json!({ "key": key, "phase": "down", "player": LOCAL, "pubkey": OWNER }))
}

fn release_at(game: &mut Game, key: &str, at: (f64, f64)) -> Vec<Value> {
    call(game, "key", json!({ "key": key, "phase": "up", "player": LOCAL, "pubkey": OWNER, "world": { "x": at.0, "z": at.1 } }))
}

fn position_of(game: &Game, unit: u32) -> (f64, f64) {
    let ids = game.unit_ids();
    let spots = game.units();
    let i = ids.iter().position(|&id| id == unit).expect("the unit is alive");
    (f64::from(spots[i * 3]), f64::from(spots[i * 3 + 1]))
}

fn ability_keys(dir: &Path) -> Vec<(String, String)> {
    std::fs::read_to_string(dir.join("ability_keys.lua"))
        .expect("read ability_keys.lua")
        .lines()
        .filter_map(|line| {
            let (name, key) = line.trim().trim_end_matches(',').split_once(" = ")?;
            Some((name.to_owned(), key.trim_matches('"').to_owned()))
        })
        .collect()
}

fn aim_at(at: (f64, f64)) -> (f64, f64) {
    (at.0 + 2.0, at.1)
}

fn targeted_slot(game: &mut Game, dir: &Path, hero: u32) -> Option<(String, u64)> {
    let keys = ability_keys(dir);
    let mut abilities: Vec<Value> = serde_json::from_str(&game.abilities_json(hero)).expect("abilities json");
    abilities.sort_by_key(|ability| ability["target"] != "point");
    for ability in abilities {
        if ability["target"] == "none" {
            continue;
        }
        let slot = ability["slot"].as_u64().expect("slot");
        let name = ability["name"].as_str().expect("name");
        let key = match keys.iter().find(|(known, _)| known == name) {
            Some((_, key)) => key.clone(),
            None => ["q", "w", "e", "r"].get(slot as usize).copied().unwrap_or_default().to_owned(),
        };
        let lines = press(game, &key);
        if arm_of(&lines, slot).is_some() {
            press(game, "escape");
            return Some((key, slot));
        }
        advance(game, 1);
    }
    None
}

#[test]
fn quick_cast_press_and_release_casts_at_the_cursor() {
    let dir = scenario_dir();
    let image = image_for(&dir);
    let (mut game, hero, _) = start_match(&dir, &image);
    let (key, slot) = targeted_slot(&mut game, &dir, hero).expect("the hero has a targeted skill");
    assert!(game.skill_up(hero, slot as u32), "the hero learns the targeted skill");
    advance(&mut game, 1);

    let on = ui(&mut game, "quickcast", json!({ "slot": slot }));
    println!("quick_cast setting after toggle slot={slot} value={:?}", setting_of(&on, slot));
    assert_eq!(setting_of(&on, slot), Some(1), "the toggle turns quick cast on");

    let down = press(&mut game, &key);
    let arm = arm_of(&down, slot).expect("a quick press arms the skill");
    println!("quick_cast press key={key} slot={slot} range={} caster={}", arm["range"], arm["caster"]);
    assert!(arm["range"].as_f64().unwrap_or(0.0) > 0.0, "the quick press shows a cast range");
    assert_eq!(arm["caster"], json!(hero), "the range is centred on the hero");

    let at = aim_at(position_of(&game, hero));
    let up = release_at(&mut game, &key, at);
    println!("quick_cast release key={key} ended={} refusal={:?}", ended(&up), refusal_of(&up));
    assert_eq!(refusal_of(&up), None, "the release does not refuse");
    assert!(ended(&up), "the release ends the arming");
    advance(&mut game, 5);
    let cooldown = game.ability_cooldown(hero, slot as u32);
    println!("quick_cast cooldown_after_release={cooldown} sim_refusal={:?}", game.last_refusal());
    assert!(cooldown > 0, "the release cast the skill");
}

#[test]
fn normal_press_arms_and_waits_for_a_click() {
    let dir = scenario_dir();
    let image = image_for(&dir);
    let (mut game, hero, _) = start_match(&dir, &image);
    let (key, slot) = targeted_slot(&mut game, &dir, hero).expect("the hero has a targeted skill");
    assert!(game.skill_up(hero, slot as u32), "the hero learns the targeted skill");
    advance(&mut game, 1);

    let down = press(&mut game, &key);
    let arm = arm_of(&down, slot).expect("a normal press arms the skill");
    println!("normal_press key={key} slot={slot} range={}", arm["range"]);
    assert!(arm["range"].is_null(), "a normal press shows no range");

    let at = aim_at(position_of(&game, hero));
    let up = release_at(&mut game, &key, at);
    println!("normal_press release ended={}", ended(&up));
    assert!(!ended(&up), "the release leaves the normal arming in place");
    advance(&mut game, 5);
    assert_eq!(game.ability_cooldown(hero, slot as u32), 0, "nothing is cast before a click");

    let (hx, hz) = aim_at(position_of(&game, hero));
    call(&mut game, "pointer", json!({ "kind": "down", "x": hx, "y": hz, "button": 0, "player": LOCAL, "pubkey": OWNER }));
    let click = call(&mut game, "pointer", json!({ "kind": "up", "x": hx, "y": hz, "button": 0, "player": LOCAL, "pubkey": OWNER }));
    println!("normal_press click ended={} refusal={:?}", ended(&click), refusal_of(&click));
    advance(&mut game, 5);
    let cooldown = game.ability_cooldown(hero, slot as u32);
    println!("normal_press cooldown_after_click={cooldown} sim_refusal={:?}", game.last_refusal());
    assert!(cooldown > 0, "a click after a normal press casts the skill");
}

#[test]
fn net_worth_shows_both_teams_while_k_is_held() {
    let dir = scenario_dir();
    let image = image_for(&dir);
    let (mut game, _hero, seated) = start_match(&dir, &image);
    let teams: Vec<u64> = read_ui(&dir)["teams"].as_array().expect("teams").iter().map(|t| t.as_u64().expect("team")).collect();
    let catalog: Value = serde_json::from_str(&game.item_catalog_json()).expect("catalog json");
    let costs: Vec<i64> = catalog.as_array().expect("catalog rows").iter().map(|row| row["cost"].as_i64().expect("cost")).collect();
    let mut gold = [0i64; 2];
    let mut worth = [0i64; 2];

    let mut down = press(&mut game, "k");
    advance(&mut game, PANEL_REFRESH_TICKS);
    down.extend(drain(&mut game));
    let unit_ids = game.unit_ids();
    let unit_types = game.unit_types();
    let type_names = game.unit_type_names();
    for player in 0..10usize {
        let hero = unit_ids
            .iter()
            .zip(&unit_types)
            .find(|(_, kind)| type_names[**kind as usize] == seated[player])
            .map(|(id, _)| *id)
            .expect("the seated hero is alive");
        let inventory: Value = serde_json::from_str(&game.inventory_json(hero)).expect("inventory json");
        let items = bank_value(&inventory["slots"], &costs);
        let stash = bank_value(&inventory["stash"], &costs);
        let cash = i64::from(game.gold(player as u8));
        let side = teams[player] as usize;
        gold[side] += cash;
        worth[side] += cash + items + stash;
        println!("net_worth player={player} team={side} hero={} gold={cash} items={items} stash={stash} worth={}", seated[player], cash + items + stash);
    }
    println!("net_worth per-team gold={gold:?} worth={worth:?}");
    let tree = networth_widget(&down).expect("holding k shows the net worth widget");
    let mut texts = Vec::new();
    strings_of(tree, &mut texts);
    println!("net_worth key_down gold={gold:?} texts={texts:?}");
    for (side, name) in [(0usize, "Sentinel"), (1, "Scourge")] {
        let shown: i64 = texts
            .iter()
            .find(|text| text.contains(name))
            .and_then(|text| text.rsplit_once("  ").and_then(|(_, n)| n.parse().ok()))
            .unwrap_or_else(|| panic!("side {side} shows a net worth"));
        assert_eq!(shown, worth[side], "side {side} widget net worth is its players' gold plus items plus stash");
    }

    let mut up = call(&mut game, "key", json!({ "key": "k", "phase": "up", "player": LOCAL, "pubkey": OWNER }));
    advance(&mut game, PANEL_REFRESH_TICKS);
    up.extend(drain(&mut game));
    println!("net_worth key_up removed={}", networth_removed(&up));
    assert!(networth_removed(&up), "releasing k hides the widget");
}
