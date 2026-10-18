// End-to-end 5v5 match witness on the moba scenario. Heroes follow a scripted policy; creeps and
// towers act only through the scenario's behaviours. It
// prints one `milestone <name> ...` line per first occurrence of: cast, attack, death, respawn,
// bounty, camp, camp_respawn, roshan, winner. Exits non-zero when a milestone never happens or
// no side is declared winner within the tick budget. Last line on success:
// `moba 5v5 match ok winner=<n> tick=<t>`.
//
// Every command the heroes issue goes through a replay recording; at the end the match is replayed from
// a fresh load of the scenario and must reach the same state hash (`replay ok hash=<h> ticks=<n>`).
// With a third argument the recording is also written to that file.
//
// Usage: cargo run -p server --example moba_match -- <scenario dir> [max_ticks] [replay file]

use std::collections::HashSet;
use std::path::Path;

use server::replay::{Command, Recording, Replayer};
use sim::{Sim, ABILITY_SLOTS};

const NEUTRAL: u8 = 0xFF;
const CAMP_PHASE_END: u32 = 6000;
const SKILL_EVERY: u32 = 16;

#[derive(Default)]
struct Seen(HashSet<&'static str>);

impl Seen {
    fn first(&mut self, name: &'static str) -> bool {
        self.0.insert(name)
    }
}

fn side(player: u8) -> u8 {
    player % 2
}

fn enemy_hero_of(sim: &Sim, me: usize) -> Option<usize> {
    let p = sim.player(me);
    (0..sim.count()).find(|&j| {
        sim.alive(j) && sim.is_hero(j) && sim.player(j) != NEUTRAL && sim.hostile(p, sim.player(j))
    })
}

fn first_alive(sim: &Sim, pred: impl Fn(usize) -> bool) -> Option<usize> {
    (0..sim.count()).find(|&j| sim.alive(j) && pred(j))
}

const DEFEND_RADIUS_TILES: i64 = 25;

fn defender_of_base(sim: &Sim, h: usize) -> Option<usize> {
    let side_h = side(sim.player(h));
    let base = first_alive(sim, |j| sim.kind_name(j) == Some("base") && side(sim.player(j)) == side_h)?;
    let (bx, by) = sim.position_fixed(base);
    let reach = (DEFEND_RADIUS_TILES << sim::fixed::SHIFT).pow(2);
    let mut best: Option<(i64, usize)> = None;
    for j in 0..sim.count() {
        if !sim.alive(j) || sim.player(j) == NEUTRAL || !sim.hostile(sim.player(h), sim.player(j)) {
            continue;
        }
        let (x, y) = sim.position_fixed(j);
        let d = (x as i64 - bx as i64).pow(2) + (y as i64 - by as i64).pow(2);
        if d <= reach && best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, j));
        }
    }
    best.map(|(_, j)| j)
}

fn rival_base_of(sim: &Sim, h: usize) -> Option<usize> {
    let side_h = side(sim.player(h));
    first_alive(sim, |j| sim.kind_name(j) == Some("base") && side(sim.player(j)) != side_h)
}

fn is_roshan(sim: &Sim, j: usize) -> bool {
    sim.kind_name(j) == Some("roshan")
}

fn is_camp_unit(sim: &Sim, j: usize) -> bool {
    sim.player(j) == NEUTRAL && sim.kind_name(j).is_some_and(|k| k.starts_with("neutral_") || k == "creep")
}

fn is_enemy_structure(sim: &Sim, h: usize, j: usize) -> bool {
    sim.hostile(sim.player(h), sim.player(j))
        && sim.kind_name(j).is_some_and(|k| k.starts_with("tower") || k.starts_with("barracks") || k == "base")
}

fn nearest_by(sim: &Sim, h: usize, pred: impl Fn(usize) -> bool) -> Option<usize> {
    let (hx, hy) = sim.position_fixed(h);
    (0..sim.count())
        .filter(|&j| sim.alive(j) && pred(j))
        .min_by_key(|&j| {
            let (x, y) = sim.position_fixed(j);
            (x as i64 - hx as i64).pow(2) + (y as i64 - hy as i64).pow(2)
        })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dir = args.get(1).map(String::as_str).unwrap_or("../scenarios/moba");
    let max_ticks: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(40_000);

    let mut sim = match server::mods::load_data(Path::new(dir), 4096) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("load failed: {e}");
            std::process::exit(2);
        }
    };
    let mut rec = Recording::new(dir, 4096, &sim);
    let heroes: Vec<usize> = (0..sim.count()).filter(|&i| sim.is_hero(i)).collect();
    let sides: HashSet<u8> = heroes.iter().map(|&h| side(sim.player(h))).collect();
    println!("setup heroes={} sides={} players={}", heroes.len(), sides.len(), {
        let mut p: Vec<u8> = heroes.iter().map(|&h| sim.player(h)).collect();
        p.sort();
        p.dedup();
        p.len()
    });

    let mut seen = Seen::default();
    let mut dead_at: Vec<Option<u32>> = vec![None; sim.count()];
    let mut camp_dead: HashSet<usize> = HashSet::new();
    let mut gold_before: Vec<i32> = vec![0; sim.count()];

    while sim.winner().is_none() && sim.ticks() < max_ticks {
        let t = sim.ticks();
        let phase_camp = t < CAMP_PHASE_END && !seen.0.contains("camp");
        let roshan_alive = first_alive(&sim, |j| is_roshan(&sim, j)).is_some();
        let phase_roshan = !phase_camp && roshan_alive;

        if t % SKILL_EVERY == 0 {
            for &h in &heroes {
                if !sim.alive(h) {
                    continue;
                }
                for slot in 0..ABILITY_SLOTS {
                    if sim.skill_points(h) == 0 {
                        break;
                    }
                    rec.apply(&mut sim, Command::SkillUp { unit: h, slot });
                }
            }
        }

        for &h in &heroes {
            if !sim.alive(h) {
                if dead_at[h].is_none() && sim.hero_respawn_left(h) >= 0 {
                    dead_at[h] = Some(t);
                    if seen.first("death") {
                        println!("milestone death tick={t} hero={h} side={}", side(sim.player(h)));
                    }
                }
                continue;
            }
            if let Some(died) = dead_at[h].take() {
                if seen.first("respawn") {
                    println!("milestone respawn tick={t} hero={h} after_ticks={}", t - died);
                }
            }

            let hostile_hero = enemy_hero_of(&sim, h);
            if let Some(foe) = hostile_hero {
                for slot in 0..ABILITY_SLOTS {
                    if sim.skill_rank(h, slot) == 0 {
                        continue;
                    }
                    if rec.apply(&mut sim, Command::Cast { unit: h, slot, target: foe as i32, x: 0, y: 0 }) && seen.first("cast") {
                        println!("milestone cast tick={t} hero={h} slot={slot} target={foe}");
                    }
                }
            }

            let target = if phase_camp {
                nearest_by(&sim, h, |j| is_camp_unit(&sim, j))
            } else {
                defender_of_base(&sim, h)
                    .or_else(|| if phase_roshan { first_alive(&sim, |j| is_roshan(&sim, j)) } else { None })
                    .or_else(|| nearest_by(&sim, h, |j| is_enemy_structure(&sim, h, j)))
                    .or_else(|| rival_base_of(&sim, h))
                    .or(hostile_hero)
                    .or_else(|| first_alive(&sim, |j| sim.hostile(sim.player(h), sim.player(j)) && j != h))
            };
            if let Some(tg) = target {
                match sim.order_attack_or_approach(h, tg) {
                    Ok(true) => {
                        rec.record_applied(&sim, Command::Attack { unit: h, target: tg });
                        if seen.first("attack") {
                            println!("milestone attack tick={t} hero={h} target={tg}");
                        }
                    }
                    Ok(false) => rec.record_applied(&sim, Command::Attack { unit: h, target: tg }),
                    Err(_) => {
                        rec.record_applied(&sim, Command::Attack { unit: h, target: tg });
                        let (x, y) = sim.position_fixed(tg);
                        rec.apply(&mut sim, Command::Order { unit: h, x, y });
                    }
                }
            }
        }

        for &h in &heroes {
            gold_before[h] = sim.hero_gold(h);
        }
        rec.tick(&mut sim, true, true);

        for ev in sim.drain_kill_events() {
            let killer = ev.killer as usize;
            let victim = ev.victim as usize;
            if sim.is_hero(killer) && sim.hostile(ev.killer_player, ev.victim_player) {
                let gold = sim.hero_gold(killer);
                if gold > gold_before[killer] && seen.first("bounty") {
                    println!("milestone bounty tick={} killer={killer} gold={gold}", ev.tick);
                }
            }
            if ev.victim_player == NEUTRAL && !is_roshan(&sim, victim) && !is_camp_unit(&sim, victim) {
                continue;
            }
            if ev.victim_player == NEUTRAL {
                if is_roshan(&sim, victim) {
                    if seen.first("roshan") {
                        println!("milestone roshan tick={} killer={killer}", ev.tick);
                    }
                } else if sim.is_hero(killer) && seen.first("camp") {
                    println!("milestone camp tick={} victim={victim} killer={killer}", ev.tick);
                }
                camp_dead.insert(victim);
            }
        }
        for &v in camp_dead.clone().iter() {
            if sim.alive(v) {
                camp_dead.remove(&v);
                if seen.first("camp_respawn") {
                    println!("milestone camp_respawn tick={} unit={v}", sim.ticks());
                }
            }
        }
    }

    let tick = sim.ticks();
    for t in sim.post_game() {
        eprintln!("tally player={} kills={} deaths={}", t.player, t.kills, t.deaths);
    }
    for &h in &heroes {
        eprintln!("hero={h} side={} alive={} gold={} hp={}", side(sim.player(h)), sim.alive(h), sim.hero_gold(h), sim.hp(h));
    }
    for j in 0..sim.count() {
        if is_roshan(&sim, j) {
            eprintln!("roshan unit={j} alive={} hp={}", sim.alive(j), sim.hp(j));
        }
    }
    rec.check(&sim);
    let text = rec.render(&sim);
    if let Some(path) = args.get(3) {
        if let Err(e) = std::fs::write(path, &text) {
            eprintln!("replay write failed: {e}");
            std::process::exit(2);
        }
    }
    let mut fresh = server::mods::load_data(Path::new(dir), 4096).unwrap_or_else(|e| {
        eprintln!("reload failed: {e}");
        std::process::exit(2);
    });
    let replayed = Recording::parse(&text).and_then(|r| Replayer::new(r, &fresh)).and_then(|mut p| p.run_to_end(&mut fresh));
    match replayed {
        Ok(h) if h == sim.state_hash() && fresh.ticks() == tick => println!("replay ok hash={h:016x} ticks={tick}"),
        other => {
            eprintln!("replay diverged: {other:?} live={:016x} ticks={tick}", sim.state_hash());
            std::process::exit(1);
        }
    }
    let required = [
        "cast", "attack", "death", "respawn", "bounty", "camp", "camp_respawn", "roshan",
    ];
    let missing: Vec<&str> = required.iter().copied().filter(|m| !seen.0.contains(m)).collect();
    match sim.winner() {
        Some(w) if missing.is_empty() => {
            println!("milestone winner tick={tick} side={w}");
            println!("moba 5v5 match ok winner={w} tick={tick}");
        }
        Some(w) => {
            eprintln!("missing milestones: {missing:?} winner={w} tick={tick}");
            std::process::exit(1);
        }
        None => {
            eprintln!("no winner by tick={tick}; missing milestones: {missing:?}");
            std::process::exit(1);
        }
    }
}
