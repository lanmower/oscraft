use mod_host::ModHost;
use serde_json::{json, Value};

use crate::checks::{pack_run, MAX_UNITS};

const TPS: u32 = sim::TICKS_PER_SECOND as u32;
const ROUND_S: u32 = 600;
const KINDS: [(&str, i32); 3] = [("super_creep_golem", 2000), ("super_creep_fish", 5500), ("super_creep_hydra", 5800)];
const GROUND_ORDER: [&str; 12] = [
    "courier", "courier_penguin", "courier_raccoon", "courier_crab", "courier_seal", "courier_skink", "courier_dog",
    "courier_dune_worm", "courier_carrion_beetle", "courier_fel_boar", "courier_rabbit", "courier_pack_horse",
];
const FLYING_ORDER: [&str; 7] = ["courier_flying", "courier_zeppelin", "courier_vulture", "courier_owl", "courier_viper", "courier_frost_wyrm", "courier_flying_panda"];
const SPECIALS: [&str; 3] = ["courier_mini_pudge", "courier_grunt", "courier_pandaren"];
const TECHIES: &str = "courier_mini_techies";

fn events(host: &mut ModHost) -> Result<Vec<Value>, String> {
    if !host.disabled_hooks().is_empty() {
        return Err(format!("a pack hook was switched off: {:?} {:?}", host.disabled_hooks(), host.hook_errors()));
    }
    host.flush_ui();
    host.take_ui()
        .into_iter()
        .map(|body| serde_json::from_str::<Value>(&format!("{{{body}}}")).map_err(|e| format!("ui event {body}: {e}")))
        .collect()
}

fn ui(host: &mut ModHost, player: usize, call: &str, args: Value) -> Result<(), String> {
    host.dispatch_action("ui", &json!({ "call": call, "args": args, "player": player, "pubkey": format!("k{player}") }))?;
    events(host).map(|_| ())
}

fn expect(label: &str, ok: bool, detail: impl FnOnce() -> String) -> Result<(), String> {
    if ok { Ok(()) } else { Err(format!("{label}: {}", detail())) }
}

fn super_units(host: &ModHost) -> Vec<(usize, String, u8)> {
    let s = host.sim();
    (0..MAX_UNITS)
        .filter(|&u| s.alive(u))
        .filter_map(|u| s.kind_name(u).filter(|n| n.starts_with("super_creep_")).map(|n| (u, n.to_owned(), s.player(u))))
        .collect()
}

fn run_with_mode(code: Option<&str>, seconds: u32) -> Result<(ModHost, Vec<String>), String> {
    let mut host = pack_run::start()?;
    events(&mut host)?;
    ui(&mut host, 0, "claim", json!({ "slot": 0 }))?;
    if let Some(code) = code {
        ui(&mut host, 0, "mode", json!({ "code": code }))?;
    }
    let mut said = Vec::new();
    for _ in 0..seconds {
        pack_run::advance(&mut host, TPS)?;
        for e in events(&mut host)? {
            if e["kind"] == "message" {
                said.push(e["text"].as_str().unwrap_or("").to_owned());
            }
        }
    }
    Ok((host, said))
}

pub fn super_creeps_round(_image: &[u8]) -> Result<(), String> {
    let (host, said) = run_with_mode(Some("sc"), ROUND_S - 1)?;
    expect("no super creep before the first 600 s round", super_units(&host).is_empty(), || format!("{:?}", super_units(&host)))?;
    let (mut host, _) = (host, said);
    pack_run::advance(&mut host, 2 * TPS)?;
    let spawned = super_units(&host);
    expect("one super creep per side at 600 s", spawned.len() == 2, || format!("{spawned:?}"))?;
    let (first, second) = (&spawned[0], &spawned[1]);
    expect("both sides get the same kind", first.1 == second.1, || format!("{} vs {}", first.1, second.1))?;
    expect("sides own their creep", first.2 != second.2 && first.2 <= 1 && second.2 <= 1, || format!("owners {} and {}", first.2, second.2))?;
    let hp = KINDS.iter().find(|(k, _)| *k == first.1).map(|(_, hp)| *hp).ok_or_else(|| format!("unknown kind {}", first.1))?;
    for (u, _, _) in &spawned {
        let got = host.sim().hp(*u);
        expect("6.88g hit points", got == sim::fixed::from_int(hp), || format!("unit {u} hp {got}, want {}", sim::fixed::from_int(hp)))?;
    }
    let before: Vec<(i32, i32)> = spawned.iter().map(|(u, _, _)| host.sim().position_fixed(*u)).collect();
    pack_run::advance(&mut host, 20 * TPS)?;
    for ((u, _, _), start) in spawned.iter().zip(&before) {
        let now = host.sim().position_fixed(*u);
        let moved = (f64::from(now.0 - start.0).powi(2) + f64::from(now.1 - start.1).powi(2)).sqrt() / 65536.0;
        expect("a super creep walks its lane", moved >= 3.0, || format!("unit {u} moved {moved:.1} tiles"))?;
    }
    println!("super creeps round ok kind={} hp={hp}", first.1);
    Ok(())
}

pub fn super_creeps_message(_image: &[u8]) -> Result<(), String> {
    let (_, said) = run_with_mode(Some("sc"), ROUND_S + 1)?;
    let lines: Vec<&String> = said.iter().filter(|m| m.contains("has spawned for the")).collect();
    expect("one message per side", lines.len() == 2, || format!("{lines:?}"))?;
    expect("Sentinel message", lines.iter().any(|m| m.contains("for the Sentinel and is taking the")), || format!("{lines:?}"))?;
    expect("Scourge message", lines.iter().any(|m| m.contains("for the Scourge and is taking the")), || format!("{lines:?}"))?;
    let path = |m: &str| m.rsplit("taking the ").next().unwrap_or("").to_owned();
    expect("both sides share the lane", path(lines[0]) == path(lines[1]), || format!("{lines:?}"))?;
    println!("super creeps message ok {}", lines[0]);
    Ok(())
}

pub fn super_creeps_off(_image: &[u8]) -> Result<(), String> {
    let (host, _) = run_with_mode(None, ROUND_S + 1)?;
    expect("no super creeps without the sc mode", super_units(&host).is_empty(), || format!("{:?}", super_units(&host)))?;
    println!("super creeps off ok");
    Ok(())
}

fn courier_of(host: &ModHost, player: u8) -> Option<usize> {
    let s = host.sim();
    (0..MAX_UNITS).find(|&u| s.alive(u) && s.player(u) == player && s.kind_name(u).is_some_and(|n| n.starts_with("courier")))
}

fn seated() -> Result<(ModHost, usize), String> {
    let mut host = pack_run::start()?;
    host.dispatch_action("seat_pick", &json!({"player": 0, "hero": "antimage"}))?;
    pack_run::advance(&mut host, 4)?;
    events(&mut host)?;
    let courier = courier_of(&host, 0).ok_or("seat 0 has no courier")?;
    host.dispatch_action("select", &json!({"player": 0, "ids": [courier]}))?;
    events(&mut host)?;
    Ok((host, courier))
}

fn press_skin(host: &mut ModHost, courier: usize) -> Result<String, String> {
    let args = json!({"key": "g", "phase": "down", "shift": false, "ctrl": false, "alt": false, "meta": false, "player": 0});
    host.dispatch_action("key", &args)?;
    events(host)?;
    pack_run::advance(host, 4)?;
    host.sim().kind_name(courier).map(str::to_owned).ok_or_else(|| "the courier lost its kind".to_owned())
}

fn allowed_after(previous: &str) -> Result<Vec<&'static str>, String> {
    if SPECIALS.contains(&previous) {
        return Ok(vec![TECHIES]);
    }
    if previous == TECHIES {
        return Ok(vec!["courier_rabbit"]);
    }
    let at = GROUND_ORDER.iter().position(|k| *k == previous).ok_or_else(|| format!("kind {previous} is outside the ground cycle"))?;
    let mut allowed = vec![GROUND_ORDER[(at + 1) % GROUND_ORDER.len()]];
    allowed.extend(SPECIALS);
    Ok(allowed)
}

pub fn courier_skin_order(_image: &[u8]) -> Result<(), String> {
    let (mut host, courier) = seated()?;
    let mut seen = std::collections::BTreeSet::new();
    let mut previous = "courier".to_owned();
    for press in 0..600 {
        let now = press_skin(&mut host, courier)?;
        let allowed = allowed_after(&previous)?;
        expect("6.88g ground order", allowed.contains(&now.as_str()), || format!("press {press}: {previous} became {now}, allowed {allowed:?}"))?;
        seen.insert(now.clone());
        previous = now;
    }
    for kind in SPECIALS.iter().chain([&TECHIES]) {
        expect("random skins show up", seen.contains(*kind), || format!("{kind} never appeared in {seen:?}"))?;
    }
    for (at, from) in GROUND_ORDER.iter().enumerate() {
        let want = GROUND_ORDER[(at + 1) % GROUND_ORDER.len()];
        let mut reached = false;
        for _ in 0..200 {
            let k = crate::checks::kind(&host.sim(), from)?;
            expect("morph into the start kind", host.sim_mut().morph(courier, k), || format!("morph to {from} refused"))?;
            if press_skin(&mut host, courier)? == want {
                reached = true;
                break;
            }
        }
        expect("every ordered step is reachable", reached, || format!("{from} never became {want}"))?;
    }
    println!("courier skin order ok {} kinds seen in the walk", seen.len());
    Ok(())
}

pub fn courier_skin_flying(_image: &[u8]) -> Result<(), String> {
    let (mut host, _) = seated()?;
    let flying = {
        let mut s = host.sim_mut();
        let k = crate::checks::kind(&s, "courier_flying")?;
        s.spawn_for(0, k, sim::fixed::from_int(60), sim::fixed::from_int(268)).ok_or("flying courier refused")?
    };
    host.dispatch_action("select", &json!({"player": 0, "ids": [flying]}))?;
    events(&mut host)?;
    for round in 0..2 {
        for step in 1..=FLYING_ORDER.len() {
            let now = press_skin(&mut host, flying)?;
            let want = FLYING_ORDER[step % FLYING_ORDER.len()];
            expect("6.88g flying order", now == want, || format!("round {round} step {step}: {now}, want {want}"))?;
        }
    }
    let hp = host.sim().hp(flying);
    expect("a morph keeps the flying courier alive at full life", hp == sim::fixed::from_int(150), || format!("hp {hp}"))?;
    println!("courier skin flying ok");
    Ok(())
}
