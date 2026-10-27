// Scratch instrument for drain-d-1, not committed: drives the real moba pack on the piccolo host.
#[allow(dead_code)]
#[path = "/workspace/oscraft/rust/crates/mod-host/tests/common/mod.rs"]
mod common;

use mod_host::{build_image, ModHost};
use serde_json::{json, Value};
use std::path::PathBuf;

const FIX: f64 = 65536.0;
type Order = (u8, i32, i32);

fn fx(v: f64) -> i32 {
    (v * FIX) as i32
}

fn boot() -> ModHost {
    let dir = PathBuf::from("/workspace/oscraft/scenarios/moba");
    let image = build_image(&dir).expect("image");
    let mut host = ModHost::new(&image).expect("host");
    let ui: Value = serde_json::from_str(&std::fs::read_to_string(dir.join("moba_ui.json")).expect("ui")).expect("ui json");
    let rules = std::fs::read_to_string(dir.join("attr_rules.txt")).expect("rules");
    assert!(host.sim_mut().set_attr_rules_text(&rules), "rules load");
    host.load_mods(&common::pack_mods(&dir, &common::pack_roots(&dir))).expect("mods load");
    host.dispatch_action("setup", &json!({ "ui": ui, "player": 0 })).expect("setup");
    host.take_ui();
    host
}

fn unit_of(host: &ModHost, player: u8) -> (usize, f64, f64) {
    let sim = host.sim();
    for i in 0..sim.count() {
        if sim.alive(i) && sim.player(i) == player {
            let (x, z) = sim.position_fixed(i);
            return (i, x as f64 / FIX, z as f64 / FIX);
        }
    }
    panic!("seat {player} has no living unit");
}

fn step(host: &mut ModHost) -> Vec<String> {
    host.advance().expect("tick");
    host.take_ui()
}

fn pointer(host: &mut ModHost, kind: &str, x: f64, z: f64, button: i64) -> Vec<String> {
    host.dispatch_action("pointer", &json!({ "player": 0, "kind": kind, "x": x, "y": z, "button": button })).expect("pointer");
    host.take_ui()
}

fn click(host: &mut ModHost, x: f64, z: f64, button: i64) -> Vec<String> {
    let mut ui = pointer(host, "down", x, z, button);
    ui.extend(pointer(host, "up", x, z, button));
    ui
}

fn shift(host: &mut ModHost, down: bool) {
    let phase = if down { "down" } else { "up" };
    host.dispatch_action("key", &json!({ "key": "shift", "phase": phase, "shift": down, "ctrl": false, "alt": false, "meta": false, "player": 0, "world": { "x": 0.0, "z": 0.0 } })).expect("key");
    host.take_ui();
}

fn near(o: Order, tx: f64, tz: f64) -> bool {
    (o.1 as f64 / FIX - tx).abs() < 2.0 && (o.2 as f64 / FIX - tz).abs() < 2.0
}

fn tiles(o: Order) -> String {
    format!("kind{}@({:.2},{:.2})", o.0, o.1 as f64 / FIX, o.2 as f64 / FIX)
}

// Orders the unit by a plain or a shift right click, then ticks on until the unit has no order left.
fn arm(queued: bool) -> (f64, f64, Order, Order, Vec<Order>, u32) {
    let mut host = boot();
    let (id, x, z) = unit_of(&host, 0);
    click(&mut host, x, z, 0);
    step(&mut host);
    click(&mut host, x + 3.0, z, 2);
    step(&mut host);
    let first = host.sim().order_of(id);
    if queued {
        shift(&mut host, true);
    }
    click(&mut host, x - 3.0, z + 3.0, 2);
    if queued {
        shift(&mut host, false);
    }
    step(&mut host);
    let after = host.sim().order_of(id);
    let mut seen: Vec<Order> = Vec::new();
    let mut ticks = 0;
    for _ in 0..1500 {
        step(&mut host);
        ticks += 1;
        let o = host.sim().order_of(id);
        if seen.last() != Some(&o) {
            seen.push(o);
        }
    }
    (x, z, first, after, seen, ticks)
}

fn row1() -> bool {
    let mut all = true;
    for queued in [false, true] {
        let (x, z, first, after, seen, ticks) = arm(queued);
        let (ax, az) = (x + 3.0, z);
        let (bx, bz) = (x - 3.0, z + 3.0);
        let ok = if queued {
            near(first, ax, az) && near(after, ax, az) && seen.iter().any(|o| near(*o, bx, bz))
        } else {
            near(after, bx, bz)
        };
        all &= ok;
        let later: Vec<String> = seen.iter().map(|o| tiles(*o)).collect();
        println!(
            "witness controls-shift-queue shift={queued} unit@({x:.2},{z:.2}) first={} after_second_click={} later={} ticks={ticks} ok={ok}",
            tiles(first),
            tiles(after),
            later.join(" -> ")
        );
    }
    all
}

fn selection_ids(ui: &[String]) -> Option<Vec<usize>> {
    let b = ui.iter().rev().find(|b| b.contains("\"kind\":\"selection\""))?;
    let start = b.find("\"ids\":[")? + 7;
    let end = start + b[start..].find(']')?;
    Some(b[start..end].split(',').filter(|s| !s.trim().is_empty()).map(|s| s.trim().parse().expect("id")).collect())
}

fn row2() -> bool {
    let mut host = boot();
    let (hero, hx, hz) = unit_of(&host, 0);
    let kind = host.sim().unit_type(hero) as u32;
    let on_a = host.sim_mut().spawn_for(0, kind, fx(hx + 1.0), fx(hz + 1.0)).expect("spawn on screen");
    let on_b = host.sim_mut().spawn_for(0, kind, fx(hx - 1.0), fx(hz + 1.0)).expect("spawn on screen");
    let off = host.sim_mut().spawn_for(0, kind, fx(hx + 30.0), fx(hz)).expect("spawn off screen");
    for u in host.units(Some(0)) {
        let sim = host.sim();
        eprintln!("diag unit {u} kind={} alive={} pos={:?}", sim.kind_of(u), sim.alive(u), sim.position_fixed(u));
    }
    host.dispatch_action("viewport", &json!({ "player": 0, "x0": hx - 6.0, "z0": hz - 6.0, "x1": hx + 6.0, "z1": hz + 6.0 })).expect("viewport");
    host.take_ui();
    let mut ui = click(&mut host, hx, hz, 0);
    ui.extend(click(&mut host, hx, hz, 0));
    ui.extend(step(&mut host));
    eprintln!("diag ui {:?}", ui);
    let double = selection_ids(&ui);
    let mut want = vec![hero, on_a, on_b];
    want.sort();
    let double_ok = double.as_ref().map(|v| { let mut v = v.clone(); v.sort(); v == want }).unwrap_or(false);

    let mut single_host = boot();
    let (hero1, sx, sz) = unit_of(&single_host, 0);
    let mut single_ui = click(&mut single_host, sx, sz, 0);
    single_ui.extend(step(&mut single_host));
    let single = selection_ids(&single_ui);
    let single_ok = single.as_deref() == Some(&[hero1][..]);

    println!("witness controls-double-click-select-type double_click_ids={:?} want={:?} off_screen_id={off} ok={double_ok}", double, want);
    println!("witness controls-double-click-select-type single_click_ids={:?} want={:?} ok={single_ok}", single, [hero1]);
    double_ok && single_ok
}

fn pf(ok: bool) -> &'static str {
    if ok { "PASS" } else { "FAIL" }
}

// Usage: drain-d-1-witness [shift|dbl]. With no argument both rows run; each row is witnessed in its own dispatch.
fn main() {
    let only = std::env::args().nth(1).unwrap_or_default();
    let mut ok = true;
    if only != "dbl" {
        let r = row1();
        println!("RESULT controls-shift-queue={}", pf(r));
        ok &= r;
    }
    if only != "shift" {
        let r = row2();
        println!("RESULT controls-double-click-select-type={}", pf(r));
        ok &= r;
    }
    std::process::exit(if ok { 0 } else { 1 });
}
