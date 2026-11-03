use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use web::Game;

const OWNER: &str = "probe-owner";
const KILLER: u8 = 0;
const LOCAL: u8 = 1;
const START_STEPS: usize = 60;
const KILL_STEPS: usize = 800;
const VICTIM_HP: i32 = 60;
const SPAWN_STEP: f32 = 48.0;
const FIGHT_GAP_TICKS: usize = 300;
const TIMED_HIDE_TICKS: usize = 800;
const GOLD_LOST: &str = "st_gold_lost";

struct Unit {
    id: u32,
    player: u8,
    kind: String,
    kind_index: u32,
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
    call(game, "ui", json!({ "call": command, "args": args, "player": LOCAL, "pubkey": OWNER }))
}

fn step(game: &mut Game, lines: &mut Vec<Value>, ticks: usize) {
    for _ in 0..ticks {
        let outcome = game.tick();
        assert!(outcome.is_empty(), "tick: {outcome}");
        lines.extend(drain(game));
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
            kind_index: types[i],
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

fn start_match(dir: &Path, image: &[u8]) -> (Game, Vec<String>) {
    let mut game = Game::new(image).expect("game");
    game.set_fog(false);
    load_pack(&mut game, dir);
    let ui_file: Value = serde_json::from_slice(&std::fs::read(dir.join("moba_ui.json")).expect("read moba_ui.json")).expect("moba_ui.json");
    call(&mut game, "setup", json!({ "ui": ui_file, "player": LOCAL, "pubkey": OWNER }));
    for _ in 0..60 {
        let outcome = game.tick();
        assert!(outcome.is_empty(), "tick: {outcome}");
        drain(&mut game);
    }
    ui(&mut game, "claim", json!({ "slot": 0 }));
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
    for seat in 0..10 {
        ui(&mut game, "ready", json!({ "slot": seat, "ready": true }));
    }
    for step_index in 0..START_STEPS {
        if step_index % 3 == 0 {
            ui(&mut game, "start", json!({}));
        }
        for _ in 0..100 {
            let outcome = game.tick();
            assert!(outcome.is_empty(), "tick: {outcome}");
            drain(&mut game);
        }
        if alive_units(&game).iter().any(|unit| heroes.contains(&unit.kind)) {
            break;
        }
    }
    for player in 0..10usize {
        let hero = heroes[(20 + player) % heroes.len()].clone();
        call(&mut game, "seat_pick", json!({ "player": player, "hero": hero }));
    }
    for _ in 0..200 {
        let outcome = game.tick();
        assert!(outcome.is_empty(), "tick: {outcome}");
        drain(&mut game);
    }
    (game, heroes)
}

fn hero_of(game: &Game, heroes: &[String], player: u8) -> Unit {
    alive_units(game)
        .into_iter()
        .find(|unit| unit.player == player && heroes.contains(&unit.kind))
        .expect("hero of player")
}

fn spot_of(game: &Game, unit: u32) -> (f32, f32) {
    let index = game.unit_ids().iter().position(|&id| id == unit).expect("unit is alive");
    let spots = game.units();
    (spots[index * 3], spots[index * 3 + 1])
}

fn clear_world(game: &mut Game, heroes: &[String], keep: &[u32]) {
    for unit in alive_units(game) {
        if !heroes.contains(&unit.kind) && !keep.contains(&unit.id) {
            game.remove_unit(unit.id);
        }
    }
}

fn spawn_beside(game: &mut Game, kind: u32, player: u8, killer: u32) -> u32 {
    let (killer_x, killer_z) = spot_of(game, killer);
    for (dx, dy) in [(0.0, 0.0), (SPAWN_STEP, 0.0), (-SPAWN_STEP, 0.0), (0.0, SPAWN_STEP), (0.0, -SPAWN_STEP), (2.0 * SPAWN_STEP, 0.0), (-2.0 * SPAWN_STEP, 0.0)] {
        let index = game.spawn_attributed(player, kind, killer_x + dx, killer_z + dy);
        if index >= 0 {
            return index as u32;
        }
    }
    panic!("a hero of player {player} spawns beside the killer");
}

fn kill(game: &mut Game, lines: &mut Vec<Value>, heroes: &[String], killer: u32, victim: u32) {
    let hp = game.unit_hp(victim);
    if hp > VICTIM_HP {
        game.hurt(victim, hp - VICTIM_HP);
    }
    for _ in 0..KILL_STEPS {
        clear_world(game, heroes, &[killer, victim]);
        game.attack(killer, victim);
        step(game, lines, 8);
        if game.deaths_of(victim) > 0 {
            return;
        }
    }
    panic!("the victim dies within {KILL_STEPS} steps");
}

fn kill_counts(game: &Game, heroes: &[String]) -> Vec<(u32, String, u32)> {
    alive_units(game)
        .into_iter()
        .filter(|unit| heroes.contains(&unit.kind))
        .map(|unit| (unit.id, unit.kind.clone(), game.kills_of(unit.id)))
        .collect()
}

fn credited_kind(before: &[(u32, String, u32)], after: &[(u32, String, u32)]) -> Option<String> {
    after
        .iter()
        .find(|(id, _, kills)| before.iter().any(|(was, _, was_kills)| was == id && kills > was_kills))
        .map(|(_, kind, _)| kind.clone())
}

fn wait(game: &mut Game, lines: &mut Vec<Value>, heroes: &[String], ticks: usize, keep: &[u32]) {
    for _ in 0..ticks / 8 {
        clear_world(game, heroes, keep);
        step(game, lines, 8);
    }
}

fn find_call<'a>(node: &'a Value, call: &str) -> Option<&'a Value> {
    if node["call"] == call {
        return Some(node);
    }
    node["kids"].as_array().into_iter().flatten().find_map(|kid| find_call(kid, call))
}

fn shown_tree<'a>(lines: &'a [Value], id: &str) -> Option<&'a Value> {
    lines
        .iter()
        .rev()
        .find(|line| line["kind"] == "widget" && line["data"]["id"] == id)
        .and_then(|line| (line["data"]["remove"] != true).then(|| &line["data"]["tree"]))
}

fn texts(node: &Value, out: &mut Vec<String>) {
    if let Some(text) = node["text"].as_str() {
        out.push(text.to_owned());
    }
    for kid in node["kids"].as_array().into_iter().flatten() {
        texts(kid, out);
    }
}

fn texts_of(node: &Value) -> Vec<String> {
    let mut out = Vec::new();
    texts(node, &mut out);
    out
}

fn find_grid(node: &Value) -> Option<&Value> {
    if node["t"] == "grid" {
        return Some(node);
    }
    node["kids"].as_array().into_iter().flatten().find_map(find_grid)
}

fn number_after(parts: &[&str], keyword: &str) -> i64 {
    let at = parts.iter().position(|part| *part == keyword).expect("keyword in the side line");
    parts[at + 1].trim_end_matches(',').parse().expect("side line number")
}

#[test]
fn a_two_death_fight_and_a_single_death_show_their_recaps() {
    let dir = scenario_dir();
    let image = image_for(&dir);
    let (mut game, heroes) = start_match(&dir, &image);
    let mut lines: Vec<Value> = Vec::new();

    let killer = hero_of(&game, &heroes, KILLER).id;
    let enemies: Vec<u8> = (2..10u8).filter(|&p| game.hostile(u32::from(KILLER), u32::from(p))).take(2).collect();
    assert_eq!(enemies.len(), 2, "two enemy seats other than the local seat");
    assert!(game.hostile(u32::from(KILLER), u32::from(LOCAL)), "the local seat is an enemy of the killer");
    let victim_kinds: Vec<u32> = enemies.iter().map(|&p| hero_of(&game, &heroes, p).kind_index).collect();
    let local_original = hero_of(&game, &heroes, LOCAL);
    let local_kind = local_original.kind_index;
    game.remove_unit(local_original.id);
    let type_names = game.unit_type_names();
    let victim_names: Vec<String> = victim_kinds.iter().map(|&kind| type_names[kind as usize].clone()).collect();
    let local_name = type_names[local_kind as usize].clone();

    wait(&mut game, &mut lines, &heroes, FIGHT_GAP_TICKS, &[killer]);

    let first = spawn_beside(&mut game, victim_kinds[0], enemies[0], killer);
    let before = kill_counts(&game, &heroes);
    kill(&mut game, &mut lines, &heroes, killer, first);
    let first_killer = credited_kind(&before, &kill_counts(&game, &heroes)).expect("a hero is credited with the first kill");
    assert!(shown_tree(&lines, "recap").is_none(), "one death shows no teamfight recap");
    let fight_start = game.ticks();

    let second = spawn_beside(&mut game, victim_kinds[1], enemies[1], killer);
    let before = kill_counts(&game, &heroes);
    kill(&mut game, &mut lines, &heroes, killer, second);
    let second_killer = credited_kind(&before, &kill_counts(&game, &heroes)).expect("a hero is credited with the second kill");
    let gap = game.ticks() - fight_start;
    println!("probe fight window_ticks={gap} first_death_tick={fight_start} second_death_tick={}", game.ticks());
    assert!(gap <= 240, "the two deaths fall inside the 15 s window");

    step(&mut game, &mut lines, 8);
    let fight = shown_tree(&lines, "recap").expect("two deaths show the teamfight recap");
    let fight_texts = texts_of(fight);
    println!("probe teamfight title={} lines={:?}", fight["title"], fight_texts);
    let cells = texts_of(find_grid(fight).expect("teamfight grid"));
    println!("probe teamfight header={:?}", &cells[..3]);
    let death_rows: Vec<&[String]> = cells[3..].chunks(3).collect();
    for row in &death_rows {
        println!("probe teamfight death_row time={} died={} killed_by={}", row[0], row[1], row[2]);
    }
    assert_eq!(death_rows.len(), 2, "one row per death in the fight");
    let killers = [&first_killer, &second_killer];
    for (i, row) in death_rows.iter().enumerate() {
        assert!(row[0].contains(':'), "the time cell is m:ss");
        assert_eq!(row[1], victim_names[i], "the died cell is the victim hero");
        assert_eq!(&row[2], killers[i], "the killed-by cell is the hero the sim credits with the kill");
    }

    let side_lines: Vec<&String> = fight_texts.iter().filter(|text| text.contains(" gained ")).collect();
    assert_eq!(side_lines.len(), 2, "one gold line per team");
    let mut lost_total = 0;
    let mut nets = Vec::new();
    for line in &side_lines {
        println!("probe teamfight side_line={line}");
        let parts: Vec<&str> = line.split_whitespace().collect();
        lost_total += number_after(&parts, "lost");
        nets.push(number_after(&parts, "net"));
    }
    let victim_losses: i64 = [first, second].iter().map(|&v| game.counter(v, GOLD_LOST) as i64).sum();
    assert_eq!(lost_total, victim_losses, "the teams' lost gold is the two victims' gold lost");

    let swing_line = fight_texts.iter().find(|text| text.starts_with("Gold swing:")).expect("gold swing line");
    println!("probe teamfight swing={swing_line} nets={nets:?}");
    let swing_amount: i64 = swing_line.rsplit('+').next().expect("swing amount").parse().expect("swing number");
    assert_eq!(swing_amount, (nets[0] - nets[1]).abs(), "the swing is the difference of the teams' net gold");
    assert!(swing_amount > 0, "the killing team gains gold from the fight");

    let mut changed_at = game.ticks();
    let mut last_text = fight_texts.clone();
    let mut hidden_at = None;
    for _ in 0..TIMED_HIDE_TICKS {
        let mark = lines.len();
        step(&mut game, &mut lines, 1);
        for line in lines[mark..].iter().filter(|line| line["kind"] == "widget" && line["data"]["id"] == "recap") {
            if line["data"]["remove"] == true {
                hidden_at = Some(game.ticks());
            } else {
                let text = texts_of(&line["data"]["tree"]);
                if text != last_text {
                    last_text = text;
                    changed_at = game.ticks();
                }
            }
        }
        if hidden_at.is_some() {
            break;
        }
    }
    let hidden_at = hidden_at.expect("the teamfight recap hides on its own after its show window");
    let hide_gap = hidden_at - changed_at;
    println!("probe teamfight timed_hide after_last_death_ticks={hide_gap}");
    assert!((120..=140).contains(&hide_gap), "the recap hides about 8 s after its last death");

    let third = spawn_beside(&mut game, victim_kinds[0], enemies[0], killer);
    let third_killer_before = kill_counts(&game, &heroes);
    kill(&mut game, &mut lines, &heroes, killer, third);
    let fourth = spawn_beside(&mut game, victim_kinds[1], enemies[1], killer);
    kill(&mut game, &mut lines, &heroes, killer, fourth);
    let _ = third_killer_before;
    step(&mut game, &mut lines, 8);
    let second_fight = shown_tree(&lines, "recap").expect("a second two-death fight shows the teamfight recap");
    let close = find_call(second_fight, "recap_close").expect("the teamfight recap has a close button");
    let fight_id = close["args"]["fight"].as_u64().expect("close button names the fight");
    let closed = ui(&mut game, "recap_close", json!({ "fight": fight_id }));
    let removed = closed.iter().any(|line| line["kind"] == "widget" && line["data"]["id"] == "recap" && line["data"]["remove"] == true);
    lines.extend(closed);
    println!("probe teamfight close_button fight={fight_id} removed_on_click={removed}");
    assert!(removed, "the close button removes the teamfight recap");
    assert!(shown_tree(&lines, "recap").is_none(), "the closed teamfight recap stays hidden");

    let single = spawn_beside(&mut game, local_kind, LOCAL, killer);
    let before = kill_counts(&game, &heroes);
    kill(&mut game, &mut lines, &heroes, killer, single);
    let single_killer = credited_kind(&before, &kill_counts(&game, &heroes)).expect("a hero is credited with the single kill");
    step(&mut game, &mut lines, 8);
    if let Some(tree) = shown_tree(&lines, "recap") {
        assert!(!texts_of(tree).contains(&local_name), "the single death is not a teamfight row");
    }

    let select = shown_tree(&lines, "select").expect("the respawn screen is shown");
    let death_texts = texts_of(select);
    println!("probe death_recap lines={death_texts:?}");
    let killed_by = death_texts.iter().find(|t| t.starts_with("Killed by ")).expect("killed by field");
    let source = death_texts.iter().find(|t| t.starts_with("Damage by source")).expect("damage by source field");
    let hit_by = death_texts.iter().find(|t| t.starts_with("Hit by ")).expect("hit by field");
    let gold_lost = death_texts.iter().find(|t| t.starts_with("Gold lost ")).expect("gold lost field");
    println!("probe death_recap killed_by={killed_by:?} source={source:?} hit_by={hit_by:?} gold_lost={gold_lost:?}");
    assert_eq!(killed_by, &format!("Killed by {single_killer}"), "the death recap names the killer");
    assert_eq!(source, "Damage by source: not recorded: the sim keeps no per-source totals", "the source is reported unknown");
    assert!(hit_by.contains(&single_killer), "the killer is among the hitters");
    let expected_loss = game.counter(single, GOLD_LOST) as i64;
    assert_eq!(gold_lost, &format!("Gold lost {expected_loss}g"), "the gold lost is the sim's st_gold_lost of the death");
}
