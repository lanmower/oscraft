use mod_host::ModHost;
use serde_json::{json, Value};

use crate::checks::pack_run;

const BOARD: &str = "scoreboard_688g";
const OBSERVER: &str = "observer_info_688g";

fn panels(host: &mut ModHost, id: &str) -> Vec<Value> {
    host.take_ui()
        .iter()
        .filter_map(|b| serde_json::from_str::<Value>(&format!("{{{b}}}")).ok())
        .filter(|v| v["kind"] == "panel" && v["id"] == id)
        .collect()
}

fn hero_of(host: &ModHost, p: u8) -> Result<usize, String> {
    let sim = host.sim();
    (0..sim.count()).find(|&i| sim.alive(i) && sim.player(i) == p && sim.is_attributed(i)).ok_or_else(|| format!("seat {p} has no hero"))
}

fn row<'a>(panel: &'a Value, label: &str) -> Result<Vec<String>, String> {
    panel["rows"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| r.as_array())
        .find(|r| r.first().and_then(|c| c.as_str()).is_some_and(|c| c.contains(label)))
        .map(|r| r.iter().map(|c| c.as_str().unwrap_or_default().to_owned()).collect())
        .ok_or_else(|| format!("no row {label}: {panel}"))
}

fn open_board(host: &mut ModHost) -> Result<(), String> {
    host.dispatch_action("key", &json!({"key":"tab","phase":"down","shift":false,"ctrl":false,"alt":false,"meta":false,"player":0,"world":{"x":0.0,"z":0.0}}))
}

fn want(what: &str, got: &str, expected: &str) -> Result<(), String> {
    if got == expected { Ok(()) } else { Err(format!("{what} = {got:?}, want {expected:?}")) }
}

fn match_with_kills() -> Result<(ModHost, usize, usize, usize), String> {
    let mut host = pack_run::start()?;
    let home = host.sim().team_of(0);
    let foes: Vec<u8> = (1..10u8).filter(|&p| host.sim().team_of(p) != home).take(2).collect();
    if foes.len() != 2 {
        return Err(format!("seats 1..10 hold {} opponents of seat 0, want 2", foes.len()));
    }
    for p in [0, foes[0], foes[1]] {
        host.dispatch_action("seat_pick", &json!({"player": p, "hero": "antimage"}))?;
    }
    pack_run::advance(&mut host, 2)?;
    let (killer, a, b) = (hero_of(&host, 0)?, hero_of(&host, foes[0])?, hero_of(&host, foes[1])?);
    for victim in [a, b] {
        host.sim_mut().hit_by(victim, i32::MAX / 2, killer as i32);
        pack_run::advance(&mut host, 2)?;
    }
    Ok((host, killer, a, b))
}

pub fn scoreboard_rows(_image: &[u8]) -> Result<(), String> {
    let (mut host, killer, a, _) = match_with_kills()?;
    open_board(&mut host)?;
    pack_run::advance(&mut host, 8)?;
    let board = panels(&mut host, BOARD).pop().ok_or("no scoreboard panel after the key press")?;
    let title = board["title"].as_str().unwrap_or_default();
    want("title", title, "DotA Scores  -  - |c00ff03032|r/|c0020c0000|r")?;
    want("player", &board["player"].to_string(), "0")?;
    let columns = board["columns"].as_array().ok_or("no columns")?;
    if columns.len() != 4 {
        return Err(format!("columns {columns:?}, want a label and three heroes"));
    }
    let gold = row(&board, "Bonus Gold")?;
    let xp = row(&board, "Bonus XP")?;
    if gold[1] == "|c00FF66000|r" || !gold[1].starts_with("|c00FF6600") {
        return Err(format!("the viewer's own Bonus Gold cell must be orange and non-zero: {gold:?}"));
    }
    if !xp[1].starts_with("|c00FF6600") || xp[2].starts_with("|c00FF6600") {
        return Err(format!("only the viewer's Bonus XP cell is orange: {xp:?}"));
    }
    let dead = row(&board, "Time Dead")?;
    if dead[1] != "0:00" || dead[2] == "0:00" || dead[2] != dead[3] || !dead[2].contains(':') {
        return Err(format!("Time Dead {dead:?}: the killer lived, both victims waited the same respawn"));
    }
    let multi = row(&board, "MultiKills")?;
    want("MultiKills killer", &multi[1], "1/0/0/0")?;
    want("MultiKills victim", &multi[2], "0/0/0/0")?;
    let consumables = row(&board, "Consumables")?;
    want("Consumables viewer", &consumables[1], "|c00FF66000|r")?;
    want("Consumables other", &consumables[2], "0")?;
    let details = board["rows"].as_array().ok_or("no rows")?;
    let start = details.iter().position(|r| r[0].as_str().is_some_and(|c| c.contains("Kill Details"))).ok_or("no Kill Details row")?;
    let first = details[start + 1].as_array().ok_or("no kill detail row")?;
    let cell = |r: usize, c: usize| details[start + 1 + r][c].as_str().unwrap_or_default().to_owned();
    want("killer vs first victim", &cell(0, 1), "antimage 1/0")?;
    want("killer vs second victim", &cell(1, 1), "antimage 1/0")?;
    want("first victim vs killer", &cell(0, 2), "antimage 0/1")?;
    if first.len() != 4 {
        return Err(format!("kill detail row {first:?}"));
    }
    let _ = (killer, a);
    Ok(())
}

pub fn scoreboard_closes(_image: &[u8]) -> Result<(), String> {
    let (mut host, ..) = match_with_kills()?;
    pack_run::advance(&mut host, 8)?;
    if !panels(&mut host, BOARD).is_empty() {
        return Err("the scoreboard panel is sent while the board is closed".into());
    }
    open_board(&mut host)?;
    pack_run::advance(&mut host, 8)?;
    if panels(&mut host, BOARD).len() != 1 {
        return Err("an opened board is sent once".into());
    }
    pack_run::advance(&mut host, 8)?;
    if !panels(&mut host, BOARD).is_empty() {
        return Err("an unchanged board is sent again".into());
    }
    open_board(&mut host)?;
    pack_run::advance(&mut host, 8)?;
    let removal = host.take_ui();
    if !removal.iter().any(|b| b.contains(&format!(r#""id":"{BOARD}","remove":true"#))) {
        return Err(format!("closing the board does not remove it: {removal:?}"));
    }
    Ok(())
}

pub fn observer_panel(_image: &[u8]) -> Result<(), String> {
    let (mut host, ..) = match_with_kills()?;
    pack_run::advance(&mut host, 20)?;
    let seen = panels(&mut host, OBSERVER);
    let mut seats: Vec<i64> = seen.iter().filter_map(|p| p["player"].as_i64()).collect();
    seats.sort_unstable();
    seats.dedup();
    if seats != [10, 11, 12, 13, 14, 15, 255] {
        return Err(format!("observer panel seats {seats:?}, want 10..=15, the spectator seat 255 and no hero seat"));
    }
    let panel = seen.last().ok_or("no observer panel")?;
    want("title", panel["title"].as_str().unwrap_or_default(), "Observer Info")?;
    let sentinel = row(panel, "The Sentinel")?;
    want("sentinel line", &sentinel[0], "|c00ED2323The Sentinel (2/0 | 0-0)|r")?;
    let scourge = row(panel, "The Scourge")?;
    want("scourge line", &scourge[0], "|c00067A06The Scourge (0/2 | 0-0)|r")?;
    Ok(())
}
