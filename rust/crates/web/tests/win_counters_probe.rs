use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use web::Game;

const OWNER: &str = "probe-owner";
const EARNED: &str = "st_gold_earned";
const KILLER: u8 = 0;
const VICTIM: u8 = 1;
const COL_KILLS: usize = 2;
const COL_ASSISTS: usize = 4;
const COL_GOLD: usize = 7;
const COL_XP: usize = 8;
const COL_DEATHS: usize = 3;
const COL_LAST_HITS: usize = 5;
const COL_DENIES: usize = 6;
const COL_DAMAGE_DEALT: usize = 9;
const COL_DAMAGE_TAKEN: usize = 10;
const COL_HEALING: usize = 11;
const COL_ITEMS: usize = 12;
const COL_RATING: usize = 13;
const FIXED_POINT_UNIT: i64 = 65536;
const TICKS_PER_SECOND: i64 = 16;
const MINUTE_MILLI: i64 = 60_000;
const SCALE: i64 = 1000;
const RATING_STEPS: i64 = 9;
const RATING_TOP: i64 = 100 * SCALE;
const RATED_COLUMNS: [(usize, i64); 11] = [
    (COL_KILLS, 25),
    (COL_DEATHS, -15),
    (COL_ASSISTS, 15),
    (COL_LAST_HITS, 20),
    (COL_DENIES, 10),
    (COL_GOLD, 10),
    (COL_XP, 5),
    (COL_DAMAGE_DEALT, 5),
    (COL_DAMAGE_TAKEN, -5),
    (COL_HEALING, 5),
    (COL_ITEMS, 5),
];
const START_STEPS: usize = 60;
const KILL_STEPS: usize = 800;
const KILL_FLOOR: i32 = 30;
const KILL_CEILING: i32 = 40;
const END_TICKS: usize = 400;
const SETTLE_TICKS: usize = 400;
const BESIDE_KILLER: f32 = 128.0;

struct Unit {
    id: u32,
    player: u8,
    kind: String,
    kind_index: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Gold {
    earned: f64,
    bank: i32,
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
    call(&mut game, "setup", json!({ "ui": ui_file, "player": 1, "pubkey": OWNER }));
    advance(&mut game, 60);
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
    for step in 0..START_STEPS {
        if step % 3 == 0 {
            ui(&mut game, "start", json!({}));
        }
        advance(&mut game, 100);
        if alive_units(&game).iter().any(|unit| heroes.contains(&unit.kind)) {
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

fn spot_of(game: &Game, unit: u32) -> (f32, f32) {
    let index = game.unit_ids().iter().position(|&id| id == unit).expect("unit is alive");
    let spots = game.units();
    (spots[index * 3], spots[index * 3 + 1])
}

fn gold_of(game: &Game, unit: u32, player: u8) -> Gold {
    Gold {
        earned: game.counter(unit, EARNED),
        bank: game.gold(player),
    }
}

fn settled_gold(game: &mut Game, tracked: &[(u32, u8)]) -> Vec<Gold> {
    for _ in 0..SETTLE_TICKS {
        let before: Vec<Gold> = tracked.iter().map(|&(unit, player)| gold_of(game, unit, player)).collect();
        advance(game, 1);
        let after: Vec<Gold> = tracked.iter().map(|&(unit, player)| gold_of(game, unit, player)).collect();
        if before == after {
            return after;
        }
    }
    panic!("gold never settles: the bank and bounty.earned keep moving apart");
}

#[test]
fn a_kill_and_an_assist_bank_their_gold_and_show_in_the_win_table() {
    let dir = scenario_dir();
    let image = image_for(&dir);
    let (mut game, heroes) = start_match(&dir, &image);
    let (killer, victim, ally_id, ally_player) = prepare_kill(&mut game, &heroes);
    let tracked = [(killer, KILLER), (ally_id, ally_player)];
    let before = settled_gold(&mut game, &tracked);

    kill_victim(&mut game, killer, ally_id, victim);
    assert!(game.kills_of(killer) >= 1, "the killer scores the kill");
    let after = settled_gold(&mut game, &tracked);

    let (killer_before, ally_before) = (before[0], before[1]);
    let (killer_after, ally_after) = (after[0], after[1]);
    let killer_earned = killer_after.earned - killer_before.earned;
    let ally_earned = ally_after.earned - ally_before.earned;
    let killer_banked = killer_after.bank - killer_before.bank;
    let ally_banked = ally_after.bank - ally_before.bank;
    println!("probe gold killer_earned={killer_earned} killer_banked={killer_banked} ally_earned={ally_earned} ally_banked={ally_banked}");
    assert!(killer_earned > 0.0, "the kill raises the killer's bounty.earned");
    assert!(ally_earned > 0.0, "the assist raises the assister's bounty.earned");
    assert_eq!(killer_earned, f64::from(killer_banked), "the killer's earned gold is the gold banked");
    assert_eq!(ally_earned, f64::from(ally_banked), "the assister's earned gold is the gold banked");

    let lines = end_match(&mut game);

    let rows = match_stats_rows(&lines);
    let kill_rows: Vec<&Vec<String>> = rows.iter().filter(|row| stat_cell(row, COL_KILLS) >= 1).collect();
    let assist_rows: Vec<&Vec<String>> = rows.iter().filter(|row| stat_cell(row, COL_ASSISTS) >= 1).collect();
    println!("probe win_table rows={} kill_rows={} assist_rows={}", rows.len(), kill_rows.len(), assist_rows.len());
    assert_eq!(kill_rows.len(), 1, "exactly one win-table row shows a kill");
    assert_eq!(assist_rows.len(), 1, "exactly one win-table row shows an assist");
    let kill_row = kill_rows[0];
    let assist_row = assist_rows[0];

    let end: Vec<Gold> = tracked.iter().map(|&(unit, player)| gold_of(&game, unit, player)).collect();
    let xp = i64::from(game.xp(killer));
    println!("probe win_table kill_row={kill_row:?} assist_row={assist_row:?} xp={xp}");
    assert_eq!(stat_cell(kill_row, COL_KILLS), i64::from(game.kills_of(killer)), "the kill column is the sim's kills");
    assert_eq!(stat_cell(assist_row, COL_ASSISTS), 1, "the assist column shows one assist");
    assert_eq!(stat_cell(kill_row, COL_GOLD), end[0].earned as i64, "the killer's gold cell is bounty.earned");
    assert_eq!(stat_cell(assist_row, COL_GOLD), end[1].earned as i64, "the assister's gold cell is bounty.earned");
    assert_eq!(stat_cell(kill_row, COL_XP), xp, "the killer's experience cell is the sim's xp");
    assert!(xp > 0, "the kill grants experience");
    assert_eq!(
        stat_cell(kill_row, COL_DAMAGE_DEALT),
        whole_units(game.counter(killer, "sim_damage_dealt")),
        "the damage dealt cell is the sim's damage dealt"
    );
    assert_eq!(
        stat_cell(kill_row, COL_DAMAGE_TAKEN),
        whole_units(game.counter(killer, "sim_damage_taken")),
        "the damage taken cell is the sim's damage taken"
    );
    assert_eq!(stat_cell(kill_row, COL_HEALING), whole_units(game.counter(killer, "sim_healing")), "the healing cell is the sim's healing");
    assert_eq!(stat_cell(kill_row, COL_ITEMS), held_items(&game, killer), "the items cell counts the items the hero holds");
    let ratings = expected_ratings(&rows, i64::from(game.ticks()) / TICKS_PER_SECOND);
    for (row, rating) in rows.iter().zip(&ratings) {
        assert_eq!(stat_cell(row, COL_RATING), *rating, "the rating cell is computed from the win table's counters: {row:?}");
    }
    println!("probe ratings={:?}", rows.iter().map(|row| stat_cell(row, COL_RATING)).collect::<Vec<_>>());
    println!(
        "probe killer_row damage_dealt={} damage_taken={} healing={} items={}",
        stat_cell(kill_row, COL_DAMAGE_DEALT),
        stat_cell(kill_row, COL_DAMAGE_TAKEN),
        stat_cell(kill_row, COL_HEALING),
        stat_cell(kill_row, COL_ITEMS)
    );
}

fn prepare_kill(game: &mut Game, heroes: &[String]) -> (u32, u32, u32, u8) {
    let units = alive_units(game);
    let killer = hero_of(&units, heroes, KILLER);
    let far_victim = units.iter().find(|unit| unit.player == VICTIM && heroes.contains(&unit.kind)).expect("victim hero");
    let (victim_kind, far_victim_id) = (far_victim.kind_index, far_victim.id);
    let ally = units
        .iter()
        .find(|unit| unit.player != KILLER && heroes.contains(&unit.kind) && !game.hostile(u32::from(KILLER), u32::from(unit.player)))
        .expect("allied hero");
    let (ally_id, ally_player) = (ally.id, ally.player);
    let (killer_x, killer_z) = spot_of(game, killer);
    game.remove_unit(far_victim_id);
    let victim_index = game.spawn_attributed(VICTIM, victim_kind, killer_x + BESIDE_KILLER, killer_z);
    assert!(victim_index >= 0, "the victim hero spawns beside the killer");
    (killer, victim_index as u32, ally_id, ally_player)
}

fn kill_victim(game: &mut Game, killer: u32, ally_id: u32, victim: u32) {
    let lifesteal = lifesteal_item(game);
    game.give_item(killer, lifesteal, 1);
    game.hurt(killer, 100);
    game.attack(killer, victim);
    game.attack(ally_id, victim);
    for _ in 0..KILL_STEPS {
        for unit in alive_units(game) {
            if (unit.kind.contains("creep") || unit.kind.contains("tower")) && unit.id != killer && unit.id != ally_id && unit.id != victim {
                game.remove_unit(unit.id);
            }
        }
        advance(game, 2);
        let victim_hp = game.unit_hp(victim);
        if victim_hp > KILL_CEILING {
            game.hurt(victim, victim_hp - KILL_FLOOR);
        }
        if game.kills_of(killer) > 0 {
            break;
        }
    }
}

fn end_match(game: &mut Game) -> Vec<Value> {
    let player0: Vec<u32> = alive_units(game).iter().filter(|unit| unit.player == KILLER).map(|unit| unit.id).collect();
    for unit in &player0 {
        game.remove_unit(*unit);
    }
    let mut lines = drain(game);
    for _ in 0..END_TICKS {
        if game.winner() >= 0 {
            break;
        }
        let outcome = game.tick();
        assert!(outcome.is_empty(), "tick: {outcome}");
        lines.extend(drain(game));
    }
    assert!(game.winner() >= 0, "the base kill ends the match");
    lines
}

fn match_stats_seconds(lines: &[Value]) -> i64 {
    lines
        .iter()
        .rev()
        .find(|value| value["kind"] == "panel" && value["id"] == "match_stats")
        .and_then(|value| value["text"].as_str())
        .and_then(|text| text.split_whitespace().next())
        .and_then(|number| number.parse().ok())
        .expect("match_stats seconds")
}

fn win_tree(lines: &[Value]) -> &Value {
    lines
        .iter()
        .rev()
        .find(|line| line["kind"] == "widget" && line["data"]["id"] == "win")
        .map(|line| &line["data"]["tree"])
        .expect("the win widget is shown")
}

fn tree_nodes<'a>(node: &'a Value, out: &mut Vec<&'a Value>) {
    out.push(node);
    for kid in node["kids"].as_array().into_iter().flatten() {
        tree_nodes(kid, out);
    }
}

fn labels_of(nodes: &[&Value]) -> Vec<String> {
    nodes
        .iter()
        .filter(|node| node["t"] == "label")
        .map(|node| node["text"].as_str().unwrap_or_default().to_owned())
        .collect()
}

fn portrait_names(nodes: &[&Value]) -> Vec<(u64, String)> {
    nodes
        .iter()
        .filter(|node| node["call"] == "post_pick")
        .map(|node| {
            let player = node["args"]["player"].as_u64().expect("portrait player");
            let name = node["kids"]
                .as_array()
                .expect("portrait kids")
                .iter()
                .filter(|kid| kid["t"] == "label")
                .last()
                .and_then(|kid| kid["text"].as_str())
                .expect("portrait name")
                .to_owned();
            (player, name)
        })
        .collect()
}

fn row_values(labels: &[String], name: &str) -> (String, String) {
    let at = labels.iter().position(|label| label == name).expect("comparison row");
    (labels[at - 1].clone(), labels[at + 1].clone())
}

fn row_named<'a>(rows: &'a [Vec<String>], name: &str) -> &'a [String] {
    rows.iter().find(|row| row[0] == name).expect("win row for the hero")
}

#[test]
fn the_post_game_overview_compares_two_heroes_and_derives_the_bars() {
    let dir = scenario_dir();
    let image = image_for(&dir);
    let (mut game, heroes) = start_match(&dir, &image);
    let (killer, victim, ally_id, _) = prepare_kill(&mut game, &heroes);
    kill_victim(&mut game, killer, ally_id, victim);
    assert!(game.kills_of(killer) >= 1, "the killer scores the kill");
    let lines = end_match(&mut game);

    let seconds = match_stats_seconds(&lines).max(1);
    let rows = match_stats_rows(&lines);
    assert_eq!(rows.len(), 10, "the post-game table has ten rows");
    let mut picked = ui(&mut game, "post_pick", json!({ "player": 0 }));
    picked.extend(ui(&mut game, "post_pick", json!({ "player": 5 })));
    let overview = win_tree(&picked).clone();
    let mut nodes = Vec::new();
    tree_nodes(&overview, &mut nodes);
    let labels = labels_of(&nodes);
    let portraits = portrait_names(&nodes);
    let portrait_of = |player: u64| portraits.iter().find(|(p, _)| *p == player).map(|(_, name)| name.clone()).expect("portrait");
    let left_name = portrait_of(0);
    let right_name = portrait_of(5);
    let left = row_named(&rows, &left_name);
    let right = row_named(&rows, &right_name);
    let vs_at = labels.iter().position(|label| label == "vs").expect("comparison heads");
    assert_eq!(labels[vs_at - 1], left_name, "the left column is player 0's hero");
    assert_eq!(labels[vs_at + 1], right_name, "the right column is player 5's hero");

    let (level_left, level_right) = row_values(&labels, "Level");
    assert!(level_left.parse::<i64>().expect("left level") >= 1 && level_right.parse::<i64>().expect("right level") >= 1);
    let kda = |row: &[String]| format!("{} / {} / {}", row[COL_KILLS], row[COL_DEATHS], row[COL_ASSISTS]);
    assert_eq!(row_values(&labels, "K / D / A"), (kda(left), kda(right)), "the K / D / A line is the win table's kills, deaths and assists");
    let per_minute = |value: i64| (value * 60 / seconds).to_string();
    assert_eq!(row_values(&labels, "Gold per minute"), (per_minute(stat_cell(left, COL_GOLD)), per_minute(stat_cell(right, COL_GOLD))));
    assert_eq!(row_values(&labels, "XP per minute"), (per_minute(stat_cell(left, COL_XP)), per_minute(stat_cell(right, COL_XP))));
    let totals = [
        ("Gold", COL_GOLD),
        ("Experience", COL_XP),
        ("Last hits", COL_LAST_HITS),
        ("Denies", COL_DENIES),
        ("Damage dealt", COL_DAMAGE_DEALT),
        ("Healing", COL_HEALING),
    ];
    for (name, column) in totals {
        assert_eq!(
            row_values(&labels, name),
            (stat_cell(left, column).to_string(), stat_cell(right, column).to_string()),
            "{name} is the win table's value"
        );
    }
    let items_of = |row: &[String], text: &str| {
        if stat_cell(row, COL_ITEMS) == 0 {
            text == "none"
        } else {
            text.split(", ").count() as i64 == stat_cell(row, COL_ITEMS)
        }
    };
    let (items_left, items_right) = row_values(&labels, "Items");
    assert!(items_of(left, &items_left) && items_of(right, &items_right), "the items text names the held items");

    let percent = |row: &[String], column: usize| {
        let rate = |r: &[String]| stat_cell(r, column) * MINUTE_MILLI / seconds;
        let peak = rows.iter().map(|r| rate(r.as_slice())).max().unwrap_or(0);
        if peak > 0 {
            rate(row) * 100 / peak
        } else {
            0
        }
    };
    let expected_bars = vec![
        percent(left, COL_ASSISTS),
        percent(right, COL_ASSISTS),
        percent(left, COL_DAMAGE_DEALT),
        percent(right, COL_DAMAGE_DEALT),
        percent(left, COL_LAST_HITS),
        percent(right, COL_LAST_HITS),
    ];
    let bars: Vec<i64> = nodes.iter().filter(|node| node["t"] == "bar").map(|node| node["value"].as_i64().expect("bar value")).collect();
    assert_eq!(bars, expected_bars, "supporting, fighting and farming are percent of the best hero");
    let not_tracked = labels.iter().filter(|label| label.as_str() == "not tracked").count();
    assert_eq!(not_tracked, 12, "pushing has no bar on either side and the five averages are not tracked for both heroes");
    assert!(labels.iter().any(|label| label.starts_with("Match length")), "the timeline shows the match length");
    assert!(nodes.iter().any(|node| node["call"] == "post_team_view"), "the comparison offers Back to team view");

    println!(
        "probe overview left={left_name} right={right_name} seconds={seconds} kda=[{}] [{}] gpm=[{}] [{}] xpm=[{}] [{}] bars={bars:?} not_tracked={not_tracked}",
        kda(left),
        kda(right),
        per_minute(stat_cell(left, COL_GOLD)),
        per_minute(stat_cell(right, COL_GOLD)),
        per_minute(stat_cell(left, COL_XP)),
        per_minute(stat_cell(right, COL_XP))
    );
    println!("probe overview items left=[{items_left}] right=[{items_right}] levels=[{level_left}] [{level_right}]");

    let team = ui(&mut game, "post_team_view", json!({}));
    let mut team_nodes = Vec::new();
    tree_nodes(win_tree(&team), &mut team_nodes);
    let team_labels = labels_of(&team_nodes);
    assert!(team_labels.iter().any(|label| label.contains("Team totals")), "the team view shows the team totals");
    assert!(!team_labels.iter().any(|label| label.as_str() == "vs"), "the team view hides the comparison");
    println!("probe team_view totals={:?}", team_labels.iter().filter(|label| label.contains("Team totals")).collect::<Vec<_>>());

    for tab in ["mvp", "graphs", "scoreboard"] {
        let shown = ui(&mut game, "post_tab", json!({ "tab": tab }));
        let mut tab_nodes = Vec::new();
        tree_nodes(win_tree(&shown), &mut tab_nodes);
        let tab_labels = labels_of(&tab_nodes);
        let active = tab_nodes.iter().any(|node| node["call"] == "post_tab" && node["args"]["tab"] == tab && node["disabled"] == true);
        assert!(active, "the {tab} tab button is the active one");
        match tab {
            "mvp" => {
                let top = tab_labels.iter().find(|label| label.starts_with("1 ")).expect("the top rated hero");
                println!("probe mvp top={top}");
            }
            "graphs" => assert!(tab_labels.iter().any(|label| label.starts_with("Not tracked: graphs")), "the graphs tab says it is not tracked"),
            _ => assert!(tab_labels.iter().any(|label| label == "LH"), "the scoreboard tab shows the win table"),
        }
    }
}

fn whole_units(fixed: f64) -> i64 {
    fixed as i64 / FIXED_POINT_UNIT
}

fn expected_ratings(rows: &[Vec<String>], seconds: i64) -> Vec<i64> {
    let span = seconds.max(1);
    let rate = |row: &[String], column: usize| stat_cell(row, column) * MINUTE_MILLI / span;
    let peaks: Vec<i64> = RATED_COLUMNS
        .iter()
        .map(|&(column, _)| rows.iter().map(|row| rate(row, column)).max().unwrap_or(0).max(0))
        .collect();
    rows.iter()
        .map(|row| {
            let points: i64 = RATED_COLUMNS
                .iter()
                .zip(&peaks)
                .map(|(&(column, weight), &peak)| {
                    let norm = if peak > 0 { rate(row, column) * SCALE / peak } else { 0 };
                    weight * norm
                })
                .sum();
            1 + points.max(0) * RATING_STEPS / RATING_TOP
        })
        .collect()
}

fn held_items(game: &Game, unit: u32) -> i64 {
    let inventory: Value = serde_json::from_str(&game.inventory_json(unit)).expect("inventory json");
    inventory["slots"]
        .as_array()
        .expect("inventory slots")
        .iter()
        .filter(|slot| slot[0].as_u64().unwrap_or(0) > 0 && slot[1].as_u64().unwrap_or(0) > 0)
        .count() as i64
}
