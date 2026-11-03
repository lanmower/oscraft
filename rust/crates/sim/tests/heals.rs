mod common;

use std::path::Path;

use common::*;
use sim::fixed;

const PURIFICATION_HEAL: i32 = 90;

#[test]
fn an_ability_heal_counts_as_healing_done_by_its_caster() {
    let mut sim = open_field();
    let rules = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../scenarios/moba/attr_rules.txt"))
        .expect("the moba attribute rules");
    assert!(sim.set_attr_rules_text(&rules), "the moba attribute rules load");
    let kind = sim.catalog().unit_index("omniknight").expect("omniknight in the moba catalog") as u32;
    let healer = sim.spawn_attributed(PLAYER, kind, centre(4), centre(5)).expect("healer spawns");
    assert!(sim.grant_energy(healer, 400).is_ok());
    let ally = spawn(&mut sim, PLAYER, 5, 5);
    let hurt = sim.hp(ally) - fixed::from_int(200);
    assert!(sim.set_hp_within(ally, hurt));

    assert!(sim.give_ability(healer, 0, "omniknight_purification"));
    assert!(sim.rank_up(healer, 0).is_ok(), "the heal ability takes a rank");
    assert!(sim.cast_ability(healer, 0, ally as i32, 0, 0), "the heal ability casts on the ally");
    for _ in 0..60 {
        sim.tick();
    }

    assert_eq!(
        sim.counter(healer, "sim_healing"),
        i64::from(fixed::from_int(PURIFICATION_HEAL)),
        "the caster's healing done is the amount its ability healed"
    );
}
