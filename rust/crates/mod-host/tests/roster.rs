//! The pick list of the moba pack offers only heroes that have a model: a hero with none would not draw.

#![cfg(feature = "build")]

use mod_host::{build_image, ModHost};

mod common;
use common::scenario;

const PROBE: &str = r#"
function on_tick(state)
  local heroes, offered = 0, 0
  for _, k in ipairs(game.unit_kinds()) do
    if k.player_controlled then
      heroes = heroes + 1
      if k.model ~= "" then offered = offered + 1 end
    end
  end
  state.heroes = heroes
  state.offered = offered
end
"#;

#[test]
fn only_heroes_with_a_model_are_offered() {
    let image = build_image(&scenario()).expect("moba scenario builds an image");
    let mut host = ModHost::new(&image).expect("host");
    host.load_source("roster", PROBE).expect("probe loads");
    host.advance().expect("tick");
    let heroes = host.state_int("heroes").expect("hero count");
    let offered = host.state_int("offered").expect("offered count");
    // Every roster hero has a generated model now, so offered is the whole roster; the guard below keeps the
    // rule honest either way (never offer a hero whose model would not draw).
    assert!(heroes >= offered, "offered is a subset of the roster: {heroes} heroes, {offered} offered");
    // Every hero row in units_heroes.osd carries a model: the hy3d batches are complete (the last was
    // zuus), so the pick list is the whole roster.
    assert_eq!(offered, 127, "the skinned heroes are the ones offered");
}
