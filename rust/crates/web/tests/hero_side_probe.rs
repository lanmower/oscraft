use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use web::Game;

const OWNER: &str = "probe-owner";
const JOINER: &str = "probe-joiner";
const OWNER_PLAYER: u64 = 1;
const JOINER_PLAYER: u64 = 6;
const JOINER_SLOT: u64 = 5;
const SENTINEL_HERO: &str = "antimage";
const SCOURGE_HERO: &str = "crystal_maiden";
const UNTAGGED_HERO: &str = "axe";
const UNTAGGED_PICK: &str = "bloodseeker";
const FIXTURE_SIDES: &str = "return {\n  antimage = \"sentinel\",\n  crystal_maiden = \"scourge\",\n}\n";

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

fn load_pack(game: &mut Game, dir: &Path, sides_text: &str) {
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
            let text = if file == "hero_sides.lua" {
                sides_text.to_owned()
            } else {
                std::fs::read_to_string(dir.join(&file)).expect("read module")
            };
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

fn act(game: &mut Game, player: u64, pubkey: &str, command: &str, args: Value) -> Vec<Value> {
    call(game, "ui", json!({ "call": command, "args": args, "player": player, "pubkey": pubkey }))
}

fn advance(game: &mut Game, ticks: usize) {
    for _ in 0..ticks {
        let outcome = game.tick();
        assert!(outcome.is_empty(), "tick: {outcome}");
    }
}

fn lobby_data(lines: &[Value]) -> Option<&Value> {
    lines.iter().rev().find(|line| line["kind"] == "lobby").map(|line| &line["data"])
}

fn lobby_error(lines: &[Value]) -> String {
    lobby_data(lines).and_then(|data| data["error"].as_str()).unwrap_or_default().to_owned()
}

fn allowed(lines: &[Value], hero: &str) -> bool {
    lobby_data(lines)
        .and_then(|data| data["heroes"].as_array())
        .and_then(|heroes| heroes.iter().find(|entry| entry["unit"] == hero))
        .map(|entry| entry["allowed"] == true)
        .expect("the lobby lists the hero")
}

fn seat_hero(lines: &[Value], slot: usize) -> String {
    lobby_data(lines).and_then(|data| data["slots"][slot]["hero"].as_str()).unwrap_or_default().to_owned()
}

fn seat_owner(lines: &[Value], slot: usize) -> String {
    lobby_data(lines).and_then(|data| data["slots"][slot]["owner"].as_str()).unwrap_or_default().to_owned()
}

fn start_lobby(dir: &Path, sides_text: &str, viewer: u64, pubkey: &str) -> (Game, Value) {
    let image = image_for(dir);
    let mut game = Game::new(&image).expect("game");
    game.set_fog(false);
    load_pack(&mut game, dir, sides_text);
    let ui_file: Value = serde_json::from_slice(&std::fs::read(dir.join("moba_ui.json")).expect("read moba_ui.json")).expect("moba_ui.json");
    call(&mut game, "setup", json!({ "ui": ui_file, "player": viewer, "pubkey": pubkey }));
    advance(&mut game, 60);
    (game, ui_file)
}

fn scourge_client_view(dir: &Path) -> Vec<Value> {
    let (mut game, _) = start_lobby(dir, FIXTURE_SIDES, JOINER_PLAYER, JOINER);
    let claim = act(&mut game, JOINER_PLAYER, OWNER, "claim", json!({ "slot": 0 }));
    assert!(lobby_error(&claim).is_empty(), "the host takes the first seat");
    let seated = act(&mut game, JOINER_PLAYER, OWNER, "seat", json!({ "slot": JOINER_SLOT, "seat_owner": JOINER }));
    assert_eq!(seat_owner(&seated, JOINER_SLOT as usize), JOINER, "the host seats the joiner in the Scourge half");
    let picked = act(&mut game, JOINER_PLAYER, JOINER, "pick", json!({ "slot": JOINER_SLOT, "hero": UNTAGGED_PICK }));
    assert!(lobby_error(&picked).is_empty(), "the Scourge client's seat picks an untagged hero");
    picked
}

#[test]
fn probe_default_mode_pools_follow_side_tags() {
    let dir = scenario_dir();
    let (mut game, ui_file) = start_lobby(&dir, FIXTURE_SIDES, OWNER_PLAYER, OWNER);
    let mode_pool = ui_file["text"]["lobby"]["refusals"]["mode-pool"].as_str().expect("mode-pool text").to_owned();

    let claim = act(&mut game, OWNER_PLAYER, OWNER, "claim", json!({ "slot": 0 }));
    assert!(lobby_error(&claim).is_empty(), "the host takes the first seat");
    let seated = act(&mut game, OWNER_PLAYER, OWNER, "seat", json!({ "slot": JOINER_SLOT, "seat_owner": JOINER }));
    assert_eq!(seat_owner(&seated, JOINER_SLOT as usize), JOINER, "the host seats the joiner in the Scourge half");
    let sentinel_view = [
        ("sentinel_tagged", allowed(&seated, SENTINEL_HERO)),
        ("scourge_tagged", allowed(&seated, SCOURGE_HERO)),
        ("untagged", allowed(&seated, UNTAGGED_HERO)),
    ];
    println!("probe sentinel_seat allowed={sentinel_view:?}");
    assert!(sentinel_view[0].1, "a Sentinel seat may pick a Sentinel-tagged hero");
    assert!(!sentinel_view[1].1, "a Sentinel seat may not pick a Scourge-tagged hero");
    assert!(sentinel_view[2].1, "an untagged hero stays open to a Sentinel seat");

    let untagged = act(&mut game, JOINER_PLAYER, JOINER, "pick", json!({ "slot": JOINER_SLOT, "hero": UNTAGGED_PICK }));
    assert!(lobby_error(&untagged).is_empty(), "the Scourge seat picks an untagged hero");
    let scourge_client = scourge_client_view(&dir);
    let scourge_view = [
        ("sentinel_tagged", allowed(&scourge_client, SENTINEL_HERO)),
        ("scourge_tagged", allowed(&scourge_client, SCOURGE_HERO)),
        ("untagged", allowed(&scourge_client, UNTAGGED_HERO)),
    ];
    println!("probe scourge_seat allowed={scourge_view:?}");
    assert!(!scourge_view[0].1, "a Scourge seat may not pick a Sentinel-tagged hero");
    assert!(scourge_view[1].1, "a Scourge seat may pick a Scourge-tagged hero");
    assert!(scourge_view[2].1, "an untagged hero stays open to a Scourge seat");

    let joiner_refused = act(&mut game, JOINER_PLAYER, JOINER, "pick", json!({ "slot": JOINER_SLOT, "hero": SENTINEL_HERO }));
    println!("probe scourge_seat pick sentinel_tagged error=\"{}\"", lobby_error(&joiner_refused));
    assert_eq!(lobby_error(&joiner_refused), mode_pool, "the server refuses a Scourge seat's Sentinel-tagged pick");
    assert_eq!(seat_hero(&joiner_refused, JOINER_SLOT as usize), UNTAGGED_PICK, "a refused pick changes no seat");

    let owner_refused = act(&mut game, OWNER_PLAYER, OWNER, "pick", json!({ "slot": 0, "hero": SCOURGE_HERO }));
    println!("probe sentinel_seat pick scourge_tagged error=\"{}\"", lobby_error(&owner_refused));
    assert_eq!(lobby_error(&owner_refused), mode_pool, "the server refuses a Sentinel seat's Scourge-tagged pick");

    let random = act(&mut game, JOINER_PLAYER, JOINER, "pick", json!({ "slot": JOINER_SLOT, "hero": "" }));
    let drawn = seat_hero(&random, JOINER_SLOT as usize);
    println!("probe scourge_seat random drew={drawn:?} error=\"{}\"", lobby_error(&random));
    assert!(lobby_error(&random).is_empty(), "a random pick for the Scourge seat succeeds");
    assert!(!drawn.is_empty() && drawn != SENTINEL_HERO, "the random Scourge pick is never the Sentinel-tagged hero");

    let owner_pick = act(&mut game, OWNER_PLAYER, OWNER, "pick", json!({ "slot": 0, "hero": SENTINEL_HERO }));
    assert!(lobby_error(&owner_pick).is_empty(), "the Sentinel seat picks its tagged hero");
    let joiner_own = act(&mut game, JOINER_PLAYER, JOINER, "pick", json!({ "slot": JOINER_SLOT, "hero": SCOURGE_HERO }));
    assert!(lobby_error(&joiner_own).is_empty(), "the Scourge seat picks its tagged hero");

    let flagged = act(&mut game, OWNER_PLAYER, OWNER, "mode", json!({ "code": "du" }));
    assert!(lobby_error(&flagged).is_empty(), "the host turns on duplicate mode");
    let flagged_pick = act(&mut game, JOINER_PLAYER, JOINER, "pick", json!({ "slot": JOINER_SLOT, "hero": SENTINEL_HERO }));
    println!("probe duplicate_mode scourge_seat pick sentinel_tagged error=\"{}\"", lobby_error(&flagged_pick));
    assert!(lobby_error(&flagged_pick).is_empty(), "a flagged mode keeps its own rules: tags do not apply");
}

#[test]
fn probe_shipped_side_table_leaves_every_hero_open() {
    let dir = scenario_dir();
    let shipped = std::fs::read_to_string(dir.join("hero_sides.lua")).expect("read hero_sides.lua");
    let (mut game, _) = start_lobby(&dir, &shipped, OWNER_PLAYER, OWNER);
    let claim = act(&mut game, OWNER_PLAYER, OWNER, "claim", json!({ "slot": 0 }));
    assert!(lobby_error(&claim).is_empty(), "the host takes the first seat");
    let seated = act(&mut game, OWNER_PLAYER, OWNER, "seat", json!({ "slot": JOINER_SLOT, "seat_owner": JOINER }));
    assert!(allowed(&seated, SENTINEL_HERO), "with no tags the Sentinel seat may pick the hero");
    assert!(allowed(&seated, SCOURGE_HERO), "with no tags the Sentinel seat may pick the other hero");
    let joined = act(&mut game, JOINER_PLAYER, JOINER, "pick", json!({ "slot": JOINER_SLOT, "hero": SENTINEL_HERO }));
    assert!(lobby_error(&joined).is_empty(), "with no tags the Scourge seat picks the Sentinel-side hero");
    println!("probe shipped_side_table open heroes allowed for both seats");
}
