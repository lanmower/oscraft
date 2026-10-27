use std::collections::HashMap;

use mod_host::ModHost;
use serde_json::{json, Value};

use crate::checks::{pack_dir, pack_run};

const TPS: u32 = sim::TICKS_PER_SECOND as u32;
const SENTINEL_LAST: usize = 59;
const CM_SIDES: &str = "ABABABBAABABBABAABBA";
const CM_KINDS: &str = "bbbbppppbbbbppppbbpp";

struct Pack {
    pool: Vec<String>,
    groups: Vec<u8>,
    display: HashMap<String, String>,
}

fn pack() -> Result<Pack, String> {
    let data = std::fs::read_to_string(pack_dir().join("draft_data.lua")).map_err(|e| format!("draft_data.lua: {e}"))?;
    let start = data.find("data.heroes = {").ok_or("draft_data.lua has no data.heroes")?;
    let body = &data[start..start + data[start..].find('}').ok_or("data.heroes is not closed")?];
    let pool: Vec<String> = body.split('"').skip(1).step_by(2).map(str::to_owned).collect();
    let g = data.find("data.groups = \"").ok_or("draft_data.lua has no data.groups")? + "data.groups = \"".len();
    let groups: Vec<u8> = data[g..g + data[g..].find('"').ok_or("data.groups is not closed")?].bytes().map(|b| b - b'0').collect();
    if pool.len() != 112 || groups.len() != 112 {
        return Err(format!("draft_data.lua holds {} heroes and {} groups, want 112 of each", pool.len(), groups.len()));
    }
    let panels = std::fs::read_to_string(pack_dir().join("panels.lua")).map_err(|e| format!("panels.lua: {e}"))?;
    let mut display = HashMap::new();
    for line in panels.lines() {
        let line = line.trim();
        if let Some((id, rest)) = line.split_once(" = { \"") {
            if id.chars().all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit()) {
                if let Some((name, _)) = rest.split_once('"') {
                    display.insert(id.to_owned(), name.to_owned());
                }
            }
        }
    }
    Ok(Pack { pool, groups, display })
}

impl Pack {
    fn shown(&self, id: &str) -> String {
        self.display.get(id).cloned().unwrap_or_else(|| id.to_owned())
    }

    fn index(&self, id: &str) -> Option<usize> {
        self.pool.iter().position(|h| h == id).map(|i| i + 1)
    }

    fn id_of(&self, cell: &str) -> Option<String> {
        self.pool.iter().find(|h| cell.contains(&self.shown(h))).cloned()
    }
}

fn expect(label: &str, ok: bool, detail: impl FnOnce() -> String) -> Result<(), String> {
    if ok {
        return Ok(());
    }
    Err(format!("{label}: {}", detail()))
}

struct Draft {
    host: ModHost,
    events: Vec<Value>,
    pack: Pack,
}

impl Draft {
    fn drain(&mut self) -> Result<(), String> {
        if !self.host.disabled_hooks().is_empty() {
            return Err(format!("a pack hook was switched off: {:?} {:?}", self.host.disabled_hooks(), self.host.hook_errors()));
        }
        self.host.flush_ui();
        for body in self.host.take_ui() {
            self.events.push(serde_json::from_str::<Value>(&format!("{{{body}}}")).map_err(|e| format!("ui event {body}: {e}"))?);
        }
        Ok(())
    }

    fn act(&mut self, player: usize, call: &str, args: Value) -> Result<(), String> {
        self.host.dispatch_action("ui", &json!({ "call": call, "args": args, "player": player, "pubkey": format!("k{player}") }))?;
        self.drain()
    }

    fn run(&mut self, ticks: u32) -> Result<(), String> {
        pack_run::advance(&mut self.host, ticks)?;
        self.drain()
    }

    fn seconds(&mut self, secs: u32) -> Result<(), String> {
        self.run(secs * TPS)
    }

    fn new(codes: &[&str]) -> Result<Draft, String> {
        let mut host = pack_run::start()?;
        host.flush_ui();
        host.take_ui();
        let mut d = Draft { host, events: Vec::new(), pack: pack()? };
        for slot in 0..10 {
            d.act(slot, "claim", json!({ "slot": slot }))?;
        }
        for code in codes {
            d.act(0, "mode", json!({ "code": code }))?;
        }
        for slot in 0..10 {
            d.act(slot, "ready", json!({ "slot": slot, "ready": true }))?;
        }
        d.host.dispatch_action("ui", &json!({ "call": "world_ready", "args": {}, "player": 0 }))?;
        d.drain()?;
        Ok(d)
    }

    fn start(&mut self) -> Result<(), String> {
        self.act(0, "start", json!({}))
    }

    fn with_bots(codes: &[&str], seat: usize) -> Result<Draft, String> {
        let mut host = pack_run::start()?;
        host.flush_ui();
        host.take_ui();
        let mut d = Draft { host, events: Vec::new(), pack: pack()? };
        d.act(seat, "claim", json!({ "slot": seat }))?;
        for code in codes {
            d.act(seat, "mode", json!({ "code": code }))?;
        }
        d.act(seat, "fill", json!({}))?;
        d.act(seat, "ready", json!({ "slot": seat, "ready": true }))?;
        d.host.dispatch_action("ui", &json!({ "call": "world_ready", "args": {}, "player": 0 }))?;
        d.drain()?;
        d.act(seat, "start", json!({}))?;
        Ok(d)
    }

    fn panel(&self, id: &str, player: Option<usize>) -> Option<&Value> {
        self.events.iter().rev().find(|e| {
            e["kind"] == "panel" && e["id"] == id && e.get("remove").is_none() && player.is_none_or(|p| e["player"] == json!(p))
        })
    }

    fn rows(&self, id: &str, player: Option<usize>) -> Result<Vec<Vec<String>>, String> {
        let panel = self.panel(id, player).ok_or_else(|| format!("no panel {id} for {player:?}"))?;
        Ok(panel["rows"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|r| r.as_array().into_iter().flatten().map(|c| c.as_str().unwrap_or_default().to_owned()).collect())
            .collect())
    }

    fn text(&self, id: &str, player: Option<usize>) -> String {
        self.panel(id, player).and_then(|p| p["text"].as_str()).unwrap_or_default().to_owned()
    }

    fn said(&self, needle: &str) -> bool {
        self.events.iter().any(|e| e["kind"] == "message" && e["text"].as_str().is_some_and(|t| t.contains(needle)))
    }

    fn started(&self) -> bool {
        self.events.iter().any(|e| e["kind"] == "lobby" && e["data"]["started"] == true)
    }

    fn slots(&self) -> Vec<String> {
        self.events
            .iter()
            .rev()
            .find(|e| e["kind"] == "lobby" && e["data"]["started"] == false)
            .and_then(|e| e["data"]["slots"].as_array())
            .map(|s| s.iter().map(|x| x["hero"].as_str().unwrap_or_default().to_owned()).collect())
            .unwrap_or_default()
    }

    fn pool_cells(&self) -> Result<Vec<String>, String> {
        Ok(self.rows("draft_pool", None)?.into_iter().flatten().filter(|c| !c.is_empty()).collect())
    }

    fn card_ids(&self) -> Result<Vec<String>, String> {
        self.pool_cells()?.iter().map(|c| self.pack.id_of(c).ok_or_else(|| format!("card {c:?} is not a pool hero"))).collect()
    }

    fn press(&mut self, player: usize, hero: &str) -> Result<(), String> {
        self.act(player, "pick", json!({ "slot": player, "hero": hero }))
    }

    fn filled(&self, id: &str, col: usize) -> Result<usize, String> {
        Ok(self.rows(id, None)?.iter().filter(|r| r.get(col).is_some_and(|c| !c.is_empty() && c != "-")).count())
    }
}

fn captain(side: usize) -> usize {
    side * 5
}

fn side_of_label(label: &str) -> usize {
    usize::from(label != "Sentinel")
}

fn mode_panel_title(d: &Draft, title: &str) -> Result<(), String> {
    let panel = d.panel("draft", None).ok_or("no draft panel after the start press")?;
    expect("draft panel title", panel["title"] == title, || format!("{}", panel["title"]))
}

pub fn modes_accepted(_image: &[u8]) -> Result<(), String> {
    let mut d = Draft::new(&["cm", "rd"])?;
    d.start()?;
    mode_panel_title(&d, "Captains Mode")?;
    expect("draft begins without starting the match", !d.started(), || "the lobby reported a start".to_owned())?;
    let again = d.panel("draft", None).map(|p| p["rows"].as_array().map_or(0, Vec::len)).unwrap_or(0);
    expect("captains mode lists twenty turns", again == 20, || format!("{again} rows"))?;
    d.act(0, "start", json!({}))?;
    expect("a second start press cannot start the match while drafting", !d.started(), || "started".to_owned())?;
    let mut x = Draft::new(&["cm", "x"])?;
    x.start()?;
    mode_panel_title(&x, "Captains Mode")?;
    let mut ext = Draft::new(&["cd", "x"])?;
    ext.start()?;
    expect("extended captains draft deals 42 heroes", ext.pool_cells()?.len() == 42, || format!("{}", ext.pool_cells().map(|c| c.len()).unwrap_or(0)))?;
    expect("extended captains draft has twelve turns", ext.rows("draft", None)?.len() == 12, || "turn count".to_owned())?;
    println!("draft modes ok");
    Ok(())
}

fn claim_all(d: &mut Draft, picks: &[Vec<String>; 2]) -> Result<(), String> {
    d.seconds(2)?;
    let wrong = picks[1][0].clone();
    d.press(0, &wrong)?;
    expect("a seat cannot claim the other team's hero", d.slots()[0].is_empty(), || format!("{:?}", d.slots()))?;
    for seat in 0..10usize {
        let hero = picks[seat / 5][seat % 5].clone();
        d.press(seat, &hero)?;
    }
    let slots = d.slots();
    for seat in 0..10usize {
        expect("a seat holds the hero it claimed", slots[seat] == picks[seat / 5][seat % 5], || format!("seat {seat}: {slots:?}"))?;
    }
    Ok(())
}

pub fn captains_mode(_image: &[u8]) -> Result<(), String> {
    let mut d = Draft::new(&["cm"])?;
    d.start()?;
    mode_panel_title(&d, "Captains Mode")?;
    let rows = d.rows("draft", None)?;
    let first = rows[0][1].clone();
    let a = side_of_label(&first);
    for (k, row) in rows.iter().enumerate() {
        let want_side = if CM_SIDES.as_bytes()[k] == b'A' { a } else { 1 - a };
        expect("turn team", side_of_label(&row[1]) == want_side, || format!("turn {} is {} for {:?}", k + 1, row[1], rows))?;
        let want_kind = if CM_KINDS.as_bytes()[k] == b'b' { "ban" } else { "pick" };
        expect("turn kind", row[2] == want_kind, || format!("turn {} is {}", k + 1, row[2]))?;
    }
    let pool = d.pack.pool.clone();
    expect("monkey_king is not a 6.88g pool hero", d.pack.index("monkey_king").is_none(), || "monkey_king is in draft_data".to_owned())?;
    d.press(captain(a), &pool[0])?;
    expect("a captain cannot ban before the first turn", d.filled("draft", 3)? == 0, || "banned early".to_owned())?;
    d.seconds(14)?;
    expect("no turn before 15 s", d.filled("draft", 3)? == 0 && !d.text("draft", None).contains("captain"), || d.text("draft", None))?;
    d.seconds(2)?;
    expect("the first turn announces its captain", d.text("draft", None).contains("captain") && d.text("draft", None).contains("ban"), || d.text("draft", None))?;
    expect("the draft announces the first captain", d.said("has been randomly selected to begin the draft"), || "no first captain message".to_owned())?;
    let mut used: Vec<String> = Vec::new();
    let mut picks: [Vec<String>; 2] = [Vec::new(), Vec::new()];
    for k in 0..20usize {
        let side = if CM_SIDES.as_bytes()[k] == b'A' { a } else { 1 - a };
        if k == 0 {
            d.press(captain(1 - side), &pool[1])?;
            expect("the other captain cannot act out of turn", d.filled("draft", 3)? == 0, || "acted".to_owned())?;
            d.press(captain(side) + 1, &pool[1])?;
            expect("a non-captain cannot act", d.filled("draft", 3)? == 0, || "acted".to_owned())?;
        }
        if k == 1 {
            d.press(captain(side), &used[0])?;
            expect("a taken hero cannot be chosen again", d.filled("draft", 3)? == 1, || "retook a banned hero".to_owned())?;
            d.press(captain(side), "monkey_king")?;
            expect("a hero outside the 6.88g pool cannot be chosen", d.filled("draft", 3)? == 1, || "took monkey_king".to_owned())?;
        }
        let hero = pool.iter().find(|h| !used.contains(h)).ok_or("pool exhausted")?.clone();
        d.press(captain(side), &hero)?;
        expect("the chosen hero is on the board", d.filled("draft", 3)? == k + 1, || format!("turn {} left {} rows filled", k + 1, d.filled("draft", 3).unwrap_or(0)))?;
        used.push(hero.clone());
        if CM_KINDS.as_bytes()[k] == b'p' {
            picks[side].push(hero);
        }
        d.seconds(1)?;
    }
    expect("each team holds five picks", picks[0].len() == 5 && picks[1].len() == 5, || format!("{picks:?}"))?;
    expect("no hero is claimed before the draft ends", d.slots().iter().all(String::is_empty), || format!("{:?}", d.slots()))?;
    claim_all(&mut d, &picks)?;
    d.seconds(1)?;
    d.start()?;
    expect("the match starts once the draft is done", d.started(), || "no start event".to_owned())?;
    println!("captains mode ok first={first}");
    Ok(())
}

pub fn captains_mode_timeout(_image: &[u8]) -> Result<(), String> {
    let mut d = Draft::new(&["cm"])?;
    d.start()?;
    let a = side_of_label(&d.rows("draft", None)?[0][1]);
    let a_label = d.rows("draft", None)?[0][1].clone();
    let pool = d.pack.pool.clone();
    d.run(15 * TPS)?;
    d.run((30 + 110) * TPS)?;
    expect("the first ban is still open at 30 s plus 110 s bonus", d.filled("draft", 3)? == 0, || d.text("draft", None))?;
    d.run(2 * TPS)?;
    let rows = d.rows("draft", None)?;
    expect("an unused first ban becomes no ban", rows[0][3].contains("(none)") && rows[0][3].contains("(auto)"), || format!("{:?}", rows[0]))?;
    expect("the timeout used up the team's bonus time", d.text("draft", None).contains(&format!("{a_label} 0 s")), || d.text("draft", None))?;
    let b = 1 - a;
    d.press(captain(b), &pool[0])?;
    d.press(captain(a), &pool[1])?;
    d.press(captain(b), &pool[2])?;
    expect("three bans are down", d.filled("draft", 3)? == 4, || format!("{:?}", d.rows("draft", None)))?;
    d.run(41 * TPS)?;
    d.run(TPS)?;
    let rows = d.rows("draft", None)?;
    expect("an unused pick becomes a random free hero", rows[4][3].contains("(auto)") && !rows[4][3].contains("(none)"), || format!("{:?}", rows[4]))?;
    println!("captains mode timeout ok");
    Ok(())
}

pub fn claim_leftovers(_image: &[u8]) -> Result<(), String> {
    let mut d = Draft::new(&["cm"])?;
    d.start()?;
    let a = side_of_label(&d.rows("draft", None)?[0][1]);
    d.seconds(16)?;
    let pool = d.pack.pool.clone();
    let mut used: Vec<String> = Vec::new();
    let mut picks: [Vec<String>; 2] = [Vec::new(), Vec::new()];
    for k in 0..20usize {
        let side = if CM_SIDES.as_bytes()[k] == b'A' { a } else { 1 - a };
        let hero = pool.iter().find(|h| !used.contains(h)).ok_or("pool exhausted")?.clone();
        d.press(captain(side), &hero)?;
        used.push(hero.clone());
        if CM_KINDS.as_bytes()[k] == b'p' {
            picks[side].push(hero);
        }
        d.seconds(1)?;
    }
    d.seconds(2)?;
    d.press(0, &picks[0][2].clone())?;
    d.seconds(95)?;
    d.run(2)?;
    let slots = d.slots();
    let want_first = [&picks[0][2], &picks[0][0], &picks[0][1], &picks[0][3], &picks[0][4]];
    for (seat, hero) in want_first.iter().enumerate() {
        expect("a seat that claimed nothing takes the first free team pick in seat order", slots[seat] == **hero, || format!("seat {seat}: {slots:?}"))?;
    }
    for seat in 5..10usize {
        expect("an unclaimed team takes its picks in seat order", slots[seat] == picks[1][seat - 5], || format!("seat {seat}: {slots:?}"))?;
    }
    println!("claim leftovers ok");
    Ok(())
}

fn press_free(_d: &Draft, cards: &[String], taken: &[String]) -> Result<String, String> {
    cards.iter().find(|c| !taken.contains(c)).cloned().ok_or_else(|| "no free card".to_owned())
}

fn press_card(d: &mut Draft, player: usize, cards: &[String], taken: &[String]) -> Result<String, String> {
    let hero = cards.iter().find(|c| !taken.contains(c)).ok_or("no free card")?.clone();
    d.press(player, &hero)?;
    Ok(hero)
}

pub fn captains_draft(_image: &[u8]) -> Result<(), String> {
    let mut d = Draft::new(&["cd"])?;
    d.start()?;
    mode_panel_title(&d, "Captains Draft")?;
    let cards = d.card_ids()?;
    expect("captains draft deals 28 distinct heroes", cards.len() == 28 && cards.iter().collect::<std::collections::BTreeSet<_>>().len() == 28, || format!("{cards:?}"))?;
    expect("the pool is the first 6.88g heroes", cards.iter().all(|c| d.pack.index(c).is_some()), || format!("{cards:?}"))?;
    let rows = d.rows("draft", None)?;
    expect("ten turns", rows.len() == 10, || format!("{}", rows.len()))?;
    let counts: Vec<&str> = rows.iter().map(|r| r[2].as_str()).collect();
    expect("turn plan", counts == ["ban 1", "ban 1", "ban 1", "ban 1", "pick 1", "pick 2", "pick 2", "pick 2", "pick 2", "pick 1"], || format!("{counts:?}"))?;
    let outside = d.pack.pool.iter().find(|h| !cards.contains(h)).cloned().ok_or("every pool hero is a card")?;
    d.seconds(16)?;
    let first_row = d.rows("draft", None)?[0][1].clone();
    let a = side_of_label(&first_row);
    expect("the first turn names its captain", d.text("draft", None).contains("captain"), || d.text("draft", None))?;
    d.press(captain(a), &outside)?;
    expect("a hero outside the dealt cards cannot be chosen", d.filled("draft", 3)? == 0, || format!("{:?}", d.rows("draft", None)))?;
    let mut taken: Vec<String> = Vec::new();
    let mut picks: [Vec<String>; 2] = [Vec::new(), Vec::new()];
    let turns = [1usize, 1, 1, 1, 1, 2, 2, 2, 2, 1];
    for (t, count) in turns.iter().enumerate() {
        let side = if t % 2 == 0 { a } else { 1 - a };
        for n in 0..*count {
            if n == 0 && t == 1 {
                let stray = press_free(&d, &cards, &taken)?;
                d.press(captain(a), &stray)?;
                expect("the team that just finished cannot act", d.filled("draft", 3)? == 1, || "acted out of turn".to_owned())?;
            }
            let hero = press_card(&mut d, captain(side), &cards, &taken)?;
            taken.push(hero.clone());
            if t >= 4 {
                picks[side].push(hero);
            }
        }
        expect("the turn's heroes are logged", d.rows("draft", None)?[t][3].matches(", ").count() + 1 == *count, || format!("{:?}", d.rows("draft", None)))?;
        d.seconds(1)?;
    }
    expect("each team holds five picks", picks[0].len() == 5 && picks[1].len() == 5, || format!("{picks:?}"))?;
    expect("the pool panel marks every card", d.pool_cells()?.iter().filter(|c| c.contains("(ban)") || c.contains("(pick)")).count() == 14, || format!("{:?}", d.pool_cells()))?;
    d.seconds(7)?;
    claim_all(&mut d, &picks)?;
    d.start()?;
    expect("the match starts once the draft is done", d.started(), || "no start event".to_owned())?;
    println!("captains draft ok first={first_row}");
    Ok(())
}

pub fn captains_draft_timeout(_image: &[u8]) -> Result<(), String> {
    let mut d = Draft::new(&["cd"])?;
    d.start()?;
    d.run(15 * TPS)?;
    d.run(149 * TPS)?;
    expect("the first ban stays open while the reserve lasts", d.filled("draft", 3)? == 0, || d.text("draft", None))?;
    d.run(2 * TPS)?;
    let rows = d.rows("draft", None)?;
    expect("an exhausted reserve bans a random card", !rows[0][3].is_empty(), || format!("{:?}", rows[0]))?;
    expect("the draft says it banned at random", d.said("has randomly banned"), || "no random ban message".to_owned())?;
    let a = side_of_label(&rows[0][1]);
    let cards = d.card_ids()?;
    let free = |d: &Draft| -> Result<String, String> {
        let marked: Vec<String> = d.pool_cells()?.iter().filter(|c| c.contains("(ban)") || c.contains("(pick)")).filter_map(|c| d.pack.id_of(c)).collect();
        cards.iter().find(|c| !marked.contains(c)).cloned().ok_or_else(|| "no free card".to_owned())
    };
    let hero = free(&d)?;
    d.press(captain(1 - a), &hero)?;
    expect("the other team's captain bans in turn two", d.filled("draft", 3)? == 2, || format!("{:?}", d.rows("draft", None)))?;
    d.seconds(1)?;
    d.seconds(4)?;
    expect("a team with an empty reserve still gets six seconds", d.filled("draft", 3)? == 2, || format!("{:?}", d.rows("draft", None)))?;
    d.seconds(3)?;
    expect("then its empty reserve picks for it", d.filled("draft", 3)? == 3, || format!("{:?}", d.rows("draft", None)))?;
    println!("captains draft timeout ok");
    Ok(())
}

pub fn random_draft(_image: &[u8]) -> Result<(), String> {
    let mut d = Draft::new(&["rd"])?;
    d.start()?;
    mode_panel_title(&d, "Random Draft")?;
    let cards = d.card_ids()?;
    expect("random draft deals 24 distinct heroes", cards.len() == 24 && cards.iter().collect::<std::collections::BTreeSet<_>>().len() == 24, || format!("{cards:?}"))?;
    expect("techies is never dealt", !cards.iter().any(|c| c == "techies"), || format!("{cards:?}"))?;
    let rows = d.rows("draft", None)?;
    expect("six stages", rows.len() == 6, || format!("{rows:?}"))?;
    d.seconds(15)?;
    d.run(2)?;
    let rows = d.rows("draft", None)?;
    let a = side_of_label(&rows[0][1]);
    expect("stage one is one seat of the first team", rows[0][3] == "Active" && !rows[0][2].contains(','), || format!("{rows:?}"))?;
    let stray = captain(1 - a);
    d.press(stray, &cards[0])?;
    expect("a seat outside the stage cannot pick", d.slots()[stray].is_empty(), || format!("{:?}", d.slots()))?;
    let outside = d.pack.pool.iter().find(|h| !cards.contains(h)).cloned().ok_or("every pool hero is a card")?;
    d.press(captain(a), &outside)?;
    expect("a hero outside the 24 cannot be picked", d.slots()[captain(a)].is_empty(), || format!("{:?}", d.slots()))?;
    d.press(captain(a), &cards[5])?;
    expect("the stage's seat picks its hero", d.slots()[captain(a)] == cards[5], || format!("{:?}", d.slots()))?;
    d.press(captain(a), &cards[6])?;
    expect("a second pick is refused", d.slots()[captain(a)] == cards[5], || format!("{:?}", d.slots()))?;
    d.seconds(20)?;
    let rows = d.rows("draft", None)?;
    expect("stage two is two seats of the other team", rows[1][3] == "Active" && rows[1][2].contains(','), || format!("{rows:?}"))?;
    let b = 1 - a;
    d.press(captain(b), &cards[0])?;
    expect("the first seat of stage two picks", d.slots()[captain(b)] == cards[0], || format!("{:?}", d.slots()))?;
    d.seconds(20)?;
    expect("the seat that did not pick is given the lowest free card", d.slots()[captain(b) + 1] == cards[1], || format!("{:?} want {}", d.slots(), cards[1]))?;
    d.seconds(20 * 4)?;
    d.run(2)?;
    let slots = d.slots();
    expect("every seat holds a distinct dealt hero", slots.iter().all(|h| cards.contains(h)) && slots.iter().collect::<std::collections::BTreeSet<_>>().len() == 10, || format!("{slots:?}"))?;
    d.start()?;
    expect("the match starts once the draft is done", d.started(), || "no start event".to_owned())?;
    println!("random draft ok first={}", rows[0][1]);
    Ok(())
}

pub fn single_draft(_image: &[u8]) -> Result<(), String> {
    let mut d = Draft::new(&["sd"])?;
    d.start()?;
    let mut all: Vec<String> = Vec::new();
    for seat in 0..10usize {
        let rows = d.rows("draft", Some(seat))?;
        expect("three offers", rows.len() == 3, || format!("{rows:?}"))?;
        let attrs: Vec<&str> = rows.iter().map(|r| r[1].as_str()).collect();
        expect("agility, strength, intelligence", attrs == ["Agility", "Strength", "Intelligence"], || format!("{attrs:?}"))?;
        for (n, row) in rows.iter().enumerate() {
            let id = d.pack.id_of(&row[0]).ok_or_else(|| format!("offer {:?} is not a pool hero", row[0]))?;
            let group = d.pack.groups[d.pack.index(&id).unwrap() - 1];
            expect("offer attribute", group == [2u8, 1, 3][n], || format!("{id} is group {group}"))?;
            all.push(id);
        }
    }
    expect("no hero is offered twice", all.iter().collect::<std::collections::BTreeSet<_>>().len() == 30, || format!("{all:?}"))?;
    let theirs = all[3].clone();
    d.press(0, &theirs)?;
    expect("a hero offered to another seat cannot be picked", d.slots()[0].is_empty(), || format!("{:?}", d.slots()))?;
    d.press(0, &all[1])?;
    expect("a seat picks one of its three", d.slots()[0] == all[1], || format!("{:?}", d.slots()))?;
    d.press(0, &all[2])?;
    expect("the first pick stands", d.slots()[0] == all[1], || format!("{:?}", d.slots()))?;
    d.seconds(119)?;
    expect("nobody is assigned before the 120 s window ends", d.slots()[1].is_empty(), || format!("{:?}", d.slots()))?;
    d.seconds(1)?;
    d.run(2)?;
    let slots = d.slots();
    for seat in 1..10usize {
        expect("a seat with no pick is given its first offer", slots[seat] == all[seat * 3], || format!("seat {seat}: {slots:?}"))?;
    }
    d.start()?;
    expect("the match starts once every seat holds a hero", d.started(), || "no start event".to_owned())?;
    println!("single draft ok");
    Ok(())
}

pub fn reverse_draft(_image: &[u8]) -> Result<(), String> {
    let mut d = Draft::new(&["rv"])?;
    d.start()?;
    let pool = d.pack.pool.clone();
    d.press(0, &pool[0])?;
    d.press(3, &pool[1])?;
    d.press(2, &pool[0])?;
    expect("a seat sees its own pick and only a done mark for others", d.rows("draft", Some(0))?.iter().any(|r| r[2] != "Done" && r[2] != "Wait"), || format!("{:?}", d.rows("draft", Some(0))))?;
    d.seconds(1)?;
    let view = d.rows("draft", Some(1))?;
    expect("another seat sees only that seat picked", view[0][2] == "Done" && view[3][2] == "Done" && view[2][2] == "Wait", || format!("{view:?}"))?;
    d.seconds(55)?;
    d.press(1, &pool[2])?;
    d.seconds(2)?;
    d.press(4, &pool[10])?;
    d.seconds(3)?;
    d.run(2)?;
    let slots = d.slots();
    expect("a pick goes to the opposing seat of the same place", slots[5] == pool[0] && slots[8] == pool[1] && slots[6] == pool[2], || format!("{slots:?}"))?;
    for seat in [0usize, 1, 2, 3, 4, 7, 9] {
        let idx = d.pack.index(&slots[seat]).ok_or_else(|| format!("seat {seat} holds {:?}, not a pool hero", slots[seat]))?;
        let own_half = if seat < 5 { idx <= SENTINEL_LAST } else { idx > SENTINEL_LAST };
        expect("a seat whose partner did not pick gets a random hero of its own half of the pool", own_half, || format!("seat {seat} holds {} (index {idx})", slots[seat]))?;
    }
    expect("a hero picked twice is dealt once", slots.iter().filter(|h| **h == pool[0]).count() == 1, || format!("{slots:?}"))?;
    expect("heroes are distinct", slots.iter().collect::<std::collections::BTreeSet<_>>().len() == 10, || format!("{slots:?}"))?;
    d.start()?;
    expect("the match starts once the draft is done", d.started(), || "no start event".to_owned())?;
    println!("reverse draft ok");
    Ok(())
}

pub fn bot_seats(_image: &[u8]) -> Result<(), String> {
    let mut d = Draft::with_bots(&["cm"], 1)?;
    mode_panel_title(&d, "Captains Mode")?;
    expect("starting the draft clears the heroes the bots were filled with", d.slots().iter().all(String::is_empty), || format!("{:?}", d.slots()))?;
    d.seconds(15)?;
    d.run(2)?;
    let rows = d.rows("draft", None)?;
    expect("bot captains act for every turn at once", rows.iter().all(|r| r[3] != "-"), || format!("{rows:?}"))?;
    expect("a bot pick is marked auto", rows.iter().filter(|r| r[2] == "pick").all(|r| r[3].contains("(auto)")), || format!("{rows:?}"))?;
    expect("a bot ban is no ban", rows.iter().filter(|r| r[2] == "ban").all(|r| r[3].contains("(none)")), || format!("{rows:?}"))?;
    let picks: Vec<String> = rows.iter().filter(|r| r[2] == "pick" && side_of_label(&r[1]) == 0).filter_map(|r| d.pack.id_of(&r[3])).collect();
    expect("the Sentinel team holds five picks", picks.len() == 5, || format!("{picks:?}"))?;
    d.seconds(2)?;
    let slots = d.slots();
    expect("bot seats claim team picks in seat order when the claim window opens", [0usize, 2, 3, 4].iter().zip(&picks).all(|(seat, hero)| slots[*seat] == *hero), || format!("{slots:?} {picks:?}"))?;
    d.press(1, &picks[4])?;
    expect("the human seat claims what is left", d.slots()[1] == picks[4], || format!("{:?}", d.slots()))?;
    expect("the Scourge bots hold their team's picks as well", d.slots()[5..].iter().all(|h| !h.is_empty()), || format!("{:?}", d.slots()))?;
    d.start()?;
    d.act(1, "start", json!({}))?;
    expect("the match starts once the draft is done", d.started(), || "no start event".to_owned())?;
    println!("draft bot seats ok");
    Ok(())
}
