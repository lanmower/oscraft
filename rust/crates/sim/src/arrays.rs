//! The unit arrays and the Sim handle. Every buffer is sized here, so nothing
//! in `tick` allocates.

use crate::fixed;
use crate::loader::{Catalog, FILTER_STRUCTURE, FILTER_WORKER};
use crate::pathfinding::{Grid, Pool, DEFAULT_MAX_FIELDS};
use crate::rules::{Refusal, REFUSAL_KINDS};
use crate::*;

#[derive(Clone, Copy, Default)]
pub(crate) struct Reveal {
    pub(crate) p: u8,
    pub(crate) x0: i32,
    pub(crate) y0: i32,
    pub(crate) x1: i32,
    pub(crate) y1: i32,
    pub(crate) until: i32,
    pub(crate) live: bool,
}

/// Counters the host reads. Refusals are kept apart from misses on purpose.
#[derive(Default, Clone, Copy, Debug)]
pub struct Stats {
    pub deaths: usize,
    pub vision_refreshes: usize,
    pub harvested: usize,
    pub trained: usize,
    pub trains_refused: usize,
    pub last_train_refusal: Refusal,
    pub retargets: usize,
    pub retargets_deferred: usize,
    pub casts: usize,
    pub casts_refused: usize,
    pub refused_by: [usize; REFUSAL_KINDS],
    pub last_refusal: Refusal,
    pub buffs_dropped: usize,
    pub summoned: usize,
    pub reveals_dropped: usize,
    pub missiles_lost: usize,
    pub ability_damage: i32,
}

pub struct Sim {
    pub(crate) grid: Grid,
    pub(crate) cat: Catalog,
    pub(crate) n: usize,
    pub(crate) cap: usize,
    pub(crate) head: usize,

    pub(crate) pos_x: Vec<i32>,
    pub(crate) pos_y: Vec<i32>,
    pub(crate) vel_x: Vec<i32>,
    pub(crate) vel_y: Vec<i32>,
    pub(crate) tgt_x: Vec<i32>,
    pub(crate) tgt_y: Vec<i32>,
    pub(crate) hp: Vec<i32>,
    pub(crate) hp_max: Vec<i32>,
    pub(crate) radius: Vec<i32>,
    pub(crate) speed: Vec<i32>,
    pub(crate) damage: Vec<i32>,
    pub(crate) armor: Vec<i32>,
    pub(crate) range_q: Vec<i32>,
    pub(crate) range_sq: Vec<i32>,
    pub(crate) period: Vec<i32>,
    pub(crate) cooldown: Vec<i32>,
    pub(crate) shield: Vec<i32>,
    pub(crate) shield_max: Vec<i32>,
    /// Tick of each unit's last damage taken (or its spawn tick), for the regeneration delay.
    pub(crate) last_hit: Vec<u32>,
    pub(crate) sight_cells: Vec<i32>,
    pub(crate) player: Vec<u8>,
    pub(crate) alive: Vec<u8>,
    pub(crate) type_of: Vec<i32>,
    pub(crate) filter: Vec<u32>,
    pub(crate) response: Vec<u8>,
    pub(crate) product: Vec<i32>,
    pub(crate) flying: Vec<u8>,
    pub(crate) targets: Vec<u8>,
    pub(crate) kind: Vec<u8>,
    pub(crate) blocked: Vec<u8>,
    pub(crate) routed: Vec<u8>,
    pub(crate) order_kind: Vec<u8>,
    pub(crate) patrol_x: Vec<i32>,
    pub(crate) patrol_y: Vec<i32>,

    pub(crate) amount: Vec<i32>,
    pub(crate) wstate: Vec<u8>,
    pub(crate) wtarget: Vec<i32>,
    pub(crate) wtimer: Vec<i32>,
    pub(crate) carry: Vec<i32>,
    pub(crate) carry_kind: Vec<u8>,
    pub(crate) harvest_amt: Vec<i32>,
    pub(crate) trainee: Vec<i32>,
    pub(crate) train_left: Vec<i32>,
    pub(crate) train_hold: Vec<u8>,
    pub(crate) minerals: Vec<i32>,
    pub(crate) vespene: Vec<i32>,

    pub(crate) abil_idx: Vec<i32>,
    pub(crate) abil_cd: Vec<i32>,
    pub(crate) energy: Vec<i32>,
    pub(crate) abil_on: bool,

    pub(crate) changed_at: Vec<u32>,
    pub(crate) trig_hit: Vec<u8>,
    pub(crate) trig_spawn: Vec<u8>,
    pub(crate) trig_idle: Vec<u8>,
    pub(crate) trig_ready: Vec<u8>,

    pub(crate) buff_kind: Vec<i32>,
    pub(crate) buff_left: Vec<i32>,

    pub(crate) healer_player: Vec<u8>,
    pub(crate) healer_left: Vec<i32>,
    pub(crate) healer_timer: Vec<i32>,
    pub(crate) healer_period: Vec<i32>,
    pub(crate) healer_amt: Vec<i32>,
    pub(crate) healer_rad: Vec<i32>,
    pub(crate) healer_x: Vec<i32>,
    pub(crate) healer_y: Vec<i32>,
    pub(crate) healer_target: Vec<i32>,

    pub(crate) m_alive: Vec<u8>,
    pub(crate) m_timer: Vec<i32>,
    pub(crate) m_caster: Vec<i32>,
    pub(crate) m_ability: Vec<i32>,
    pub(crate) m_target: Vec<i32>,
    pub(crate) m_x: Vec<i32>,
    pub(crate) m_y: Vec<i32>,
    pub(crate) log_tick: Vec<u32>,
    pub(crate) log_unit: Vec<i32>,
    pub(crate) log_kind: Vec<u8>,
    pub(crate) log_x: Vec<i32>,
    pub(crate) log_y: Vec<i32>,
    pub(crate) log_len: usize,
    pub(crate) log_dropped: u64,

    pub(crate) bucket_n: Vec<u8>,
    pub(crate) bucket_idx: Vec<u32>,
    pub(crate) scratch: Vec<u32>,

    pub(crate) paths: Pool,
    pub(crate) cur_field: Option<usize>,

    pub(crate) vis: Vec<u8>,
    pub(crate) vw: i32,
    pub(crate) vh: i32,
    pub(crate) vcells: usize,
    /// Last seen position per (player, unit), index `p * cap + unit`. `seen` is 1 once seen.
    pub(crate) seen: Vec<u8>,
    pub(crate) seen_x: Vec<i32>,
    pub(crate) seen_y: Vec<i32>,
    pub(crate) has_seer: [bool; MAX_PLAYERS],
    pub(crate) team: [u8; MAX_PLAYERS],
    pub(crate) alliance: [u16; MAX_PLAYERS],
    pub(crate) reveals: [Reveal; REVEAL_CAP],
    pub(crate) fog: bool,
    pub(crate) harvest: bool,
    pub(crate) produce: bool,

    pub(crate) rng: u64,
    pub(crate) ticks: u32,
    pub(crate) scan_left: i32,
    pub stats: Stats,
}

impl Sim {
    /// An empty simulation with room for `max_units` on a 1x1 open grid and no catalog.
    /// Load a map with `loader::load`, or call `set_grid` before spawning.
    pub fn new(max_units: usize) -> Sim {
        Sim::with_catalog(max_units, Grid::open(1, 1), Catalog::default())
    }

    pub fn with_catalog(max_units: usize, grid: Grid, cat: Catalog) -> Sim {
        let cap = max_units;
        let cells = (grid.w * grid.h).max(1);
        let vw = ((grid.w as i32 + VIS_SCALE - 1) / VIS_SCALE).max(1);
        let vh = ((grid.h as i32 + VIS_SCALE - 1) / VIS_SCALE).max(1);
        let vcells = (vw * vh) as usize;
        let paths = Pool::new(grid.w, grid.h, DEFAULT_MAX_FIELDS);
        let mut s = Sim {
            grid,
            cat,
            n: 0,
            cap,
            head: 0,
            pos_x: vec![0; cap],
            pos_y: vec![0; cap],
            vel_x: vec![0; cap],
            vel_y: vec![0; cap],
            tgt_x: vec![0; cap],
            tgt_y: vec![0; cap],
            hp: vec![0; cap],
            hp_max: vec![0; cap],
            radius: vec![0; cap],
            speed: vec![0; cap],
            damage: vec![0; cap],
            armor: vec![0; cap],
            range_q: vec![0; cap],
            range_sq: vec![0; cap],
            period: vec![1; cap],
            cooldown: vec![0; cap],
            shield: vec![0; cap],
            shield_max: vec![0; cap],
            last_hit: vec![0; cap],
            sight_cells: vec![0; cap],
            player: vec![0; cap],
            alive: vec![0; cap],
            type_of: vec![-1; cap],
            filter: vec![0; cap],
            response: vec![0; cap],
            product: vec![-1; cap],
            flying: vec![0; cap],
            targets: vec![0; cap],
            kind: vec![KIND_ACTOR; cap],
            blocked: vec![0; cap],
            routed: vec![0; cap],
            order_kind: vec![ORDER_MOVE; cap],
            patrol_x: vec![0; cap],
            patrol_y: vec![0; cap],
            amount: vec![0; cap],
            wstate: vec![W_SEEK; cap],
            wtarget: vec![-1; cap],
            wtimer: vec![0; cap],
            carry: vec![0; cap],
            carry_kind: vec![0; cap],
            harvest_amt: vec![0; cap],
            trainee: vec![-1; cap],
            train_left: vec![0; cap],
            train_hold: vec![0; cap],
            minerals: vec![0; MAX_PLAYERS],
            vespene: vec![0; MAX_PLAYERS],
            abil_idx: vec![-1; cap * ABILITY_SLOTS],
            abil_cd: vec![0; cap * ABILITY_SLOTS],
            energy: vec![0; cap],
            abil_on: false,
            changed_at: vec![0; cap],
            trig_hit: vec![0; cap],
            trig_spawn: vec![0; cap],
            trig_idle: vec![0; cap],
            trig_ready: vec![0; cap * ABILITY_SLOTS],
            buff_kind: vec![-1; cap * BUFF_SLOTS],
            buff_left: vec![0; cap * BUFF_SLOTS],
            healer_player: vec![0; HEALER_SLOTS],
            healer_left: vec![0; HEALER_SLOTS],
            healer_timer: vec![0; HEALER_SLOTS],
            healer_period: vec![1; HEALER_SLOTS],
            healer_amt: vec![0; HEALER_SLOTS],
            healer_rad: vec![0; HEALER_SLOTS],
            healer_x: vec![0; HEALER_SLOTS],
            healer_y: vec![0; HEALER_SLOTS],
            healer_target: vec![-1; HEALER_SLOTS],
            m_alive: vec![0; MISSILE_CAP],
            m_timer: vec![0; MISSILE_CAP],
            log_tick: vec![0; ORDER_LOG_CAP],
            log_unit: vec![0; ORDER_LOG_CAP],
            log_kind: vec![0; ORDER_LOG_CAP],
            log_x: vec![0; ORDER_LOG_CAP],
            log_y: vec![0; ORDER_LOG_CAP],
            log_len: 0,
            log_dropped: 0,
            m_caster: vec![-1; MISSILE_CAP],
            m_ability: vec![-1; MISSILE_CAP],
            m_target: vec![-1; MISSILE_CAP],
            m_x: vec![0; MISSILE_CAP],
            m_y: vec![0; MISSILE_CAP],
            bucket_n: vec![0; cells],
            bucket_idx: vec![0; cells * BUCKET_CAP],
            scratch: vec![0; AREA_BUF],
            paths,
            cur_field: None,
            vis: vec![0; MAX_PLAYERS * vcells],
            seen: vec![0; MAX_PLAYERS * cap],
            seen_x: vec![0; MAX_PLAYERS * cap],
            seen_y: vec![0; MAX_PLAYERS * cap],
            vw,
            vh,
            vcells,
            has_seer: [false; MAX_PLAYERS],
            team: std::array::from_fn(|p| p as u8),
            alliance: [0; MAX_PLAYERS],
            reveals: [Reveal::default(); REVEAL_CAP],
            fog: true,
            harvest: true,
            produce: true,
            rng: 0,
            ticks: 0,
            scan_left: 0,
            stats: Stats::default(),
        };
        s.rebucket();
        s
    }

    /// Edits one terrain cell at runtime: blocked (not walkable) or open. The walkability bit is
    /// the one flow fields and placement read, so pathing agrees with it at once. Refusal:
    /// InvalidCell for a cell outside the grid, which changes nothing.
    pub fn set_cell_blocked(&mut self, x: i32, y: i32, blocked: bool) -> Result<(), Refusal> {
        if !self.grid.set_blocked(x, y, blocked) {
            return Err(self.reject(Refusal::InvalidCell));
        }
        self.paths.invalidate();
        Ok(())
    }

    /// Replaces the grid. Refused once any unit exists, because the fog and bucket
    /// planes are sized from it.
    pub fn set_grid(&mut self, grid: Grid) -> bool {
        if self.n > 0 {
            return false;
        }
        let cat = std::mem::take(&mut self.cat);
        *self = Sim::with_catalog(self.cap, grid, cat);
        true
    }

    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    pub fn catalog(&self) -> &Catalog {
        &self.cat
    }

    pub fn count(&self) -> usize {
        self.n
    }

    pub fn alive_count(&self) -> usize {
        (0..self.n).filter(|&i| self.alive[i] != 0).count()
    }

    pub fn ticks(&self) -> u32 {
        self.ticks
    }

    pub fn seed(&mut self, v: u64) {
        self.rng = v.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    }

    pub fn set_fog(&mut self, on: bool) {
        self.fog = on;
    }

    pub fn set_harvest(&mut self, on: bool) {
        self.harvest = on;
    }

    pub fn set_production(&mut self, on: bool) {
        self.produce = on;
    }

    pub fn add_resources(&mut self, p: u8, minerals: i32, vespene: i32) {
        if (p as usize) < MAX_PLAYERS {
            self.minerals[p as usize] += minerals;
            self.vespene[p as usize] += vespene;
        }
    }

    /// Puts player `p` on team `team`. Players on one team are allied. Each player starts on
    /// its own team, so every other player is hostile until a mod changes it.
    /// Refusals: a player or team index at or past MAX_PLAYERS is InvalidPlayer or InvalidTeam.
    pub fn set_team(&mut self, p: u8, team: u8) -> Result<(), Refusal> {
        if (p as usize) >= MAX_PLAYERS {
            return Err(self.reject(Refusal::InvalidPlayer));
        }
        if (team as usize) >= MAX_PLAYERS {
            return Err(self.reject(Refusal::InvalidTeam));
        }
        self.team[p as usize] = team;
        Ok(())
    }

    /// Makes players `a` and `b` allied (`on`) or not allied (`!on`), one way round, in both
    /// directions. A player is always allied with itself. Refusal: InvalidPlayer for an index
    /// at or past MAX_PLAYERS.
    pub fn set_alliance(&mut self, a: u8, b: u8, on: bool) -> Result<(), Refusal> {
        if (a as usize) >= MAX_PLAYERS || (b as usize) >= MAX_PLAYERS {
            return Err(self.reject(Refusal::InvalidPlayer));
        }
        if a == b {
            return Ok(());
        }
        let (ai, bi) = (a as usize, b as usize);
        if on {
            self.alliance[ai] |= 1 << bi;
            self.alliance[bi] |= 1 << ai;
        } else {
            self.alliance[ai] &= !(1 << bi);
            self.alliance[bi] &= !(1 << ai);
        }
        Ok(())
    }

    /// True when a unit of player `a` is an enemy of a unit of player `b`: different players
    /// on different teams with no alliance between them. Indices past MAX_PLAYERS are enemies.
    pub(crate) fn hostile(&self, a: u8, b: u8) -> bool {
        if a == b {
            return false;
        }
        let (ai, bi) = (a as usize, b as usize);
        if ai >= MAX_PLAYERS || bi >= MAX_PLAYERS {
            return true;
        }
        let same_team = self.team[ai] == self.team[bi];
        let allied = self.alliance[ai] & (1 << bi) != 0;
        !(same_team || allied)
    }

    pub fn minerals(&self, p: u8) -> i32 {
        if (p as usize) < MAX_PLAYERS { self.minerals[p as usize] } else { 0 }
    }

    pub fn vespene(&self, p: u8) -> i32 {
        if (p as usize) < MAX_PLAYERS { self.vespene[p as usize] } else { 0 }
    }

    /// Spawns unit type `kind` (an index into the catalog) for player 0.
    pub fn spawn(&mut self, kind: u32, x: i32, y: i32) -> Option<usize> {
        self.spawn_for(0, kind, x, y)
    }

    /// Spawns unit type `kind` for `player` at a Q16.16 point. None when the catalog
    /// has no such type or no slot is free.
    pub fn spawn_for(&mut self, player: u8, kind: u32, x: i32, y: i32) -> Option<usize> {
        if self.head >= self.cap {
            return None;
        }
        let ty = kind as usize;
        let def = *self.cat.units.get(ty)?;
        if !def.is_air() && !self.grid.walkable(fixed::to_int(x), fixed::to_int(y)) {
            return None;
        }
        let i = self.head;
        self.head += 1;
        self.n = self.head;

        self.pos_x[i] = x;
        self.pos_y[i] = y;
        self.tgt_x[i] = x;
        self.tgt_y[i] = y;
        self.vel_x[i] = 0;
        self.vel_y[i] = 0;
        self.hp[i] = fixed::from_int(def.health);
        self.hp_max[i] = self.hp[i];
        self.shield[i] = fixed::from_int(def.shield);
        self.shield_max[i] = self.shield[i];
        self.radius[i] = fixed::from_milli(def.radius);
        self.speed[i] = fixed::from_milli(def.speed);
        self.armor[i] = fixed::from_int(def.armor);
        if def.has_weapon() {
            self.damage[i] = fixed::from_int(def.weapon_damage);
            self.range_q[i] = fixed::from_milli(def.weapon_range);
            self.period[i] = def.weapon_period.max(1);
        } else {
            self.damage[i] = 0;
            self.range_q[i] = 0;
            self.period[i] = 1;
        }
        self.range_sq[i] = fixed::mul(self.range_q[i], self.range_q[i]);
        self.cooldown[i] = 0;
        self.sight_cells[i] = fixed::round(fixed::from_milli(def.sight)).max(0);
        self.player[i] = player;
        self.alive[i] = 1;
        self.type_of[i] = ty as i32;
        self.filter[i] = def.filter;
        self.response[i] = def.response;
        self.product[i] = def.product_index().map_or(-1, |p| p as i32);
        self.flying[i] = u8::from(def.is_air());
        self.targets[i] = if self.range_q[i] > 0 && self.range_q[i] <= MELEE_RANGE {
            LAYER_GROUND
        } else {
            LAYER_GROUND | LAYER_AIR
        };
        self.kind[i] = self.classify(def);
        self.blocked[i] = 0;
        self.routed[i] = 0;
        self.order_kind[i] = ORDER_MOVE;
        self.patrol_x[i] = x;
        self.patrol_y[i] = y;
        self.amount[i] = match self.kind[i] {
            KIND_MINERAL => FIELD_MINERAL,
            KIND_GEYSER => FIELD_GAS,
            _ => 0,
        };
        self.wstate[i] = W_SEEK;
        self.wtarget[i] = -1;
        self.wtimer[i] = 0;
        self.carry[i] = 0;
        self.carry_kind[i] = 0;
        self.harvest_amt[i] = def.harvest_amount;
        self.trainee[i] = if self.kind[i] == KIND_PRODUCER { self.product[i] } else { -1 };
        self.train_left[i] = 0;
        self.train_hold[i] = 0;
        self.trig_hit[i] = 0;
        self.trig_spawn[i] = 1;
        self.last_hit[i] = self.ticks;
        self.trig_idle[i] = 0;
        for slot in 0..ABILITY_SLOTS {
            self.trig_ready[i * ABILITY_SLOTS + slot] = 0;
        }
        for k in 0..BUFF_SLOTS {
            self.buff_kind[i * BUFF_SLOTS + k] = -1;
        }
        for slot in 0..ABILITY_SLOTS {
            self.abil_idx[i * ABILITY_SLOTS + slot] = -1;
            self.abil_cd[i * ABILITY_SLOTS + slot] = 0;
        }
        for slot in 0..(def.abil_count as usize).min(ABILITY_SLOTS) {
            if let Some(&ai) = self.cat.uabl.get(def.abil_first as usize + slot) {
                self.abil_idx[i * ABILITY_SLOTS + slot] = ai as i32;
                self.abil_on = true;
            }
        }
        self.energy[i] = 0;
        if (player as usize) < MAX_PLAYERS {
            self.has_seer[player as usize] = true;
        }
        self.touch(i);
        self.bucket_add(i);
        Some(i)
    }

    /// Kind from the filter mask and product: a worker gathers, a structure that
    /// trains workers is a base, and any other product-holder is a producer.
    fn classify(&self, def: crate::loader::Unit) -> u8 {
        if def.filter & FILTER_WORKER != 0 {
            return KIND_WORKER;
        }
        if let Some(p) = def.product_index() {
            if self.cat.units[p].filter & FILTER_WORKER != 0 && def.filter & FILTER_STRUCTURE != 0 {
                return KIND_BASE;
            }
            return KIND_PRODUCER;
        }
        match def.resource_kind {
            1 => KIND_MINERAL,
            2 => KIND_GEYSER,
            _ => KIND_ACTOR,
        }
    }

    pub fn position_fixed(&self, i: usize) -> (i32, i32) {
        if i >= self.n { (0, 0) } else { (self.pos_x[i], self.pos_y[i]) }
    }

    pub fn hp(&self, i: usize) -> i32 {
        if i >= self.n { 0 } else { self.hp[i] }
    }

    pub fn player(&self, i: usize) -> u8 {
        if i >= self.n { 0 } else { self.player[i] }
    }

    pub fn alive(&self, i: usize) -> bool {
        i < self.n && self.alive[i] != 0
    }

    pub fn order(&mut self, i: usize, x: i32, y: i32) {
        if i < self.n {
            let (x, y) = self.clamp_to_grid(x, y);
            self.tgt_x[i] = x;
            self.tgt_y[i] = y;
            self.order_kind[i] = ORDER_MOVE;
            self.touch(i);
            self.record_order(i);
        }
    }

    /// Marks unit `i` as changed in the current tick. The delta read (`changed_since`) reports
    /// marks made after a tick. A mark stores the tick number plus one, so a poll that took
    /// `ticks()` as its `since` sees every change made from that tick on.
    pub(crate) fn touch(&mut self, i: usize) {
        if let Some(c) = self.changed_at.get_mut(i) {
            *c = self.ticks.wrapping_add(1);
        }
    }

    /// How many live units changed after tick `since`. Size the buffer for `changed_since` with it.
    pub fn changed_count_since(&self, since: u32) -> usize {
        (0..self.n).filter(|&i| self.changed_at[i] > since).count()
    }

    /// Writes the indices of live units that changed after tick `since` into `out`, in index
    /// order, and returns how many were written. Units past `out.len()` are dropped, so size
    /// `out` with `changed_count_since` first. Poll with `ticks()` as the next `since`.
    pub fn changed_since(&self, since: u32, out: &mut [u32]) -> usize {
        let mut k = 0;
        for i in 0..self.n {
            if k == out.len() {
                break;
            }
            if self.changed_at[i] > since {
                out[k] = i as u32;
                k += 1;
            }
        }
        k
    }

    /// Appends an accepted order of unit `i` (its kind and resulting destination, at the current
    /// tick). A full log counts the order as dropped instead of storing it.
    pub(crate) fn record_order(&mut self, i: usize) {
        if self.log_len >= ORDER_LOG_CAP {
            self.log_dropped += 1;
            return;
        }
        let k = self.log_len;
        self.log_tick[k] = self.ticks;
        self.log_unit[k] = i as i32;
        self.log_kind[k] = self.order_kind[i];
        self.log_x[k] = self.tgt_x[i];
        self.log_y[k] = self.tgt_y[i];
        self.log_len += 1;
    }

    /// Number of accepted orders held in the record log.
    pub fn order_log_len(&self) -> usize {
        self.log_len
    }

    /// Orders accepted past the log capacity, which are counted but not stored.
    pub fn order_log_dropped(&self) -> u64 {
        self.log_dropped
    }

    /// Accepted order `k` as (tick, unit, kind, x, y), or None past the stored log.
    pub fn order_record(&self, k: usize) -> Option<(u32, i32, u8, i32, i32)> {
        if k >= self.log_len {
            return None;
        }
        Some((self.log_tick[k], self.log_unit[k], self.log_kind[k], self.log_x[k], self.log_y[k]))
    }

    pub fn order_all(&mut self, player: u8, x: i32, y: i32) {
        for i in 0..self.n {
            if self.alive[i] != 0 && self.player[i] == player {
                self.order(i, x, y);
            }
        }
    }

    pub fn paths(&self) -> &Pool {
        &self.paths
    }

    pub(crate) fn bucket_add(&mut self, i: usize) {
        let (w, h) = (self.grid.w as i32, self.grid.h as i32);
        let cx = fixed::to_int(self.pos_x[i]);
        let cy = fixed::to_int(self.pos_y[i]);
        if cx < 0 || cy < 0 || cx >= w || cy >= h {
            return;
        }
        let cell = (cy * w + cx) as usize;
        let cnt = self.bucket_n[cell] as usize;
        if cnt < BUCKET_CAP {
            self.bucket_idx[cell * BUCKET_CAP + cnt] = i as u32;
            self.bucket_n[cell] += 1;
        }
    }

    pub(crate) fn rebucket(&mut self) {
        self.bucket_n.fill(0);
        let (w, h) = (self.grid.w as i32, self.grid.h as i32);
        for i in 0..self.n {
            if self.alive[i] == 0 {
                continue;
            }
            let cx = fixed::to_int(self.pos_x[i]);
            let cy = fixed::to_int(self.pos_y[i]);
            if cx < 0 || cy < 0 || cx >= w || cy >= h {
                continue;
            }
            let cell = (cy * w + cx) as usize;
            let cnt = self.bucket_n[cell] as usize;
            if cnt < BUCKET_CAP {
                self.bucket_idx[cell * BUCKET_CAP + cnt] = i as u32;
                self.bucket_n[cell] += 1;
            }
        }
    }

    /// Clamps a world point onto the grid. Orders store the clamped point.
    pub fn clamp_to_grid(&self, x: i32, y: i32) -> (i32, i32) {
        let max_x = fixed::from_int(self.grid.w as i32 - 1);
        let max_y = fixed::from_int(self.grid.h as i32 - 1);
        (x.clamp(0, max_x), y.clamp(0, max_y))
    }

    pub(crate) fn layer_of(&self, i: usize) -> u8 {
        if self.flying[i] != 0 { LAYER_AIR } else { LAYER_GROUND }
    }

    pub(crate) fn live_at(&self, t: i32) -> bool {
        t >= 0 && (t as usize) < self.n && self.alive[t as usize] != 0
    }
}

/// Cells are one world unit each (the loader refuses any other cell size).
pub(crate) fn cell_of(world: i32) -> i32 {
    fixed::to_int(world)
}

pub(crate) fn dist2(ax: i32, ay: i32, bx: i32, by: i32) -> i64 {
    let dx = ax.wrapping_sub(bx) as i64;
    let dy = ay.wrapping_sub(by) as i64;
    dx * dx + dy * dy
}
