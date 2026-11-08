use std::path::{Path, PathBuf};

use mod_host::{build_image, ModHost};
use sim::fixed::{self, HALF};

const SLOW: usize = 1;
const LEORIC_KIND: &str = "skeleton_king";
const ABILITY: &str = "skeleton_king_reincarnation_leoric";
const FOE_KIND: &str = "sentinel_creep";
const HERO_PLAYER: u8 = 0;
const FOE_PLAYER: u8 = 1;
const DELAY_TICKS: u32 = 48;
const RANK_ONE_COOLDOWN: i32 = 260 * 16;
const MANA_COST: i32 = 160;
const SLOW_KEEP: i32 = 16384;

const PROBE: &str = r#"
local aegis = require("aegis")
local probe = {}
probe.on_died = function(unit, killer, killer_player, bounty, victim_player, killer_attributed) aegis.on_died(unit) end
probe.on_tick = function(state) aegis.on_tick() end
return probe
"#;

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
    visit(&files, "aegis", &mut mods);
    mods.push((
        "leoric_reincarnation_probe.lua".to_owned(),
        PROBE.to_owned(),
    ));
    let image = build_image(&dir).expect("build the moba image");
    let mut host = ModHost::new(&image).expect("load the moba image");
    assert!(
        host.sim_mut().set_attr_rules_text(&rules),
        "the pack rules are accepted"
    );
    host.load_mods(&mods)
        .expect("load the reincarnation module and the probe");
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

#[test]
fn leoric_revives_where_he_died_after_three_seconds_and_slows_enemies_within_900_units() {
    let mut host = probe_host();
    let hero = spawn(&mut host, LEORIC_KIND, HERO_PLAYER, 92, 240);
    let near = spawn(&mut host, FOE_KIND, FOE_PLAYER, 97, 240);
    let far = spawn(&mut host, FOE_KIND, FOE_PLAYER, 112, 240);
    let (slot, mana_before) = {
        let mut sim = host.sim_mut();
        assert!(
            sim.give_ability(hero, 0, ABILITY),
            "Reincarnation Leoric is granted"
        );
        assert!(
            sim.rank_up(hero, 0).is_ok(),
            "Reincarnation Leoric takes its first rank"
        );
        let max = sim.unit_max_mana(hero);
        assert!(
            sim.set_mana_within(hero, max),
            "the hero has the mana to revive"
        );
        (0usize, sim.mana_of(hero))
    };
    assert!(mana_before >= MANA_COST);

    advance(&mut host, 1);
    let (death_x, death_y) = host.sim().position_fixed(hero);
    let lethal = host.sim().hp(hero) + fixed::from_int(1000);
    host.sim_mut()
        .damage_target(-1, FOE_PLAYER, lethal, hero as i32);
    let mut ticks_to_revive = 0u32;
    while !host.sim().alive(hero) {
        advance(&mut host, 1);
        ticks_to_revive += 1;
        assert!(
            ticks_to_revive < 4 * DELAY_TICKS,
            "the hero revives within four seconds"
        );
    }

    assert!(
        ticks_to_revive.abs_diff(DELAY_TICKS) <= 3,
        "the hero revives 3 seconds after death (took {ticks_to_revive} ticks)"
    );
    let sim = host.sim();
    assert_eq!(
        sim.position_fixed(hero),
        (death_x, death_y),
        "the hero revives where he died"
    );
    assert_eq!(
        sim.mana_of(hero),
        mana_before - MANA_COST,
        "the revive charges the mana cost"
    );
    let cooldown = sim.ability_cooldown(hero, slot);
    assert!(
        (cooldown - (RANK_ONE_COOLDOWN - DELAY_TICKS as i32)).abs() <= 3,
        "the revive leaves the 260 second cooldown running from the death ({cooldown} ticks)"
    );
    assert!(
        sim.status_left(near, SLOW) > 0,
        "the enemy 5 cells away is slowed"
    );
    assert_eq!(
        sim.slow_multiplier(near),
        SLOW_KEEP,
        "the slow keeps 25% of speed (75% slow)"
    );
    assert_eq!(
        sim.status_left(far, SLOW),
        0,
        "the enemy 20 cells away is outside the 900 unit radius"
    );
}
