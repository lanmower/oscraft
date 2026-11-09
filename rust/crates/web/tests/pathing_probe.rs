use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::path::{Path, PathBuf};

use mod_host::{build_image, ModHost};
use sim::fixed::{self, HALF, ONE};
use sim::Sim;

const PLAYER: u8 = 0;
const FOE: u8 = 1;
const ORDER_MOVE: u8 = 0;
const ORDER_ATTACK_MOVE: u8 = 2;
const ORDER_FOLLOW: u8 = 5;
const HERO: &str = "antimage";
const FOE_HERO: &str = "axe";
const GROUND: &str = "sentinel_creep";
const ACQUIRE_GAME_UNITS: u32 = 640;
const START: (i32, i32) = (92, 240);
const GOAL: (i32, i32) = (108, 232);
const POCKET: (i32, i32) = (110, 232);
const LEAD: (i32, i32) = (116, 240);
const WALL_X: i32 = 98;
const WALL_TOP: i32 = 234;
const WALL_LEN: i32 = 13;
const ARRIVE_CELLS: f64 = 1.0;
const MOVE_TICKS: u32 = 1500;
const STATION_CELLS: f64 = 2.0;
const STATION_SLACK_CELLS: f64 = 0.5;

fn pack_dir() -> PathBuf {
    match std::env::var_os("PROBE_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../scenarios/moba"),
    }
}

fn collect(dir: &Path, rel: &str, out: &mut Vec<(String, String)>) {
    for entry in std::fs::read_dir(dir).expect("read the pack directory") {
        let path = entry.expect("read a pack entry").path();
        let name = path.file_name().expect("a pack file name").to_string_lossy().into_owned();
        if path.is_dir() {
            if name != "fixtures" {
                collect(&path, &format!("{rel}{name}/"), out);
            }
        } else if path.extension().is_some_and(|e| e == "lua") {
            out.push((format!("{rel}{name}"), std::fs::read_to_string(&path).expect("read a lua module")));
        }
    }
}

fn stem(rel: &str) -> &str {
    let file = rel.rsplit('/').next().unwrap_or(rel);
    file.strip_suffix(".lua").unwrap_or(file)
}

fn visit(files: &[(String, String)], module: &str, out: &mut Vec<(String, String)>) {
    let Some((rel, src)) = files.iter().find(|(rel, _)| stem(rel) == module) else { return };
    if out.iter().any(|(name, _)| name == rel) {
        return;
    }
    out.push((rel.clone(), src.clone()));
    for required in src.split("require(\"").skip(1).filter_map(|rest| rest.split('"').next()) {
        visit(files, required, out);
    }
}

fn pack_host() -> ModHost {
    let dir = pack_dir();
    let manifest: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("manifest.json")).expect("the pack manifest"))
        .expect("a valid pack manifest");
    let rules = std::fs::read_to_string(dir.join(manifest["rules"].as_str().expect("the manifest rules file")))
        .expect("the pack rules file");
    let mut files = Vec::new();
    collect(&dir, "", &mut files);
    files.sort();
    let mut mods = Vec::new();
    for root in manifest["mods"].as_array().expect("the manifest mods list") {
        visit(&files, stem(root.as_str().expect("a mod file name")), &mut mods);
    }
    let image = build_image(&dir).expect("build the moba image");
    let mut host = ModHost::new(&image).expect("load the moba image");
    assert!(host.sim_mut().set_attr_rules_text(&rules), "the pack rules are accepted");
    host.load_mods(&mods).expect("load the moba mods");
    host.sim_mut().set_fog(false);
    host
}

fn fixed_point(cell: (i32, i32)) -> (i32, i32) {
    (fixed::from_int(cell.0) + HALF, fixed::from_int(cell.1) + HALF)
}

fn cell_point(cell: (i32, i32)) -> (f64, f64) {
    (f64::from(cell.0) + 0.5, f64::from(cell.1) + 0.5)
}

fn spawn(host: &mut ModHost, player: u8, kind: &str, hero: bool, cell: (i32, i32)) -> usize {
    let mut sim = host.sim_mut();
    let index = sim.catalog().unit_index(kind).expect("the unit kind is in the moba catalog") as u32;
    let (x, y) = fixed_point(cell);
    let spawned = if hero { sim.spawn_attributed(player, index, x, y) } else { sim.spawn_for(player, index, x, y) };
    spawned.expect("the unit fits on the map")
}

fn at(host: &ModHost, unit: usize) -> (f64, f64) {
    let (x, y) = host.sim().position_fixed(unit);
    (f64::from(x) / f64::from(ONE), f64::from(y) / f64::from(ONE))
}

fn cell(host: &ModHost, unit: usize) -> (i32, i32) {
    let (x, y) = host.sim().position_fixed(unit);
    (fixed::to_int(x), fixed::to_int(y))
}

fn dist(a: (f64, f64), b: (f64, f64)) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

fn assert_on_open_cell(host: &ModHost, unit: usize, tick: u32) {
    let (x, y) = cell(host, unit);
    assert!(host.sim().grid().walkable(x, y), "unit {unit} stood on a blocked cell ({x}, {y}) at tick {tick}");
}

fn shortest_cells(sim: &Sim, from: (i32, i32), to: (i32, i32)) -> Option<f64> {
    let grid = sim.grid();
    let w = grid.w as i32;
    let index = |x: i32, y: i32| (y * w + x) as usize;
    let mut best = vec![i64::MAX; grid.w * grid.h];
    let mut open = BinaryHeap::new();
    best[index(from.0, from.1)] = 0;
    open.push(Reverse((0i64, from.0, from.1)));
    while let Some(Reverse((cost, x, y))) = open.pop() {
        if (x, y) == to {
            return Some(cost as f64 / 1000.0);
        }
        if cost > best[index(x, y)] {
            continue;
        }
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
            let (nx, ny) = (x + dx, y + dy);
            if !grid.walkable(nx, ny) {
                continue;
            }
            let diagonal = dx != 0 && dy != 0;
            if diagonal && (!grid.walkable(x + dx, y) || !grid.walkable(x, y + dy)) {
                continue;
            }
            let next = cost + if diagonal { 1414 } else { 1000 };
            if next < best[index(nx, ny)] {
                best[index(nx, ny)] = next;
                open.push(Reverse((next, nx, ny)));
            }
        }
    }
    None
}

fn raise_wall(host: &mut ModHost) {
    let mut sim = host.sim_mut();
    for y in WALL_TOP..WALL_TOP + WALL_LEN {
        sim.set_cell_blocked(WALL_X, y, true).expect("the wall cell is on the map");
    }
}

fn no_acquire(host: &mut ModHost, kind: &str) {
    host.sim_mut().set_acquire(kind, 0).expect("the kind takes no acquire range");
}

fn acquire_range(host: &mut ModHost, kind: &str) {
    host.sim_mut().set_acquire(kind, ACQUIRE_GAME_UNITS).expect("the kind takes an acquire range");
}

fn struck_by(host: &ModHost, attacker: usize, target: usize) -> bool {
    host.sim().recent_hitters(target, 1).contains(&(attacker as u32))
}

fn weapon_cells(host: &ModHost, unit: usize) -> f64 {
    f64::from(host.sim().weapon(unit).1) / f64::from(ONE)
}

#[test]
fn a_hero_walks_around_a_wall_to_its_destination_at_near_shortest_length() {
    walk_around_wall(HERO, true);
}

#[test]
fn a_ground_unit_walks_around_a_wall_to_its_destination_at_near_shortest_length() {
    walk_around_wall(GROUND, false);
}

fn walk_around_wall(kind: &str, hero: bool) {
    let mut host = pack_host();
    let mover = spawn(&mut host, PLAYER, kind, hero, START);
    raise_wall(&mut host);
    let optimal = shortest_cells(&host.sim(), START, GOAL).expect("the wall leaves a route to the goal");
    let (gx, gy) = fixed_point(GOAL);
    host.sim_mut().issue_order_kind(mover, ORDER_MOVE, gx, gy).expect("the move order is accepted");

    let mut length = 0.0;
    let mut prev = at(&host, mover);
    let mut arrived = None;
    for t in 1..=MOVE_TICKS {
        host.advance().expect("advance the match");
        assert_on_open_cell(&host, mover, t);
        let now = at(&host, mover);
        length += dist(prev, now);
        prev = now;
        if dist(now, cell_point(GOAL)) <= ARRIVE_CELLS {
            arrived = Some(t);
            break;
        }
    }
    println!("probe walk_around_wall kind={kind} optimal={optimal:.2} length={length:.2} ticks={arrived:?} end={prev:?}");
    assert!(arrived.is_some(), "{kind} reached the goal within {MOVE_TICKS} ticks (ended at {prev:?})");
    assert!(length <= optimal * 1.25 + 1.0, "{kind} path length {length:.2} exceeds the shortest {optimal:.2} by more than 25%");
}

#[test]
fn a_hero_chases_a_static_hero_around_a_wall_and_strikes_from_reach() {
    chase_around_wall(FOE_HERO, true, false);
}

#[test]
fn a_hero_chases_a_static_ground_unit_around_a_wall_and_strikes_from_reach() {
    chase_around_wall(GROUND, false, false);
}

#[test]
fn a_hero_chases_a_hero_that_walks_away_and_strikes_from_reach() {
    chase_around_wall(FOE_HERO, true, true);
}

#[test]
fn a_hero_chases_a_ground_unit_that_walks_away_and_strikes_from_reach() {
    chase_around_wall(GROUND, false, true);
}

fn chase_around_wall(kind: &str, foe_hero: bool, foe_walks: bool) {
    let mut host = pack_host();
    let attacker = spawn(&mut host, PLAYER, HERO, true, START);
    let target = spawn(&mut host, FOE, kind, foe_hero, GOAL);
    no_acquire(&mut host, kind);
    raise_wall(&mut host);
    let optimal = shortest_cells(&host.sim(), START, GOAL).expect("the wall leaves a route to the target");
    let range = weapon_cells(&host, attacker);
    if foe_walks {
        let (lx, ly) = fixed_point(LEAD);
        host.sim_mut().order(target, lx, ly);
    }
    let accepted = host.sim_mut().order_attack_or_approach(attacker, target).expect("the attack order is accepted");
    assert!(!accepted || range > 0.0, "the attack order is queued");

    let mut length = 0.0;
    let mut prev = at(&host, attacker);
    let mut struck = None;
    let mut trace = Vec::new();
    for t in 1..=MOVE_TICKS {
        host.advance().expect("advance the match");
        assert_on_open_cell(&host, attacker, t);
        let now = at(&host, attacker);
        length += dist(prev, now);
        prev = now;
        if t % 50 == 0 {
            trace.push(format!(
                "t={t} attacker={:?} tgt={:?} order={} target={:?} target_order={} gap={:.2} strike={}",
                now,
                host.sim().order_of(attacker).1,
                host.sim().order_of(attacker).0,
                at(&host, target),
                host.sim().order_of(target).0,
                dist(now, at(&host, target)),
                host.sim().commanded_strike_index(attacker),
            ));
        }
        if struck_by(&host, attacker, target) {
            let gap = dist(at(&host, attacker), at(&host, target));
            struck = Some((t, gap, length));
            break;
        }
    }
    if struck.is_none() {
        for line in &trace {
            println!("probe chase trace kind={kind} walks={foe_walks} {line}");
        }
    }
    println!(
        "probe chase kind={kind} walks={foe_walks} optimal_to_start_cell={optimal:.2} range={range:.2} struck={struck:?} attacker_end={prev:?}"
    );
    let (tick, gap, travelled) = struck.expect("the attacker chased the target and struck it");
    assert!(gap <= range + ARRIVE_CELLS, "the strike at tick {tick} came from {gap:.2} cells, beyond reach {range:.2}");
    if !foe_walks {
        assert!(travelled <= optimal + 2.0, "the chase travelled {travelled:.2} cells against a shortest {optimal:.2}");
    }
}

#[test]
fn a_follower_keeps_station_on_a_hero_leader_around_a_wall() {
    follow_around_wall(HERO, true);
}

#[test]
fn a_follower_keeps_station_on_a_ground_leader_around_a_wall() {
    follow_around_wall(GROUND, false);
}

fn follow_around_wall(kind: &str, leader_hero: bool) {
    let mut host = pack_host();
    let follower = spawn(&mut host, PLAYER, HERO, true, START);
    let leader = spawn(&mut host, PLAYER, kind, leader_hero, GOAL);
    raise_wall(&mut host);
    host.sim_mut().issue_follow(follower, leader).expect("the follow order is accepted");

    let gap = |host: &ModHost| dist(at(host, follower), at(host, leader));
    let mut station = None;
    for t in 1..=MOVE_TICKS {
        host.advance().expect("advance the match");
        assert_on_open_cell(&host, follower, t);
        if gap(&host) <= STATION_CELLS {
            station = Some(t);
            break;
        }
    }
    println!("probe follow kind={kind} station_tick={station:?} gap={:.2}", gap(&host));
    assert!(station.is_some(), "the follower reached station around the wall");

    let (lx, ly) = fixed_point(LEAD);
    host.sim_mut().order(leader, lx, ly);
    let mut worst = 0.0_f64;
    for t in 1..=600 {
        if t == 300 {
            let (sx, sy) = fixed_point((100, 240));
            host.sim_mut().order(leader, sx, sy);
        }
        host.advance().expect("advance the match");
        assert_on_open_cell(&host, follower, t);
        if t >= 450 {
            worst = worst.max(gap(&host));
        }
    }
    println!("probe follow kind={kind} worst_gap_after_leader_moves={worst:.2} order={}", host.sim().order_of(follower).0);
    assert_eq!(host.sim().order_of(follower).0, ORDER_FOLLOW, "the follow order persists");
    assert!(
        worst <= STATION_CELLS + STATION_SLACK_CELLS,
        "the follower keeps station on a moving {kind} leader (worst gap {worst:.2} cells)"
    );
}

const SETTLE_TICKS: u32 = 300;
const SAMPLE_TICKS: u32 = 300;
const JITTER_RADIUS_CELLS: f64 = 1.5;
const REVERSAL_LIMIT: u32 = 30;
const POCKET_RING: [(i32, i32); 8] = [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)];
const PRESSED_START: (i32, i32) = (POCKET.0 - 2, POCKET.1);

fn wall_pocket(host: &mut ModHost, centre: (i32, i32)) {
    let mut sim = host.sim_mut();
    for (dx, dy) in POCKET_RING {
        sim.set_cell_blocked(centre.0 + dx, centre.1 + dy, true).expect("the pocket cell is on the map");
    }
}

fn settle_and_measure(host: &mut ModHost, unit: usize) -> (f64, u32) {
    for t in 1..=SETTLE_TICKS {
        host.advance().expect("advance the match");
        assert_on_open_cell(host, unit, t);
    }
    let mut samples = Vec::new();
    for t in 1..=SAMPLE_TICKS {
        host.advance().expect("advance the match");
        assert_on_open_cell(host, unit, SETTLE_TICKS + t);
        samples.push(at(host, unit));
    }
    let n = samples.len() as f64;
    let mean = (samples.iter().map(|p| p.0).sum::<f64>() / n, samples.iter().map(|p| p.1).sum::<f64>() / n);
    let radius = samples.iter().map(|&p| dist(p, mean)).fold(0.0, f64::max);
    let mut reversals = 0;
    let mut last: Option<(f64, f64)> = None;
    for pair in samples.windows(2) {
        let step = (pair[1].0 - pair[0].0, pair[1].1 - pair[0].1);
        if step.0.hypot(step.1) < 1e-3 {
            continue;
        }
        if let Some(previous) = last {
            if previous.0 * step.0 + previous.1 * step.1 < 0.0 {
                reversals += 1;
            }
        }
        last = Some(step);
    }
    println!("probe settle unit={unit} radius={radius:.3} reversals={reversals} end={:?}", samples.last());
    (radius, reversals)
}

fn assert_settles(kind: &str, unit: usize, radius: f64, reversals: u32) {
    assert!(radius <= JITTER_RADIUS_CELLS, "{kind} unit {unit} wanders {radius:.2} cells after settling");
    assert!(
        reversals <= REVERSAL_LIMIT,
        "{kind} unit {unit} reverses direction {reversals} times in {SAMPLE_TICKS} ticks after settling"
    );
}

#[test]
fn a_hero_with_an_unreachable_order_target_settles_instead_of_jittering() {
    let mut host = pack_host();
    let hero = spawn(&mut host, PLAYER, HERO, true, START);
    wall_pocket(&mut host, POCKET);
    let (gx, gy) = fixed_point(POCKET);
    host.sim_mut().issue_order_kind(hero, ORDER_MOVE, gx, gy).expect("the move order is accepted");
    let (radius, reversals) = settle_and_measure(&mut host, hero);
    assert_settles(HERO, hero, radius, reversals);
}

#[test]
fn a_ground_unit_with_an_unreachable_order_target_settles_instead_of_jittering() {
    let mut host = pack_host();
    let unit = spawn(&mut host, PLAYER, GROUND, false, START);
    wall_pocket(&mut host, POCKET);
    let (gx, gy) = fixed_point(POCKET);
    host.sim_mut().issue_order_kind(unit, ORDER_MOVE, gx, gy).expect("the move order is accepted");
    let (radius, reversals) = settle_and_measure(&mut host, unit);
    assert_settles(GROUND, unit, radius, reversals);
}

#[test]
fn a_ground_unit_pressed_against_an_obstacle_settles_instead_of_jittering() {
    let mut host = pack_host();
    let unit = spawn(&mut host, PLAYER, GROUND, false, PRESSED_START);
    wall_pocket(&mut host, POCKET);
    let (gx, gy) = fixed_point(POCKET);
    host.sim_mut().issue_order_kind(unit, ORDER_MOVE, gx, gy).expect("the move order is accepted");
    let (radius, reversals) = settle_and_measure(&mut host, unit);
    assert_settles(GROUND, unit, radius, reversals);
}

#[test]
fn a_crowd_ordered_to_one_point_settles_instead_of_jittering() {
    let mut host = pack_host();
    let crowd: Vec<usize> = (0..6).map(|k| spawn(&mut host, PLAYER, GROUND, false, (START.0 + k % 3, START.1 + k / 3))).collect();
    let (gx, gy) = fixed_point(GOAL);
    for &unit in &crowd {
        host.sim_mut().issue_order_kind(unit, ORDER_MOVE, gx, gy).expect("the move order is accepted");
    }
    for _ in 0..SETTLE_TICKS {
        host.advance().expect("advance the match");
    }
    let mut samples = vec![Vec::new(); crowd.len()];
    for _ in 0..SAMPLE_TICKS {
        host.advance().expect("advance the match");
        for (k, &unit) in crowd.iter().enumerate() {
            samples[k].push(at(&host, unit));
        }
    }
    for (k, &unit) in crowd.iter().enumerate() {
        let points = &samples[k];
        let n = points.len() as f64;
        let mean = (points.iter().map(|p| p.0).sum::<f64>() / n, points.iter().map(|p| p.1).sum::<f64>() / n);
        let radius = points.iter().map(|&p| dist(p, mean)).fold(0.0, f64::max);
        let mut reversals = 0;
        let mut last: Option<(f64, f64)> = None;
        for pair in points.windows(2) {
            let step = (pair[1].0 - pair[0].0, pair[1].1 - pair[0].1);
            if step.0.hypot(step.1) < 1e-3 {
                continue;
            }
            if let Some(previous) = last {
                if previous.0 * step.0 + previous.1 * step.1 < 0.0 {
                    reversals += 1;
                }
            }
            last = Some(step);
        }
        println!("probe crowd unit={unit} radius={radius:.3} reversals={reversals}");
        assert_settles(GROUND, unit, radius, reversals);
    }
}

const TOWER_KIND: &str = "tower_t3";
const TOWER_CELL: (i32, i32) = (110, 240);
const HERO_BESIDE_TOWER: (i32, i32) = (111, 240);
const STRUCTURE_TICKS: u32 = 30;
const DRIFT_HOLDER_CELL: (i32, i32) = (100, 236);
const DRIFT_HOLDER_GOAL: (i32, i32) = (97, 240);
const DRIFT_HOLDER_AWAY: (i32, i32) = (84, 236);
const DRIFT_HOLDER_TICKS: u32 = 150;
const DRIFT_HOLDER_AWAY_TICKS: u32 = 120;
const DRIFT_GOAL: (i32, i32) = (99, 240);
const DRIFT_MOVE_TICKS: u32 = 600;

#[test]
fn a_structure_keeps_its_spot_when_a_hero_stands_beside_it() {
    let mut host = pack_host();
    let tower = spawn(&mut host, PLAYER, TOWER_KIND, false, TOWER_CELL);
    spawn(&mut host, PLAYER, HERO, true, HERO_BESIDE_TOWER);
    let before = host.sim().position_fixed(tower);
    for t in 1..=STRUCTURE_TICKS {
        host.advance().expect("advance the match");
        assert_eq!(host.sim().position_fixed(tower), before, "the {TOWER_KIND} moved by tick {t}");
    }
    println!("probe structure kind={TOWER_KIND} before={before:?} after={:?}", host.sim().position_fixed(tower));
}

#[test]
fn a_unit_whose_goal_drifted_reaches_its_target_past_a_wall() {
    let mut host = pack_host();
    let holder = spawn(&mut host, PLAYER, GROUND, false, DRIFT_HOLDER_CELL);
    raise_wall(&mut host);
    let (hx, hy) = fixed_point(DRIFT_HOLDER_GOAL);
    host.sim_mut().issue_order_kind(holder, ORDER_MOVE, hx, hy).expect("the move order is accepted");
    for _ in 0..DRIFT_HOLDER_TICKS {
        host.advance().expect("advance the match");
    }
    let (ax, ay) = fixed_point(DRIFT_HOLDER_AWAY);
    host.sim_mut().issue_order_kind(holder, ORDER_MOVE, ax, ay).expect("the move order is accepted");
    for _ in 0..DRIFT_HOLDER_AWAY_TICKS {
        host.advance().expect("advance the match");
    }

    let unit = spawn(&mut host, PLAYER, GROUND, false, START);
    let (gx, gy) = fixed_point(DRIFT_GOAL);
    host.sim_mut().issue_order_kind(unit, ORDER_MOVE, gx, gy).expect("the move order is accepted");
    let destination = cell_point(DRIFT_GOAL);
    let mut arrived = None;
    for t in 1..=DRIFT_MOVE_TICKS {
        host.advance().expect("advance the match");
        assert_on_open_cell(&host, unit, t);
        if dist(at(&host, unit), destination) <= ARRIVE_CELLS {
            arrived = Some(t);
            break;
        }
    }
    let end = at(&host, unit);
    println!("probe drift_goal unit={unit} arrived={arrived:?} end={end:?} short={:.2}", dist(end, destination));
    assert!(arrived.is_some(), "the unit stopped {:.2} cells short of its drifted goal (ended at {end:?})", dist(end, destination));
}

const CREEP_MOVE_FROM: (i32, i32) = (90, 232);
const CREEP_MOVE_TO: (i32, i32) = (112, 232);
const CREEP_CROWD_CENTRE: (i32, i32) = (100, 232);
const CREEP_ARRIVE_TICKS: u32 = 400;
const STALL_RUN_LIMIT: u32 = 30;

fn creep_crowd() -> Vec<(i32, i32)> {
    let mut crowd = Vec::new();
    for dy in [-2, 0, 2] {
        for dx in [-2, 0, 2] {
            crowd.push((CREEP_CROWD_CENTRE.0 + dx, CREEP_CROWD_CENTRE.1 + dy));
        }
    }
    crowd
}

fn route_past_creeps(creep_player: u8) {
    let mut host = pack_host();
    let mover = spawn(&mut host, PLAYER, HERO, true, CREEP_MOVE_FROM);
    for cell in creep_crowd() {
        spawn(&mut host, creep_player, GROUND, false, cell);
    }
    let (dx, dy) = fixed_point(CREEP_MOVE_TO);
    host.sim_mut().issue_order_kind(mover, ORDER_MOVE, dx, dy).expect("the move order is accepted");

    let destination = cell_point(CREEP_MOVE_TO);
    let mut prev = at(&host, mover);
    let mut stalls = 0u32;
    let mut run = 0u32;
    let mut longest = 0u32;
    let mut arrived = None;
    for t in 1..=CREEP_ARRIVE_TICKS {
        host.advance().expect("advance the match");
        assert_on_open_cell(&host, mover, t);
        let now = at(&host, mover);
        let at_destination = dist(now, destination) <= ARRIVE_CELLS;
        if dist(prev, now) <= 1e-3 && !at_destination {
            stalls += 1;
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
        prev = now;
        if at_destination {
            arrived = Some(t);
            break;
        }
    }
    println!(
        "probe creep_crowd creep_player={creep_player} arrived={arrived:?} stalls={stalls} longest_stall_run={longest} end={prev:?} order={} hp={}/{} strike={}",
        host.sim().order_of(mover).0,
        host.sim().hp(mover),
        host.sim().hp_max(mover),
        host.sim().commanded_strike_index(mover),
    );
    assert!(arrived.is_some(), "the hero reached {CREEP_MOVE_TO:?} past the creep crowd within {CREEP_ARRIVE_TICKS} ticks (ended at {prev:?})");
    assert!(longest <= STALL_RUN_LIMIT, "the hero stalled {longest} ticks in a row against the creep crowd");
}

#[test]
fn a_hero_routes_past_a_friendly_creep_crowd_on_its_direct_route() {
    route_past_creeps(PLAYER);
}

#[test]
fn a_hero_routes_past_an_enemy_creep_crowd_on_its_direct_route() {
    route_past_creeps(FOE);
}

#[test]
fn an_attack_move_engages_a_hero_enemy_on_the_way() {
    attack_move_engages(&[(FOE_HERO, true, 5, 0)]);
}

#[test]
fn an_attack_move_engages_a_ground_enemy_on_the_way() {
    attack_move_engages(&[(GROUND, false, 5, 0)]);
}

#[test]
fn an_attack_move_engages_every_hero_and_ground_enemy_on_the_way() {
    attack_move_engages(&[(FOE_HERO, true, 5, 0), (GROUND, false, 14, -4)]);
}

fn attack_move_engages(enemies: &[(&str, bool, i32, i32)]) {
    let mut host = pack_host();
    let attacker = spawn(&mut host, PLAYER, HERO, true, START);
    acquire_range(&mut host, HERO);
    let mut foes = Vec::new();
    for &(kind, hero, dx, dy) in enemies {
        no_acquire(&mut host, kind);
        let foe = spawn(&mut host, FOE, kind, hero, (START.0 + dx, START.1 + dy));
        foes.push((kind, foe, None::<(u32, f64)>));
    }
    let destination = (START.0 + 25, START.1);
    let (dx, dy) = fixed_point(destination);
    host.sim_mut().issue_order_kind(attacker, ORDER_ATTACK_MOVE, dx, dy).expect("the attack-move order is accepted");
    let mut arrived = None;
    for t in 1..=MOVE_TICKS {
        host.advance().expect("advance the match");
        assert_on_open_cell(&host, attacker, t);
        let to_destination = dist(at(&host, attacker), cell_point(destination));
        for entry in foes.iter_mut() {
            if entry.2.is_none() && struck_by(&host, attacker, entry.1) {
                entry.2 = Some((t, to_destination));
            }
        }
        if arrived.is_none() && to_destination <= ARRIVE_CELLS {
            arrived = Some(t);
        }
        if arrived.is_some() && foes.iter().all(|entry| entry.2.is_some()) {
            break;
        }
    }
    for (kind, foe, struck) in &foes {
        println!("probe attack_move enemy={kind} foe={foe} struck={struck:?} arrived={arrived:?}");
    }
    assert!(host.sim().alive(attacker), "the attacker is alive after the engagement");
    assert!(arrived.is_some(), "the attack-move resumed and reached its destination");
    for (kind, _, struck) in &foes {
        let (tick, to_destination) = struck.unwrap_or_else(|| panic!("the attack-move never engaged the {kind} enemy"));
        assert!(
            to_destination > ARRIVE_CELLS,
            "the {kind} enemy was struck at tick {tick} only {to_destination:.2} cells short of the destination"
        );
    }
}

const BUNDLE_SIZE: usize = 40;
const BUNDLE_FROM: (i32, i32, i32, i32) = (88, 236, 96, 246);
const BUNDLE_TO: (i32, i32, i32, i32) = (104, 230, 118, 250);
const BUNDLE_TICKS: u32 = 2400;
const BUNDLE_STALL_LIMIT: u32 = 60;

fn open_cells_in(host: &ModHost, rect: (i32, i32, i32, i32), count: usize) -> Vec<(i32, i32)> {
    let grid = host.sim().grid();
    let (x0, y0, x1, y1) = rect;
    let cells: Vec<(i32, i32)> = (y0..=y1).flat_map(|y| (x0..=x1).map(move |x| (x, y))).filter(|&(x, y)| grid.walkable(x, y)).collect();
    assert!(cells.len() >= count, "the rectangle {rect:?} holds {} open cells, {count} are needed", cells.len());
    cells.into_iter().take(count).collect()
}

#[test]
fn a_bundle_larger_than_the_path_field_cache_reaches_each_goal_behind_a_wall() {
    let mut host = pack_host();
    raise_wall(&mut host);
    let starts = open_cells_in(&host, BUNDLE_FROM, BUNDLE_SIZE);
    let goals = open_cells_in(&host, BUNDLE_TO, BUNDLE_SIZE);
    let units: Vec<usize> = starts.iter().map(|&c| spawn(&mut host, PLAYER, GROUND, false, c)).collect();
    for (&unit, &goal) in units.iter().zip(&goals) {
        let (gx, gy) = fixed_point(goal);
        host.sim_mut().issue_order_kind(unit, ORDER_MOVE, gx, gy).expect("the move order is accepted");
    }
    let mut prev: Vec<(f64, f64)> = units.iter().map(|&u| at(&host, u)).collect();
    let mut arrived: Vec<Option<u32>> = vec![None; units.len()];
    let mut run = vec![0u32; units.len()];
    let mut longest = vec![0u32; units.len()];
    for t in 1..=BUNDLE_TICKS {
        host.advance().expect("advance the match");
        for (k, &unit) in units.iter().enumerate() {
            if arrived[k].is_some() {
                continue;
            }
            let now = at(&host, unit);
            if dist(now, cell_point(goals[k])) <= ARRIVE_CELLS {
                arrived[k] = Some(t);
            } else if dist(prev[k], now) <= 1e-3 {
                run[k] += 1;
                longest[k] = longest[k].max(run[k]);
            } else {
                run[k] = 0;
            }
            prev[k] = now;
        }
        if arrived.iter().all(Option::is_some) {
            break;
        }
    }
    let stuck: Vec<usize> = (0..units.len()).filter(|&k| arrived[k].is_none()).collect();
    let worst = longest.iter().copied().max().unwrap_or(0);
    println!("probe bundle units={} arrived={} stuck={stuck:?} longest_stall_run={worst}", units.len(), units.len() - stuck.len());
    assert!(stuck.is_empty(), "{} of {} units never reached their goal behind the wall within {BUNDLE_TICKS} ticks (first stuck unit index {:?})", stuck.len(), units.len(), stuck.first());
    assert!(worst <= BUNDLE_STALL_LIMIT, "a unit stalled {worst} ticks in a row while the bundle moved");
}

const MELEE_FOE_CELL_GAP: i32 = 5;
const MELEE_TICKS: u32 = 300;
const MELEE_REACH_SLACK_CELLS: f64 = 0.05;

#[test]
fn a_melee_hit_lands_only_within_its_weapon_range() {
    let mut host = pack_host();
    no_acquire(&mut host, GROUND);
    let foe_cell = (START.0 + MELEE_FOE_CELL_GAP, START.1);
    assert!(host.sim().grid().walkable(foe_cell.0, foe_cell.1), "the foe cell {foe_cell:?} is open");
    let attacker = spawn(&mut host, PLAYER, GROUND, false, START);
    let foe = spawn(&mut host, FOE, GROUND, false, foe_cell);
    let reach = weapon_cells(&host, attacker);
    host.sim_mut().order_attack_or_approach(attacker, foe).expect("the attack order is accepted");
    let mut hp = host.sim().hp(foe);
    let mut hits = 0u32;
    for t in 1..=MELEE_TICKS {
        host.advance().expect("advance the match");
        let now = host.sim().hp(foe);
        if now < hp {
            hits += 1;
            let gap = dist(at(&host, attacker), at(&host, foe));
            assert!(gap <= reach + MELEE_REACH_SLACK_CELLS, "a melee hit landed at tick {t} from {gap:.2} cells, past the {reach:.2}-cell weapon range");
        }
        hp = now;
    }
    println!("probe melee_reach reach={reach:.2} hits={hits} foe_hp={hp}");
    assert!(hits > 0, "the melee unit never struck the foe within {MELEE_TICKS} ticks");
}
