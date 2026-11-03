use std::path::Path;
use std::time::Instant;

use sim::{Sim, ABILITY_SLOTS};

const NEUTRAL: u8 = 0xFF;
const ORDER_EVERY: u32 = 16;
const HASH_EVERY: u32 = 1000;

fn rival_base(sim: &Sim, h: usize) -> Option<usize> {
    let side = sim.player(h) % 2;
    (0..sim.count()).find(|&j| sim.alive(j) && sim.kind_name(j) == Some("base") && sim.player(j) % 2 != side)
}

fn foe_attributed(sim: &Sim, h: usize) -> Option<usize> {
    let p = sim.player(h);
    (0..sim.count()).find(|&j| sim.alive(j) && sim.is_attributed(j) && sim.player(j) != NEUTRAL && sim.hostile(p, sim.player(j)))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dir = args.get(1).map(String::as_str).unwrap_or("../scenarios/moba");
    let ticks: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(6000);
    let image = osdt_data::generate(Path::new(dir)).expect("compile scenario");
    let mut sim = sim::load(&image, 4096).expect("load scenario");
    if let Some(file) = osdt_data::rules_path(Path::new(dir)).expect("manifest") {
        let rules = std::fs::read_to_string(&file).expect("rules file");
        assert!(sim.set_attr_rules_text(&rules), "{} has a bad line", file.display());
    }
    let attributed: Vec<usize> = (0..sim.count()).filter(|&i| sim.is_attributed(i)).collect();
    let mut total = std::time::Duration::ZERO;
    let mut worst = std::time::Duration::ZERO;
    for t in 0..ticks {
        if t % ORDER_EVERY == 0 {
            for &h in &attributed {
                if !sim.alive(h) {
                    continue;
                }
                for slot in 0..ABILITY_SLOTS {
                    let level = sim.xp_curve().iter().take_while(|&&t| sim.xp(h) >= t).count() as u32;
                    if sim.next_rank_gate(h, slot).is_some_and(|gate| gate <= level + 1) {
                        let _ = sim.rank_up(h, slot);
                    }
                }
                if let Some(foe) = foe_attributed(&sim, h) {
                    for slot in 0..ABILITY_SLOTS {
                        if sim.ability_rank(h, slot) > 0 {
                            sim.cast_ability(h, slot, foe as i32, 0, 0);
                        }
                    }
                }
                if let Some(b) = rival_base(&sim, h) {
                    if sim.order_attack_or_approach(h, b).is_err() {
                        let (x, y) = sim.position_fixed(b);
                        sim.order(h, x, y);
                    }
                }
            }
        }
        let start = Instant::now();
        sim.tick();
        let d = start.elapsed();
        total += d;
        worst = worst.max(d);
        if (t + 1) % HASH_EVERY == 0 {
            println!("hash tick={} {:016x}", t + 1, sim.state_hash());
        }
    }
    let n = sim.ticks().max(1) as f64;
    println!("grid={}x{} fields={}", sim.grid().w, sim.grid().h, sim.paths().live());
    println!("units={} ticks={} mean_us={:.1} worst_us={}", sim.count(), sim.ticks(), total.as_micros() as f64 / n, worst.as_micros());
    for (s, name) in Sim::TICK_SYSTEMS.iter().enumerate() {
        let mut us = 0u64;
        for b in 0..Sim::TICK_COST_BUCKETS {
            us += Sim::tick_cost_bucket(s, b) as u64 * (b as u64 * Sim::TICK_COST_BUCKET_US + Sim::TICK_COST_BUCKET_US / 2);
        }
        println!("system {name:<12} mean_us={:.1}", us as f64 / n);
    }
    println!("final hash {:016x}", sim.state_hash());
}
