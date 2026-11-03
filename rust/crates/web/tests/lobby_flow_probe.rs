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

fn read_ui(dir: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(dir.join("moba_ui.json")).expect("read moba_ui.json")).expect("moba_ui.json")
}

fn lobby_error(lines: &[Value]) -> String {
    lines
        .iter()
        .find(|line| line["kind"] == "lobby")
        .and_then(|line| line["data"]["error"].as_str())
        .unwrap_or_default()
        .to_owned()
}

fn lobby_data(lines: &[Value]) -> Option<&Value> {
    lines.iter().rev().find(|line| line["kind"] == "lobby").map(|line| &line["data"])
}

fn lobby_events(lines: &[Value], started: bool) -> usize {
    lines.iter().filter(|line| line["kind"] == "lobby" && line["data"]["started"] == started).count()
}

fn lobby_widgets(lines: &[Value]) -> Vec<&Value> {
    lines.iter().filter(|line| line["kind"] == "widget" && line["data"]["id"] == "lobby").map(|line| &line["data"]).collect()
}

fn allowed_heroes(lines: &[Value]) -> Vec<String> {
    lobby_data(lines)
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

fn kid_labels(tree: &Value) -> Vec<String> {
    tree["kids"]
        .as_array()
        .map(|kids| {
            kids.iter()
                .map(|kid| {
                    let mut texts = Vec::new();
                    label_texts(kid, &mut texts);
                    texts.join(" / ")
                })
                .collect()
        })
        .unwrap_or_default()
}

fn report(label: &str, lines: &[Value]) {
    println!(
        "probe {label} lobby_error=\"{}\" lobby_events started_false={} started_true={}",
        lobby_error(lines),
        lobby_events(lines, false),
        lobby_events(lines, true)
    );
    for widget in lobby_widgets(lines) {
        if widget["remove"] == true {
            println!("probe {label} lobby_widget=remove");
        } else {
            println!("probe {label} lobby_widget=present kids={:?}", kid_labels(&widget["tree"]));
        }
    }
}

fn ui_as(game: &mut Game, pubkey: &str, command: &str, args: Value) -> Vec<Value> {
    call(game, "ui", json!({ "call": command, "args": args, "player": 1, "pubkey": pubkey }))
}

fn seat_owners(lines: &[Value]) -> Vec<String> {
    lobby_data(lines)
        .map(|data| {
            data["slots"]
                .as_array()
                .expect("slots")
                .iter()
                .map(|slot| slot["owner"].as_str().unwrap_or_default().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

fn negotiated(lines: &[Value]) -> bool {
    lobby_data(lines).map(|data| data["negotiated"] == true).unwrap_or(false)
}

fn index_of(kids: &[String], needle: &str) -> Option<usize> {
    kids.iter().position(|kid| kid.contains(needle))
}

fn strip_index_of(kids: &[String], team: &str) -> Option<usize> {
    kids.iter().position(|kid| kid.starts_with(team))
}

#[test]
fn probe_lobby_order_and_hide_on_start() {
    let dir = scenario_dir();
    let image = image_for(&dir);
    let mut game = Game::new(&image).expect("game");
    game.set_fog(false);
    load_pack(&mut game, &dir);
    let ui_file = read_ui(&dir);
    let fill_text = ui_file["text"]["lobby"]["fillBots"].as_str().expect("fillBots text").to_owned();
    let team_text = ui_file["text"]["lobby"]["teamNames"][0].as_str().expect("team name").to_owned();

    let setup = call(&mut game, "setup", json!({ "ui": ui_file, "player": 1, "pubkey": OWNER }));
    report("setup", &setup);
    advance(&mut game, 60);

    let claim = ui(&mut game, "claim", json!({ "slot": 0 }));
    report("join_claim", &claim);
    let claim_kids = lobby_widgets(&claim).first().map(|widget| kid_labels(&widget["tree"])).unwrap_or_default();
    let fill_at = index_of(&claim_kids, &fill_text);
    let team_at = strip_index_of(&claim_kids, &team_text);
    println!("probe join_claim order fill_at={fill_at:?} team_at={team_at:?}");
    let mut problems = Vec::new();
    if !(fill_at.is_some() && team_at.is_some() && fill_at < team_at) {
        problems.push(format!("negotiation rows come before the team strips: fill_at={fill_at:?} team_at={team_at:?}"));
    }

    let mode = ui(&mut game, "mode", json!({ "code": "ai" }));
    report("mode_ai_on", &mode);
    let fill = ui(&mut game, "fill", json!({}));
    report("fill", &fill);
    let heroes = allowed_heroes(&fill);
    let mut own_hero = String::new();
    for hero in &heroes {
        let lines = ui(&mut game, "pick", json!({ "slot": 0, "hero": hero }));
        if lobby_error(&lines).is_empty() {
            own_hero = hero.clone();
            break;
        }
    }
    assert!(!own_hero.is_empty(), "the local seat picks a hero");
    let world = ui(&mut game, "world_ready", json!({}));
    report("world_ready", &world);
    let mut ready_lines = Vec::new();
    for seat in 0..10 {
        ready_lines = ui(&mut game, "ready", json!({ "slot": seat, "ready": true }));
    }
    report("ready_last_seat", &ready_lines);

    let mut start_lines = Vec::new();
    for step in 0..60 {
        if step % 3 == 0 {
            let lines = ui(&mut game, "start", json!({}));
            if lobby_events(&lines, true) > 0 {
                start_lines = lines;
                println!("probe start ok at step={step}");
                break;
            }
        }
        advance(&mut game, 100);
    }
    report("start", &start_lines);
    assert!(lobby_events(&start_lines, true) > 0, "the start call began the match");
    assert!(lobby_widgets(&start_lines).iter().any(|widget| widget["remove"] == true), "the lobby widget is removed on start");

    let late_ready = ui(&mut game, "ready", json!({ "slot": 0, "ready": true }));
    report("after_start_ready", &late_ready);
    let late_mode = ui(&mut game, "mode", json!({ "code": "ai" }));
    report("after_start_mode", &late_mode);
    let late_claim = ui(&mut game, "claim", json!({ "slot": 0 }));
    report("after_start_claim", &late_claim);
    let late = [&late_ready, &late_mode, &late_claim];
    let reopened: usize = late.iter().map(|lines| lobby_events(lines, false)).sum();
    let shown_again: usize = late.iter().map(|lines| lobby_widgets(lines).iter().filter(|widget| widget["remove"] != true).count()).sum();
    println!("probe after_start reopened_lobby_events={reopened} lobby_widgets_shown={shown_again}");
    if reopened != 0 {
        problems.push(format!("{reopened} lobby events report the match as not started after start"));
    }
    if shown_again != 0 {
        problems.push(format!("{shown_again} lobby widgets shown after start"));
    }
    assert!(problems.is_empty(), "lobby flow problems: {problems:#?}");
}

#[test]
fn probe_join_refused_before_negotiation_then_accepted() {
    let dir = scenario_dir();
    let image = image_for(&dir);
    let mut game = Game::new(&image).expect("game");
    game.set_fog(false);
    load_pack(&mut game, &dir);
    let ui_file = read_ui(&dir);
    let refusal_text = ui_file["text"]["lobby"]["refusals"]["negotiate-first"].as_str().expect("refusal text").to_owned();
    call(&mut game, "setup", json!({ "ui": ui_file, "player": 1, "pubkey": OWNER }));
    advance(&mut game, 60);
    let joiner = "probe-joiner";

    let creation = ui_as(&mut game, OWNER, "claim", json!({ "slot": 0 }));
    println!("probe creation claim slot=0 error=\"{}\" owner={:?}", lobby_error(&creation), seat_owners(&creation)[0]);
    assert!(lobby_error(&creation).is_empty(), "the creator takes a seat on creation");
    let before = seat_owners(&creation);

    let refused = ui_as(&mut game, joiner, "claim", json!({ "slot": 5 }));
    let shown: Vec<String> = lobby_widgets(&refused)
        .iter()
        .flat_map(|widget| kid_labels(&widget["tree"]))
        .filter(|kid| kid.contains(&refusal_text))
        .collect();
    println!("probe join before negotiation slot=5 error=\"{}\" negotiated={}", lobby_error(&refused), negotiated(&refused));
    println!("probe join before negotiation lobby_shows={shown:?}");
    println!("probe join before negotiation seat_owners_unchanged={}", seat_owners(&refused) == before);
    assert_eq!(lobby_error(&refused), refusal_text, "a join before negotiation is refused");
    assert!(!shown.is_empty(), "the refusal reason is shown in the lobby");
    assert_eq!(seat_owners(&refused), before, "a refused join changes no seat");

    let mode = ui_as(&mut game, OWNER, "mode", json!({ "code": "ai" }));
    assert!(lobby_error(&mode).is_empty(), "the host chooses a mode");
    let filled = ui_as(&mut game, OWNER, "fill", json!({}));
    println!("probe negotiation complete mode=ai fill error=\"{}\" negotiated={}", lobby_error(&filled), negotiated(&filled));
    assert!(negotiated(&filled), "a mode and filled-or-bot seats complete the negotiation");

    let joined = ui_as(&mut game, joiner, "claim", json!({ "slot": 5 }));
    println!("probe join after negotiation slot=5 error=\"{}\" owner={:?}", lobby_error(&joined), seat_owners(&joined)[5]);
    assert!(lobby_error(&joined).is_empty(), "a join after negotiation is accepted");
    assert_eq!(seat_owners(&joined)[5], joiner, "the joiner holds the seat on the chosen side");
}
