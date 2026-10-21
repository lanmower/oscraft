//! The moba panels (scenarios/moba/panels.lua) run on the piccolo host and send widget trees: the HUD and the lobby open
//! on the first refresh, a panel key toggles its panel, a selection draws the selection panel, and the lobby's rules answer
//! a claim. The page draws the trees (js/client/widgets.js) and decides nothing; this checks what the pack sends.

#![cfg(feature = "build")]

use std::path::Path;

use mod_host::{build_image, ModHost};
use serde_json::json;

fn scenario() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../scenarios/moba")
}

// The moba pack as the page loads it: every manifest mod, in manifest order, then the setup the page sends.
fn host_with_panels() -> ModHost {
    let image = build_image(&scenario()).expect("moba scenario builds an image");
    let mut host = ModHost::new(&image).expect("host");
    let read = |name: &str| std::fs::read_to_string(scenario().join(name)).expect(name);
    let manifest: serde_json::Value = serde_json::from_str(&read("manifest.json")).expect("manifest json");
    let mods: Vec<(String, String)> = manifest["mods"]
        .as_array()
        .expect("manifest mods list")
        .iter()
        .map(|m| {
            let file = m.as_str().expect("mod file name");
            (file.to_owned(), read(file))
        })
        .collect();
    host.load_mods(&mods).expect("moba mods load");
    let ui: serde_json::Value = serde_json::from_str(&read("moba_ui.json")).expect("ui json");
    host.dispatch_action("setup", &json!({ "ui": ui, "player": 0 })).expect("setup");
    host
}

fn key(host: &mut ModHost, name: &str) {
    host.dispatch_action("key", &json!({ "key": name, "phase": "down", "shift": false, "ctrl": false, "alt": false, "meta": false, "player": 0, "world": { "x": 0.0, "z": 0.0 } })).expect("key");
}

// Runs n ticks. The panels refresh every fourth tick, so any window of five ticks holds a refresh: a test that expects a
// change to show runs five ticks after it.
fn ticks(host: &mut ModHost, n: usize) {
    for _ in 0..n {
        host.advance().expect("tick");
    }
}

// The widget event for panel `id` among the queued UI bodies.
fn panel<'a>(ui: &'a [String], id: &str) -> Option<&'a String> {
    let want = format!("\"id\":\"{id}\"");
    ui.iter().find(|b| b.contains("\"widget\"") && b.contains(&want))
}

#[test]
fn hud_and_lobby_are_sent_on_the_first_refresh() {
    let mut host = host_with_panels();
    ticks(&mut host, 5);
    let ui = host.take_ui();
    let hud = panel(&ui, "hud").expect("the hud panel is sent");
    assert!(hud.contains("\"slot\":\"hud\"") && hud.contains("\"player\":0"), "hud names its slot and seat: {hud}");
    assert!(hud.contains("gold ") && hud.contains("pre-match"), "hud shows gold and the pre-match clock: {hud}");
    let lobby = panel(&ui, "lobby").expect("the lobby is sent before the horn");
    assert!(lobby.contains("Hero select") && lobby.contains("Fill with bots"), "lobby tree: {lobby}");
}

#[test]
fn an_unchanged_panel_is_not_sent_again() {
    let mut host = host_with_panels();
    ticks(&mut host, 5);
    host.take_ui();
    ticks(&mut host, 7);
    let ui = host.take_ui();
    assert!(panel(&ui, "lobby").is_none(), "the lobby is unchanged, so not sent again: {ui:?}");
    assert!(panel(&ui, "hud").is_none(), "the hud is unchanged, so not sent again: {ui:?}");
}

#[test]
fn the_shop_key_opens_the_shop_panel_and_its_key_closes_it() {
    let mut host = host_with_panels();
    ticks(&mut host, 5);
    host.take_ui();
    key(&mut host, "p");
    ticks(&mut host, 5);
    let ui = host.take_ui();
    let shop = panel(&ui, "shop").expect("the shop panel opens on its key");
    assert!(shop.contains("\"slot\":\"right\"") && shop.contains("Buy"), "the shop lists buy buttons: {}", &shop[..shop.len().min(400)]);
    key(&mut host, "p");
    ticks(&mut host, 5);
    let ui = host.take_ui();
    assert!(panel(&ui, "shop").is_some_and(|b| b.contains("\"remove\":true")), "the shop closes: {ui:?}");
}

#[test]
fn a_claim_takes_a_lobby_seat() {
    let mut host = host_with_panels();
    ticks(&mut host, 5);
    host.take_ui();
    host.dispatch_action("ui", &json!({ "call": "claim", "args": { "slot": 0 } })).expect("claim");
    ticks(&mut host, 5);
    let ui = host.take_ui();
    let lobby = panel(&ui, "lobby").expect("the lobby is sent after the claim");
    assert!(lobby.contains("Slot 1: you"), "the claimed seat reads as yours: {lobby}");
}

#[test]
fn a_selection_opens_the_selection_panel() {
    let mut host = host_with_panels();
    let id = {
        let sim = host.sim();
        (0..sim.count()).find(|&i| sim.alive(i) && sim.player(i) == 0).expect("seat 0 has a unit") as i64
    };
    host.dispatch_action("selection", &json!({ "player": 0, "ids": [id] })).expect("selection");
    ticks(&mut host, 5);
    let ui = host.take_ui();
    let select = panel(&ui, "select").expect("the selection panel is sent for a selected unit");
    assert!(select.contains("\"slot\":\"bar-left\"") && select.contains("\"t\":\"bar\""), "the selection panel has its bars: {}", &select[..select.len().min(300)]);
}

#[test]
fn a_hero_draws_the_skill_inventory_team_and_scoreboard_panels() {
    let mut host = host_with_panels();
    let hero = {
        let sim = host.sim();
        (0..sim.count()).find(|&i| sim.alive(i) && sim.player(i) == 0 && sim.is_attributed(i)).expect("seat 0 has a hero") as i64
    };
    host.dispatch_action("selection", &json!({ "player": 0, "ids": [hero] })).expect("selection");
    key(&mut host, "tab");
    ticks(&mut host, 5);
    let ui = host.take_ui();
    let skills = panel(&ui, "skills").expect("a hero's skill panel is sent");
    assert!(skills.contains("Abilities") && skills.contains("Talents"), "skills lists abilities and talents: {}", &skills[..skills.len().min(300)]);
    assert!(panel(&ui, "inventory").is_some_and(|b| b.contains("stash")), "the inventory panel draws its stash");
    assert!(panel(&ui, "team").is_some_and(|b| b.contains("kills")), "the team strip lists the kills");
    assert!(panel(&ui, "scoreboard").is_some_and(|b| b.contains("Scoreboard") && b.contains("\"slot\":\"centre\"")), "the tab key opens the scoreboard");
}
