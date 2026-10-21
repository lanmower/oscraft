//! The bottom ability bar of the selection panel: one cell per cast key (moba_ui.json keys.cast), each labelled with its key and
//! showing the name of the ability in that key's slot. After a picked hero spawns and is selected, the bar holds one cell per
//! key in slot order, each name matches the hero's slot, and no ability name repeats across two keys.

#![cfg(feature = "build")]

use mod_host::{build_image, ModHost};
use serde_json::{json, Value};
use sim::ABILITY_SLOTS;

mod common;
use common::scenario;

// The moba pack as the page loads it, with the seat's antimage picked and spawned. Returns the host and the hero's unit id.
fn host_with_picked_antimage() -> (ModHost, usize) {
    let image = build_image(&scenario()).expect("moba scenario builds an image");
    let mut host = ModHost::new(&image).expect("host");
    let rules = std::fs::read_to_string(scenario().join("attr_rules.txt")).expect("rules");
    assert!(host.sim_mut().set_attr_rules_text(&rules), "the pack's rules file loads");
    let mods = common::pack_mods(&scenario(), &common::pack_roots(&scenario()));
    host.load_mods(&mods).expect("moba mods load");
    let ui: Value = serde_json::from_str(&std::fs::read_to_string(scenario().join("moba_ui.json")).expect("ui")).expect("ui json");
    host.dispatch_action("setup", &json!({ "ui": ui, "player": 0 })).expect("setup");
    host.dispatch_action("seat_pick", &json!({ "player": 0, "hero": "antimage" })).expect("seat_pick");
    host.advance().expect("tick");
    let kind = host.sim().catalog().unit_index("antimage").expect("antimage is a unit kind") as i32;
    let hero = (0..host.sim().count())
        .find(|&i| host.sim().alive(i) && host.sim().unit_type(i) == kind)
        .expect("the picked antimage is alive");
    (host, hero)
}

// The cast keys of moba_ui.json as (key, slot), in slot order.
fn cast_keys_by_slot() -> Vec<(String, usize)> {
    let ui: Value = serde_json::from_str(&std::fs::read_to_string(scenario().join("moba_ui.json")).expect("ui")).expect("ui json");
    let mut keys: Vec<(String, usize)> = ui["keys"]["cast"]
        .as_object()
        .expect("keys.cast is an object")
        .iter()
        .map(|(key, slot)| (key.clone(), slot.as_u64().expect("slot is a number") as usize))
        .collect();
    keys.sort_by_key(|&(_, slot)| slot);
    keys
}

// Selects the hero, runs past a refresh, and returns the tree of the selection panel that draws the hero (dir "col").
fn selected_tree(host: &mut ModHost, hero: usize) -> Value {
    host.dispatch_action("selection", &json!({ "player": 0, "ids": [hero as i64] })).expect("selection");
    for _ in 0..5 {
        host.advance().expect("tick");
    }
    // A UI body is the event's members without the enclosing braces: "kind":"widget","data":{...}.
    host.take_ui()
        .iter()
        .filter_map(|body| serde_json::from_str::<Value>(&format!("{{{body}}}")).ok())
        .filter(|v| v["data"]["id"] == "select")
        .map(|v| v["data"]["tree"].clone())
        .filter(|tree| tree["dir"] == "col")
        .last()
        .expect("the selection panel draws the selected hero")
}

// The ability cells of the selection panel: its second row, after the face and stats row.
fn ability_cells(tree: &Value) -> Vec<Value> {
    let row = &tree["kids"][1];
    assert_eq!(row["dir"], "row", "the second row of the selection panel is the ability bar: {tree}");
    row["kids"].as_array().expect("bar cells").clone()
}

#[test]
fn the_ability_bar_has_one_cell_per_key_with_its_slots_ability() {
    let (mut host, hero) = host_with_picked_antimage();
    let slots: Vec<Option<String>> = {
        let sim = host.sim();
        let cat = sim.catalog();
        (0..ABILITY_SLOTS)
            .map(|slot| sim.ability_in_slot(hero, slot).map(|ai| cat.name(cat.abilities[ai].name).to_owned()))
            .collect()
    };
    let cast = cast_keys_by_slot();
    let tree = selected_tree(&mut host, hero);
    let cells = ability_cells(&tree);

    assert_eq!(cells.len(), cast.len(), "one cell per cast key: {cells:?}");
    // Each cell as (key label, ability name); an empty slot's cell has tone "empty" and shows no name.
    let bar: Vec<(String, Option<String>)> = cells
        .iter()
        .map(|cell| {
            let kids = cell["kids"].as_array().expect("cell kids");
            let key = kids[0]["text"].as_str().expect("key label").to_owned();
            let name = if cell["tone"] == "empty" { None } else { Some(kids[1]["text"].as_str().expect("name label").to_owned()) };
            (key, name)
        })
        .collect();
    let want: Vec<(String, Option<String>)> = cast.iter().map(|(key, slot)| (key.clone(), slots[*slot].clone())).collect();
    assert_eq!(bar, want, "each key shows the ability in its slot");

    // The antimage's slots are mana break, blink, counterspell, talents; the bar names them under q, w, e, r, and t is empty.
    let names: Vec<Option<&str>> = bar.iter().map(|(_, name)| name.as_deref()).collect();
    assert_eq!(
        names,
        vec![Some("antimage_mana_break"), Some("antimage_blink"), Some("antimage_counterspell"), Some("antimage_talents"), None],
        "the bar reads q w e r t as the hero's slots 0 to 4"
    );

    let mut shown: Vec<&str> = names.iter().flatten().copied().collect();
    let count = shown.len();
    shown.sort_unstable();
    shown.dedup();
    assert_eq!(shown.len(), count, "no ability name repeats across two keys: {bar:?}");
}
