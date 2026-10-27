use mod_host::ModHost;
use serde_json::{json, Value};

use crate::checks::{pack_run, role, MAX_UNITS};

const TPS: u32 = sim::TICKS_PER_SECOND as u32;

struct Out(Vec<Value>);

impl Out {
    fn messages(&self, player: i64) -> Vec<String> {
        self.0
            .iter()
            .filter(|e| e["kind"] == "message" && (e["player"] == player || e["player"] == -1))
            .map(|e| e["text"].as_str().unwrap_or("").to_owned())
            .collect()
    }

    fn said(&self, player: i64, needle: &str) -> bool {
        self.messages(player).iter().any(|m| m.contains(needle))
    }

    fn need(&self, player: i64, needle: &str) -> Result<(), String> {
        if self.said(player, needle) {
            return Ok(());
        }
        Err(format!("player {player} was not told {needle:?}; saw {:?}", self.messages(player)))
    }

    fn settings(&self, player: i64, key: &str) -> Vec<i64> {
        self.0
            .iter()
            .filter(|e| e["kind"] == "setting" && e["player"] == player && e["key"] == key)
            .filter_map(|e| e["value"].as_i64())
            .collect()
    }

    fn feedback(&self, player: i64, kind: &str) -> usize {
        self.0.iter().filter(|e| e["kind"] == "feedback" && e["data"]["player"] == player && e["data"]["kind"] == kind).count()
    }

    fn lobby_slots(&self) -> Option<Vec<(String, String)>> {
        let last = self.0.iter().rev().find(|e| e["kind"] == "lobby")?;
        let slots = last["data"]["slots"].as_array()?;
        Some(slots.iter().map(|s| (s["owner"].as_str().unwrap_or("").to_owned(), s["hero"].as_str().unwrap_or("").to_owned())).collect())
    }
}

fn drain(host: &mut ModHost) -> Result<Out, String> {
    if !host.disabled_hooks().is_empty() {
        return Err(format!("a pack hook was switched off: {:?} {:?}", host.disabled_hooks(), host.hook_errors()));
    }
    host.flush_ui();
    let events = host
        .take_ui()
        .into_iter()
        .map(|body| serde_json::from_str::<Value>(&format!("{{{body}}}")).map_err(|e| format!("ui event {body}: {e}")))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Out(events))
}

fn fresh_host() -> Result<ModHost, String> {
    let mut host = pack_run::start()?;
    drain(&mut host)?;
    Ok(host)
}

fn chat(host: &mut ModHost, player: usize, line: &str) -> Result<Out, String> {
    let mut args = json!({ "text": line, "player": player, "pubkey": format!("k{player}") });
    if line.starts_with('-') {
        let (cmd, rest) = line.split_once(' ').unwrap_or((line, ""));
        let digits: String = rest.trim().chars().take_while(|c| c.is_ascii_digit() || *c == '-').collect();
        args["cmd"] = json!(cmd.to_lowercase());
        args["n"] = json!(digits.parse::<i64>().unwrap_or(0));
    }
    host.dispatch_action("chat", &args)?;
    drain(host)
}

fn ui(host: &mut ModHost, player: usize, call: &str, args: Value) -> Result<Out, String> {
    host.dispatch_action("ui", &json!({ "call": call, "args": args, "player": player, "pubkey": format!("k{player}") }))?;
    drain(host)
}

fn claim(host: &mut ModHost, slots: usize) -> Result<(), String> {
    for slot in 0..slots {
        ui(host, slot, "claim", json!({ "slot": slot }))?;
    }
    Ok(())
}

fn pick_heroes(host: &mut ModHost) -> Result<Vec<usize>, String> {
    let mut ids = Vec::new();
    for player in 0..10usize {
        let hero = role(&format!("hero.{}", char::from(b'a' + player as u8)))?;
        host.dispatch_action("seat_pick", &json!({ "player": player, "hero": hero }))?;
        pack_run::advance(host, 2)?;
        let id = (0..MAX_UNITS)
            .find(|&u| host.sim().alive(u) && host.sim().player(u) as usize == player && host.sim().kind_name(u) == Some(hero))
            .ok_or_else(|| format!("seat_pick left player {player} without a {hero}"))?;
        ids.push(id);
    }
    drain(host)?;
    Ok(ids)
}

fn begin_match(host: &mut ModHost) -> Result<Vec<usize>, String> {
    claim(host, 10)?;
    for player in 0..10 {
        chat(host, player, "-random")?;
        ui(host, player, "ready", json!({ "slot": player, "ready": true }))?;
    }
    host.dispatch_action("ui", &json!({ "call": "world_ready", "args": {}, "player": 0 }))?;
    ui(host, 0, "start", json!({}))?;
    pick_heroes(host)
}

fn right_click(host: &mut ModHost, player: usize, x: f64, y: f64) -> Result<Out, String> {
    for kind in ["down", "up"] {
        host.dispatch_action("pointer", &json!({ "player": player, "kind": kind, "x": x, "y": y, "button": 2 }))?;
    }
    drain(host)
}

fn expect(label: &str, ok: bool, detail: impl FnOnce() -> String) -> Result<(), String> {
    if ok {
        return Ok(());
    }
    Err(format!("{label}: {}", detail()))
}

pub fn names(_image: &[u8]) -> Result<(), String> {
    let mut host = fresh_host()?;
    let first = chat(&mut host, 0, "-names")?;
    first.need(0, "Player's names turned into numbers")?;
    expect("names setting", first.settings(0, "names") == vec![1], || format!("{:?}", first.settings(0, "names")))?;
    chat(&mut host, 0, "-names")?.need(0, "turned into hero names")?;
    chat(&mut host, 0, "-names")?.need(0, "restored to default")?;
    let numbers = chat(&mut host, 0, "-names 1")?;
    numbers.need(0, "turned into numbers")?;
    let heroes = chat(&mut host, 0, "-names 2")?;
    heroes.need(0, "turned into hero names")?;
    expect("names 2 setting", heroes.settings(0, "names") == vec![2], || format!("{:?}", heroes.settings(0, "names")))?;
    let default = chat(&mut host, 0, "-names 7")?;
    default.need(0, "restored to default")?;
    expect("names other player", default.settings(1, "names").is_empty(), || "another player's setting changed".into())?;
    println!("names ok");
    Ok(())
}

pub fn gameinfo(_image: &[u8]) -> Result<(), String> {
    let mut host = fresh_host()?;
    claim(&mut host, 1)?;
    let plain = chat(&mut host, 0, "-gameinfo")?;
    plain.need(0, "Normal Mode")?;
    plain.need(0, "No mode has been specified")?;
    ui(&mut host, 0, "mode", json!({ "code": "ap" }))?;
    ui(&mut host, 0, "mode", json!({ "code": "fr" }))?;
    let info = chat(&mut host, 0, "-gameinfo")?;
    info.need(0, "|c006699CCAll Pick|r:")?;
    info.need(0, "You may pick a hero from any tavern.")?;
    info.need(0, "Reduces death times by 50%")?;
    expect("normal mode hidden", !info.said(0, "Normal Mode"), || "Normal Mode printed beside active modes".into())?;
    println!("gameinfo ok");
    Ok(())
}

pub fn matchup(_image: &[u8]) -> Result<(), String> {
    let mut host = fresh_host()?;
    begin_match(&mut host)?;
    let out = chat(&mut host, 0, "-matchup")?;
    out.need(0, "controls")?;
    out.need(0, "(Level 1)")?;
    let lines: Vec<String> = out.messages(0).into_iter().filter(|m| m.contains("controls")).collect();
    expect("opposing team only", lines.len() == 5, || format!("{} lines {lines:?}", lines.len()))?;
    let short = chat(&mut host, 7, "-ma")?;
    expect("short form", short.messages(7).iter().filter(|m| m.contains("controls")).count() == 5, || "-ma did not list five heroes".into())?;
    host.dispatch_action("peer_left", &json!({ "player": 6 }))?;
    pack_run::advance(&mut host, 2)?;
    drain(&mut host)?;
    let after = chat(&mut host, 0, "-matchup")?;
    expect("left tag", after.messages(0).iter().any(|m| m.contains("(Left at ")), || format!("{:?}", after.messages(0)))?;
    println!("matchup ok");
    Ok(())
}

pub fn creepstats(_image: &[u8]) -> Result<(), String> {
    let mut host = fresh_host()?;
    let heroes = begin_match(&mut host)?;
    host.sim_mut().set_counter(heroes[0], "st_creep", 7);
    host.sim_mut().set_counter(heroes[0], "st_deny", 3);
    host.sim_mut().set_counter(heroes[0], "st_neutral", 2);
    let own = chat(&mut host, 0, "-cs")?;
    own.need(0, "\tCreeps Killed: 7 Creeps Denied: 3 Neutrals Killed: 2")?;
    chat(&mut host, 0, "-creepstats")?.need(0, "Creeps Killed: 7")?;
    let team = chat(&mut host, 0, "-csa")?;
    let rows: Vec<String> = team.messages(0).into_iter().filter(|m| m.contains(": Creeps Killed:")).collect();
    expect("team rows", rows.len() == 5, || format!("{rows:?}"))?;
    expect("hero row", rows.iter().any(|m| m.contains("Creeps Killed: 7 Creeps Denied: 3")), || format!("{rows:?}"))?;
    let off = chat(&mut host, 0, "-csoff")?;
    expect("csoff", off.settings(0, "creepstats") == vec![0] && off.settings(0, "showdeny") == vec![0], || format!("{:?}", off.0))?;
    expect("csoff is silent", off.messages(0).is_empty(), || format!("{:?}", off.messages(0)))?;
    let on = chat(&mut host, 0, "-cson")?;
    expect("cson", on.settings(0, "creepstats") == vec![1] && on.settings(0, "showdeny") == vec![1], || format!("{:?}", on.0))?;
    println!("creepstats ok");
    Ok(())
}

pub fn deny_flags(_image: &[u8]) -> Result<(), String> {
    let mut host = fresh_host()?;
    let hide = chat(&mut host, 3, "-hidedeny")?;
    expect("hidedeny", hide.settings(3, "showdeny") == vec![0] && hide.messages(3).is_empty(), || format!("{:?}", hide.0))?;
    let show = chat(&mut host, 3, "-showdeny")?;
    expect("showdeny", show.settings(3, "showdeny") == vec![1], || format!("{:?}", show.0))?;
    let info = chat(&mut host, 3, "-di")?;
    info.need(3, "-di functionality is on by default. Use -cson and -csoff to toggle it on and off.")?;
    chat(&mut host, 3, "-denyinfo")?.need(3, "-cson and -csoff")?;
    println!("deny flags ok");
    Ok(())
}

pub fn weather(_image: &[u8]) -> Result<(), String> {
    let mut host = fresh_host()?;
    for (word, value) in [("snow", 1), ("rain", 2), ("moonlight", 3), ("wind", 4), ("off", 0)] {
        let out = chat(&mut host, 2, &format!("-weather {word}"))?;
        expect(word, out.settings(2, "weather") == vec![value], || format!("{:?}", out.settings(2, "weather")))?;
    }
    let unknown = chat(&mut host, 2, "-weather hail")?;
    expect("unknown weather", unknown.0.is_empty(), || format!("{:?}", unknown.0))?;
    let random = chat(&mut host, 2, "-weather random")?;
    expect("random", random.settings(2, "weather") == vec![5], || format!("{:?}", random.settings(2, "weather")))?;
    let first = random.settings(2, "weather_kind");
    expect("random kind", first.len() == 1 && (1..=4).contains(&first[0]), || format!("{first:?}"))?;
    pack_run::advance(&mut host, 300 * TPS + 2)?;
    let later = drain(&mut host)?.settings(2, "weather_kind");
    expect("random kind rolls again after 300 s", later.len() == 1 && (1..=4).contains(&later[0]), || format!("{later:?}"))?;
    println!("weather ok");
    Ok(())
}

pub fn music(_image: &[u8]) -> Result<(), String> {
    let mut host = fresh_host()?;
    for (word, value) in [("off", 0), ("nightelf1", 1), ("human2", 5), ("undead3", 12), ("other9", 21), ("special", 22)] {
        let out = chat(&mut host, 1, &format!("-music {word}"))?;
        expect(word, out.settings(1, "music") == vec![value], || format!("{:?}", out.settings(1, "music")))?;
    }
    let random = chat(&mut host, 1, "-music random")?.settings(1, "music");
    expect("random", random.len() == 1 && (1..=22).contains(&random[0]), || format!("{random:?}"))?;
    let help = chat(&mut host, 1, "-music nothing")?;
    help.need(1, "Command is used by typing -music theme. Where theme is one of the following:")?;
    help.need(1, "off, random, nightelf1, nightelf2, nightelf3, human1")?;
    expect("help leaves music alone", help.settings(1, "music").is_empty(), || "setting changed".into())?;
    println!("music ok");
    Ok(())
}

pub fn courier(_image: &[u8]) -> Result<(), String> {
    let mut host = fresh_host()?;
    let out = chat(&mut host, 0, "-courier")?;
    expect("courier answers nothing", out.0.is_empty(), || format!("{:?}", out.0))?;
    println!("courier ok");
    Ok(())
}

pub fn share(_image: &[u8]) -> Result<(), String> {
    let mut host = fresh_host()?;
    let granted = chat(&mut host, 0, "-share 2")?;
    granted.need(0, "Shared unit control with")?;
    expect("share setting", granted.settings(0, "share2") == vec![1], || format!("{:?}", granted.settings(0, "share2")))?;
    let own = chat(&mut host, 0, "-share 1")?;
    expect("sharing with yourself does nothing", own.0.is_empty(), || format!("{:?}", own.0))?;
    let out_of_range = chat(&mut host, 0, "-share 6")?;
    expect("share 6 is silent", out_of_range.0.is_empty(), || format!("{:?}", out_of_range.0))?;
    let removed = chat(&mut host, 1, "-rmshare 1")?;
    expect("rmshare", removed.settings(0, "share2") == vec![0] && removed.messages(1).is_empty(), || format!("{:?}", removed.0))?;
    chat(&mut host, 0, "-share 3")?;
    let taken = chat(&mut host, 0, "-unshare 3")?;
    taken.need(0, "Took unit control away from")?;
    expect("unshare setting", taken.settings(0, "share3") == vec![0], || format!("{:?}", taken.settings(0, "share3")))?;
    println!("share ok");
    Ok(())
}

pub fn shared_control(_image: &[u8]) -> Result<(), String> {
    let mut host = fresh_host()?;
    let heroes = begin_match(&mut host)?;
    let (x, y) = host.sim().position_fixed(heroes[0]);
    let (tx, ty) = (f64::from(x) / 65536.0 + 12.0, f64::from(y) / 65536.0);
    host.dispatch_action("select", &json!({ "player": 1, "ids": [heroes[0]] }))?;
    let before = right_click(&mut host, 1, tx, ty)?;
    expect("an ally cannot command an unshared hero", before.feedback(1, "move") == 0, || format!("{:?}", before.0))?;
    chat(&mut host, 0, "-share 2")?;
    host.dispatch_action("select", &json!({ "player": 1, "ids": [heroes[0]] }))?;
    let granted = right_click(&mut host, 1, tx, ty)?;
    expect("an ally commands a shared hero", granted.feedback(1, "move") == 1, || format!("{:?}", granted.0))?;
    chat(&mut host, 0, "-unshare 2")?;
    host.dispatch_action("select", &json!({ "player": 1, "ids": [heroes[0]] }))?;
    let revoked = right_click(&mut host, 1, tx, ty)?;
    expect("an unshared hero is no longer commanded", revoked.feedback(1, "move") == 0, || format!("{:?}", revoked.0))?;
    chat(&mut host, 0, "-share 2")?;
    chat(&mut host, 1, "-rmshare 1")?;
    host.dispatch_action("select", &json!({ "player": 1, "ids": [heroes[0]] }))?;
    let removed = right_click(&mut host, 1, tx, ty)?;
    expect("-rmshare drops the grant from the grantee side", removed.feedback(1, "move") == 0, || format!("{:?}", removed.0))?;
    println!("shared control ok");
    Ok(())
}

pub fn movespeed(_image: &[u8]) -> Result<(), String> {
    let mut host = fresh_host()?;
    chat(&mut host, 0, "-ms")?.need(0, "No Hero is selected")?;
    let heroes = begin_match(&mut host)?;
    let first = chat(&mut host, 0, "-ms")?;
    let hero = role("hero.a")?;
    let line = first.messages(0).into_iter().find(|m| m.contains("movespeed is")).ok_or("no movespeed line")?;
    expect("hero named", line.contains(hero) || line.contains("Axe"), || line.clone())?;
    let speed: i64 = line.rsplit(' ').next().and_then(|n| n.parse().ok()).ok_or_else(|| format!("no number in {line}"))?;
    expect("speed positive", speed > 0, || line.clone())?;
    let second_kind = host.sim().catalog().unit_index(role("hero.b")?).ok_or("no second hero kind")?;
    let (x, y) = host.sim().position_fixed(heroes[0]);
    host.sim_mut().spawn_attributed(0, second_kind as u32, x, y).ok_or("second hero refused")?;
    let both = chat(&mut host, 0, "-movespeed")?;
    let count = both.messages(0).iter().filter(|m| m.contains("movespeed is")).count();
    expect("every hero of the player", count == 2, || format!("{count} lines {:?}", both.messages(0)))?;
    println!("movespeed ok speed={speed}");
    Ok(())
}

pub fn drop_vote(_image: &[u8]) -> Result<(), String> {
    let mut host = fresh_host()?;
    begin_match(&mut host)?;
    chat(&mut host, 1, "-drop 3")?.need(1, "Only the host (Player 1) can kick players!")?;
    let bad = chat(&mut host, 0, "-drop 1")?;
    bad.need(0, "You have to choose a valid player: ")?;
    bad.need(0, "-drop 2")?;
    bad.need(0, "-drop 10")?;
    expect("host not listed", !bad.said(0, "-drop 1|r"), || "the host is offered as a target".into())?;
    let ask = chat(&mut host, 0, "-drop 6")?;
    ask.need(2, "has requested to kick player: ")?;
    ask.need(2, "Type |c006699CC-yes|r to accept.")?;
    chat(&mut host, 0, "-drop 7")?.need(0, "There is already a kickrequest running.")?;
    for (n, voter) in [1usize, 2, 3].into_iter().enumerate() {
        let out = chat(&mut host, voter, "-yes")?;
        out.need(0, &format!("accepted the request. ({}/4)", n + 1))?;
    }
    let repeat = chat(&mut host, 1, "-yes")?;
    expect("a second yes counts nothing", repeat.messages(0).is_empty(), || format!("{:?}", repeat.messages(0)))?;
    let last = chat(&mut host, 4, "-yes")?;
    last.need(0, "(4/4)")?;
    last.need(0, "has been removed from the game.")?;
    last.need(0, "has left the game")?;
    chat(&mut host, 5, "-yes")?;
    let next = chat(&mut host, 0, "-drop 7")?;
    next.need(0, "has requested to kick player: ")?;
    pack_run::advance(&mut host, 60 * TPS + 2)?;
    let expired = drain(&mut host)?;
    expired.need(0, "Kick time has expired without enough votes.")?;
    println!("drop vote ok");
    Ok(())
}

pub fn kickafk(_image: &[u8]) -> Result<(), String> {
    let mut host = fresh_host()?;
    begin_match(&mut host)?;
    chat(&mut host, 0, "-kickafk")?;
    let quiet = chat(&mut host, 0, "-kickafk 2")?;
    expect("nobody is afk yet", !quiet.said(0, "booted"), || format!("{:?}", quiet.messages(0)))?;
    pack_run::advance(&mut host, 300 * TPS + 40)?;
    drain(&mut host)?;
    let list = chat(&mut host, 0, "-kickafk")?;
    list.need(0, "has been afk for more than 5.")?;
    list.need(0, "minutes")?;
    list.need(0, "-kickafk 2 ")?;
    list.need(0, "to boot this player from the game")?;
    let again = chat(&mut host, 0, "-kickafk")?;
    expect("a listed player is not listed twice", !again.said(0, "-kickafk 2 "), || format!("{:?}", again.messages(0)))?;
    let enemy = chat(&mut host, 0, "-kickafk 7")?;
    expect("an enemy is never booted", !enemy.said(0, "booted"), || format!("{:?}", enemy.messages(0)))?;
    let booted = chat(&mut host, 0, "-kickafk 2")?;
    booted.need(1, "has been booted for being afk too long!")?;
    booted.need(0, "has left the game")?;
    println!("kickafk ok");
    Ok(())
}

pub fn lobby_random(_image: &[u8]) -> Result<(), String> {
    let mut host = fresh_host()?;
    claim(&mut host, 2)?;
    let plain = chat(&mut host, 0, "-random")?;
    plain.need(0, "You will be given a random hero soon.")?;
    let picked = plain.lobby_slots().ok_or("no lobby event after -random")?;
    expect("slot 0 holds a hero", !picked[0].1.is_empty(), || format!("{picked:?}"))?;
    let again = chat(&mut host, 0, "-random")?;
    again.need(0, "You cannot be assigned a random hero.")?;
    let attribute = chat(&mut host, 1, "-random agi")?;
    attribute.need(1, "You will be given a random hero soon.")?;
    let slots = attribute.lobby_slots().ok_or("no lobby event after -random agi")?;
    expect("slot 1 holds another hero", !slots[1].1.is_empty() && slots[1].1 != slots[0].1, || format!("{slots:?}"))?;
    let open_seat = chat(&mut host, 9, "-random")?;
    open_seat.need(9, "You cannot be assigned a random hero.")?;
    println!("lobby random ok");
    Ok(())
}

pub fn lobby_random_cost(_image: &[u8]) -> Result<(), String> {
    let mut host = fresh_host()?;
    claim(&mut host, 3)?;
    chat(&mut host, 0, "-random")?;
    chat(&mut host, 1, "-random int")?;
    ui(&mut host, 0, "fill", json!({}))?;
    chat(&mut host, 2, "-random agi")?;
    for slot in 0..3 {
        ui(&mut host, slot, "ready", json!({ "slot": slot, "ready": true }))?;
    }
    host.dispatch_action("ui", &json!({ "call": "world_ready", "args": {}, "player": 0 }))?;
    ui(&mut host, 0, "start", json!({}))?;
    pick_heroes(&mut host)?;
    pack_run::advance(&mut host, 4)?;
    let gold: Vec<i32> = (0..4u8).map(|p| host.sim().resource_a(p)).collect();
    expect("plain random costs 50 and an attribute random 100 against a seat that never randomed", gold[3] - gold[0] == 50 && gold[3] - gold[1] == 100 && gold[3] - gold[2] == 100, || format!("gold {gold:?}"))?;
    println!("lobby random cost ok gold={gold:?}");
    Ok(())
}

pub fn lobby_repick(_image: &[u8]) -> Result<(), String> {
    let mut host = fresh_host()?;
    claim(&mut host, 1)?;
    chat(&mut host, 0, "-repick")?;
    chat(&mut host, 0, "-random")?;
    ui(&mut host, 0, "ready", json!({ "slot": 0, "ready": true }))?;
    let out = chat(&mut host, 0, "-repick")?;
    out.need(0, "You can now pick a new hero")?;
    let slots = out.lobby_slots().ok_or("no lobby event after -repick")?;
    expect("hero cleared", slots[0].1.is_empty(), || format!("{slots:?}"))?;
    let after = chat(&mut host, 0, "-random")?;
    after.need(0, "You cannot be assigned a random hero.")?;
    println!("lobby repick ok");
    Ok(())
}

pub fn match_repick(_image: &[u8]) -> Result<(), String> {
    let mut host = fresh_host()?;
    begin_match(&mut host)?;
    chat(&mut host, 0, "-repick")?.need(0, "Too late to repick")?;
    chat(&mut host, 0, "-random")?.need(0, "You cannot be assigned a random hero.")?;
    println!("match repick ok");
    Ok(())
}

pub fn rollhero(_image: &[u8]) -> Result<(), String> {
    let mut host = fresh_host()?;
    let first = chat(&mut host, 4, "-rollhero")?;
    first.need(4, "You randomly rolled ")?;
    chat(&mut host, 4, "-rh")?.need(4, "Do not spam.")?;
    pack_run::advance(&mut host, TPS + 1)?;
    drain(&mut host)?;
    chat(&mut host, 4, "-rh")?.need(4, "You randomly rolled ")?;
    println!("rollhero ok");
    Ok(())
}

pub fn lobby_swap(_image: &[u8]) -> Result<(), String> {
    let mut host = fresh_host()?;
    claim(&mut host, 2)?;
    chat(&mut host, 0, "-swap")?.need(0, "You must have a living hero in order to swap with someone.")?;
    let a = chat(&mut host, 0, "-random")?.lobby_slots().ok_or("no lobby event")?[0].1.clone();
    chat(&mut host, 0, "-swap")?.need(0, "No hero available to swap with.")?;
    let b = chat(&mut host, 1, "-random")?.lobby_slots().ok_or("no lobby event")?[1].1.clone();
    let list = chat(&mut host, 0, "-swap")?;
    list.need(0, "Swap Hero Options:")?;
    list.need(0, "to make a choice or -swapcancel to cancel swap requests")?;
    let request = chat(&mut host, 0, "-swap 2")?;
    request.need(0, "You have requested to swap with")?;
    request.need(1, "wants to swap with you.")?;
    let listed = chat(&mut host, 1, "-swap")?;
    listed.need(1, "(Requested Swap With You)")?;
    let invalid = chat(&mut host, 0, "-swap 4")?;
    invalid.need(0, "Invalid swap selection")?;
    let done = chat(&mut host, 1, "-swap 1")?;
    let slots = done.lobby_slots().ok_or("no lobby event after the swap")?;
    expect("heroes exchanged", slots[0].1 == b && slots[1].1 == a, || format!("was {a} {b}, now {slots:?}"))?;
    chat(&mut host, 0, "-swap 2")?;
    chat(&mut host, 0, "-swapcancel")?;
    let after_cancel = chat(&mut host, 1, "-swap 1")?;
    expect("a cancelled request does not swap", after_cancel.lobby_slots().is_none_or(|s| s[0].1 == b), || format!("{:?}", after_cancel.lobby_slots()))?;
    let all = chat(&mut host, 0, "-swapall")?;
    expect("swapall asks the one ally with a hero", all.said(1, "wants to swap with you.") || all.lobby_slots().is_some(), || format!("{:?}", all.0))?;
    println!("lobby swap ok");
    Ok(())
}

pub fn match_swap(_image: &[u8]) -> Result<(), String> {
    let mut host = fresh_host()?;
    begin_match(&mut host)?;
    chat(&mut host, 0, "-swap")?.need(0, "Too late to use -swaphero.")?;
    chat(&mut host, 0, "-swap 2")?.need(0, "Too late to use -swaphero.")?;
    println!("match swap ok");
    Ok(())
}

pub fn switch_flow(_image: &[u8]) -> Result<(), String> {
    let mut host = fresh_host()?;
    claim(&mut host, 1)?;
    chat(&mut host, 0, "-switch")?.need(0, "-switch has been disabled for this game.")?;
    chat(&mut host, 0, "-ok")?.need(0, "-switch has been disabled for this game.")?;
    ui(&mut host, 0, "mode", json!({ "code": "so" }))?;
    let list = chat(&mut host, 0, "-switch")?;
    list.need(0, "-switch 1")?;
    list.need(0, "to switch with")?;
    let request = chat(&mut host, 0, "-switch 1")?;
    request.need(3, "-switch accept")?;
    request.need(3, "-no")?;
    request.need(0, "You have requested to switch places with")?;
    chat(&mut host, 1, "-no")?.need(0, "has rejected the switch offer.")?;
    chat(&mut host, 0, "-switch 1")?;
    let first = chat(&mut host, 1, "-ok")?;
    first.need(0, "has accepted the switch offer")?;
    first.need(0, "(2/10)")?;
    let mut last = first;
    for voter in [2usize, 3, 4, 5, 6, 7, 8] {
        last = chat(&mut host, voter, "-switch accept")?;
    }
    last.need(0, "has been completed successfully")?;
    let slots = last.lobby_slots().ok_or("no lobby event after the switch")?;
    expect("seat 0 moved to the rival side", slots[5].0 == "k0" && slots[0].0.is_empty(), || format!("{slots:?}"))?;
    chat(&mut host, 0, "-switch 2")?;
    pack_run::advance(&mut host, 60 * TPS + 2)?;
    drain(&mut host)?.need(0, "Switch time has expired without enough votes")?;
    println!("switch flow ok");
    Ok(())
}
