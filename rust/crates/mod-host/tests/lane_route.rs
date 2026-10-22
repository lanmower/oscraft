//! The moba map's lane routes are readable from the pack: route_len and route_point give a side's lane path, and it
//! starts at that side's base. The bots walk these points (bot_lib.lua walk_lane).

#![cfg(feature = "build")]

use mod_host::{build_image, ModHost};

mod common;
use common::scenario;

const PROBE: &str = r#"
function on_tick(state)
  state.top_len = route_len(0, 1)
  state.mid_len = route_len(0, 2)
  state.bot_len = route_len(0, 3)
  state.missing = route_len(0, 9)
  local x, y = route_point(0, 1, 0)
  state.first_x = x
  state.first_y = y
end
"#;

#[test]
fn the_lane_routes_are_readable_and_start_at_the_side_base() {
    let image = build_image(&scenario()).expect("moba scenario builds an image");
    let mut host = ModHost::new(&image).expect("host");
    host.load_source("lanes", PROBE).expect("probe loads");
    host.advance().expect("tick");
    for lane in ["top_len", "mid_len", "bot_len"] {
        assert!(host.state_int(lane).unwrap_or(0) > 1, "{lane} is a route with points");
    }
    assert_eq!(host.state_int("missing"), Some(0), "a route the map does not have has no points");
    // routes.osd: route(0, 1) starts at (60500, 221500) milli, which is 60.5 and 221.5 cells in fixed-point (65536 a cell).
    let fx = host.state_int("first_x").expect("first x");
    let fy = host.state_int("first_y").expect("first y");
    assert!((fx - 60_500 * 65_536 / 1000).abs() < 65_536 * 2, "the top route starts at its base x: {fx}");
    assert!((fy - 221_500 * 65_536 / 1000).abs() < 65_536 * 2, "the top route starts at its base y: {fy}");
}
