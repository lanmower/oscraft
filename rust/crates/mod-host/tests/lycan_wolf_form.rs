use std::path::{Path, PathBuf};

use mod_host::{build_image, ModHost};
use sim::fixed::{self, HALF};

const HERO_KIND: &str = "lycan";
const WOLF_KIND: &str = "lycan_wolf_form";
const ABILITY: &str = "lycan_shapeshift";
const HERO_PLAYER: u8 = 0;
const FOE_PLAYER: u8 = 1;
const TICKS_PER_SECOND: u32 = 16;
const FORM_TICKS: u32 = 18 * TICKS_PER_SECOND;

fn pack_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../scenarios/moba")
}

fn collect(dir: &Path, rel: &str, out: &mut Vec<(String, String)>) {
    for entry in std::fs::read_dir(dir).expect("read the pack directory") {
        let path = entry.expect("read a pack entry").path();
        let name = path
            .file_name()
            .expect("a pack file name")
            .to_string_lossy()
            .into_owned();
        if path.is_dir() {
            if name != "fixtures" {
                collect(&path, &format!("{rel}{name}/"), out);
            }
        } else if path.extension().is_some_and(|e| e == "lua") {
            out.push((
                format!("{rel}{name}"),
                std::fs::read_to_string(&path).expect("read a lua module"),
            ));
        }
    }
}

fn stem(rel: &str) -> &str {
    let file = rel.rsplit('/').next().unwrap_or(rel);
    file.strip_suffix(".lua").unwrap_or(file)
}

fn visit(files: &[(String, String)], module: &str, out: &mut Vec<(String, String)>) {
    let Some((rel, src)) = files.iter().find(|(rel, _)| stem(rel) == module) else {
        return;
    };
    if out.iter().any(|(name, _)| name == rel) {
        return;
    }
    out.push((rel.clone(), src.clone()));
    for required in src
        .split("require(\"")
        .skip(1)
        .filter_map(|rest| rest.split('"').next())
    {
        visit(files, required, out);
    }
}

fn probe_host() -> ModHost {
    let dir = pack_dir();
    let rules = std::fs::read_to_string(dir.join("attr_rules.txt")).expect("the pack rules file");
    let mut files = Vec::new();
    collect(&dir, "", &mut files);
    files.sort();
    let mut mods = Vec::new();
    visit(&files, "trigger_lycan", &mut mods);
    let image = build_image(&dir).expect("build the moba image");
    let mut host = ModHost::new(&image).expect("load the moba image");
    assert!(
        host.sim_mut().set_attr_rules_text(&rules),
        "the pack rules are accepted"
    );
    host.load_mods(&mods).expect("load the lycan trigger");
    host.sim_mut().set_fog(false);
    host
}

fn spawn(host: &mut ModHost, kind_name: &str, player: u8, cell_x: i32, cell_y: i32) -> usize {
    let mut sim = host.sim_mut();
    let kind = sim
        .catalog()
        .unit_index(kind_name)
        .expect("the kind is in the moba catalog");
    let x = fixed::from_int(cell_x) + HALF;
    let y = fixed::from_int(cell_y) + HALF;
    sim.spawn_attributed(player, kind as u32, x, y)
        .expect("the unit fits on the map")
}

fn advance(host: &mut ModHost, ticks: u32) {
    for _ in 0..ticks {
        host.advance().expect("advance the match");
    }
}

fn kind_of(host: &ModHost, unit: usize) -> Option<String> {
    host.sim().kind_name(unit).map(str::to_owned)
}

#[test]
fn shapeshift_turns_the_lycan_into_the_wolf_form_for_eighteen_seconds_and_back() {
    let mut host = probe_host();
    let hero = spawn(&mut host, HERO_KIND, HERO_PLAYER, 92, 240);
    {
        let mut sim = host.sim_mut();
        assert!(sim.give_ability(hero, 0, ABILITY), "Shapeshift is granted");
        assert!(sim.rank_up(hero, 0).is_ok(), "Shapeshift takes its first rank");
        let max = sim.unit_max_mana(hero);
        assert!(sim.set_mana_within(hero, max), "the hero has the mana to shift");
        assert!(sim.cast_ability(hero, 0, -1, 0, 0), "the hero casts Shapeshift");
    }
    advance(&mut host, 1);
    assert_eq!(
        kind_of(&host, hero).as_deref(),
        Some(WOLF_KIND),
        "Shapeshift morphs the Lycanthrope into the wolf form (6.88g E015)"
    );

    let mut ticks_in_form = 1u32;
    while kind_of(&host, hero).as_deref() == Some(WOLF_KIND) {
        advance(&mut host, 1);
        ticks_in_form += 1;
        assert!(
            ticks_in_form < 2 * FORM_TICKS,
            "the wolf form ends within two form lengths"
        );
    }
    assert!(
        ticks_in_form.abs_diff(FORM_TICKS) <= 3,
        "the wolf form lasts 18 seconds (lasted {ticks_in_form} ticks)"
    );
    assert_eq!(
        kind_of(&host, hero).as_deref(),
        Some(HERO_KIND),
        "the Lycanthrope returns to his own form when the shift ends"
    );
    assert_eq!(
        host.sim().ability_rank(hero, 0),
        1,
        "the shift keeps the Shapeshift rank"
    );
}

const CRIT_CHANCE: usize = 6;
const CRIT_MULT: usize = 7;
const CRIT_MULT_PCT: i32 = 170;

fn crit_of(host: &ModHost, unit: usize) -> (i32, i32) {
    let sim = host.sim();
    (sim.stat(unit, CRIT_CHANCE), sim.stat(unit, CRIT_MULT))
}

fn shift_at_rank(rank: u32) -> (ModHost, usize) {
    let mut host = probe_host();
    let hero = spawn(&mut host, HERO_KIND, HERO_PLAYER, 92, 240);
    {
        let mut sim = host.sim_mut();
        assert!(sim.give_ability(hero, 0, ABILITY), "Shapeshift is granted");
        for _ in 0..rank {
            assert!(sim.rank_up(hero, 0).is_ok(), "Shapeshift takes another rank");
        }
        let max = sim.unit_max_mana(hero);
        assert!(sim.set_mana_within(hero, max), "the hero has the mana to shift");
        assert!(sim.cast_ability(hero, 0, -1, 0, 0), "the hero casts Shapeshift");
    }
    advance(&mut host, 1);
    (host, hero)
}

#[test]
fn shapeshift_grants_shapeshift_crit_strike_at_thirty_percent_and_one_point_seven_times_at_rank_one() {
    let (mut host, hero) = shift_at_rank(1);
    assert_eq!(
        crit_of(&host, hero),
        (30, CRIT_MULT_PCT),
        "Shapeshift Crit Strike gives 30% chance for 1.7 times damage at its first level (6.88g A3IO DataA/DataB)"
    );
    while kind_of(&host, hero).as_deref() == Some(WOLF_KIND) {
        advance(&mut host, 1);
    }
    advance(&mut host, 3);
    assert_eq!(
        crit_of(&host, hero).0,
        0,
        "the crit chance ends with the wolf form"
    );
}

#[test]
fn shapeshift_crit_strike_chance_rises_to_forty_percent_at_rank_three() {
    let (host, hero) = shift_at_rank(3);
    assert_eq!(
        crit_of(&host, hero),
        (40, CRIT_MULT_PCT),
        "Shapeshift Crit Strike gives 40% chance at its third level (6.88g A3IO DataA)"
    );
}

#[test]
fn the_wolf_form_swings_on_e015_damage_point_and_backswing() {
    let host = probe_host();
    let sim = host.sim();
    let kind = sim
        .catalog()
        .unit_index(WOLF_KIND)
        .expect("the wolf form is in the moba catalog");
    let ext = &sim.catalog().unit_ext[kind];
    assert_eq!(
        ext.attack_point, 260,
        "the wolf's damage point is 0.26 s (6.88g E015 dmgpt1 0.26)"
    );
    assert_eq!(
        ext.attack_backswing, 0,
        "the wolf has no backswing (6.88g E015 backSw1 0.0)"
    );
}

#[test]
fn dying_in_the_wolf_form_ends_it_and_the_revived_lycan_is_a_lycan_without_crit() {
    let (mut host, hero) = shift_at_rank(1);
    assert_eq!(
        kind_of(&host, hero).as_deref(),
        Some(WOLF_KIND),
        "the Lycanthrope is in the wolf form before he dies"
    );
    let lethal = host.sim().hp(hero) + fixed::from_int(1000);
    host.sim_mut()
        .damage_target(-1, FOE_PLAYER, lethal, hero as i32);
    advance(&mut host, 1);
    assert!(!host.sim().alive(hero), "the lethal hit kills the Lycanthrope");
    advance(&mut host, 8);
    let (x, y) = (fixed::from_int(92) + HALF, fixed::from_int(240) + HALF);
    assert!(
        host.sim_mut().revive_at_point(hero, x, y),
        "the Lycanthrope revives where he died"
    );
    advance(&mut host, 2);
    assert_eq!(
        kind_of(&host, hero).as_deref(),
        Some(HERO_KIND),
        "the death event ends the wolf form (6.88g TAO runs on EVENT_WIDGET_DEATH and removes the form), so he revives as a Lycan"
    );
    assert_eq!(
        crit_of(&host, hero).0,
        0,
        "the Shapeshift crit ends with the form (6.88g TAO removes A3IO on death)"
    );
}
