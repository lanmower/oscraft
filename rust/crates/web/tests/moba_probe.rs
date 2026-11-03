use std::path::{Path, PathBuf};

use replay::Recording;
use serde_json::{json, Value};
use web::Game;

const OWNER: &str = "probe-owner";

struct Unit {
    id: u32,
    player: u8,
    kind: String,
}

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

fn advance(game: &mut Game, ticks: usize) {
    for _ in 0..ticks {
        let outcome = game.tick();
        assert!(outcome.is_empty(), "tick: {outcome}");
    }
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

fn label_texts(node: &Value, out: &mut Vec<String>) {
    if let Some(text) = node.get("text").and_then(Value::as_str) {
        out.push(text.to_owned());
    }
    if let Some(kids) = node.get("kids").and_then(Value::as_array) {
        for kid in kids {
            label_texts(kid, out);
        }
    }
}

fn win_widget(lines: &[Value]) -> Option<String> {
    lines
        .iter()
        .rev()
        .find(|value| value["kind"] == "widget" && value["data"]["id"] == "win")
        .map(|value| {
            let mut texts = Vec::new();
            label_texts(&value["data"]["tree"], &mut texts);
            texts.join(" | ")
        })
}

fn print_win(label: &str, lines: &[Value]) {
    println!("probe {label}: ui_lines={}", lines.len());
    for value in lines {
        match value["kind"].as_str() {
            Some("widget") if value["data"]["id"] == "win" => {
                let mut texts = Vec::new();
                label_texts(&value["data"]["tree"], &mut texts);
                println!("probe {label} win_widget: {}", texts.join(" | "));
            }
            Some("panel") if value["id"] == "match_stats" => {
                let rows = value["rows"].as_array().cloned().unwrap_or_default();
                println!("probe {label} match_stats_rows={}", rows.len());
                for row in rows {
                    println!("probe {label} match_stats_row: {row}");
                }
            }
            Some("win") => println!("probe {label} win_action: {}", value["data"]["action"]),
            _ => {}
        }
    }
}

fn start_match(dir: &Path, image: &[u8]) -> (Game, Vec<String>) {
    let mut game = Game::new(image).expect("game");
    game.set_fog(false);
    load_pack(&mut game, dir);
    let ui_file: Value = serde_json::from_slice(&std::fs::read(dir.join("moba_ui.json")).expect("read moba_ui.json")).expect("moba_ui.json");
    call(&mut game, "setup", json!({ "ui": ui_file, "player": 1, "pubkey": OWNER }));
    advance(&mut game, 60);
    ui(&mut game, "claim", json!({ "slot": 0 }));
    let fill = ui(&mut game, "fill", json!({}));
    println!("probe fill error=\"{}\"", lobby_error(&fill));
    let heroes = allowed_heroes(&fill);
    println!("probe allowed_heroes={}", heroes.len());
    let mut own_hero = String::new();
    for hero in &heroes {
        let lines = ui(&mut game, "pick", json!({ "slot": 0, "hero": hero }));
        let error = lobby_error(&lines);
        if error.is_empty() {
            own_hero = hero.clone();
            break;
        }
        println!("probe pick seat=0 hero={hero} error=\"{error}\"");
    }
    println!("probe own_hero={own_hero}");
    let world = ui(&mut game, "world_ready", json!({}));
    println!("probe world_ready error=\"{}\" ui_lines={}", lobby_error(&world), world.len());
    for seat in 0..10 {
        let lines = ui(&mut game, "ready", json!({ "slot": seat, "ready": true }));
        println!("probe ready seat={seat} error=\"{}\"", lobby_error(&lines));
    }
    for step in 0..60 {
        if step % 3 == 0 {
            let lines = ui(&mut game, "start", json!({}));
            println!("probe start step={step} error=\"{}\" ui_lines={}", lobby_error(&lines), lines.len());
        }
        advance(&mut game, 100);
        if alive_units(&game).iter().any(|unit| heroes.contains(&unit.kind)) {
            println!("probe heroes_spawned at step={step}");
            break;
        }
    }
    for player in 0..10usize {
        let hero = heroes[(20 + player) % heroes.len()].clone();
        call(&mut game, "seat_pick", json!({ "player": player, "hero": hero }));
    }
    advance(&mut game, 200);
    (game, heroes)
}

fn hero_of(units: &[Unit], heroes: &[String], player: u8) -> u32 {
    units
        .iter()
        .find(|unit| unit.player == player && heroes.contains(&unit.kind))
        .map(|unit| unit.id)
        .expect("hero of player")
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

fn match_stats_rows(lines: &[Value]) -> Vec<Vec<String>> {
    lines
        .iter()
        .rev()
        .find(|value| value["kind"] == "panel" && value["id"] == "match_stats")
        .map(|value| {
            value["rows"]
                .as_array()
                .expect("match_stats rows")
                .iter()
                .map(|row| row.as_array().expect("cells").iter().map(|cell| cell.as_str().unwrap_or_default().to_owned()).collect())
                .collect()
        })
        .unwrap_or_default()
}

fn stat_cell(row: &[String], column: usize) -> i64 {
    row.get(column).and_then(|cell| cell.parse().ok()).unwrap_or(0)
}

#[test]
fn probe_win_screen() {
    let dir = scenario_dir();
    let image = image_for(&dir);
    let (mut game, heroes) = start_match(&dir, &image);
    let units = alive_units(&game);
    let killer = hero_of(&units, &heroes, 0);
    let victim = hero_of(&units, &heroes, 1);
    let ally = units
        .iter()
        .find(|unit| unit.player != 0 && heroes.contains(&unit.kind) && !game.hostile(0, u32::from(unit.player)))
        .map(|unit| unit.id)
        .expect("allied hero");
    let lifesteal = lifesteal_item(&game);
    game.give_item(killer, lifesteal, 1);
    game.hurt(killer, 100);
    game.attack(killer, victim);
    game.attack(ally, victim);
    for _ in 0..800 {
        for unit in alive_units(&game) {
            if (unit.kind.contains("creep") || unit.kind.contains("tower")) && unit.id != killer && unit.id != ally && unit.id != victim {
                game.remove_unit(unit.id);
            }
        }
        advance(&mut game, 2);
        let victim_hp = game.unit_hp(victim);
        if victim_hp > 40 {
            game.hurt(victim, victim_hp - 30);
        }
        if game.kills_of(killer) > 0 {
            break;
        }
    }
    println!("probe hero_kill killer={killer} victim={victim} ally={ally} kills={}", game.kills_of(killer));
    let units = alive_units(&game);
    let count = |p: u8| units.iter().filter(|u| u.player == p).count();
    println!("probe units={} p0={} p1={}", units.len(), count(0), count(1));
    for player in 0..10u8 {
        let owned: Vec<&str> = units.iter().filter(|u| u.player == player && heroes.contains(&u.kind)).map(|u| u.kind.as_str()).collect();
        println!("probe seat p{player} heroes={owned:?}");
    }
    let player0: Vec<u32> = units.iter().filter(|u| u.player == 0).map(|u| u.id).collect();
    for unit in &player0 {
        game.remove_unit(*unit);
    }
    advance(&mut game, 30);
    let match_end = drain(&mut game);
    print_win("match_end", &match_end);
    let stats_rows = match_stats_rows(&match_end);
    assert!(stats_rows.iter().any(|row| stat_cell(row, 2) >= 1), "a kill shows in the K column");
    assert!(stats_rows.iter().any(|row| stat_cell(row, 4) >= 1), "an assist shows in the A column");
    println!("probe winner {}", game.winner());
    let frozen = win_widget(&match_end).expect("win widget at match end");
    advance(&mut game, 200);
    let later = drain(&mut game);
    print_win("after_200", &later);
    let after = win_widget(&later);
    let unchanged = after.as_ref().is_none_or(|text| *text == frozen);
    println!("probe after_200 widget_unchanged={unchanged}");
    assert!(unchanged, "win widget changed after match end");
    print_win("refresh", &ui(&mut game, "win_refresh", json!({})));
    print_win("save_replay", &ui(&mut game, "win_save_replay", json!({})));
    let replay = game.replay_text();
    println!("probe replay_text bytes={} head=\"{}\"", replay.len(), replay.lines().next().unwrap_or_default());
    Recording::parse(&replay).expect("replay_text parses");
    print_win("replay_saved", &ui(&mut game, "replay_saved", json!({ "name": "oscraft-replay-probe.replay" })));
    print_win("play_again", &ui(&mut game, "win_play_again", json!({})));
    print_win("new_game", &ui(&mut game, "win_new_game", json!({})));
    println!("probe removed_units={}", player0.len());
}
