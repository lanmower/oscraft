use crate::checks::pack_run;
use crate::checks::role;
use crate::checks::{run_past_first_camp_mark, scenario_rule, MAX_UNITS};
use mod_host::ModHost;
use sim::fixed;

fn camp_units(host: &ModHost) -> usize {
    let s = host.sim();
    let n = (0..MAX_UNITS).filter(|&u| s.alive(u) && s.player(u) == 0xFF && s.kind_name(u).is_some_and(|n| n.starts_with("neutral_"))).count();
    n
}

pub fn spawn_group_boss_window(_image: &[u8]) -> Result<(), String> {
    let mut host = pack_run::start()?;
    pack_run::advance(&mut host, 2)?;
    let boss = (0..MAX_UNITS)
        .filter(|&u| {
            let s = host.sim();
            s.alive(u) && s.player(u) == 0xFF
        })
        .max_by_key(|&u| host.sim().hp(u))
        .ok_or("moba image places no neutral unit")?;
    pack_run::hurt_to_life(&mut host, boss, 0);
    let died = host.sim().ticks();
    let min = (scenario_rule("roshan_respawn_min_s")? as u32) * sim::TICKS_PER_SECOND as u32;
    let max = min + (scenario_rule("roshan_respawn_spread_s")? as u32) * sim::TICKS_PER_SECOND as u32;
    let mut back = None;
    for _ in 0..max + 16 {
        pack_run::advance(&mut host, 1)?;
        if host.sim().alive(boss) {
            back = Some(host.sim().ticks() - died);
            break;
        }
    }
    let waited = back.ok_or("Roshan never respawned within the 660 s window")?;
    if waited < min || waited > max + 4 {
        return Err(format!("Roshan respawned after {waited} ticks, want {min} to {max}"));
    }
    println!("camp roshan window ok waited_ticks={waited} window={min}..{max}");
    Ok(())
}

fn camp_check_rects() -> Result<Vec<(i64, i64, i64, i64)>, String> {
    let text = std::fs::read_to_string(crate::checks::pack_dir().join("camps_688g.lua")).map_err(|e| format!("camps_688g.lua: {e}"))?;
    let mut rects = Vec::new();
    for line in text.lines().filter(|l| l.contains("check = {")) {
        let inner = line.split("check = {").nth(1).and_then(|r| r.split('}').next()).ok_or("camps_688g.lua check rect")?;
        let v: Vec<i64> = inner.split(',').map(|n| n.trim().parse::<i32>().map(|m| i64::from(fixed::from_milli(m))).map_err(|e| format!("rect number {n}: {e}"))).collect::<Result<_, _>>()?;
        if v.len() != 4 {
            return Err(format!("camp check rect holds {} numbers", v.len()));
        }
        rects.push((v[0], v[1], v[2], v[3]));
    }
    Ok(rects)
}

struct CampGeom {
    id: i64,
    stagger: u32,
    cx: i64,
    cy: i64,
    x1: i64,
    y1: i64,
    x2: i64,
    y2: i64,
}

fn camp_geoms() -> Result<Vec<CampGeom>, String> {
    let text = std::fs::read_to_string(crate::checks::pack_dir().join("camps_688g.lua")).map_err(|e| format!("camps_688g.lua: {e}"))?;
    let numbers = |line: &str, key: &str| -> Result<Vec<i64>, String> {
        let inner = line.split(key).nth(1).and_then(|r| r.split('}').next()).ok_or_else(|| format!("camps_688g.lua has no {key} row"))?;
        inner.split(',').map(|n| n.trim().parse::<i32>().map(|m| i64::from(fixed::from_milli(m))).map_err(|e| format!("camp number {n}: {e}"))).collect()
    };
    let mut geoms = Vec::new();
    for (stagger, line) in text.lines().filter(|l| l.contains("check = {")).enumerate() {
        let check = numbers(line, "check = {")?;
        let centre = numbers(line, "centre = {")?;
        let id = line
            .split("id = ")
            .nth(1)
            .and_then(|r| r.split(',').next())
            .and_then(|n| n.trim().parse::<i64>().ok())
            .ok_or("camps_688g.lua camp id")?;
        if check.len() != 4 || centre.len() != 2 {
            return Err(format!("camp {id} holds a {} number check rect and a {} number centre", check.len(), centre.len()));
        }
        geoms.push(CampGeom { id, stagger: stagger as u32, cx: centre[0], cy: centre[1], x1: check[0], y1: check[1], x2: check[2], y2: check[3] });
    }
    if geoms.is_empty() {
        return Err("camps_688g.lua holds no camp".into());
    }
    Ok(geoms)
}

fn camp_members(host: &ModHost, geoms: &[CampGeom], index: usize) -> Vec<usize> {
    let s = host.sim();
    (0..MAX_UNITS)
        .filter(|&k| {
            if !s.alive(k) || s.player(k) != 0xFF || !s.kind_name(k).is_some_and(|n| n.starts_with("neutral_")) {
                return false;
            }
            let (x, y) = s.position_fixed(k);
            let (x, y) = (i64::from(x), i64::from(y));
            let mut nearest = None;
            let mut best = i64::MAX;
            for (i, g) in geoms.iter().enumerate() {
                let dx = x - g.cx;
                let dy = y - g.cy;
                let d = dx * dx + dy * dy;
                if d < best {
                    best = d;
                    nearest = Some(i);
                }
            }
            nearest == Some(index)
        })
        .collect()
}

fn camp_blockers(host: &ModHost, camp: &CampGeom, members: &[usize]) -> Vec<usize> {
    let s = host.sim();
    members
        .iter()
        .copied()
        .filter(|&k| {
            let (x, y) = s.position_fixed(k);
            let (x, y) = (i64::from(x), i64::from(y));
            x >= camp.x1 && x <= camp.x2 && y >= camp.y1 && y <= camp.y2
        })
        .collect()
}

pub fn spawn_group_minute_refill(_image: &[u8]) -> Result<(), String> {
    let mut host = pack_run::start()?;
    run_past_first_camp_mark(&mut host)?;
    let geoms = camp_geoms()?;
    let mark = (scenario_rule("camp_respawn_s")? as u32) * sim::TICKS_PER_SECOND as u32;
    let spawners: Vec<(i64, i64)> = {
        let s = host.sim();
        (0..MAX_UNITS)
            .filter(|&k| s.alive(k) && s.kind_name(k).is_some_and(|n| n.starts_with("lane_spawner")))
            .map(|k| {
                let (x, y) = s.position_fixed(k);
                (i64::from(x), i64::from(y))
            })
            .collect()
    };
    let mut chosen: Option<(usize, Vec<usize>, Vec<usize>)> = None;
    let mut chosen_gap = i64::MIN;
    for (index, camp) in geoms.iter().enumerate() {
        let mates = camp_members(&host, &geoms, index);
        let blockers = camp_blockers(&host, camp, &mates);
        if mates.len() < 2 || blockers.len() < 2 {
            continue;
        }
        let gap = spawners.iter().map(|&(x, y)| (x - camp.cx) * (x - camp.cx) + (y - camp.cy) * (y - camp.cy)).min().unwrap_or(i64::MAX);
        if gap > chosen_gap {
            chosen_gap = gap;
            chosen = Some((index, mates, blockers));
        }
    }
    let (index, mates, blockers) = chosen.ok_or("no camp holds 2 members with 2 of them standing in its check rect after the first camp spawn")?;
    let camp = &geoms[index];
    let next_mark = |now: u32| -> u32 {
        let stagger = u64::from(camp.stagger);
        let m = u64::from(mark);
        ((u64::from(now) - stagger) / m * m + m + stagger) as u32
    };
    let victim = blockers[0];
    let blocked_at = host.sim().ticks();
    let blocked_due = next_mark(blocked_at);
    host.sim_mut().hurt(victim, fixed::from_int(10000));
    pack_run::advance(&mut host, blocked_due - blocked_at + 8)?;
    if host.sim().alive(victim) || camp_members(&host, &geoms, index).len() != mates.len() - 1 {
        return Err(format!("camp {} refilled at its minute mark tick {} while {} of its members still stood in its check rect", camp.id, blocked_due, blockers.len() - 1));
    }
    for &k in mates.iter().filter(|&&k| k != victim) {
        host.sim_mut().hurt(k, fixed::from_int(10000));
    }
    let cleared_at = host.sim().ticks();
    let refill_due = next_mark(cleared_at);
    pack_run::advance(&mut host, refill_due - cleared_at - 1)?;
    if !camp_members(&host, &geoms, index).is_empty() {
        return Err(format!("camp {} refilled before its minute mark tick {}", camp.id, refill_due));
    }
    pack_run::advance(&mut host, 4)?;
    let refilled = camp_members(&host, &geoms, index);
    if refilled.len() < 2 {
        return Err(format!("a cleared camp {} holds {} members after its minute mark tick {}, want at least 2", camp.id, refilled.len(), refill_due));
    }
    println!("camp minute refill ok camp={} members={} refilled={} blocked_tick={} refill_tick={}", camp.id, mates.len(), refilled.len(), blocked_due, refill_due);
    Ok(())
}

pub fn spawn_group_first_spawn(_image: &[u8]) -> Result<(), String> {
    let mut host = pack_run::start()?;
    pack_run::advance(&mut host, 3)?;
    let r_unit_boss = role("unit.boss")?;
    let boss = {
        let s = host.sim();
        let n = (0..MAX_UNITS).filter(|&u| s.alive(u) && s.player(u) == 0xFF && s.kind_name(u) == Some(r_unit_boss)).count();
        n
    };
    if camp_units(&host) != 0 || boss == 0 {
        return Err(format!("after the pack's first ticks {} camp units live and {boss} bosses, want 0 and at least 1", camp_units(&host)));
    }
    let first = (scenario_rule("camp_first_spawn_s")? as u32) * sim::TICKS_PER_SECOND as u32;
    pack_run::advance(&mut host, first - 7)?;
    if camp_units(&host) != 0 {
        return Err("a camp unit lived before the first camp spawn".into());
    }
    pack_run::advance(&mut host, 16)?;
    let n = camp_units(&host);
    if n == 0 {
        return Err("no camp unit lived after the first camp spawn".into());
    }
    println!("camp first spawn ok at_tick={first} camp_units={n}");
    Ok(())
}

pub fn spawn_group_single_pit(_image: &[u8]) -> Result<(), String> {
    let mut host = pack_run::start()?;
    let r_unit_boss = role("unit.boss")?;
    let r_unit_timed_boss = role("unit.timed_boss")?;
    let milli = |key: &str| -> Result<i64, String> { Ok(i64::from(fixed::from_milli(scenario_rule(key)?))) };
    let (cx, cy) = (milli("roshan_leash_cx_milli")?, milli("roshan_leash_cy_milli")?);
    let (hx, hy) = (milli("roshan_leash_hx_milli")?, milli("roshan_leash_hy_milli")?);
    let roshan = (0..MAX_UNITS)
        .find(|&u| {
            let s = host.sim();
            s.alive(u) && s.kind_name(u) == Some(r_unit_boss)
        })
        .ok_or("no Roshan at tick 0")?;
    let in_pit = |host: &ModHost| {
        let (x, y) = host.sim().position_fixed(roshan);
        (i64::from(x) - cx).abs() <= hx && (i64::from(y) - cy).abs() <= hy
    };
    let tormentors = |host: &ModHost| {
        let s = host.sim();
        let n = (0..MAX_UNITS).filter(|&u| s.alive(u) && s.kind_name(u) == Some(r_unit_timed_boss)).count();
        n
    };
    if !in_pit(&host) {
        return Err("Roshan does not start inside the pit rect".into());
    }
    let tps = sim::TICKS_PER_SECOND as u32;
    pack_run::advance(&mut host, 900 * tps + 16)?;
    if !host.sim().alive(roshan) {
        return Err("Roshan died with no hero near the pit".into());
    }
    if !in_pit(&host) {
        return Err("after 15:00 Roshan stands outside the pit rect".into());
    }
    if tormentors(&host) != 0 {
        return Err("a Tormentor lives after 15:00".into());
    }
    println!("camp single pit ok roshan_in_pit_after_ticks={}", 900 * tps + 16);
    Ok(())
}

pub fn spawn_group_ward_does_not_block(_image: &[u8]) -> Result<(), String> {
    let mut host = pack_run::start()?;
    run_past_first_camp_mark(&mut host)?;
    let inside = |host: &ModHost, r: (i64, i64, i64, i64)| -> Vec<usize> {
        let s = host.sim();
        let units: Vec<usize> = (0..MAX_UNITS)
            .filter(|&k| {
                let (x, y) = s.position_fixed(k);
                let (x, y) = (i64::from(x), i64::from(y));
                s.alive(k) && s.player(k) == 0xFF && s.kind_name(k).is_some_and(|n| n.starts_with("neutral_")) && x >= r.0 && x <= r.2 && y >= r.1 && y <= r.3
            })
            .collect();
        units
    };
    let rect = camp_check_rects()?
        .into_iter()
        .find(|&r| inside(&host, r).len() >= 2)
        .ok_or("no camp rect holds at least 2 neutral units after the first camp spawn")?;
    for k in inside(&host, rect) {
        host.sim_mut().hurt(k, fixed::from_int(10000));
    }
    pack_run::advance(&mut host, 4)?;
    let ward = host.sim().catalog().unit_index("observer_ward").ok_or("catalog has no observer_ward")? as u32;
    let (cx, cy) = (((rect.0 + rect.2) / 2) as i32, ((rect.1 + rect.3) / 2) as i32);
    let placed = (0..12)
        .find_map(|d| host.sim_mut().spawn_for(0, ward, cx + fixed::from_int(d), cy))
        .ok_or("no open cell for a ward inside the camp rect")?;
    let mark = (scenario_rule("camp_respawn_s")? as u32) * sim::TICKS_PER_SECOND as u32;
    pack_run::advance(&mut host, mark + 8)?;
    let refilled = inside(&host, rect).len();
    if !host.sim().alive(placed) {
        return Err("the ward died before the camp mark".into());
    }
    if refilled < 2 {
        return Err(format!("a ward standing in the camp rect held the camp back: {refilled} units after the mark, want at least 2"));
    }
    println!("camp ward does not block ok refilled={refilled}");
    Ok(())
}
