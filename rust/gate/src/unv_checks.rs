use mod_host::ModHost;
use serde_json::{json, Value};
use sim::fixed;

use crate::checks::{kind, pack_run};
use crate::mod_checks;

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

fn expect(label: &str, ok: bool, detail: impl FnOnce() -> String) -> Result<(), String> {
    if ok { Ok(()) } else { Err(format!("{label}: {}", detail())) }
}

fn with_modes(codes: &[&str]) -> Result<ModHost, String> {
    let mut host = pack_run::start()?;
    host.dispatch_action("ui", &json!({ "call": "claim", "args": { "slot": 0 } })).map_err(|e| format!("claim: {e}"))?;
    for code in codes {
        host.dispatch_action("ui", &json!({ "call": "mode", "args": { "code": code } })).map_err(|e| format!("mode {code}: {e}"))?;
    }
    pack_run::advance(&mut host, 2)?;
    Ok(host)
}

fn spawn_named_hero(host: &mut ModHost, name: &str, dx: i32) -> Result<usize, String> {
    let hero_kind = kind(&host.sim(), name)?;
    let (x, y) = host.sim().position_fixed(0);
    host.sim_mut().spawn_attributed(0, hero_kind, x + fixed::from_int(dx), y).ok_or_else(|| format!("spawn_attributed refused {name}"))
}

pub fn seat_colours(_image: &[u8]) -> Result<(), String> {
    let mut host = pack_run::start()?;
    pack_run::advance(&mut host, 3)?;
    let seen = events(&mut host)?;
    let want: [(i64, i64); 10] = [
        (0, 0x0042ff), (1, 0xe55bb0), (2, 0x1ce6b9), (3, 0x959697), (4, 0x540081),
        (5, 0x7ebff1), (6, 0xfffc01), (7, 0x106246), (8, 0xff8000), (9, 0x4e2a04),
    ];
    for (seat, colour) in want {
        let event = seen.iter().rev().find(|e| e["kind"] == "seat" && e["seat"] == seat && e["colour"].is_i64());
        let event = event.ok_or_else(|| format!("no seat event with a colour for seat {seat}"))?;
        expect("seat colour", event["colour"] == colour, || format!("seat {seat}: {event}"))?;
        let name = format!("Player {}", seat + 1);
        expect("seat name", event["name"] == name.as_str(), || format!("seat {seat}: {event}"))?;
    }
    println!("seat colours ok");
    Ok(())
}

pub fn mini_hero_scale(_image: &[u8]) -> Result<(), String> {
    let mut host = with_modes(&["mi"])?;
    let scaled = spawn_named_hero(&mut host, "bane", 2)?;
    let plain = spawn_named_hero(&mut host, "juggernaut", 4)?;
    pack_run::advance(&mut host, 3)?;
    let (a, b) = (host.sim().unit_scale(scaled), host.sim().unit_scale(plain));
    expect("a hero with a 6.88g Qu of 1.5 shrinks to 0.75", a == 75, || format!("bane scale {a}"))?;
    expect("a hero without a Qu entry shrinks to 0.5", b == 50, || format!("juggernaut scale {b}"))?;
    println!("mini hero scale ok bane={a} juggernaut={b}");
    Ok(())
}

fn ground_items_beside(host: &ModHost, hero: usize) -> usize {
    let s = host.sim();
    let (hx, hy) = s.position_fixed(hero);
    let reach = i64::from(fixed::from_int(1)) * i64::from(fixed::from_int(1));
    s.ground_items().iter().filter(|g| (i64::from(g.x) - i64::from(hx)).pow(2) + (i64::from(g.y) - i64::from(hy)).pow(2) <= reach).count()
}

fn dead_meepo_drops(modes: &[&str]) -> Result<usize, String> {
    let mut host = with_modes(modes)?;
    let hero = spawn_named_hero(&mut host, "meepo", 6)?;
    let catalog_items: Vec<usize> = {
        let sim = host.sim();
        let catalog = sim.catalog();
        (0..catalog.items.len()).filter(|&i| !matches!(catalog.name(catalog.items[i].name), "rapier" | "gem")).collect()
    };
    let mut held = 0;
    for item in catalog_items {
        if held == 6 {
            break;
        }
        if host.sim_mut().grant_item(hero, item, 1) {
            held += 1;
        }
    }
    expect("meepo holds six items", held == 6, || format!("{held} granted"))?;
    pack_run::advance(&mut host, 2)?;
    for _ in 0..8 {
        if !host.sim().alive(hero) {
            break;
        }
        host.sim_mut().hurt(hero, fixed::from_int(10000));
    }
    expect("meepo dies", !host.sim().alive(hero), || "survived".into())?;
    pack_run::advance(&mut host, 4)?;
    Ok(ground_items_beside(&host, hero))
}

pub fn item_drop_clone_exempt(_image: &[u8]) -> Result<(), String> {
    let (off, on) = (dead_meepo_drops(&[])?, dead_meepo_drops(&["id"])?);
    expect("the original Meepo (H00I) is not exempt from Item Drop: only the clone H00J is", on == off + 1, || format!("{on} items beside meepo against {off} without the mode"))?;
    println!("item drop clone exempt ok");
    Ok(())
}

pub fn spectator_panel_addressing(image: &[u8]) -> Result<(), String> {
    let src = r#"
function on_tick(state)
  if state.done then return end
  state.done = 1
  game.panel("watch", { title = "W", player = 255, text = "body" })
  game.message(255, "to the spectator", 5)
  game.sound("tick", 255)
  state.high = pcall(game.panel, "x", { player = 256 }) and 0 or 1
  state.low = pcall(game.panel, "x", { player = -2 }) and 0 or 1
end
"#;
    let (mut host, _mod) = mod_checks::start(image, "unv-spectator-panel", src)?;
    pack_run::advance(&mut host, 2)?;
    for key in ["high", "low"] {
        let got = host.state_int(key).ok_or_else(|| format!("state.{key} unset"))?;
        expect("an out-of-range seat is refused", got == 1, || format!("state.{key} = {got}: {:?}", host.hook_errors()))?;
    }
    let ui = events(&mut host)?;
    for kind in ["panel", "message", "sound"] {
        expect("an event for seat 255 reaches the host queue", ui.iter().any(|e| e["kind"] == kind && e["player"] == 255), || format!("no {kind} for 255: {ui:?}"))?;
    }
    println!("spectator panel addressing ok");
    Ok(())
}

pub fn camera_chat(_image: &[u8]) -> Result<(), String> {
    let mut host = pack_run::start()?;
    pack_run::advance(&mut host, 2)?;
    events(&mut host)?;
    let mut say = |line: &str| -> Result<Vec<Value>, String> {
        let (cmd, rest) = line.split_once(' ').unwrap_or((line, ""));
        let n: i64 = rest.trim().parse().unwrap_or(0);
        host.dispatch_action("chat", &json!({ "text": line, "player": 0, "pubkey": "k0", "cmd": cmd, "n": n }))?;
        events(&mut host)
    };
    let free = say("-w")?;
    expect("-w frees the rotation", free.iter().any(|e| e["kind"] == "camera_lock" && e["locked"] == false), || format!("{free:?}"))?;
    let back = say("-w")?;
    expect("-w again locks the rotation", back.iter().any(|e| e["kind"] == "camera_lock" && e["locked"] == true), || format!("{back:?}"))?;
    let far = say("-cam 3000")?;
    expect("-cam 3000 zooms the default 1650 out by 1650/3000", far.iter().any(|e| e["kind"] == "camera_distance" && (e["zoom"].as_f64().unwrap_or(0.0) - 0.55).abs() < 1e-9), || format!("{far:?}"))?;
    let bad = say("-cam 100")?;
    expect("-cam 100 is refused with the 6.88g text", bad.iter().any(|e| e["kind"] == "message" && e["text"] == "Incorrect camera distance."), || format!("{bad:?}"))?;
    println!("camera chat ok");
    Ok(())
}
