//! on_died receives the same six arguments, and the same payouts come back, on both backends.
//! Run once per backend and diff the PARITY lines: the native build with the default features, the
//! piccolo build with `--no-default-features --features "piccolo build"`.

#![cfg(feature = "build")]

use std::path::Path;

use mod_host::{Cmd, ModHost};
use sim::Payout;

fn image() -> Vec<u8> {
    let scenario = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../scenarios/moba");
    mod_host::build_image(&scenario).expect("moba scenario builds an image")
}

fn show(c: &Cmd) -> String {
    match c {
        Cmd::AddGold { player, amount } => format!("gold player={player} amount={amount}"),
        Cmd::AddUnitGold { unit, amount } => format!("unit_gold unit={unit} amount={amount}"),
        _ => "other".to_owned(),
    }
}

fn payout(victim: u32, victim_player: u8, killer: Option<u32>, killer_player: Option<u8>, bounty: i32) -> Payout {
    Payout { tick: 0, victim, victim_player, killer, killer_player, bounty }
}

const PROBE: &str = r#"
GOLD_BY_MOD = true
function on_tick(state) end
function on_died(unit, killer, killer_player, bounty, victim_player)
  game.cmd_add_unit_gold(unit, bounty)
  game.cmd_add_gold(victim_player, game.is_hero(killer) and 1 or 0)
  game.cmd_add_gold(killer_player or 0, killer or -1)
end
"#;

#[test]
fn on_died_receives_five_arguments_with_bounty() {
    let mut host = ModHost::new(&image()).expect("host");
    host.load_source("probe", PROBE).expect("probe loads");
    let fed = host.feed_payout(&payout(7, 2, Some(3), Some(1), 55)).expect("fed");
    let lines: Vec<String> = fed.iter().map(show).collect();
    println!("PARITY probe hero-kill {lines:?}");
    assert_eq!(lines, ["unit_gold unit=7 amount=55", "gold player=2 amount=1", "gold player=1 amount=3"]);
    let fed = host.feed_payout(&payout(7, 2, None, None, 9)).expect("fed");
    let lines: Vec<String> = fed.iter().map(show).collect();
    println!("PARITY probe attackerless {lines:?}");
    assert_eq!(lines, ["unit_gold unit=7 amount=9", "gold player=2 amount=0", "gold player=0 amount=-1"]);
}

#[test]
fn moba_match_pays_the_same_for_fixed_kills() {
    let src = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../mods/moba_match.lua")).expect("mod");
    let mut host = ModHost::new(&image()).expect("host");
    host.load_source("moba_match", &src).expect("moba_match loads");
    let kills = [
        ("hero kill", payout(10, 2, Some(20), Some(0), 40)),
        ("creep kill", payout(21, 1, Some(22), Some(1), 17)),
        ("attackerless", payout(30, 2, None, None, 30)),
        ("self kill", payout(31, 3, Some(31), Some(3), 12)),
        ("no bounty", payout(32, 0, Some(20), Some(0), 0)),
    ];
    let want: [&[&str]; 5] = [
        &["unit_gold unit=20 amount=32", "gold player=0 amount=8", "other"],
        &["gold player=1 amount=17"],
        &["gold player=1 amount=30"],
        &["gold player=0 amount=12"],
        &[],
    ];
    for ((name, p), want) in kills.iter().zip(want) {
        let fed = host.feed_payout(p).expect("fed");
        let lines: Vec<String> = fed.iter().map(show).collect();
        println!("PARITY payout {name} {lines:?}");
        assert_eq!(lines, want, "{name}");
    }
}
