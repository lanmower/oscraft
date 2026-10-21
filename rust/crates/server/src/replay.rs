use std::collections::VecDeque;
use std::io::Write;
use std::str::{FromStr, SplitWhitespace};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::time::Duration;

use sim::{AbilitySpec, Effect, ItemSpec, ORDER_HOLD, Refusal, Sim, StateError, UnitSpec, UpgradeSpec};

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

/// Applies the orders recorded at the sim's current tick, takes the state hash, then steps the
/// sim one tick. The hash is taken after the orders and before the step.
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

/// Replays the accepted-order log of `source` into `fresh`, which must start at or before the
/// first recorded tick and hold the same map, catalog and units as the live run.
///
/// `checkpoints` holds `(tick, state_hash)` for every tick that has a recorded order, sorted by
/// tick. Each hash is taken after that tick's orders are applied and before the tick steps.
/// Returns the tick `fresh` reached.
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

/// Plays the order log back at a chosen speed, paused or running, and seeks to any tick up to
/// the source's end. Seeking restores the nearest snapshot at or before the target and re-runs
/// the orders from there; every stepped tick is checked against its state hash.
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
    /// Covers every tick from `fresh` up to the source's current tick. With `trace` empty, the
    /// reference hashes come from one replay pass from `fresh` (a determinism check of the
    /// replay itself); otherwise `trace` holds `(tick, state_hash)` for each of those ticks,
    /// contiguous and sorted, as the live run recorded them.
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

    /// Speed is a multiple of real time: 1 runs at the room's tick rate.
    pub fn set_speed(&mut self, speed: u32) -> Result<(), ReplayError> {
        if speed == 0 || speed > MAX_SPEED {
            return Err(ReplayError::BadSpeed { speed });
        }
        self.speed = speed;
        Ok(())
    }

    /// Steps by the ticks `elapsed` real time is worth at the current speed. Does nothing
    /// while paused or at the end of the trace.
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

pub const FORMAT_HEADER: &str = "oscraft-replay 2";
// Version 1 held orders, casts, items, fog and cadence only; it still parses, and its records are a subset of version 2.
const FORMAT_HEADER_V1: &str = "oscraft-replay 1";
pub const CHECK_EVERY: u32 = 64;
pub const MAX_KEPT: usize = 4_000_000;

/// One input the sim accepts between ticks, from a player or from a mod. A replay is the initial
/// data image plus these, each tagged with the sim tick it was applied at, in order. A mod command
/// that names a handle is recorded with the unit the handle resolved to, so playback needs no mod.
#[derive(Clone)]
pub enum Command {
    Order { unit: usize, x: i32, y: i32 },
    Hold { unit: usize },
    OrderAll { player: u8, x: i32, y: i32 },
    Attack { unit: usize, target: usize },
    Cast { unit: usize, slot: usize, target: i32, x: i32, y: i32 },
    SkillUp { unit: usize, slot: usize },
    UseItem { unit: usize, slot: usize, target: i32, x: i32, y: i32 },
    Buy { unit: usize, item: usize },
    Sell { unit: usize, slot: usize },
    Unstash { unit: usize, slot: usize },
    Fog { on: bool },
    /// A mod's per-attacker target class (255 = clear, back to the Rust rule).
    ClassOverride { unit: u32, class: u8 },
    Cadence { paths: bool, vision: bool },
    /// A mod spawn for `owner` in `lane` (0 when the mod names none). `lane` is recorded for the mod, not read by the sim.
    Spawn { handle: u32, index: u32, x: i32, y: i32, owner: u8, lane: i32 },
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
    AddGold { player: u8, amount: i32 },
    AddHeroGold { unit: usize, amount: i32 },
    Victory { player: u8 },
    Defeat { player: u8 },
    Ui { body: String },
}

fn num<T: FromStr>(it: &mut SplitWhitespace) -> Option<T> {
    it.next()?.parse().ok()
}

// A trailing field an older line may lack: absent reads as `default`, present must parse.
fn num_or<T: FromStr>(it: &mut SplitWhitespace, default: T) -> Option<T> {
    match it.next() {
        None => Some(default),
        Some(t) => t.parse().ok(),
    }
}

// A text field is `x` followed by the hex bytes of the string, so a name or a UI body never breaks its line.
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
    /// Applies the command; false when the sim refused it. A refusal leaves unit state alone but
    /// bumps the refusal counters, which the state hash covers.
    pub fn apply(&self, sim: &mut Sim) -> bool {
        match self {
            Command::Order { unit, x, y } => {
                sim.order(*unit, *x, *y);
                true
            }
            Command::Hold { unit } => sim.issue_order_kind(*unit, ORDER_HOLD, 0, 0).is_ok(),
            Command::OrderAll { player, x, y } => {
                sim.order_all(*player, *x, *y);
                true
            }
            Command::Attack { unit, target } => sim.order_attack_or_approach(*unit, *target).is_ok(),
            Command::Cast { unit, slot, target, x, y } => sim.cast_ability(*unit, *slot, *target, *x, *y),
            Command::SkillUp { unit, slot } => sim.skill_up(*unit, *slot).is_ok(),
            Command::UseItem { unit, slot, target, x, y } => sim.use_item(*unit, *slot, *target, *x, *y),
            Command::Buy { unit, item } => sim.buy_item(*unit, *item),
            Command::Sell { unit, slot } => sim.sell_item(*unit, *slot),
            Command::Unstash { unit, slot } => sim.unstash(*unit, *slot),
            Command::Fog { on } => {
                sim.set_fog(*on);
                true
            }
            Command::ClassOverride { unit, class } => {
                let class = if *class == 255 { None } else { Some(*class) };
                sim.set_class_override(*unit as usize, class).is_ok()
            }
            Command::Cadence { .. } => true,
            Command::Spawn { index, x, y, owner, lane, .. } => sim.spawn_mod_unit(*owner, *index, *x, *y, *lane).is_some(),
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
                Some(r) => sim.grant_research(*player, r),
                None => false,
            },
            Command::GiveAbility { unit, slot, name } => sim.give_ability(*unit, *slot, name),
            Command::AddGold { player, amount } => {
                sim.add_resources(*player, *amount, 0);
                true
            }
            Command::AddHeroGold { unit, amount } => sim.add_hero_gold(*unit, *amount),
            Command::Victory { player } => {
                sim.declare_victory(*player);
                true
            }
            Command::Defeat { player } => {
                sim.declare_defeat(*player);
                true
            }
            Command::Ui { .. } => true,
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
            Command::Fog { on } => {
                put(&mut o, u8::from(*on));
                "f"
            }
            Command::ClassOverride { unit, class } => {
                put(&mut o, *unit);
                put(&mut o, *class);
                "co"
            }
            Command::Cadence { paths, vision } => {
                put(&mut o, u8::from(*paths));
                put(&mut o, u8::from(*vision));
                "k"
            }
            Command::Spawn { handle, index, x, y, owner, lane } => {
                put(&mut o, handle);
                put(&mut o, index);
                put(&mut o, x);
                put(&mut o, y);
                put(&mut o, owner);
                put(&mut o, lane);
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
            Command::AddGold { player, amount } => {
                put(&mut o, player);
                put(&mut o, amount);
                "ag"
            }
            Command::AddHeroGold { unit, amount } => {
                put(&mut o, unit);
                put(&mut o, amount);
                "ah"
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
        };
        format!("{code}{o}")
    }

    fn decode(code: &str, it: &mut SplitWhitespace) -> Option<Command> {
        Some(match code {
            "o" => Command::Order { unit: num(it)?, x: num(it)?, y: num(it)? },
            "h" => Command::Hold { unit: num(it)? },
            "oa" => Command::OrderAll { player: num(it)?, x: num(it)?, y: num(it)? },
            "a" => Command::Attack { unit: num(it)?, target: num(it)? },
            "c" => Command::Cast { unit: num(it)?, slot: num(it)?, target: num(it)?, x: num(it)?, y: num(it)? },
            "s" => Command::SkillUp { unit: num(it)?, slot: num(it)? },
            "u" => Command::UseItem { unit: num(it)?, slot: num(it)?, target: num(it)?, x: num(it)?, y: num(it)? },
            "b" => Command::Buy { unit: num(it)?, item: num(it)? },
            "l" => Command::Sell { unit: num(it)?, slot: num(it)? },
            "t" => Command::Unstash { unit: num(it)?, slot: num(it)? },
            "f" => Command::Fog { on: num::<u8>(it)? != 0 },
            "co" => Command::ClassOverride { unit: num(it)?, class: num(it)? },
            "k" => Command::Cadence { paths: num::<u8>(it)? != 0, vision: num::<u8>(it)? != 0 },
            "sp" => Command::Spawn { handle: num(it)?, index: num(it)?, x: num(it)?, y: num(it)?, owner: num_or(it, 0)?, lane: num_or(it, 0)? },
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
            "ag" => Command::AddGold { player: num(it)?, amount: num(it)? },
            "ah" => Command::AddHeroGold { unit: num(it)?, amount: num(it)? },
            "vw" => Command::Victory { player: num(it)? },
            "df" => Command::Defeat { player: num(it)? },
            "ui" => Command::Ui { body: text(it)? },
            _ => return None,
        })
    }
}

/// The commands a sim run received, per tick, with the state hash every `CHECK_EVERY` ticks.
/// Text format: a header (`oscraft-replay 2`, `image`, `max_units`, `init`), then one line per
/// command `<tick> <code> <args>` and `chk <ticks> <hash>` lines. A `chk` line follows the tick
/// that brought the sim to `<ticks>` and precedes the commands applied at that tick.
pub struct Recording {
    pub image: String,
    pub max_units: u32,
    pub init: u64,
    /// Whether a mod owned the kill payout (`gold_by_mod`) when the recording started; the header carries it.
    pub gold_by_mod: bool,
    cmds: Vec<(u32, Command)>,
    checks: Vec<(u32, u64)>,
    inflight: VecDeque<(u32, Receiver<u64>)>,
    hasher: Option<Sender<(Sim, Sender<u64>)>>,
    outbox: VecDeque<Out>,
    incomplete: Option<u32>,
    sink: Option<Box<dyn Write + Send>>,
}

/// A sink line that is ready to write, or the check at a tick whose hash is still in flight.
enum Out {
    Line(String, bool),
    Check(u32),
}

/// Thread that hashes the sim snapshots a recording hands it, in the order they were sent.
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
    /// `image` names the initial data (a scenario directory, a catalog image or a map file) the
    /// playback loads; `sim` is that data freshly loaded, at tick 0.
    pub fn new(image: &str, max_units: usize, sim: &Sim) -> Recording {
        Recording {
            image: image.to_string(),
            max_units: max_units as u32,
            init: sim.state_hash(),
            gold_by_mod: sim.gold_by_mod(),
            cmds: Vec::new(),
            checks: Vec::new(),
            inflight: VecDeque::new(),
            hasher: None,
            outbox: VecDeque::new(),
            incomplete: None,
            sink: None,
        }
    }

    /// Streams the recording to `sink` from now on, after writing what it holds so far.
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

    /// Writes queued sink lines in order. A check line waits for its hash: without `block` the
    /// drain stops at the first hash still in flight, with `block` it waits for it.
    fn drain_sink(&mut self, block: bool) {
        while let Some(out) = self.outbox.pop_front() {
            let (text, flush) = match out {
                Out::Line(text, flush) => (text, flush),
                Out::Check(t) => {
                    self.settle(block);
                    match self.checks.binary_search_by_key(&t, |c| c.0) {
                        Ok(i) => (format!("chk {t} {:016x}", self.checks[i].1), true),
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

    /// Moves finished hashes into `checks`, in tick order. With `block`, waits for each one.
    fn settle(&mut self, block: bool) {
        while let Some((t, rx)) = self.inflight.front() {
            let t = *t;
            let h = if block {
                rx.recv().expect("state hash worker stopped")
            } else {
                match rx.try_recv() {
                    Ok(h) => h,
                    Err(TryRecvError::Empty) => return,
                    Err(TryRecvError::Disconnected) => panic!("state hash worker stopped"),
                }
            };
            self.inflight.pop_front();
            self.checks.push((t, h));
        }
    }

    /// Sends a copy of the sim to the hasher thread; the hash arrives on the returned receiver.
    fn hash_later(&mut self, sim: &Sim) -> Receiver<u64> {
        let (reply, rx) = mpsc::channel();
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

    /// Applies `cmd` to `sim` and records it at the sim's tick, refused or not: a refusal bumps the
    /// refusal counters the state hash covers, so playback must attempt it too.
    pub fn apply(&mut self, sim: &mut Sim, cmd: Command) -> bool {
        let accepted = cmd.apply(sim);
        self.note(sim.ticks(), cmd);
        accepted
    }

    /// Records `cmd` at the sim's tick for a caller that already applied it to `sim` itself.
    pub fn record_applied(&mut self, sim: &Sim, cmd: Command) {
        self.note(sim.ticks(), cmd);
    }

    /// Spawns unit `index` at (x, y) for a mod handle and records the spawn at the sim's tick,
    /// refused or not. Returns the unit the sim gave.
    pub fn spawn(&mut self, sim: &mut Sim, handle: u32, index: u32, x: i32, y: i32, owner: u8, lane: i32) -> Option<usize> {
        let unit = sim.spawn_mod_unit(owner, index, x, y, lane);
        self.note(sim.ticks(), Command::Spawn { handle, index, x, y, owner, lane });
        unit
    }

    /// Steps `sim` one tick at the given cadence, recording a non-default cadence, and records a
    /// state hash every `CHECK_EVERY` ticks.
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

    /// Records the state hash at the sim's current tick unless one is already held for it.
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

    /// Notes that something outside the command set changed the sim at `tick` (a data reload or a
    /// mod command with no replay form), so a playback of this recording may diverge.
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

    /// The recording as text, ending with a check at the sim's current tick.
    pub fn render(&mut self, sim: &Sim) -> String {
        self.settle(true);
        self.render_to(Some((sim.ticks(), sim.state_hash())))
    }

    fn render_to(&self, last: Option<(u32, u64)>) -> String {
        let mut out = format!("{FORMAT_HEADER}\nimage {}\nmax_units {}\ninit {:016x}\n", self.image, self.max_units, self.init);
        if self.gold_by_mod {
            out.push_str("gold_by_mod 1\n");
        }
        if let Some(t) = self.incomplete {
            out.push_str(&format!("incomplete {t}\n"));
        }
        let mut checks = self.checks.clone();
        if let Some((t, h)) = last {
            if checks.last().map(|c| c.0) != Some(t) {
                checks.push((t, h));
            }
        }
        let mut k = 0;
        for (ct, h) in checks {
            while k < self.cmds.len() && self.cmds[k].0 < ct {
                out.push_str(&format!("{} {}\n", self.cmds[k].0, self.cmds[k].1.encode()));
                k += 1;
            }
            out.push_str(&format!("chk {ct} {h:016x}\n"));
        }
        for (t, c) in &self.cmds[k..] {
            out.push_str(&format!("{t} {}\n", c.encode()));
        }
        out
    }

    /// Sets `fresh` (a sim loaded from the recording's image) to the header's `gold_by_mod`, which the
    /// init hash covers. Call it before `Replayer::new` or `Playback::new`.
    pub fn apply_header(&self, fresh: &mut Sim) {
        fresh.set_gold_by_mod(self.gold_by_mod);
    }

    pub fn parse(text: &str) -> Result<Recording, ReplayError> {
        let bad = |line: usize, why: &str| ReplayError::Parse { line, why: why.to_string() };
        let mut lines = text.lines().enumerate().map(|(i, l)| (i + 1, l.trim_end())).filter(|(_, l)| !l.is_empty());
        match lines.next() {
            Some((_, FORMAT_HEADER)) | Some((_, FORMAT_HEADER_V1)) => {}
            Some((n, _)) => return Err(bad(n, "not an oscraft replay (first line must be `oscraft-replay 1` or `2`)")),
            None => return Err(bad(0, "empty replay")),
        }
        let mut rec = Recording {
            image: String::new(),
            gold_by_mod: false,
            max_units: 0,
            init: 0,
            cmds: Vec::new(),
            checks: Vec::new(),
            inflight: VecDeque::new(),
            hasher: None,
            outbox: VecDeque::new(),
            incomplete: None,
            sink: None,
        };
        let mut have_init = false;
        for (n, line) in lines {
            if let Some(rest) = line.strip_prefix("image ") {
                rec.image = rest.to_string();
                continue;
            }
            let mut it = line.split_whitespace();
            let first = it.next().unwrap_or_default();
            match first {
                "max_units" => rec.max_units = num(&mut it).ok_or_else(|| bad(n, "max_units needs a number"))?,
                "gold_by_mod" => rec.gold_by_mod = num::<u8>(&mut it).ok_or_else(|| bad(n, "gold_by_mod needs 0 or 1"))? != 0,
                "init" => {
                    rec.init = it.next().and_then(|h| u64::from_str_radix(h, 16).ok()).ok_or_else(|| bad(n, "init needs a hex hash"))?;
                    have_init = true;
                }
                "incomplete" => rec.incomplete = Some(num(&mut it).ok_or_else(|| bad(n, "incomplete needs a tick"))?),
                "chk" => {
                    let t: u32 = num(&mut it).ok_or_else(|| bad(n, "chk needs a tick"))?;
                    let h = it.next().and_then(|h| u64::from_str_radix(h, 16).ok()).ok_or_else(|| bad(n, "chk needs a hex hash"))?;
                    if rec.checks.last().is_some_and(|c| c.0 >= t) {
                        return Err(bad(n, "chk ticks must increase"));
                    }
                    if rec.cmds.last().is_some_and(|c| c.0 >= t) {
                        return Err(bad(n, "a chk must come before the commands of its tick"));
                    }
                    rec.checks.push((t, h));
                }
                tick => {
                    let t: u32 = tick.parse().map_err(|_| bad(n, "unknown line"))?;
                    let code = it.next().ok_or_else(|| bad(n, "command code missing"))?;
                    let cmd = Command::decode(code, &mut it).ok_or_else(|| bad(n, "bad command"))?;
                    if rec.cmds.last().is_some_and(|c| c.0 > t) || rec.checks.last().is_some_and(|c| c.0 > t) {
                        return Err(bad(n, "commands must not go back in time"));
                    }
                    rec.cmds.push((t, cmd));
                }
            }
        }
        if !have_init || rec.image.is_empty() {
            return Err(bad(0, "header lacks image or init"));
        }
        Ok(rec)
    }
}

/// Drives a parsed recording against a freshly loaded sim, one tick at a time, checking every
/// recorded hash. The sim only moves forward: to go back, load a fresh sim, call `reset`, and
/// seek forward again (a saved state does not carry every transient field, so it is not used).
pub struct Replayer {
    rec: Recording,
    next: usize,
    nchk: usize,
    cad: (bool, bool),
}

impl Replayer {
    pub fn new(rec: Recording, sim: &Sim) -> Result<Replayer, ReplayError> {
        let p = Replayer { rec, next: 0, nchk: 0, cad: (true, true) };
        p.check_start(sim)?;
        Ok(p)
    }

    fn check_start(&self, sim: &Sim) -> Result<(), ReplayError> {
        if sim.ticks() != 0 {
            return Err(ReplayError::NotAtStart { tick: sim.ticks() });
        }
        let actual = sim.state_hash();
        if actual != self.rec.init {
            return Err(ReplayError::Init { expected: self.rec.init, actual });
        }
        Ok(())
    }

    /// Starts over against `fresh`, a newly loaded sim of the same data.
    pub fn reset(&mut self, fresh: &Sim) -> Result<(), ReplayError> {
        self.check_start(fresh)?;
        self.next = 0;
        self.nchk = 0;
        self.cad = (true, true);
        Ok(())
    }

    pub fn recording(&self) -> &Recording {
        &self.rec
    }

    /// The last tick the recording holds a hash for.
    pub fn end(&self) -> u32 {
        self.rec.checks.last().map_or(0, |c| c.0)
    }

    /// Hash recorded for the end tick, if any.
    pub fn end_hash(&self) -> Option<u64> {
        self.rec.checks.last().map(|c| c.1)
    }

    pub fn step(&mut self, sim: &mut Sim) -> Result<(), ReplayError> {
        let t = sim.ticks();
        if t >= self.end() {
            return Err(ReplayError::OutOfTrace { tick: t + 1, base: 0, end: self.end() });
        }
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

    /// Steps forward to `tick`; a tick behind the sim is refused (see `reset`).
    pub fn seek(&mut self, sim: &mut Sim, tick: u32) -> Result<u32, ReplayError> {
        if tick > self.end() || tick < sim.ticks() {
            return Err(ReplayError::OutOfTrace { tick, base: sim.ticks(), end: self.end() });
        }
        while sim.ticks() < tick {
            self.step(sim)?;
        }
        Ok(sim.ticks())
    }

    /// Plays from the current tick to the end of the recording and returns the final hash.
    pub fn run_to_end(&mut self, sim: &mut Sim) -> Result<u64, ReplayError> {
        while sim.ticks() < self.end() {
            self.step(sim)?;
        }
        Ok(sim.state_hash())
    }
}

#[cfg(all(test, feature = "server-tests"))]
mod tests {
    use super::*;

    fn data() -> Sim {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        crate::mods::load_data(&dir, 4096).expect("data image")
    }

    // Every mod-issued command is recorded, and playing the text back reproduces the live state
    // without any mod running.
    #[test]
    fn mod_commands_replay_to_the_same_state() {
        let mut sim = data();
        let mut rec = Recording::new("data", 4096, &sim);
        let first = (0..4096).find(|&i| sim.alive(i)).expect("a live unit");
        let kind = sim.kind_of(first) as u32;
        let (x, y) = sim.position_fixed(first);
        let spawned = (1..40).find_map(|k| rec.spawn(&mut sim, 7, kind, x + k * 3 * 65536, y, 0, 0)).expect("a free point for the spawn");
        rec.apply(&mut sim, Command::AddGold { player: 0, amount: 500 });
        rec.apply(&mut sim, Command::AddHeroGold { unit: first, amount: 5 });
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
        assert!(text.starts_with("oscraft-replay 2\n"));
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
}

#[cfg(all(test, feature = "server-tests"))]
mod changed_rules_replay {
    use super::*;

    // A recording replayed onto fresh sims under a changed rule must reach the same state on every run: the replay
    // is the mid-match data reload, so every peer rebuilding it has to agree.
    #[test]
    fn replay_under_changed_rules_is_deterministic() {
        let scenario = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("..").join("scenarios").join("duel-routes");
        let mut src = crate::mods::load_data(&scenario, 64).expect("source sim");
        src.set_fog(false);
        let mut rec = Recording::new("duel-routes", 64, &src);
        for _ in 0..1200 {
            rec.tick(&mut src, false, false);
        }
        let text = rec.render(&src);
        let run = |text: &str, first: Option<u32>| -> u64 {
            let mut fresh = crate::mods::load_data(&scenario, 64).expect("fresh sim");
            fresh.set_fog(false);
            if let Some(v) = first {
                fresh.set_camp_rule("first_spawn_s", v).expect("rule");
            }
            let parsed = Recording::parse(text).expect("recording parses");
            let mut player = Replayer::new(parsed, &fresh).expect("replayer");
            while fresh.ticks() < player.end() {
                player.step(&mut fresh).expect("step");
            }
            fresh.state_hash()
        };
        // Two replays of one recording under the same rule must agree. A rule set after load does not move the
        // load-time camp holds, so the rule change here is not a sanity check of the rule's effect (see notes).
        assert_eq!(run(&text, Some(30)), run(&text, Some(30)));
    }
}
