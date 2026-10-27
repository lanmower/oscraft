use crate::checks::{kind, pack_run, run_past_first_camp_mark, MAX_UNITS};
use mod_host::ModHost;
use sim::fixed;

const S_ATTACK_SPEED: usize = 12;
const S_SPEED_PCT: usize = 19;
const S_DAMAGE: usize = 0;
const DISARM: usize = 7;

fn neutral_arena() -> Result<(ModHost, usize), String> {
    let mut host = pack_run::start()?;
    run_past_first_camp_mark(&mut host)?;
    let u = (0..MAX_UNITS)
        .find(|&u| {
            let s = host.sim();
            s.alive(u) && s.player(u) == 0xFF && s.hp(u) > fixed::from_int(1) && s.hp(u) < fixed::from_int(5000)
        })
        .ok_or("moba image places no live neutral unit")?;
    Ok((host, u))
}

fn hero_at(host: &mut ModHost, player: u8, name: &str, x: i32, y: i32) -> Result<usize, String> {
    let k = kind(&host.sim(), name)?;
    for dy in [0, 1, -1, 2, -2, 3, -3] {
        for dx in [0, 1, -1, 2, -2] {
            if let Some(u) = host.sim_mut().spawn_attributed(player, k, x + fixed::from_int(dx), y + fixed::from_int(dy)) {
                return Ok(u);
            }
        }
    }
    Err(format!("spawn_attributed refused {name} around the point"))
}

fn learn(host: &mut ModHost, hero: usize, ability: &str) -> Result<usize, String> {
    let slot = host.sim().ability_slot(hero, ability).ok_or_else(|| format!("hero holds no {ability}"))?;
    host.sim_mut().rank_up(hero, slot).map_err(|r| format!("rank_up {ability}: {}", r.name()))?;
    Ok(slot)
}

fn advance(host: &mut ModHost, ticks: u32) -> Result<(), String> {
    pack_run::advance(host, ticks)
}

pub fn slot_napalm_stacks(_image: &[u8]) -> Result<(), String> {
    let (mut host, victim) = neutral_arena()?;
    let (vx, vy) = host.sim().position_fixed(victim);
    let hero = hero_at(&mut host, 0, "batrider", vx + fixed::from_int(3), vy)?;
    let slot = learn(&mut host, hero, "batrider_sticky_napalm")?;
    for want in 1..=2i32 {
        host.sim_mut().grant_energy(hero, 1000).map_err(|r| r.name().to_string())?;
        if !host.sim_mut().cast_ability(hero, slot, -1, vx, vy) {
            return Err(format!("cast refused: {}", host.sim().stats.last_refusal.name()));
        }
        advance(&mut host, 2)?;
        let (stacks, slow) = (host.sim().counter(victim, "napalm_stacks"), host.sim().stat(victim, S_SPEED_PCT));
        if stacks != i64::from(want) || slow != -3 * want {
            return Err(format!("cast {want}: stacks {stacks} slow {slow}, want {want} and {}", -3 * want));
        }
        advance(&mut host, 60)?;
    }
    advance(&mut host, 140)?;
    if host.sim().stat(victim, S_SPEED_PCT) != 0 {
        return Err(format!("the slow outlived the 8 s stack: {}", host.sim().stat(victim, S_SPEED_PCT)));
    }
    Ok(())
}

pub fn slot_untouchable_attacker(_image: &[u8]) -> Result<(), String> {
    let (mut host, anchor) = neutral_arena()?;
    let (ax, ay) = host.sim().position_fixed(anchor);
    let holder = hero_at(&mut host, 1, "enchantress", ax + fixed::from_int(3), ay)?;
    let attacker = hero_at(&mut host, 0, "batrider", ax + fixed::from_int(3), ay + fixed::from_int(1))?;
    learn(&mut host, holder, "enchantress_untouchable")?;
    for _ in 0..400 {
        if host.sim().counter(attacker, "untouchable_amount") != 0 {
            break;
        }
        if host.sim_mut().order_attack(attacker, holder).is_err() {
            advance(&mut host, 1)?;
            continue;
        }
        advance(&mut host, 4)?;
    }
    let got = host.sim().counter(attacker, "untouchable_amount");
    if got != -40 {
        return Err(format!("attacker attack speed debuff {got} want -40"));
    }
    host.sim_mut().grant_status(attacker, DISARM, 1_000_000);
    let held = host.sim().stat(attacker, S_ATTACK_SPEED);
    advance(&mut host, 100)?;
    if host.sim().counter(attacker, "untouchable_amount") != 0 {
        return Err("the debuff outlived 4 s".into());
    }
    let freed = host.sim().stat(attacker, S_ATTACK_SPEED);
    if freed - held != 40 {
        return Err(format!("attack speed rose {} after the debuff ended, want 40", freed - held));
    }
    Ok(())
}

pub fn slot_wall_fissure(_image: &[u8]) -> Result<(), String> {
    let (mut host, anchor) = neutral_arena()?;
    let (ax, ay) = host.sim().position_fixed(anchor);
    let hero = hero_at(&mut host, 0, "earthshaker", ax, ay)?;
    let slot = learn(&mut host, hero, "earthshaker_fissure")?;
    host.sim_mut().grant_energy(hero, 1000).map_err(|r| r.name().to_string())?;
    let (hx, hy) = host.sim().position_fixed(hero);
    let before = host.sim().obstacles().iter().filter(|o| o.alive && o.temp).count();
    if !host.sim_mut().cast_ability(hero, slot, -1, hx + fixed::from_int(8), hy) {
        return Err(format!("cast refused: {}", host.sim().stats.last_refusal.name()));
    }
    advance(&mut host, 4)?;
    let made = host.sim().obstacles().iter().filter(|o| o.alive && o.temp).count() - before.min(host.sim().obstacles().iter().filter(|o| o.alive && o.temp).count());
    if made < 10 {
        return Err(format!("the fissure made {made} blocker cells, want at least 10"));
    }
    advance(&mut host, 140)?;
    let left = host.sim().obstacles().iter().filter(|o| o.alive && o.temp).count();
    if left > before {
        return Err(format!("{} blocker cells outlived the 8 s fissure", left - before));
    }
    Ok(())
}

pub fn slot_percent_aura(_image: &[u8]) -> Result<(), String> {
    let (mut host, anchor) = neutral_arena()?;
    let (ax, ay) = host.sim().position_fixed(anchor);
    let (leader_kind, ally_kind) = (kind(&host.sim(), "neutral_alpha_wolf")?, kind(&host.sim(), "neutral_giant_wolf")?);
    let spawn = |host: &mut ModHost, k: u32| -> Result<usize, String> {
        for dx in 2..10 {
            for dy in -4..5 {
                if let Some(u) = host.sim_mut().spawn_for(1, k, ax + fixed::from_int(dx), ay + fixed::from_int(dy)) {
                    return Ok(u);
                }
            }
        }
        Err("spawn_for refused around the arena".to_string())
    };
    let leader = spawn(&mut host, leader_kind)?;
    let ally = spawn(&mut host, ally_kind)?;
    let _ = leader;
    advance(&mut host, 24)?;
    let got = host.sim().stat(ally, S_DAMAGE);
    if got != 3 {
        return Err(format!("the pack leader aura gave the ally {got} damage, want 3 (15 percent of 22)"));
    }
    Ok(())
}
