use mod_host::ModHost;
use serde_json::{json, Value};

use sim::fixed;

use crate::checks::{kind, pack_run, MAX_UNITS};
use crate::mod_checks;

const TPS: u32 = sim::TICKS_PER_SECOND as u32;
const FIX: f64 = 65536.0;

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

fn press(host: &mut ModHost, player: usize, key: &str) -> Result<Vec<Value>, String> {
    let args = json!({"key": key, "phase": "down", "shift": false, "ctrl": false, "alt": false, "meta": false, "player": player});
    host.dispatch_action("key", &args)?;
    events(host)
}

fn select(host: &mut ModHost, player: usize, ids: &[usize]) -> Result<(), String> {
    host.dispatch_action("select", &json!({"player": player, "ids": ids}))?;
    events(host).map(|_| ())
}

fn refusals(out: &[Value], reason: &str) -> usize {
    out.iter().filter(|e| e["kind"] == "feedback" && e["data"]["kind"] == "refusal" && e["data"]["reason"] == reason).count()
}

fn find(host: &ModHost, player: u8, name: &str) -> Result<usize, String> {
    let s = host.sim();
    (0..MAX_UNITS)
        .find(|&u| s.alive(u) && s.player(u) == player && s.kind_name(u) == Some(name))
        .ok_or_else(|| format!("player {player} has no {name}"))
}

fn seated() -> Result<(ModHost, usize, usize), String> {
    let mut host = pack_run::start()?;
    host.dispatch_action("seat_pick", &json!({"player": 0, "hero": "antimage"}))?;
    pack_run::advance(&mut host, 4)?;
    events(&mut host)?;
    let hero = {
        let s = host.sim();
        (0..MAX_UNITS).find(|&u| s.alive(u) && s.player(u) == 0 && s.is_attributed(u)).ok_or("seat 0 has no hero")?
    };
    let courier = find(&host, 0, "courier")?;
    Ok((host, hero, courier))
}

fn tiles(host: &ModHost, unit: usize) -> (f64, f64) {
    let (x, y) = host.sim().position_fixed(unit);
    (f64::from(x) / FIX, f64::from(y) / FIX)
}

fn reach(from: (f64, f64), to: (f64, f64)) -> f64 {
    ((from.0 - to.0).powi(2) + (from.1 - to.1).powi(2)).sqrt()
}

fn expect(label: &str, ok: bool, detail: impl FnOnce() -> String) -> Result<(), String> {
    if ok { Ok(()) } else { Err(format!("{label}: {}", detail())) }
}

pub fn courier_secret_shop(_image: &[u8]) -> Result<(), String> {
    let (mut host, _, courier) = seated()?;
    select(&mut host, 0, &[courier])?;
    let depot = (92.0, 145.0);
    let before = reach(tiles(&host, courier), depot);
    press(&mut host, 0, "f")?;
    pack_run::advance(&mut host, 3 * TPS)?;
    let after = reach(tiles(&host, courier), depot);
    expect("courier walks to the secret depot", before - after >= 8.0, || format!("distance {before:.1} -> {after:.1} tiles"))?;
    println!("courier secret shop ok {before:.1} -> {after:.1}");
    Ok(())
}

pub fn courier_drop_pickup(_image: &[u8]) -> Result<(), String> {
    let (mut host, _, home_courier) = seated()?;
    let courier_kind = kind(&host.sim(), "courier")?;
    let away = (fixed::from_int(92), fixed::from_int(145));
    let courier = host.sim_mut().spawn_for(0, courier_kind, away.0, away.1).ok_or("spawn_for refused a courier away from the heroes")?;
    expect("a second courier", courier != home_courier, || "spawn_for reused the courier".into())?;
    let item = host.sim().catalog().item_index("broadsword").ok_or("catalog has no broadsword")? as u32 + 1;
    expect("grant", host.sim_mut().grant_item(courier, item as usize - 1, 1), || "grant_item refused".into())?;
    select(&mut host, 0, &[courier])?;
    let ground = host.sim().ground_items().len();
    press(&mut host, 0, "d")?;
    pack_run::advance(&mut host, 4)?;
    expect("drop empties the courier", host.sim().inventory_count(courier, item) == 0, || "courier still holds the item".into())?;
    expect("drop lands on the ground", host.sim().ground_items().len() == ground + 1, || "ground item missing".into())?;
    let again = press(&mut host, 0, "d")?;
    expect("drop is on cooldown", refusals(&again, "cooldown") == 1, || format!("{again:?}"))?;
    press(&mut host, 0, "a")?;
    pack_run::advance(&mut host, 4)?;
    expect("pickup takes only the owner's items from couriers beside the hero", host.sim().ground_items().len() == ground + 1, || "an unowned ground item was taken".into())?;
    println!("courier drop pickup ok");
    Ok(())
}

fn item_kind(host: &ModHost, name: &str) -> Result<usize, String> {
    host.sim().catalog().item_index(name).ok_or_else(|| format!("catalog has no item {name}"))
}

fn start_of(host: &ModHost, player: u8) -> Result<(i32, i32), String> {
    host.sim().catalog().starts.iter().find(|s| s.player == player).map(|s| (s.x, s.y)).ok_or_else(|| format!("no start for seat {player}"))
}

pub fn courier_drop_filters(_image: &[u8]) -> Result<(), String> {
    let (mut host, hero, courier) = seated()?;
    let (sword, haste, aegis) = (item_kind(&host, "broadsword")?, item_kind(&host, "bottle_haste")?, item_kind(&host, "aegis")?);
    for item in [sword, haste, aegis] {
        expect("grant", host.sim_mut().grant_item(courier, item, 1), || "grant_item refused".into())?;
    }
    select(&mut host, 0, &[courier])?;
    let ground = host.sim().ground_items().len();
    press(&mut host, 0, "d")?;
    pack_run::advance(&mut host, 4)?;
    expect("a filled bottle stays on the courier", host.sim().inventory_count(courier, haste as u32 + 1) == 1, || "bottle dropped".into())?;
    expect("the Aegis stays on the courier", host.sim().inventory_count(courier, aegis as u32 + 1) == 1, || "aegis dropped".into())?;
    expect("an ordinary item leaves the courier", host.sim().inventory_count(courier, sword as u32 + 1) == 0, || "sword kept".into())?;
    let start = start_of(&host, 0)?;
    let (hx, hy) = host.sim().position_fixed(hero);
    let at_start = i64::from(hx - start.0).abs() + i64::from(hy - start.1).abs() <= i64::from(fixed::from_int(2));
    let returned = host.sim().inventory_count(hero, sword as u32 + 1) == 1 || host.sim().ground_items().iter().any(|g| (g.x, g.y) == start);
    expect("a courier in its base returns an ally's item to that seat's hero start", at_start && returned && host.sim().ground_items().len() <= ground + 1, || format!("hero at ({hx}, {hy}), start {start:?}"))?;
    println!("courier drop filters ok");
    Ok(())
}

pub fn courier_pickup_owner(_image: &[u8]) -> Result<(), String> {
    let (mut host, hero, courier) = seated()?;
    let (sword, branch) = (item_kind(&host, "broadsword")?, item_kind(&host, "branches")?);
    for item in [sword, branch] {
        expect("grant", host.sim_mut().grant_item(courier, item, 1), || "grant_item refused".into())?;
    }
    let ally = (1..10u8).find(|&p| host.sim().team_of(p) == host.sim().team_of(0)).ok_or("no ally seat")?;
    host.sim_mut().set_item_owner(courier, 1, usize::from(ally)).map_err(|e| format!("set_item_owner: {e:?}"))?;
    select(&mut host, 0, &[courier])?;
    press(&mut host, 0, "a")?;
    pack_run::advance(&mut host, 6)?;
    expect("the hero receives the item it owns", host.sim().inventory_count(hero, sword as u32 + 1) == 1, || "hero holds no broadsword".into())?;
    expect("the courier keeps an ally's item", host.sim().inventory_count(courier, branch as u32 + 1) == 1, || "branches left the courier".into())?;
    println!("courier pickup owner ok");
    Ok(())
}

pub fn courier_ping(_image: &[u8]) -> Result<(), String> {
    let (mut host, _, courier) = seated()?;
    select(&mut host, 0, &[courier])?;
    let out = press(&mut host, 0, "w")?;
    let pings = out.iter().filter(|e| e["kind"] == "ping" && e["data"]["player"] == 0).count();
    expect("ping reaches the owner", pings == 1, || format!("{out:?}"))?;
    let again = press(&mut host, 0, "w")?;
    expect("ping is on cooldown", refusals(&again, "cooldown") == 1, || format!("{again:?}"))?;
    println!("courier ping ok");
    Ok(())
}

pub fn courier_burst(_image: &[u8]) -> Result<(), String> {
    let (mut host, _, courier) = seated()?;
    select(&mut host, 0, &[courier])?;
    let ground = press(&mut host, 0, "r")?;
    expect("the ground courier has no burst", refusals(&ground, "noAbility") == 1, || format!("{ground:?}"))?;
    let flying_kind = kind(&host.sim(), "courier_flying")?;
    let (x, y) = host.sim().position_fixed(courier);
    let flying = host.sim_mut().spawn_for(0, flying_kind, x, y).ok_or("spawn_for refused the flying courier")?;
    select(&mut host, 0, &[flying])?;
    press(&mut host, 0, "r")?;
    pack_run::advance(&mut host, 2)?;
    let cooling: Vec<i32> = (0..2).map(|slot| host.sim().ability_cooldown(flying, slot)).collect();
    expect("burst starts its cooldown", cooling.iter().any(|&c| c > 0), || format!("cooldowns {cooling:?}"))?;
    let again = press(&mut host, 0, "r")?;
    expect("burst refuses while cooling", refusals(&again, "cooldown") == 1, || format!("{again:?}"))?;
    println!("courier burst ok {cooling:?}");
    Ok(())
}

pub fn glyph_fortification(_image: &[u8]) -> Result<(), String> {
    let (mut host, hero, _) = seated()?;
    let tower = find(&host, 0, "tower")?;
    let foe = hero as i32;
    select(&mut host, 0, &[])?;
    let before = host.sim().hp(tower);
    let first = press(&mut host, 0, "f")?;
    expect("the first glyph is taken", refusals(&first, "cooldown") == 0, || format!("{first:?}"))?;
    pack_run::advance(&mut host, 2)?;
    host.sim_mut().hit_by(tower, 100, foe);
    expect("a glyphed tower takes no damage", host.sim().hp(tower) == before, || format!("hp {before} -> {}", host.sim().hp(tower)))?;
    let second = press(&mut host, 0, "f")?;
    expect("the team cooldown refuses a second glyph", refusals(&second, "cooldown") == 1, || format!("{second:?}"))?;
    pack_run::advance(&mut host, 5 * TPS + 4)?;
    host.sim_mut().hit_by(tower, 100, foe);
    expect("the glyph ends after five seconds", host.sim().hp(tower) < before, || format!("hp stayed {before}"))?;
    host.sim_mut().hurt(tower, i32::MAX / 2);
    pack_run::advance(&mut host, 2)?;
    events(&mut host)?;
    let third = press(&mut host, 0, "f")?;
    expect("losing a tier one tower resets the cooldown", refusals(&third, "cooldown") == 0, || format!("{third:?}"))?;
    println!("glyph ok");
    Ok(())
}

fn say(host: &mut ModHost, line: &str) -> Result<Vec<Value>, String> {
    host.dispatch_action("chat", &json!({"text": line, "player": 0, "pubkey": "k0", "cmd": line}))?;
    events(host)
}

pub fn double_hold(_image: &[u8]) -> Result<(), String> {
    let (mut host, hero, _) = seated()?;
    select(&mut host, 0, &[hero])?;
    press(&mut host, 0, "h")?;
    pack_run::advance(&mut host, 1)?;
    press(&mut host, 0, "h")?;
    pack_run::advance(&mut host, 1)?;
    expect("double hold is off by default", !host.sim().holds_fire(hero), || "hold fire set while the feature is off".into())?;
    let on = say(&mut host, "-hold")?;
    expect("-hold turns the feature on", on.iter().any(|e| e["kind"] == "message" && e["text"].as_str().is_some_and(|t| t.contains("ON"))), || format!("{on:?}"))?;
    pack_run::advance(&mut host, TPS)?;
    press(&mut host, 0, "h")?;
    pack_run::advance(&mut host, 1)?;
    expect("one press only holds", !host.sim().holds_fire(hero), || "hold fire after one press".into())?;
    press(&mut host, 0, "h")?;
    pack_run::advance(&mut host, 1)?;
    expect("two presses within 0.2 s hold fire", host.sim().holds_fire(hero), || "hold fire missing".into())?;
    pack_run::advance(&mut host, TPS)?;
    press(&mut host, 0, "h")?;
    pack_run::advance(&mut host, 1)?;
    expect("a later press holds position again", !host.sim().holds_fire(hero), || "hold fire kept".into())?;
    let off = say(&mut host, "-hold")?;
    expect("-hold turns the feature off", off.iter().any(|e| e["kind"] == "message" && e["text"].as_str().is_some_and(|t| t.contains("OFF"))), || format!("{off:?}"))?;
    println!("double hold ok");
    Ok(())
}

pub fn sound_targeting(image: &[u8]) -> Result<(), String> {
    let src = "function on_tick(state) if game.ticks() == 1 then game.sound(\"everyone\") game.sound(\"one\", 3) end end";
    let (mut host, _guard) = mod_checks::start(image, "ctl-sound", src)?;
    mod_checks::advance(&mut host, 3)?;
    let out = events(&mut host)?;
    let heard: Vec<(String, i64)> = out
        .iter()
        .filter(|e| e["kind"] == "sound")
        .map(|e| (e["name"].as_str().unwrap_or("").to_owned(), e["player"].as_i64().unwrap_or(-9)))
        .collect();
    expect("sound targets", heard == [("everyone".to_owned(), -1), ("one".to_owned(), 3)], || format!("{heard:?}"))?;
    println!("sound targeting ok {heard:?}");
    Ok(())
}

pub fn announce_sounds(_image: &[u8]) -> Result<(), String> {
    let mut host = pack_run::start()?;
    let home = host.sim().team_of(0);
    let foe = (1..10u8).find(|&p| host.sim().team_of(p) != home).ok_or("no opposing seat")?;
    for p in [0, foe] {
        host.dispatch_action("seat_pick", &json!({"player": p, "hero": "antimage"}))?;
    }
    pack_run::advance(&mut host, 4)?;
    events(&mut host)?;
    let (killer, victim) = {
        let s = host.sim();
        let of = |p: u8| (0..MAX_UNITS).find(|&u| s.alive(u) && s.player(u) == p && s.is_attributed(u));
        (of(0).ok_or("no killer hero")?, of(foe).ok_or("no victim hero")?)
    };
    host.sim_mut().hit_by(victim, i32::MAX / 2, killer as i32);
    pack_run::advance(&mut host, 4)?;
    let first = events(&mut host)?;
    let named = |out: &[Value], name: &str| out.iter().filter(|e| e["kind"] == "sound" && e["name"] == name).count();
    expect("first blood plays its table entry", named(&first, "firstblood") == 1, || format!("{first:?}"))?;
    let tower = find(&host, foe, "tower")?;
    host.sim_mut().hit_by(tower, i32::MAX / 2, killer as i32);
    pack_run::advance(&mut host, 4)?;
    let fall = events(&mut host)?;
    let said = fall.iter().any(|e| e["kind"] == "message" && e["text"].as_str().is_some_and(|t| t.contains("destroyed a tower")));
    expect("the tower line is announced", said, || format!("{fall:?}"))?;
    expect("an id missing from the table plays nothing", !fall.iter().any(|e| e["kind"] == "sound"), || format!("{fall:?}"))?;
    println!("announce sounds ok");
    Ok(())
}
