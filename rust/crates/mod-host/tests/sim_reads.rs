//! Reads of the sim may nest: a loop over the unit count can read each unit through the host again. The browser's
//! units() does exactly that, and an exclusive borrow there trapped on every frame.

#![cfg(feature = "build")]

use std::path::Path;

use mod_host::ModHost;

#[test]
fn a_loop_over_the_sim_can_read_it_again_in_its_body() {
    let scenario = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../scenarios/moba");
    let image = mod_host::build_image(&scenario).expect("moba scenario builds an image");
    let host = ModHost::new(&image).expect("host");
    let mut living = 0;
    for i in 0..host.sim().count() {
        if host.sim().alive(i) {
            let _ = host.sim().position_fixed(i);
            living += 1;
        }
    }
    assert!(living > 0, "the moba match starts with units");
}
