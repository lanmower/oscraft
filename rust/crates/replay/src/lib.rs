use std::collections::VecDeque;
use std::io::Write;
use std::str::{FromStr, SplitWhitespace};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::time::Duration;

use sim::{carry_from, AbilitySpec, Effect, ItemSpec, ORDER_HOLD, Refusal, Sim, StateError, UnitSpec, UpgradeSpec, ItemVerdict, ALL_REFUSALS, NO_TAKE, VERDICT_TAKES};

const TICKS_PER_SECOND: u32 = 16;

pub const SNAP_EVERY: u32 = 64;
pub const MAX_SPEED: u32 = 64;

#[derive(Debug, PartialEq)]
pub enum ReplayError {
    LogIncomplete { dropped: u64 },
    MissingCheckpoint { tick: u32 },
    Ahead { tick: u32, at: u32 },
    OrderRefused { tick: u32, unit: i32, refusal: Refusal },
    HashMismatch { tick: u32, expected: u64, actual: u64 },
    TraceGap { tick: u32 },
    TraceLength { want: u32, got: u32 },
    RecordBeyondTrace { tick: u32 },
    BadSpeed { speed: u32 },
    OutOfTrace { tick: u32, base: u32, end: u32 },
    Restore { tick: u32, cause: StateError },
    Parse { line: usize, why: String },
    Init { expected: u64, actual: u64 },
    NotAtStart { tick: u32 },
    NoSegmentData { tick: u32 },
}

type Record = (u32, i32, u8, i32, i32);

fn order_records(source: &Sim) -> Result<Vec<Record>, ReplayError> {
    let dropped = source.order_log_dropped();
    if dropped > 0 {
        return Err(ReplayError::LogIncomplete { dropped });
    }
    Ok((0..source.order_log_len())
        .filter_map(|k| source.order_record(k))
        .collect())
}

fn apply(sim: &mut Sim, record: Record) -> Result<(), ReplayError> {
    let (tick, unit, kind, x, y) = record;
    sim.issue_order_kind(unit as usize, kind, x, y)
        .map_err(|refusal| ReplayError::OrderRefused { tick, unit, refusal })
}

fn step_tick(sim: &mut Sim, records: &[Record]) -> Result<u64, ReplayError> {
    let t = sim.ticks();
    let mut k = records.partition_point(|r| r.0 < t);
    while k < records.len() && records[k].0 == t {
        apply(sim, records[k])?;
        k += 1;
    }
    let hash = sim.state_hash();
    sim.tick();
    Ok(hash)
}

pub fn play(source: &Sim, fresh: &mut Sim, checkpoints: &[(u32, u64)]) -> Result<u32, ReplayError> {
    let records = order_records(source)?;
    let mut k = 0;
    while k < records.len() {
        let tick = records[k].0;
        let expected = checkpoints
            .binary_search_by_key(&tick, |c| c.0)
            .map(|i| checkpoints[i].1)
            .map_err(|_| ReplayError::MissingCheckpoint { tick })?;
        if fresh.ticks() > tick {
            return Err(ReplayError::Ahead {
                tick,
                at: fresh.ticks(),
            });
        }
        while fresh.ticks() < tick {
            fresh.tick();
        }
        while k < records.len() && records[k].0 == tick {
            apply(fresh, records[k])?;
            k += 1;
        }
        let actual = fresh.state_hash();
        if actual != expected {
            return Err(ReplayError::HashMismatch {
                tick,
                expected,
                actual,
            });
        }
    }
    Ok(fresh.ticks())
}

pub struct Playback {
    sim: Sim,
    base: u32,
    hashes: Vec<u64>,
    records: Vec<Record>,
    snapshots: Vec<(u32, Vec<u8>)>,
    speed: u32,
    paused: bool,
    carry: f64,
}

impl Playback {
    pub fn new(source: &Sim, mut fresh: Sim, trace: &[(u32, u64)]) -> Result<Playback, ReplayError> {
        let base = fresh.ticks();
        let end = source.ticks();
        if base > end {
            return Err(ReplayError::Ahead { tick: end, at: base });
        }
        let records = order_records(source)?;
        if let Some(first) = records.first() {
            if first.0 < base {
                return Err(ReplayError::Ahead {
                    tick: first.0,
                    at: base,
                });
            }
        }
        if let Some(last) = records.last() {
            if last.0 >= end {
                return Err(ReplayError::RecordBeyondTrace { tick: last.0 });
            }
        }
        let start = fresh.save_state();
        let hashes = if trace.is_empty() {
            let mut hashes = Vec::with_capacity((end - base) as usize);
            while fresh.ticks() < end {
                hashes.push(step_tick(&mut fresh, &records)?);
            }
            fresh
                .load_state(&start)
                .map_err(|cause| ReplayError::Restore { tick: base, cause })?;
            hashes
        } else {
            for (i, &(tick, _)) in trace.iter().enumerate() {
                let want = base + i as u32;
                if tick != want {
                    return Err(ReplayError::TraceGap { tick: want });
                }
            }
            if base + trace.len() as u32 != end {
                return Err(ReplayError::TraceLength {
                    want: end - base,
                    got: trace.len() as u32,
                });
            }
            trace.iter().map(|&(_, h)| h).collect()
        };
        Ok(Playback {
            sim: fresh,
            base,
            hashes,
            records,
            snapshots: vec![(base, start)],
            speed: 1,
            paused: false,
            carry: 0.0,
        })
    }

    pub fn tick(&self) -> u32 {
        self.sim.ticks()
    }

    pub fn end(&self) -> u32 {
        self.base + self.hashes.len() as u32
    }

    pub fn at_end(&self) -> bool {
        self.tick() >= self.end()
    }

    pub fn hash(&self) -> u64 {
        self.sim.state_hash()
    }

    pub fn pause(&mut self) {
        self.paused = true;
    }

    pub fn resume(&mut self) {
        self.paused = false;
    }

    pub fn set_speed(&mut self, speed: u32) -> Result<(), ReplayError> {
        if speed == 0 || speed > MAX_SPEED {
            return Err(ReplayError::BadSpeed { speed });
        }
        self.speed = speed;
        Ok(())
    }

    pub fn advance(&mut self, elapsed: Duration) -> Result<u32, ReplayError> {
        if self.paused || self.at_end() {
            return Ok(self.tick());
        }
        self.carry +=
            elapsed.as_secs_f64() * f64::from(TICKS_PER_SECOND) * f64::from(self.speed);
        let due = self.carry.floor() as u64;
        self.carry -= due as f64;
        let mut stepped = 0;
        while stepped < due && !self.at_end() {
            self.step()?;
            stepped += 1;
        }
        Ok(self.tick())
    }

    pub fn seek(&mut self, tick: u32) -> Result<u32, ReplayError> {
        if tick < self.base || tick > self.end() {
            return Err(ReplayError::OutOfTrace {
                tick,
                base: self.base,
                end: self.end(),
            });
        }
        if tick < self.sim.ticks() {
            let i = self.snapshots.partition_point(|s| s.0 <= tick) - 1;
            let at = self.snapshots[i].0;
            self.sim
                .load_state(&self.snapshots[i].1)
                .map_err(|cause| ReplayError::Restore { tick: at, cause })?;
            self.snapshots.truncate(i + 1);
        }
        while self.sim.ticks() < tick {
            self.step()?;
        }
        Ok(self.tick())
    }

    fn step(&mut self) -> Result<(), ReplayError> {
        let t = self.sim.ticks();
        let last_snapshot = self.snapshots[self.snapshots.len() - 1].0;
        if t - last_snapshot >= SNAP_EVERY {
            self.snapshots.push((t, self.sim.save_state()));
        }
        let expected = self.hashes[(t - self.base) as usize];
        let actual = step_tick(&mut self.sim, &self.records)?;
        if actual != expected {
            return Err(ReplayError::HashMismatch {
                tick: t,
                expected,
                actual,
            });
        }
        Ok(())
    }
}

pub const FORMAT_HEADER: &str = "oscraft-replay 3";
const RETIRED_FLAG: u8 = 0;
const FORMAT_HEADER_V1: &str = "oscraft-replay 1";
const FORMAT_HEADER_V2: &str = "oscraft-replay 2";
pub const CHECK_EVERY: u32 = 64;
pub const MAX_KEPT: usize = 4_000_000;

#[derive(Clone)]
pub enum Command {
    Order { unit: usize, x: i32, y: i32 },
    Hold { unit: usize },
    Kind { unit: usize, kind: u8, x: i32, y: i32 },
    OrderAll { player: u8, x: i32, y: i32 },
    Attack { unit: usize, target: usize },
    Cast { unit: usize, slot: usize, target: i32, x: i32, y: i32 },
    SkillUp { unit: usize, slot: usize },
    UseItem { unit: usize, slot: usize, target: i32, x: i32, y: i32 },
    Buy { unit: usize, item: usize },
    Sell { unit: usize, slot: usize },
    Unstash { unit: usize, slot: usize },
    Trade { verdict: ItemVerdict },
    PackStash { unit: usize },
    SaveOrder { unit: usize, x: i32, y: i32 },
    RestoreOrder { unit: usize },
    Fog { on: bool },
    ClassOverride { unit: u32, class: u8 },
    HoldOverride { unit: u32, hold: u8 },
    SetRespawn { unit: u32, delay_s: u32 },
    Invulnerable { unit: u32, ticks: u32 },
    Reduction { unit: u32, pct: u8 },
    Healing { unit: u32, hp_per_s: i32 },
    DeclareSide { side: u8 },
    AttackMove { unit: u32, x: i32, y: i32 },
    Cadence { paths: bool, vision: bool },
    Spawn { handle: u32, index: u32, x: i32, y: i32, owner: u8, route: i32 },
    Damage { caster: usize, target: usize, dmg: i32 },
    Buff { unit: usize, kind: i32, ticks: i32 },
    SetOwner { unit: usize, player: u8 },
    SetTeam { player: u8, team: u8 },
    SetAlliance { a: u8, b: u8, kind: usize },
    AddPlayer { player: u8, controller: u8 },
    RemovePlayer { player: u8 },
    DefineAbility { name: String, cooldown: i32, energy: i32, target: u8, filter: u16, effects: Vec<Effect> },
    DefineUnit { spec: UnitSpec },
    DefineItem { spec: ItemSpec },
    DefineUpgrade { spec: UpgradeSpec },
    GrantUpgrade { player: u8, name: String },
    GiveAbility { unit: usize, slot: usize, name: String },
    Remove { unit: usize },
    GiveItem { unit: usize, item: usize, count: u32 },
    AddGold { player: u8, amount: i32 },
    AddUnitGold { unit: usize, amount: i32 },
    Victory { player: u8 },
    Defeat { player: u8 },
    Ui { body: String },
    Respawn { unit: usize, kind: u32, player: u8, x: i32, y: i32 },
}

fn num<T: FromStr>(it: &mut SplitWhitespace) -> Option<T> {
    it.next()?.parse().ok()
}

fn num_or<T: FromStr>(it: &mut SplitWhitespace, default: T) -> Option<T> {
    match it.next() {
        None => Some(default),
        Some(t) => t.parse().ok(),
    }
}

fn text(it: &mut SplitWhitespace) -> Option<String> {
    let hex = it.next()?.strip_prefix('x')?;
    if hex.len() % 2 != 0 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let bytes = (0..hex.len()).step_by(2).map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok()).collect::<Option<Vec<u8>>>()?;
    String::from_utf8(bytes).ok()
}

fn texts(it: &mut SplitWhitespace) -> Option<Vec<String>> {
    let n: usize = num(it)?;
    (0..n).map(|_| text(it)).collect()
}

fn effect_of(it: &mut SplitWhitespace) -> Option<Effect> {
    Some(Effect { kind: num(it)?, p0: num(it)?, p1: num(it)?, p2: num(it)?, p3: num(it)? })
}

fn effects_of(it: &mut SplitWhitespace) -> Option<Vec<Effect>> {
    let n: usize = num(it)?;
    (0..n).map(|_| effect_of(it)).collect()
}

fn unit_spec_of(it: &mut SplitWhitespace) -> Option<UnitSpec> {
    let name = text(it)?;
    let model = text(it)?;
    let health = num(it)?;
    let speed = num(it)?;
    let radius = num(it)?;
    let armor = num(it)?;
    let shield = num(it)?;
    let sight = num(it)?;
    let filter = num(it)?;
    let cost = num(it)?;
    let bounty = num(it)?;
    let weapon: Option<(i32, i32, i32)> = if num::<u8>(it)? == 0 { None } else { Some((num(it)?, num(it)?, num(it)?)) };
    let abilities = texts(it)?;
    Some(UnitSpec { name, model, health, speed, radius, armor, shield, sight, filter, cost, bounty, weapon, abilities })
}

fn item_spec_of(it: &mut SplitWhitespace) -> Option<ItemSpec> {
    let name = text(it)?;
    let cost = num(it)?;
    let armor = num(it)?;
    let damage = num(it)?;
    let health = num(it)?;
    let ability: Option<String> = if num::<u8>(it)? == 0 { None } else { Some(text(it)?) };
    Some(ItemSpec { name, cost, armor, damage, health, ability })
}

fn upgrade_spec_of(it: &mut SplitWhitespace) -> Option<UpgradeSpec> {
    let name = text(it)?;
    let building = text(it)?;
    let time = num(it)?;
    let cost = num(it)?;
    let gas = num(it)?;
    let effect = effect_of(it)?;
    let filter = num(it)?;
    let requires = texts(it)?;
    let unlocks = texts(it)?;
    Some(UpgradeSpec { name, building, time, cost, gas, effect, filter, requires, unlocks })
}

fn put<T: std::fmt::Display>(out: &mut String, v: T) {
    out.push(' ');
    out.push_str(&v.to_string());
}

fn put_item_verdict(out: &mut String, v: &ItemVerdict) {
    match *v {
        ItemVerdict::Acquire { unit, item, allow, deny, debit, takes } => {
            put(out, "b");
            put(out, unit);
            put(out, item);
            put(out, u8::from(allow));
            put(out, ALL_REFUSALS.iter().position(|&r| r == deny).unwrap_or(0));
            put(out, debit);
            for t in takes {
                put(out, t);
            }
        }
        ItemVerdict::Release { unit, slot, allow, deny, credit } => {
            put(out, "s");
            put(out, unit);
            put(out, slot);
            put(out, u8::from(allow));
            put(out, ALL_REFUSALS.iter().position(|&r| r == deny).unwrap_or(0));
            put(out, credit);
        }
        ItemVerdict::TakeOut { unit, slot, allow, deny } => {
            put(out, "u");
            put(out, unit);
            put(out, slot);
            put(out, u8::from(allow));
            put(out, ALL_REFUSALS.iter().position(|&r| r == deny).unwrap_or(0));
        }
    }
}

fn decode_item_verdict(it: &mut SplitWhitespace) -> Option<ItemVerdict> {
    let kind: String = num(it)?;
    let (unit, slot_or_item): (usize, usize) = (num(it)?, num(it)?);
    let allow = num::<u8>(it)? != 0;
    let deny = *ALL_REFUSALS.get(num::<usize>(it)?)?;
    Some(match kind.as_str() {
        "b" => {
            let debit = num(it)?;
            let mut takes = [NO_TAKE; VERDICT_TAKES];
            for t in takes.iter_mut() {
                *t = num(it)?;
            }
            ItemVerdict::Acquire { unit, item: slot_or_item, allow, deny, debit, takes }
        }
        "s" => ItemVerdict::Release { unit, slot: slot_or_item, allow, deny, credit: num(it)? },
        "u" => ItemVerdict::TakeOut { unit, slot: slot_or_item, allow, deny },
        _ => return None,
    })
}

fn put_text(out: &mut String, s: &str) {
    out.push_str(" x");
    for b in s.bytes() {
        out.push_str(&format!("{b:02x}"));
    }
}

fn put_texts(out: &mut String, list: &[String]) {
    put(out, list.len());
    for s in list {
        put_text(out, s);
    }
}

fn put_effect(out: &mut String, e: &Effect) {
    put(out, e.kind);
    put(out, e.p0);
    put(out, e.p1);
    put(out, e.p2);
    put(out, e.p3);
}

fn put_effects(out: &mut String, list: &[Effect]) {
    put(out, list.len());
    for e in list {
        put_effect(out, e);
    }
}

fn put_unit_spec(out: &mut String, s: &UnitSpec) {
    put_text(out, &s.name);
    put_text(out, &s.model);
    for v in [s.health, s.speed, s.radius, s.armor, s.shield, s.sight] {
        put(out, v);
    }
    put(out, s.filter);
    put(out, s.cost);
    put(out, s.bounty);
    match s.weapon {
        None => put(out, 0),
        Some((a, b, c)) => {
            put(out, 1);
            put(out, a);
            put(out, b);
            put(out, c);
        }
    }
    put_texts(out, &s.abilities);
}

fn put_item_spec(out: &mut String, s: &ItemSpec) {
    put_text(out, &s.name);
    for v in [s.cost, s.armor, s.damage, s.health] {
        put(out, v);
    }
    match &s.ability {
        None => put(out, 0),
        Some(a) => {
            put(out, 1);
            put_text(out, a);
        }
    }
}

fn put_upgrade_spec(out: &mut String, s: &UpgradeSpec) {
    put_text(out, &s.name);
    put_text(out, &s.building);
    for v in [s.time, s.cost, s.gas] {
        put(out, v);
    }
    put_effect(out, &s.effect);
    put(out, s.filter);
    put_texts(out, &s.requires);
    put_texts(out, &s.unlocks);
}

impl Command {
    pub fn apply(&self, sim: &mut Sim) -> bool {
        match self {
            Command::Order { unit, x, y } => {
                sim.order(*unit, *x, *y);
                true
            }
            Command::Hold { unit } => sim.issue_order_kind(*unit, ORDER_HOLD, 0, 0).is_ok(),
            Command::Kind { unit, kind, x, y } => sim.issue_order_kind(*unit, *kind, *x, *y).is_ok(),
            Command::OrderAll { player, x, y } => {
                sim.order_all(*player, *x, *y);
                true
            }
            Command::Attack { unit, target } => sim.order_attack_or_approach(*unit, *target).is_ok(),
            Command::Cast { unit, slot, target, x, y } => sim.cast_ability(*unit, *slot, *target, *x, *y),
            Command::SkillUp { unit, slot } => sim.rank_up(*unit, *slot).is_ok(),
            Command::UseItem { unit, slot, target, x, y } => sim.use_item(*unit, *slot, *target, *x, *y),
            Command::Buy { unit, item } => sim.buy_item(*unit, *item),
            Command::Sell { unit, slot } => sim.sell_item(*unit, *slot),
            Command::Unstash { unit, slot } => sim.unstash(*unit, *slot),
            Command::Trade { verdict } => sim.apply_verdict(verdict),
            Command::PackStash { unit } => {
                sim.pack_stash(*unit);
                true
            }
            Command::SaveOrder { unit, x, y } => sim.save_order(*unit, *x, *y),
            Command::RestoreOrder { unit } => sim.restore_order(*unit),
            Command::Fog { on } => {
                sim.set_fog(*on);
                true
            }
            Command::ClassOverride { unit, class } => {
                let class = if *class == 255 { None } else { Some(*class) };
                sim.set_class_override(*unit as usize, class).is_ok()
            }
            Command::HoldOverride { unit, hold } => {
                let hold = match *hold {
                    255 => None,
                    h => Some(h == 1),
                };
                sim.set_hold_override(*unit as usize, hold).is_ok()
            }
            Command::SetRespawn { unit, delay_s } => {
                let delay = (*delay_s != u32::MAX).then_some(*delay_s);
                sim.set_respawn(*unit as usize, delay).is_ok()
            }
            Command::Invulnerable { unit, ticks } => sim.set_invulnerable(*unit as usize, *ticks).is_ok(),
            Command::Reduction { unit, pct } => sim.set_damage_reduction(*unit as usize, *pct).is_ok(),
            Command::Healing { unit, hp_per_s } => sim.set_healing(*unit as usize, *hp_per_s).is_ok(),
            Command::DeclareSide { side } => {
                sim.declare_winner(*side);
                true
            }
            Command::AttackMove { unit, x, y } => sim.issue_order_kind(*unit as usize, sim::ORDER_ATTACK_MOVE, *x, *y).is_ok(),
            Command::Cadence { .. } => true,
            Command::Spawn { index, x, y, owner, route, .. } => sim.spawn_mod_unit(*owner, *index, *x, *y, *route).is_some(),
            Command::Damage { caster, target, dmg } => {
                let player = sim.player(*caster);
                sim.damage_target(*caster as i32, player, *dmg, *target as i32);
                true
            }
            Command::Buff { unit, kind, ticks } => sim.apply_buff(*unit, *kind, *ticks),
            Command::SetOwner { unit, player } => sim.set_owner(*unit, *player).is_ok(),
            Command::SetTeam { player, team } => sim.set_team(*player, *team).is_ok(),
            Command::SetAlliance { a, b, kind } => sim.set_relation(*a, *b, *kind).is_ok(),
            Command::AddPlayer { player, controller } => sim.add_player(*player, *controller).is_ok(),
            Command::RemovePlayer { player } => sim.remove_player(*player).is_ok(),
            Command::DefineAbility { name, cooldown, energy, target, filter, effects } => sim
                .define_ability(AbilitySpec {
                    name: name.as_str(),
                    cooldown: *cooldown,
                    energy: *energy,
                    target: *target,
                    filter: *filter,
                    effects: effects.clone(),
                })
                .is_ok(),
            Command::DefineUnit { spec } => sim.define_unit(spec).is_ok(),
            Command::DefineItem { spec } => sim.define_item(spec).is_ok(),
            Command::DefineUpgrade { spec } => sim.define_upgrade(spec).is_ok(),
            Command::GrantUpgrade { player, name } => match sim.research_index(name) {
                Some(r) => sim.set_research_bit(*player, r, true),
                None => false,
            },
            Command::GiveAbility { unit, slot, name } => sim.give_ability(*unit, *slot, name),
            Command::Remove { unit } => sim.remove_unit(*unit),
            Command::GiveItem { unit, item, count } => sim.grant_item(*unit, *item, *count),
            Command::AddGold { player, amount } => {
                sim.add_resources(*player, *amount, 0);
                true
            }
            Command::AddUnitGold { unit, amount } => sim.add_unit_gold(*unit, *amount),
            Command::Victory { player } => {
                sim.declare_victory(*player);
                true
            }
            Command::Defeat { player } => {
                sim.declare_defeat(*player);
                true
            }
            Command::Ui { .. } => true,
            Command::Respawn { unit, kind, player, x, y } => sim.spawn_at(*unit, *kind, *player, *x, *y),
        }
    }

    fn encode(&self) -> String {
        let mut o = String::new();
        let code = match self {
            Command::Order { unit, x, y } => {
                put(&mut o, unit);
                put(&mut o, x);
                put(&mut o, y);
                "o"
            }
            Command::Hold { unit } => {
                put(&mut o, unit);
                "h"
            }
            Command::Kind { unit, kind, x, y } => {
                put(&mut o, unit);
                put(&mut o, kind);
                put(&mut o, x);
                put(&mut o, y);
                "ok"
            }
            Command::OrderAll { player, x, y } => {
                put(&mut o, player);
                put(&mut o, x);
                put(&mut o, y);
                "oa"
            }
            Command::Attack { unit, target } => {
                put(&mut o, unit);
                put(&mut o, target);
                "a"
            }
            Command::Cast { unit, slot, target, x, y } | Command::UseItem { unit, slot, target, x, y } => {
                put(&mut o, unit);
                put(&mut o, slot);
                put(&mut o, target);
                put(&mut o, x);
                put(&mut o, y);
                if matches!(self, Command::Cast { .. }) {
                    "c"
                } else {
                    "u"
                }
            }
            Command::SkillUp { unit, slot } => {
                put(&mut o, unit);
                put(&mut o, slot);
                "s"
            }
            Command::Buy { unit, item } => {
                put(&mut o, unit);
                put(&mut o, item);
                "b"
            }
            Command::Sell { unit, slot } => {
                put(&mut o, unit);
                put(&mut o, slot);
                "l"
            }
            Command::Unstash { unit, slot } => {
                put(&mut o, unit);
                put(&mut o, slot);
                "t"
            }
            Command::Trade { verdict } => {
                put_item_verdict(&mut o, verdict);
                "tr"
            }
            Command::PackStash { unit } => {
                put(&mut o, unit);
                "pk"
            }
            Command::SaveOrder { unit, x, y } => {
                put(&mut o, unit);
                put(&mut o, x);
                put(&mut o, y);
                "sv"
            }
            Command::RestoreOrder { unit } => {
                put(&mut o, unit);
                "ro"
            }
            Command::Fog { on } => {
                put(&mut o, u8::from(*on));
                "f"
            }
            Command::ClassOverride { unit, class } => {
                put(&mut o, *unit);
                put(&mut o, *class);
                "co"
            }
            Command::HoldOverride { unit, hold } => {
                put(&mut o, *unit);
                put(&mut o, *hold);
                "ho"
            }
            Command::SetRespawn { unit, delay_s } => {
                put(&mut o, *unit);
                put(&mut o, *delay_s);
                "rs"
            }
            Command::Invulnerable { unit, ticks } => {
                put(&mut o, *unit);
                put(&mut o, *ticks);
                "iv"
            }
            Command::Reduction { unit, pct } => {
                put(&mut o, *unit);
                put(&mut o, *pct);
                "rd"
            }
            Command::Healing { unit, hp_per_s } => {
                put(&mut o, *unit);
                put(&mut o, *hp_per_s);
                "hl"
            }
            Command::DeclareSide { side } => {
                put(&mut o, *side);
                "ds"
            }
            Command::AttackMove { unit, x, y } => {
                put(&mut o, *unit);
                put(&mut o, *x);
                put(&mut o, *y);
                "am"
            }
            Command::Cadence { paths, vision } => {
                put(&mut o, u8::from(*paths));
                put(&mut o, u8::from(*vision));
                "k"
            }
            Command::Spawn { handle, index, x, y, owner, route } => {
                put(&mut o, handle);
                put(&mut o, index);
                put(&mut o, x);
                put(&mut o, y);
                put(&mut o, owner);
                put(&mut o, route);
                "sp"
            }
            Command::Damage { caster, target, dmg } => {
                put(&mut o, caster);
                put(&mut o, target);
                put(&mut o, dmg);
                "dm"
            }
            Command::Buff { unit, kind, ticks } => {
                put(&mut o, unit);
                put(&mut o, kind);
                put(&mut o, ticks);
                "bf"
            }
            Command::SetOwner { unit, player } => {
                put(&mut o, unit);
                put(&mut o, player);
                "so"
            }
            Command::SetTeam { player, team } => {
                put(&mut o, player);
                put(&mut o, team);
                "st"
            }
            Command::SetAlliance { a, b, kind } => {
                put(&mut o, a);
                put(&mut o, b);
                put(&mut o, kind);
                "sa"
            }
            Command::AddPlayer { player, controller } => {
                put(&mut o, player);
                put(&mut o, controller);
                "ap"
            }
            Command::RemovePlayer { player } => {
                put(&mut o, player);
                "rp"
            }
            Command::DefineAbility { name, cooldown, energy, target, filter, effects } => {
                put_text(&mut o, name);
                put(&mut o, cooldown);
                put(&mut o, energy);
                put(&mut o, target);
                put(&mut o, filter);
                put_effects(&mut o, effects);
                "da"
            }
            Command::DefineUnit { spec } => {
                put_unit_spec(&mut o, spec);
                "du"
            }
            Command::DefineItem { spec } => {
                put_item_spec(&mut o, spec);
                "di"
            }
            Command::DefineUpgrade { spec } => {
                put_upgrade_spec(&mut o, spec);
                "dr"
            }
            Command::GrantUpgrade { player, name } => {
                put(&mut o, player);
                put_text(&mut o, name);
                "gu"
            }
            Command::GiveAbility { unit, slot, name } => {
                put(&mut o, unit);
                put(&mut o, slot);
                put_text(&mut o, name);
                "ga"
            }
            Command::Remove { unit } => {
                put(&mut o, unit);
                "rm"
            }
            Command::GiveItem { unit, item, count } => {
                put(&mut o, unit);
                put(&mut o, item);
                put(&mut o, count);
                "gi"
            }
            Command::AddGold { player, amount } => {
                put(&mut o, player);
                put(&mut o, amount);
                "ag"
            }
            Command::AddUnitGold { unit, amount } => {
                put(&mut o, unit);
                put(&mut o, amount);
                "au"
            }
            Command::Victory { player } => {
                put(&mut o, player);
                "vw"
            }
            Command::Defeat { player } => {
                put(&mut o, player);
                "df"
            }
            Command::Ui { body } => {
                put_text(&mut o, body);
                "ui"
            }
            Command::Respawn { unit, kind, player, x, y } => {
                put(&mut o, unit);
                put(&mut o, kind);
                put(&mut o, player);
                put(&mut o, x);
                put(&mut o, y);
                "rn"
            }
        };
        format!("{code}{o}")
    }

    fn decode(code: &str, it: &mut SplitWhitespace) -> Option<Command> {
        Some(match code {
            "o" => Command::Order { unit: num(it)?, x: num(it)?, y: num(it)? },
            "h" => Command::Hold { unit: num(it)? },
            "ok" => Command::Kind { unit: num(it)?, kind: num(it)?, x: num(it)?, y: num(it)? },
            "oa" => Command::OrderAll { player: num(it)?, x: num(it)?, y: num(it)? },
            "a" => Command::Attack { unit: num(it)?, target: num(it)? },
            "c" => Command::Cast { unit: num(it)?, slot: num(it)?, target: num(it)?, x: num(it)?, y: num(it)? },
            "s" => Command::SkillUp { unit: num(it)?, slot: num(it)? },
            "u" => Command::UseItem { unit: num(it)?, slot: num(it)?, target: num(it)?, x: num(it)?, y: num(it)? },
            "b" => Command::Buy { unit: num(it)?, item: num(it)? },
            "l" => Command::Sell { unit: num(it)?, slot: num(it)? },
            "t" => Command::Unstash { unit: num(it)?, slot: num(it)? },
            "tr" => Command::Trade { verdict: decode_item_verdict(it)? },
            "pk" => Command::PackStash { unit: num(it)? },
            "sv" => Command::SaveOrder { unit: num(it)?, x: num(it)?, y: num(it)? },
            "ro" => Command::RestoreOrder { unit: num(it)? },
            "f" => Command::Fog { on: num::<u8>(it)? != 0 },
            "co" => Command::ClassOverride { unit: num(it)?, class: num(it)? },
            "ho" => Command::HoldOverride { unit: num(it)?, hold: num(it)? },
            "rs" => Command::SetRespawn { unit: num(it)?, delay_s: num(it)? },
            "iv" => Command::Invulnerable { unit: num(it)?, ticks: num(it)? },
            "rd" => Command::Reduction { unit: num(it)?, pct: num(it)? },
            "hl" => Command::Healing { unit: num(it)?, hp_per_s: num(it)? },
            "ds" => Command::DeclareSide { side: num(it)? },
            "am" => Command::AttackMove { unit: num(it)?, x: num(it)?, y: num(it)? },
            "k" => Command::Cadence { paths: num::<u8>(it)? != 0, vision: num::<u8>(it)? != 0 },
            "sp" => Command::Spawn { handle: num(it)?, index: num(it)?, x: num(it)?, y: num(it)?, owner: num_or(it, 0)?, route: num_or(it, 0)? },
            "dm" => Command::Damage { caster: num(it)?, target: num(it)?, dmg: num(it)? },
            "bf" => Command::Buff { unit: num(it)?, kind: num(it)?, ticks: num(it)? },
            "so" => Command::SetOwner { unit: num(it)?, player: num(it)? },
            "st" => Command::SetTeam { player: num(it)?, team: num(it)? },
            "sa" => Command::SetAlliance { a: num(it)?, b: num(it)?, kind: num(it)? },
            "ap" => Command::AddPlayer { player: num(it)?, controller: num(it)? },
            "rp" => Command::RemovePlayer { player: num(it)? },
            "da" => Command::DefineAbility {
                name: text(it)?,
                cooldown: num(it)?,
                energy: num(it)?,
                target: num(it)?,
                filter: num(it)?,
                effects: effects_of(it)?,
            },
            "du" => Command::DefineUnit { spec: unit_spec_of(it)? },
            "di" => Command::DefineItem { spec: item_spec_of(it)? },
            "dr" => Command::DefineUpgrade { spec: upgrade_spec_of(it)? },
            "gu" => Command::GrantUpgrade { player: num(it)?, name: text(it)? },
            "ga" => Command::GiveAbility { unit: num(it)?, slot: num(it)?, name: text(it)? },
            "rm" => Command::Remove { unit: num(it)? },
            "gi" => Command::GiveItem { unit: num(it)?, item: num(it)?, count: num(it)? },
            "ag" => Command::AddGold { player: num(it)?, amount: num(it)? },
            "au" | "ah" => Command::AddUnitGold { unit: num(it)?, amount: num(it)? },
            "vw" => Command::Victory { player: num(it)? },
            "df" => Command::Defeat { player: num(it)? },
            "ui" => Command::Ui { body: text(it)? },
            "rn" => Command::Respawn { unit: num(it)?, kind: num(it)?, player: num(it)?, x: num(it)?, y: num(it)? },
            _ => return None,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Segment {
    pub start: u32,
    pub image: String,
    pub max_units: u32,
    pub init: u64,
}

impl Segment {
    fn of(start: u32, image: &str, max_units: usize, sim: &Sim) -> Segment {
        Segment { start, image: image.to_string(), max_units: max_units as u32, init: sim.state_hash() }
    }
}

fn segment_line(s: &Segment) -> String {
    format!("segment {} {} {:016x} {} {}", s.start, s.max_units, s.init, RETIRED_FLAG, s.image)
}

pub struct Recording {
    segments: Vec<Segment>,
    cmds: Vec<(u32, Command)>,
    checks: Vec<(u32, u64)>,
    inflight: VecDeque<(u32, Receiver<u64>)>,
    hasher: Option<Sender<(Sim, Sender<u64>)>>,
    outbox: VecDeque<Out>,
    incomplete: Option<u32>,
    hash_lost: Option<u32>,
    sink: Option<Box<dyn Write + Send>>,
}

enum Out {
    Line(String, bool),
    Check(u32),
}

fn spawn_hasher() -> Sender<(Sim, Sender<u64>)> {
    let (tx, rx) = mpsc::channel::<(Sim, Sender<u64>)>();
    std::thread::spawn(move || {
        for (snapshot, reply) in rx {
            let _ = reply.send(snapshot.state_hash());
        }
    });
    tx
}

impl Drop for Recording {
    fn drop(&mut self) {
        self.drain_sink(true);
    }
}

impl Recording {
    pub fn new(image: &str, max_units: usize, sim: &Sim) -> Recording {
        Recording {
            segments: vec![Segment::of(0, image, max_units, sim)],
            cmds: Vec::new(),
            checks: Vec::new(),
            inflight: VecDeque::new(),
            hasher: None,
            outbox: VecDeque::new(),
            incomplete: None,
            hash_lost: None,
            sink: None,
        }
    }

    pub fn set_sink(&mut self, mut sink: Box<dyn Write + Send>) {
        self.settle(true);
        let held = self.render_to(None);
        if sink.write_all(held.as_bytes()).and_then(|_| sink.flush()).is_ok() {
            self.sink = Some(sink);
        }
    }

    fn write_line(&mut self, line: &str, flush: bool) {
        if self.sink.is_some() {
            self.outbox.push_back(Out::Line(line.to_string(), flush));
            self.drain_sink(false);
        }
    }

    fn drain_sink(&mut self, block: bool) {
        while let Some(out) = self.outbox.pop_front() {
            let (text, flush) = match out {
                Out::Line(text, flush) => (text, flush),
                Out::Check(t) => {
                    self.settle(block);
                    match self.checks.binary_search_by_key(&t, |c| c.0) {
                        Ok(i) => (format!("chk {t} {:016x}", self.checks[i].1), true),
                        Err(_) if !self.inflight.iter().any(|c| c.0 == t) => continue,
                        Err(_) => {
                            self.outbox.push_front(Out::Check(t));
                            break;
                        }
                    }
                }
            };
            let Some(w) = self.sink.as_mut() else {
                self.outbox.clear();
                return;
            };
            let ok = w.write_all(text.as_bytes()).and_then(|_| w.write_all(b"\n")).and_then(|_| if flush { w.flush() } else { Ok(()) });
            if ok.is_err() {
                self.sink = None;
                self.outbox.clear();
                return;
            }
        }
    }

    fn settle(&mut self, block: bool) {
        while let Some((t, rx)) = self.inflight.front() {
            let t = *t;
            let got = if block { rx.recv().map_err(|_| TryRecvError::Disconnected) } else { rx.try_recv() };
            match got {
                Ok(h) => {
                    self.inflight.pop_front();
                    self.checks.push((t, h));
                }
                Err(TryRecvError::Empty) => return,
                Err(TryRecvError::Disconnected) => {
                    self.hash_worker_lost(t);
                    return;
                }
            }
        }
    }

    fn hash_worker_lost(&mut self, first: u32) {
        self.inflight.clear();
        self.hasher = None;
        self.hash_lost.get_or_insert(first);
        self.mark_incomplete(first);
    }

    pub fn take_hash_loss(&mut self) -> Option<u32> {
        self.hash_lost.take()
    }

    fn hash_later(&mut self, sim: &Sim) -> Receiver<u64> {
        let (reply, rx) = mpsc::channel();
        if cfg!(target_arch = "wasm32") {
            let _ = reply.send(sim.state_hash());
            return rx;
        }
        let hasher = self.hasher.get_or_insert_with(spawn_hasher);
        if let Err(mpsc::SendError((snapshot, reply))) = hasher.send((sim.clone(), reply)) {
            let _ = reply.send(snapshot.state_hash());
        }
        rx
    }

    fn last_check_tick(&self) -> Option<u32> {
        self.inflight.back().map(|c| c.0).or_else(|| self.checks.last().map(|c| c.0))
    }

    fn note(&mut self, tick: u32, cmd: Command) {
        let line = format!("{tick} {}", cmd.encode());
        if self.cmds.len() >= MAX_KEPT {
            self.mark_incomplete(tick);
        } else {
            self.cmds.push((tick, cmd));
        }
        self.write_line(&line, false);
    }

    pub fn apply(&mut self, sim: &mut Sim, cmd: Command) -> bool {
        let accepted = cmd.apply(sim);
        self.note(sim.ticks(), cmd);
        accepted
    }

    pub fn record_applied(&mut self, sim: &Sim, cmd: Command) {
        self.note(sim.ticks(), cmd);
    }

    pub fn spawn(&mut self, sim: &mut Sim, handle: u32, index: u32, x: i32, y: i32, owner: u8, route: i32) -> Option<usize> {
        let unit = sim.spawn_mod_unit(owner, index, x, y, route);
        self.note(sim.ticks(), Command::Spawn { handle, index, x, y, owner, route });
        unit
    }

    pub fn tick(&mut self, sim: &mut Sim, paths: bool, vision: bool) {
        if !(paths && vision) {
            self.note(sim.ticks(), Command::Cadence { paths, vision });
        }
        sim.tick_with_cadence(paths, vision);
        if sim.ticks() % CHECK_EVERY == 0 {
            self.check(sim);
        }
        self.drain_sink(false);
    }

    pub fn check(&mut self, sim: &Sim) {
        let t = sim.ticks();
        self.settle(false);
        if self.last_check_tick() == Some(t) {
            return;
        }
        let rx = self.hash_later(sim);
        self.inflight.push_back((t, rx));
        if self.sink.is_some() {
            self.outbox.push_back(Out::Check(t));
            self.drain_sink(false);
        }
    }

    pub fn mark_incomplete(&mut self, tick: u32) {
        if self.incomplete.is_none() {
            self.incomplete = Some(tick);
            self.write_line(&format!("incomplete {tick}"), true);
        }
    }

    pub fn incomplete(&self) -> Option<u32> {
        self.incomplete
    }

    pub fn command_count(&self) -> usize {
        self.cmds.len()
    }

    pub fn rebase(&mut self, image: &str, max_units: usize, sim: &Sim) {
        let segment = Segment::of(sim.ticks(), image, max_units, sim);
        self.write_line(&segment_line(&segment), true);
        self.segments.push(segment);
    }

    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    pub fn image(&self) -> &str {
        &self.segments[self.segments.len() - 1].image
    }

    pub fn max_units(&self) -> usize {
        self.segments[self.segments.len() - 1].max_units as usize
    }

    pub fn render(&mut self, sim: &Sim) -> String {
        self.settle(true);
        self.render_to(Some((sim.ticks(), sim.state_hash())))
    }

    fn render_to(&self, last: Option<(u32, u64)>) -> String {
        let first = &self.segments[0];
        let mut out = format!("{FORMAT_HEADER}\nimage {}\nmax_units {}\ninit {:016x}\n", first.image, first.max_units, first.init);
        if let Some(t) = self.incomplete {
            out.push_str(&format!("incomplete {t}\n"));
        }
        let mut checks = self.checks.clone();
        if let Some((t, h)) = last {
            let newest_start = self.segments[self.segments.len() - 1].start;
            if checks.last().map(|c| c.0) != Some(t) && (self.segments.len() == 1 || t > newest_start) {
                checks.push((t, h));
            }
        }
        let mut items: Vec<(u32, u8, String)> = Vec::with_capacity(checks.len() + self.segments.len() + self.cmds.len());
        for (ct, h) in checks {
            items.push((ct, 0, format!("chk {ct} {h:016x}")));
        }
        for s in &self.segments[1..] {
            items.push((s.start, 1, segment_line(s)));
        }
        for (t, c) in &self.cmds {
            items.push((*t, 2, format!("{t} {}", c.encode())));
        }
        items.sort_by_key(|i| (i.0, i.1));
        for (_, _, line) in items {
            out.push_str(&line);
            out.push('\n');
        }
        out
    }

    pub fn apply_header(&self, _fresh: &mut Sim) {}

    pub fn parse(text: &str) -> Result<Recording, ReplayError> {
        let bad = |line: usize, why: &str| ReplayError::Parse { line, why: why.to_string() };
        let mut lines = text.lines().enumerate().map(|(i, l)| (i + 1, l.trim_end())).filter(|(_, l)| !l.is_empty());
        match lines.next() {
            Some((_, FORMAT_HEADER)) | Some((_, FORMAT_HEADER_V2)) | Some((_, FORMAT_HEADER_V1)) => {}
            Some((n, _)) => return Err(bad(n, "not an oscraft replay (first line must be `oscraft-replay 1`, `2` or `3`)")),
            None => return Err(bad(0, "empty replay")),
        }
        let mut segments = vec![Segment { start: 0, image: String::new(), max_units: 0, init: 0 }];
        let mut have_init = false;
        let mut cmds: Vec<(u32, Command)> = Vec::new();
        let mut checks: Vec<(u32, u64)> = Vec::new();
        let mut incomplete = None;
        for (n, line) in lines {
            let mut it = line.split_whitespace();
            let first = it.next().unwrap_or_default();
            let header = matches!(first, "image" | "max_units" | "init" | "payout_owner" | "gold_by_mod" | "incomplete");
            if header && segments.len() > 1 {
                return Err(bad(n, "a header line must come before the segments"));
            }
            let head = &mut segments[0];
            if let Some(rest) = line.strip_prefix("image ") {
                head.image = rest.to_string();
                continue;
            }
            match first {
                "max_units" => head.max_units = num(&mut it).ok_or_else(|| bad(n, "max_units needs a number"))?,
                "payout_owner" | "gold_by_mod" => {}
                "init" => {
                    head.init = it.next().and_then(|h| u64::from_str_radix(h, 16).ok()).ok_or_else(|| bad(n, "init needs a hex hash"))?;
                    have_init = true;
                }
                "incomplete" => incomplete = Some(num(&mut it).ok_or_else(|| bad(n, "incomplete needs a tick"))?),
                "segment" => {
                    let rest = line["segment".len()..].trim_start();
                    let mut f = rest.splitn(5, ' ');
                    let start: u32 = f.next().and_then(|v| v.parse().ok()).ok_or_else(|| bad(n, "segment needs a start tick"))?;
                    let max_units: u32 = f.next().and_then(|v| v.parse().ok()).ok_or_else(|| bad(n, "segment needs max_units"))?;
                    let init = f.next().and_then(|h| u64::from_str_radix(h, 16).ok()).ok_or_else(|| bad(n, "segment needs a hex hash"))?;
                    match f.next() {
                        Some("0") | Some("1") => {}
                        _ => return Err(bad(n, "segment needs the retired 0 or 1 field")),
                    }
                    let image = f.next().filter(|s| !s.is_empty()).ok_or_else(|| bad(n, "segment needs an image"))?;
                    let last_start = segments[segments.len() - 1].start;
                    if start <= last_start {
                        return Err(bad(n, "segment start ticks must increase"));
                    }
                    if cmds.last().is_some_and(|c| c.0 >= start) {
                        return Err(bad(n, "a segment must start after the commands before it"));
                    }
                    if checks.last().is_some_and(|c| c.0 > start) {
                        return Err(bad(n, "a segment must start after the checks before it"));
                    }
                    segments.push(Segment { start, image: image.to_string(), max_units, init });
                }
                "chk" => {
                    let t: u32 = num(&mut it).ok_or_else(|| bad(n, "chk needs a tick"))?;
                    let h = it.next().and_then(|h| u64::from_str_radix(h, 16).ok()).ok_or_else(|| bad(n, "chk needs a hex hash"))?;
                    if checks.last().is_some_and(|c| c.0 >= t) {
                        return Err(bad(n, "chk ticks must increase"));
                    }
                    if segments.len() > 1 && t <= segments[segments.len() - 1].start {
                        return Err(bad(n, "a chk of an earlier segment must come before its segment line"));
                    }
                    if cmds.last().is_some_and(|c| c.0 >= t) {
                        return Err(bad(n, "a chk must come before the commands of its tick"));
                    }
                    checks.push((t, h));
                }
                tick => {
                    let t: u32 = tick.parse().map_err(|_| bad(n, "unknown line"))?;
                    let code = it.next().ok_or_else(|| bad(n, "command code missing"))?;
                    let cmd = Command::decode(code, &mut it).ok_or_else(|| bad(n, "bad command"))?;
                    if cmds.last().is_some_and(|c| c.0 > t) || checks.last().is_some_and(|c| c.0 > t) {
                        return Err(bad(n, "commands must not go back in time"));
                    }
                    if t < segments[segments.len() - 1].start {
                        return Err(bad(n, "a command comes before its segment's start tick"));
                    }
                    cmds.push((t, cmd));
                }
            }
        }
        if !have_init || segments[0].image.is_empty() {
            return Err(bad(0, "header lacks image or init"));
        }
        Ok(Recording {
            segments,
            cmds,
            checks,
            inflight: VecDeque::new(),
            hasher: None,
            outbox: VecDeque::new(),
            incomplete,
            hash_lost: None,
            sink: None,
        })
    }
}

pub struct Replayer {
    rec: Recording,
    next: usize,
    nchk: usize,
    seg: usize,
    cad: (bool, bool),
}

impl Replayer {
    pub fn new(rec: Recording, sim: &Sim) -> Result<Replayer, ReplayError> {
        let p = Replayer { rec, next: 0, nchk: 0, seg: 0, cad: (true, true) };
        p.check_start(sim)?;
        Ok(p)
    }

    fn check_start(&self, sim: &Sim) -> Result<(), ReplayError> {
        if sim.ticks() != 0 {
            return Err(ReplayError::NotAtStart { tick: sim.ticks() });
        }
        let expected = self.rec.segments[0].init;
        let actual = sim.state_hash();
        if actual != expected {
            return Err(ReplayError::Init { expected, actual });
        }
        Ok(())
    }

    pub fn reset(&mut self, fresh: &Sim) -> Result<(), ReplayError> {
        self.check_start(fresh)?;
        self.next = 0;
        self.nchk = 0;
        self.seg = 0;
        self.cad = (true, true);
        Ok(())
    }

    pub fn recording(&self) -> &Recording {
        &self.rec
    }

    pub fn segment(&self) -> usize {
        self.seg
    }

    pub fn end(&self) -> u32 {
        self.rec.checks.last().map_or(0, |c| c.0)
    }

    pub fn end_hash(&self) -> Option<u64> {
        self.rec.checks.last().map(|c| c.1)
    }

    pub fn step(&mut self, sim: &mut Sim) -> Result<(), ReplayError> {
        self.step_with(sim, &mut |_| None)
    }

    pub fn step_with(&mut self, sim: &mut Sim, load: &mut dyn FnMut(&Segment) -> Option<Sim>) -> Result<(), ReplayError> {
        let t = sim.ticks();
        if t >= self.end() {
            return Err(ReplayError::OutOfTrace { tick: t + 1, base: 0, end: self.end() });
        }
        if let Some(next) = self.rec.segments.get(self.seg + 1) {
            if next.start == t {
                let mut fresh = load(next).ok_or(ReplayError::NoSegmentData { tick: t })?;
                carry_from(sim, &mut fresh);
                let actual = fresh.state_hash();
                if actual != next.init {
                    return Err(ReplayError::Init { expected: next.init, actual });
                }
                *sim = fresh;
                self.seg += 1;
            }
        }
        let t = sim.ticks();
        while self.next < self.rec.cmds.len() && self.rec.cmds[self.next].0 <= t {
            let cmd = &self.rec.cmds[self.next].1;
            self.next += 1;
            match cmd {
                Command::Cadence { paths, vision } => self.cad = (*paths, *vision),
                _ => {
                    cmd.apply(sim);
                }
            }
        }
        let (paths, vision) = std::mem::replace(&mut self.cad, (true, true));
        sim.tick_with_cadence(paths, vision);
        let now = sim.ticks();
        while self.nchk < self.rec.checks.len() && self.rec.checks[self.nchk].0 <= now {
            let (ct, expected) = self.rec.checks[self.nchk];
            self.nchk += 1;
            if ct == now {
                let actual = sim.state_hash();
                if actual != expected {
                    return Err(ReplayError::HashMismatch { tick: ct, expected, actual });
                }
            }
        }
        Ok(())
    }

    pub fn seek(&mut self, sim: &mut Sim, tick: u32) -> Result<u32, ReplayError> {
        if tick > self.end() || tick < sim.ticks() {
            return Err(ReplayError::OutOfTrace { tick, base: sim.ticks(), end: self.end() });
        }
        while sim.ticks() < tick {
            self.step(sim)?;
        }
        Ok(sim.ticks())
    }

    pub fn run_to_end_with(&mut self, sim: &mut Sim, load: &mut dyn FnMut(&Segment) -> Option<Sim>) -> Result<u64, ReplayError> {
        while sim.ticks() < self.end() {
            self.step_with(sim, load)?;
        }
        Ok(sim.state_hash())
    }

    pub fn run_to_end(&mut self, sim: &mut Sim) -> Result<u64, ReplayError> {
        self.run_to_end_with(sim, &mut |_| None)
    }
}


#[cfg(test)]
fn load_data(path: &std::path::Path, max_units: usize) -> Result<Sim, String> {
    if !path.is_dir() {
        return sim::load_file(path, max_units).map_err(|e| format!("{e:?}"));
    }
    let image = osdt_data::generate(path).map_err(|e| format!("{e:#}"))?;
    let mut sim = sim::load(&image, max_units).map_err(|e| format!("{e:?}"))?;
    if let Some(rules_file) = osdt_data::rules_path(path).map_err(|e| format!("{e:#}"))? {
        let rules = std::fs::read_to_string(&rules_file).map_err(|e| format!("{} cannot be read: {e}", rules_file.display()))?;
        if !sim.set_attr_rules_text(&rules) {
            return Err(format!("{} has a bad line", rules_file.display()));
        }
    }
    Ok(sim)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> Sim {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        load_data(&dir, 4096).expect("data image")
    }

    #[test]
    fn mod_commands_replay_to_the_same_state() {
        let mut sim = data();
        let mut rec = Recording::new("data", 4096, &sim);
        let first = (0..4096).find(|&i| sim.alive(i)).expect("a live unit");
        let kind = sim.kind_of(first) as u32;
        let (x, y) = sim.position_fixed(first);
        let spawned = (1..40).find_map(|k| rec.spawn(&mut sim, 7, kind, x + k * 3 * 65536, y, 0, 0)).expect("a free point for the spawn");
        rec.apply(&mut sim, Command::AddGold { player: 0, amount: 500 });
        rec.apply(&mut sim, Command::AddUnitGold { unit: first, amount: 5 });
        rec.apply(&mut sim, Command::Damage { caster: first, target: spawned, dmg: 3 });
        rec.apply(&mut sim, Command::Buff { unit: spawned, kind: 1, ticks: 40 });
        rec.apply(&mut sim, Command::SetOwner { unit: spawned, player: 1 });
        rec.apply(&mut sim, Command::SetTeam { player: 1, team: 1 });
        rec.apply(&mut sim, Command::SetAlliance { a: 0, b: 1, kind: 0 });
        rec.apply(&mut sim, Command::AddPlayer { player: 2, controller: 0 });
        rec.apply(&mut sim, Command::RemovePlayer { player: 2 });
        rec.apply(&mut sim, Command::Fog { on: true });
        rec.apply(&mut sim, Command::GrantUpgrade { player: 0, name: "no such research".into() });
        rec.apply(&mut sim, Command::GiveAbility { unit: spawned, slot: 0, name: "no_such".into() });
        rec.apply(&mut sim, Command::DefineAbility {
            name: "mod_zap".into(),
            cooldown: 10,
            energy: 0,
            target: 1,
            filter: 0,
            effects: vec![Effect { kind: 0, p0: 1 << 16, p1: 0, p2: 0, p3: 0 }],
        });
        rec.apply(&mut sim, Command::DefineUnit {
            spec: UnitSpec {
                name: "mod_grunt".into(),
                model: "m".into(),
                health: 100,
                speed: 1000,
                radius: 500,
                armor: 1,
                shield: 0,
                sight: 6000,
                filter: 0,
                cost: 50,
                bounty: 10,
                weapon: Some((10, 1000, 1)),
                abilities: vec!["mod_zap".into()],
            },
        });
        rec.apply(&mut sim, Command::DefineItem {
            spec: ItemSpec { name: "mod_charm".into(), cost: 20, armor: 0, damage: 2, health: 5, ability: None },
        });
        rec.apply(&mut sim, Command::DefineUpgrade {
            spec: UpgradeSpec {
                name: "mod_tech".into(),
                building: "hall".into(),
                time: 10,
                cost: 30,
                gas: 0,
                effect: Effect { kind: 0, p0: 0, p1: 0, p2: 0, p3: 0 },
                filter: 0,
                requires: vec![],
                unlocks: vec!["mod_grunt".into()],
            },
        });
        rec.apply(&mut sim, Command::Ui { body: "{\"type\":\"msg\",\"text\":\"hi\nthere\"}".into() });
        for t in 0..200 {
            if t == 40 {
                rec.spawn(&mut sim, 8, kind, x, y, 0, 0);
                rec.apply(&mut sim, Command::Victory { player: 0 });
            }
            if t == 80 {
                rec.apply(&mut sim, Command::Cast { unit: first, slot: 0, target: spawned as i32, x: 0, y: 0 });
                rec.apply(&mut sim, Command::AddGold { player: 1, amount: 9 });
            }
            rec.tick(&mut sim, true, true);
        }
        let text = rec.render(&sim);
        assert!(text.starts_with("oscraft-replay 3\n"));
        assert!(text.contains(" sp ") && text.contains(" du ") && text.contains(" ui "));
        let parsed = Recording::parse(&text).expect("parse");
        let mut fresh = data();
        let mut play = Replayer::new(parsed, &fresh).expect("start");
        let played = play.run_to_end(&mut fresh).expect("playback");
        assert_eq!(played, sim.state_hash());
    }

    #[test]
    fn version_one_files_still_parse() {
        let text = "oscraft-replay 1\nimage data\nmax_units 4096\ninit 0000000000000000\nchk 0 0000000000000000\n0 o 1 2 3\n";
        assert!(Recording::parse(text).is_ok());
    }

    #[test]
    fn override_commands_round_trip_through_the_text() {
        let mut sim = data();
        let mut rec = Recording::new("data", 4096, &sim);
        let unit = (0..4096).find(|&i| sim.alive(i)).expect("a live unit") as u32;
        rec.apply(&mut sim, Command::ClassOverride { unit, class: 2 });
        rec.apply(&mut sim, Command::HoldOverride { unit, hold: 1 });
        rec.tick(&mut sim, true, true);
        rec.apply(&mut sim, Command::HoldOverride { unit, hold: 0 });
        rec.apply(&mut sim, Command::ClassOverride { unit, class: 255 });
        rec.apply(&mut sim, Command::HoldOverride { unit, hold: 255 });
        rec.tick(&mut sim, true, true);
        let text = rec.render(&sim);
        assert!(text.lines().any(|l| l.ends_with(&format!(" co {unit} 2"))));
        assert!(text.lines().any(|l| l.ends_with(&format!(" ho {unit} 1"))));
        assert!(text.lines().any(|l| l.ends_with(&format!(" ho {unit} 0"))));
        assert!(text.lines().any(|l| l.ends_with(&format!(" co {unit} 255"))));
        assert!(text.lines().any(|l| l.ends_with(&format!(" ho {unit} 255"))));
        let parsed = Recording::parse(&text).expect("parse");
        assert_eq!(parsed.command_count(), 5);
        let mut fresh = data();
        let mut play = Replayer::new(parsed, &fresh).expect("start");
        let played = play.run_to_end(&mut fresh).expect("playback");
        assert_eq!(played, sim.state_hash());
    }
}

#[cfg(test)]
mod changed_rules_replay {
    use super::*;

    #[test]
    fn replay_under_changed_rules_is_deterministic() {
        let scenario = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("..").join("scenarios").join("duel-routes");
        let mut src = load_data(&scenario, 64).expect("source sim");
        src.set_fog(false);
        let mut rec = Recording::new("duel-routes", 64, &src);
        for _ in 0..1200 {
            rec.tick(&mut src, false, false);
        }
        let text = rec.render(&src);
        let run = |text: &str| -> u64 {
            let mut fresh = load_data(&scenario, 64).expect("fresh sim");
            fresh.set_fog(false);
            let parsed = Recording::parse(text).expect("recording parses");
            let mut player = Replayer::new(parsed, &fresh).expect("replayer");
            while fresh.ticks() < player.end() {
                player.step(&mut fresh).expect("step");
            }
            fresh.state_hash()
        };
        assert_eq!(run(&text), run(&text));
    }
}

#[cfg(test)]
mod hash_worker_tests {
    use super::*;

    fn dead_reply() -> Receiver<u64> {
        let (tx, rx) = mpsc::channel::<u64>();
        drop(tx);
        rx
    }

    #[test]
    fn dead_hash_worker_does_not_panic_and_marks_incomplete() {
        let sim = Sim::new(64);
        let mut rec = Recording::new("open-grid:64x64", 64, &sim);
        rec.inflight.push_back((5, dead_reply()));
        rec.settle(false);
        assert_eq!(rec.incomplete(), Some(5));
        assert_eq!(rec.take_hash_loss(), Some(5));
        assert_eq!(rec.take_hash_loss(), None);

        rec.inflight.push_back((6, dead_reply()));
        rec.settle(true);
        assert!(rec.inflight.is_empty());

        rec.sink = Some(Box::new(Vec::new()));
        rec.outbox.push_back(Out::Check(6));
        rec.drain_sink(true);
        assert!(rec.outbox.is_empty());

        rec.check(&sim);
        let text = rec.render(&sim);
        assert!(text.lines().any(|l| l.starts_with("chk 0 ")), "a new worker hashes after the loss");
        assert!(text.lines().any(|l| l == "incomplete 5"));
    }
}
