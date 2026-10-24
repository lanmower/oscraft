//! The unit arrays and the Sim handle. Every buffer is sized here, so nothing
//! in `tick` allocates.

use crate::fixed;
use crate::loader::{Catalog, Doodad};
use crate::pathfinding::{Grid, Pool, DEFAULT_MAX_FIELDS};
use crate::rules::{Refusal, REFUSAL_KINDS};
use crate::*;

/// Counters the host reads. Refusals are kept apart from misses on purpose.
#[derive(Clone, Copy, Debug)]
pub struct Stats {
    pub deaths: usize,
    pub vision_refreshes: usize,
    pub casts: usize,
    pub casts_refused: usize,
    pub refused_by: [usize; REFUSAL_KINDS],
    pub last_refusal: Refusal,
    pub buffs_dropped: usize,
    pub summoned: usize,
    pub missiles_lost: usize,
    pub ability_damage: i32,
}

// Written out by hand: Default is not implemented for arrays longer than 32, and refused_by
// holds one counter per refusal kind.
impl Default for Stats {
    fn default() -> Stats {
        Stats {
            deaths: 0,
            vision_refreshes: 0,
            casts: 0,
            casts_refused: 0,
            refused_by: [0; REFUSAL_KINDS],
            last_refusal: Refusal::default(),
            buffs_dropped: 0,
            summoned: 0,
            missiles_lost: 0,
            ability_damage: 0,
        }
    }
}

#[derive(Clone)]
pub struct Sim {
    pub(crate) grid: Grid,
    pub(crate) cat: Catalog,
    /// Doodads on the map: the DODA records at load, then placed and removed at run time.
    pub(crate) doodads: Vec<Doodad>,
    /// Digest of the mod (catalog and height data), fixed at load; part of the state hash.
    pub(crate) cat_digest: u64,
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
    /// Per (player, research record): 1 once the player holds the research, index `p * research count + record`.
    /// The only research state the sim keeps; see modapi.rs (research_bit, set_research_bit).
    pub(crate) researched: Vec<u8>,
    pub(crate) sight_cells: Vec<i32>,
    pub(crate) player: Vec<u8>,
    pub(crate) alive: Vec<u8>,
    pub(crate) type_of: Vec<i32>,
    pub(crate) filter: Vec<u32>,
    pub(crate) response: Vec<u8>,
    pub(crate) flying: Vec<u8>,
    pub(crate) targets: Vec<u8>,
    pub(crate) kind: Vec<u8>,
    pub(crate) blocked: Vec<u8>,
    pub(crate) routed: Vec<u8>,
    pub(crate) order_kind: Vec<u8>,
    pub(crate) patrol_x: Vec<i32>,
    pub(crate) patrol_y: Vec<i32>,

    pub(crate) amount: Vec<i32>,
    pub(crate) resource_a: Vec<i32>,
    pub(crate) resource_b: Vec<i32>,

    pub(crate) abil_idx: Vec<i32>,
    pub(crate) abil_cd: Vec<i32>,
    /// Skill rank per ability slot, 0 when not learned. See ranks.rs.
    pub(crate) abil_rank: Vec<u8>,
    /// Ticks until each inventory slot's active item ability is ready again, INV_SLOTS per unit.
    pub(crate) item_cd: Vec<i32>,
    pub(crate) energy: Vec<i32>,
    pub(crate) abil_on: bool,

    pub(crate) changed_at: Vec<u32>,
    pub(crate) trig_hit: Vec<u8>,
    pub(crate) trig_spawn: Vec<u8>,
    pub(crate) trig_idle: Vec<u8>,
    pub(crate) trig_death: Vec<u8>,
    pub(crate) winner: i32,
    pub(crate) trig_ready: Vec<u8>,

    pub(crate) buff_kind: Vec<i32>,
    pub(crate) buff_left: Vec<i32>,
    /// Inventory slots, INV_SLOTS per unit: catalog id and count, see inventory.rs.
    pub(crate) inv_kind: Vec<u32>,
    pub(crate) inv_count: Vec<u32>,
    /// Stash bank, the same shape as the inventory: bought items overflow here when the
    /// inventory is full, and `unstash` moves one back. See inventory.rs.
    pub(crate) stash_kind: Vec<u32>,
    pub(crate) stash_count: Vec<u32>,
    /// Per unit, counts every change to its inventory or stash. Not hashed or saved. See `item_generation`.
    pub(crate) item_gen: Vec<u32>,
    /// Experience per unit; the level is derived from the curve. See xp.rs.
    pub(crate) xp_total: Vec<u32>,
    /// Cumulative xp thresholds for levels 1.. Set by the caller and not saved, like the catalog.
    pub(crate) xp_curve: Vec<u32>,
    /// Kills credited to each unit, and deaths each unit suffered, reset when its slot is spawned.
    pub(crate) unit_kills: Vec<u32>,
    pub(crate) unit_deaths: Vec<u32>,
    /// Unit respawn, the respawn-wait and gold per unit. See attributes.rs.
    pub(crate) attributed: crate::attributes::AttributedState,
    /// Named per-unit counters the mods keep (a cooldown stamp and the like), by (unit, name). Not hashed or saved.
    pub(crate) counters: std::collections::BTreeMap<(usize, String), i64>,
    /// Spawn-group home, leash and respawn timer per unit. See spawn_groups.rs.
    pub(crate) groups: crate::spawn_groups::GroupState,
    /// Aggro targets, invulnerability, damage reduction and healing per unit. See aggro.rs.
    pub(crate) aggro: crate::aggro::AggroState,
    /// Route progress per unit and the destination request log. See waypoints.rs.
    pub(crate) dests: crate::waypoints::DestState,
    /// Casts that took hold since the host last drained them, as (caster, ability). Transient. See rules.rs.
    pub(crate) cast_log: Vec<(u32, u16)>,
    /// Target, x and y of the cast each unit last took hold with (target -1 for none). Transient, not hashed.
    pub(crate) last_cast: Vec<(i32, i32, i32)>,
    /// Display effects queued by mods since the host last drained them. Transient, not hashed. See fx.rs.
    pub(crate) fx: Vec<crate::fx::FxEvent>,
    /// Hits that dealt damage since the host last drained them. Transient, not hashed. See fx.rs.
    pub(crate) hits: Vec<crate::fx::Hit>,
    /// Chase and return state of acquiring units. See acquire.rs.
    pub(crate) chase: crate::acquire::ChaseState,
    /// Acquire range per unit kind, milli-units, from the table in acquire.rs. Catalog-derived, not hashed.
    pub(crate) acquire: Vec<i32>,
    /// Per-attacker target class set by a mod (-1 = none). Mod-set state, hashed only when a value is set.
    pub(crate) class_override: Vec<i8>,
    /// Per-attacker attack-move hold set by a mod (-1 = none, 0 = walk on, 1 = hold while on an attack-move).
    pub(crate) hold_override: Vec<i8>,
    /// Per-attacker strike target set by a mod (-1 = none). Hashed only when a value is set.
    pub(crate) strike_target: Vec<i32>,
    /// Per-unit damage scale (Q16.16) the pack sets each tick from the unit's armour: damage taken is multiplied by it.
    pub(crate) armour_scale: Vec<i32>,
    /// Per-unit speed bounds (Q16.16) the pack sets: effective speed is held between floor and cap.
    pub(crate) speed_floor: Vec<i32>,
    pub(crate) speed_cap: Vec<i32>,
    /// Per-unit miss chance (Q16.16) of its shots, set by the pack; 0 means its shots cannot miss.
    pub(crate) miss_chance: Vec<i32>,
    /// Per-unit percent of magical damage kept (100 = the full hit), set by the pack from the unit's magic resistance.
    pub(crate) magic_keep: Vec<i32>,
    /// Per-unit percent of every hit kept through its damage reduction stat (100 = the full hit), set by the pack.
    pub(crate) damage_keep: Vec<i32>,
    /// Per-unit percent of every hit kept through its damage response (0 immune, the rule's percent when reduced,
    /// 100 normal), set by the pack.
    pub(crate) response_keep: Vec<i32>,
    /// Ward expiry stamps. See timed_life.rs.
    pub(crate) expiry: crate::timed_life::ExpiryState,
    /// Saved order points for mods that send units away. See order_memory.rs.
    pub(crate) order_memory: crate::order_memory::OrderMemory,
    pub(crate) item_requests: crate::store::ItemRequestState,
    /// Spell modifiers, damage over time, zones, chains and illusions. See spell.rs.
    pub(crate) spell: crate::spell::SpellState,
    /// Generic credits each unit holds (a mod's tally, see combat.rs add_credit), and its recent hitters
    /// (HITTER_SLOTS per unit: the hitter's unit id or -1, and the tick it last hit).
    pub(crate) unit_credits: Vec<u32>,
    pub(crate) hitter_id: Vec<i32>,
    pub(crate) hitter_tick: Vec<u32>,
    /// Kill events since the last drain. Derived from the tallies, so not hashed or saved.
    pub(crate) kill_queue: Vec<crate::combat::KillEvent>,
    /// The unit credited with each unit's latest death, -1 for none. Transient: not hashed or saved.
    pub(crate) killer: Vec<i32>,

    pub(crate) aura_player: Vec<u8>,
    pub(crate) aura_left: Vec<i32>,
    pub(crate) aura_timer: Vec<i32>,
    pub(crate) aura_period: Vec<i32>,
    pub(crate) aura_amt: Vec<i32>,
    pub(crate) aura_rad: Vec<i32>,
    pub(crate) aura_x: Vec<i32>,
    pub(crate) aura_y: Vec<i32>,
    pub(crate) aura_target: Vec<i32>,

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
    /// True while `tick_with_cadence` runs; orders issued inside a tick are re-derived by the tick on replay, so they are not logged.
    pub(crate) in_tick: bool,

    /// Inline slots filled per cell (at most `BUCKET_CAP`); units past them live in `bucket_over`.
    pub(crate) bucket_n: Vec<u8>,
    pub(crate) bucket_idx: Vec<u32>,
    /// Units past the inline `BUCKET_CAP` slots of a cell, in ascending insertion order.
    pub(crate) bucket_over: Vec<Vec<u32>>,
    /// Cells whose overflow list is non-empty, so a rebucket clears only those lists.
    pub(crate) bucket_over_cells: Vec<u32>,
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
    /// Channel state per unit: ticks left (0 when not channelling), the ability index (-1 when
    /// none), the cast point and the cast target (-1 for none).
    pub(crate) channel_left: Vec<i32>,
    pub(crate) channel_ai: Vec<i32>,
    pub(crate) channel_x: Vec<i32>,
    pub(crate) channel_y: Vec<i32>,
    pub(crate) channel_target: Vec<i32>,
    /// Status countdowns per unit, `STATUS_KINDS` slots each (stun, slow, silence, root, invisibility), and the Q16 slow multiplier.
    pub(crate) status_left: Vec<i32>,
    pub(crate) slow_mul: Vec<i32>,
    pub(crate) has_seer: [bool; MAX_PLAYERS],
    pub(crate) team: [u8; MAX_PLAYERS],
    pub(crate) alliance: [u16; MAX_PLAYERS],
    /// Per player, the bit set of players whose sight it shares. Symmetric, and only between allies.
    pub(crate) vision: [u16; MAX_PLAYERS],
    /// Per player, a CONTROLLER_* code: who commands the slot.
    pub(crate) controller: [u8; MAX_PLAYERS],
    pub(crate) fog: bool,

    pub(crate) rng: u64,
    pub(crate) ticks: u32,
    /// Night, as the pack's clock last set it (`game.set_clock`). Storage only: the pack decides the phase. Starts at dawn.
    pub(crate) night: bool,
    /// The clock the pack last reported: ticks into the cycle, day ticks and night ticks (for the client's dusk view).
    pub(crate) day_cycle: [u32; 3],
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

        let research_slots = cat.research.len() * MAX_PLAYERS;
        let cells = (grid.w * grid.h).max(1);
        let vw = ((grid.w as i32 + VIS_SCALE - 1) / VIS_SCALE).max(1);
        let vh = ((grid.h as i32 + VIS_SCALE - 1) / VIS_SCALE).max(1);
        let vcells = (vw * vh) as usize;
        let paths = Pool::new(grid.w, grid.h, DEFAULT_MAX_FIELDS);
        let doodads_init = cat.doodads.clone();
        let cat_digest = crate::hash::catalog_digest(&cat, &grid);
        let acquire = crate::acquire::per_kind(&cat);
        let xp_curve = cat.tune.xp_curve.clone();
        // The economy's opening resources: what every seat holds before it has earned anything. They are
        // data (economy(...)), so a scenario that declares none opens on nothing, as it did before.
        let opening_resource_a = cat.econ.opening_resource_a;
        let opening_gas = cat.econ.opening_gas;

        let mut s = Sim {
            grid,
            doodads: doodads_init,
            cat_digest,
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
            researched: vec![0; research_slots],
            sight_cells: vec![0; cap],
            player: vec![0; cap],
            alive: vec![0; cap],
            type_of: vec![-1; cap],
            filter: vec![0; cap],
            response: vec![0; cap],
            flying: vec![0; cap],
            targets: vec![0; cap],
            kind: vec![KIND_ACTOR; cap],
            blocked: vec![0; cap],
            routed: vec![0; cap],
            order_kind: vec![ORDER_MOVE; cap],
            patrol_x: vec![0; cap],
            patrol_y: vec![0; cap],
            amount: vec![0; cap],
            resource_a: vec![0; MAX_PLAYERS],
            resource_b: vec![0; MAX_PLAYERS],
            abil_idx: vec![-1; cap * ABILITY_SLOTS],
            abil_cd: vec![0; cap * ABILITY_SLOTS],
            abil_rank: vec![0; cap * ABILITY_SLOTS],
            item_cd: vec![0; cap * crate::inventory::INV_SLOTS],
            item_gen: vec![0; cap],
            energy: vec![0; cap],
            abil_on: false,
            changed_at: vec![0; cap],
            trig_hit: vec![0; cap],
            trig_spawn: vec![0; cap],
            trig_idle: vec![0; cap],
            trig_death: vec![0; cap],
            winner: -1,
            trig_ready: vec![0; cap * ABILITY_SLOTS],
            buff_kind: vec![-1; cap * BUFF_SLOTS],
            buff_left: vec![0; cap * BUFF_SLOTS],
            inv_kind: vec![0; cap * crate::inventory::INV_SLOTS],
            inv_count: vec![0; cap * crate::inventory::INV_SLOTS],
            stash_kind: vec![0; cap * crate::inventory::INV_SLOTS],
            stash_count: vec![0; cap * crate::inventory::INV_SLOTS],
            xp_total: vec![0; cap],
            xp_curve,
            unit_kills: vec![0; cap],
            attributed: crate::attributes::AttributedState::new(cap),
            counters: std::collections::BTreeMap::new(),
            groups: crate::spawn_groups::GroupState::new(cap),
            aggro: crate::aggro::AggroState::new(cap),
            dests: crate::waypoints::DestState::new(cap),
            cast_log: Vec::new(),
            last_cast: vec![(-1, 0, 0); cap],
            fx: Vec::new(),
            hits: Vec::new(),
            chase: crate::acquire::ChaseState::new(cap),
            acquire,
            class_override: vec![-1; cap],
            hold_override: vec![-1; cap],
            strike_target: vec![-1; cap],
            armour_scale: vec![fixed::ONE; cap],
            speed_floor: vec![0; cap],
            speed_cap: vec![i32::MAX; cap],
            miss_chance: vec![0; cap],
            magic_keep: vec![100; cap],
            damage_keep: vec![100; cap],
            response_keep: vec![100; cap],
            expiry: crate::timed_life::ExpiryState::new(cap),
            order_memory: crate::order_memory::OrderMemory::new(cap),
            item_requests: Default::default(),
            spell: crate::spell::SpellState::new(cap),
            unit_deaths: vec![0; cap],
            unit_credits: vec![0; cap],
            hitter_id: vec![-1; cap * crate::combat::HITTER_SLOTS],
            hitter_tick: vec![0; cap * crate::combat::HITTER_SLOTS],
            kill_queue: Vec::with_capacity(crate::combat::KILL_QUEUE_CAP),
            killer: vec![-1; cap],
            aura_player: vec![0; HEALER_SLOTS],
            aura_left: vec![0; HEALER_SLOTS],
            aura_timer: vec![0; HEALER_SLOTS],
            aura_period: vec![1; HEALER_SLOTS],
            aura_amt: vec![0; HEALER_SLOTS],
            aura_rad: vec![0; HEALER_SLOTS],
            aura_x: vec![0; HEALER_SLOTS],
            aura_y: vec![0; HEALER_SLOTS],
            aura_target: vec![-1; HEALER_SLOTS],
            m_alive: vec![0; MISSILE_CAP],
            m_timer: vec![0; MISSILE_CAP],
            log_tick: vec![0; ORDER_LOG_CAP],
            log_unit: vec![0; ORDER_LOG_CAP],
            log_kind: vec![0; ORDER_LOG_CAP],
            log_x: vec![0; ORDER_LOG_CAP],
            log_y: vec![0; ORDER_LOG_CAP],
            log_len: 0,
            log_dropped: 0,
            in_tick: false,
            m_caster: vec![-1; MISSILE_CAP],
            m_ability: vec![-1; MISSILE_CAP],
            m_target: vec![-1; MISSILE_CAP],
            m_x: vec![0; MISSILE_CAP],
            m_y: vec![0; MISSILE_CAP],
            bucket_n: vec![0; cells],
            bucket_idx: vec![0; cells * BUCKET_CAP],
            bucket_over: vec![Vec::new(); cells],
            bucket_over_cells: Vec::new(),
            scratch: vec![0; AREA_BUF],
            paths,
            cur_field: None,
            vis: vec![0; MAX_PLAYERS * vcells],
            channel_left: vec![0; cap],
            status_left: vec![0; cap * crate::status::STATUS_KINDS],
            slow_mul: vec![crate::status::SLOW_NONE; cap],
            channel_ai: vec![-1; cap],
            channel_x: vec![0; cap],
            channel_y: vec![0; cap],
            channel_target: vec![-1; cap],
            seen: vec![0; MAX_PLAYERS * cap],
            seen_x: vec![0; MAX_PLAYERS * cap],
            seen_y: vec![0; MAX_PLAYERS * cap],
            vw,
            vh,
            vcells,
            has_seer: [false; MAX_PLAYERS],
            team: std::array::from_fn(|p| p as u8),
            alliance: [0; MAX_PLAYERS],
            vision: [0; MAX_PLAYERS],
            controller: [0; MAX_PLAYERS],
            fog: true,
            rng: 0,
            ticks: 0,
            night: false,
            day_cycle: [0; 3],
            stats: Stats::default(),
        };
        // Every seat opens on the economy's opening resources. They are per-player state, so a save
        // and load carries them and the state hash covers them.
        for p in 0..MAX_PLAYERS {
            s.resource_a[p] = opening_resource_a;
            s.resource_b[p] = opening_gas;
        }
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

    /// Sets or clears the cliff bit of cell (x, y) and invalidates the cached routes. A cliff
    /// bit is part of the grid flags, so the state hash and the save cover it.
    pub fn set_cell_cliff(&mut self, x: i32, y: i32, cliff: bool) -> Result<(), Refusal> {
        if !self.grid.set_cliff(x, y, cliff) {
            return Err(self.reject(Refusal::InvalidCell));
        }
        self.paths.invalidate();
        Ok(())
    }

    /// Sets the tile id of cell (x, y). Tile ids are grid state: hashed and saved.
    pub fn set_cell_tile(&mut self, x: i32, y: i32, tile: u8) -> Result<(), Refusal> {
        if !self.grid.set_tile(x, y, tile) {
            return Err(self.reject(Refusal::InvalidCell));
        }
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
        crate::fog::bump_vision_generation();
        true
    }

    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    /// Elevation of cell (x, y) in milli-units, from the map's height grid.
    pub fn elevation_milli(&self, x: i32, y: i32) -> i32 {
        self.grid.elevation_milli(x, y)
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

    /// Whether it is night, as the pack's clock set it. The sim keeps the flag; it does not run the clock.
    pub fn is_night(&self) -> bool {
        self.night
    }

    /// The clock the pack reported with `set_clock`: ticks into the cycle, day ticks, night ticks.
    pub fn day_cycle(&self) -> [u32; 3] {
        self.day_cycle
    }

    /// Stores the day and night clock the pack computed for this tick (`game.set_clock`).
    pub fn set_clock(&mut self, night: bool, day_cycle: [u32; 3]) {
        self.night = night;
        self.day_cycle = day_cycle;
    }

    pub fn seed(&mut self, v: u64) {
        self.rng = v.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    }

    pub fn set_fog(&mut self, on: bool) {
        crate::fog::bump_vision_generation();
        self.fog = on;
    }

    pub fn fog_on(&self) -> bool {
        self.fog
    }

    /// Moves living unit `i` to player `p`. The vision map is rebuilt at once, so the unit's sight
    /// follows its new owner from this tick. Refusals: a dead or unknown unit is NoTarget; a player at or
    /// past MAX_PLAYERS is InvalidPlayer. A refused change touches nothing.
    pub fn set_owner(&mut self, i: usize, p: u8) -> Result<(), Refusal> {
        if i >= self.n || self.alive[i] == 0 {
            return Err(self.reject(Refusal::NoTarget));
        }
        if (p as usize) >= MAX_PLAYERS {
            return Err(self.reject(Refusal::InvalidPlayer));
        }
        self.player[i] = p;
        if self.fog {
            self.refresh_vision();
        }
        Ok(())
    }

    pub fn add_resources(&mut self, p: u8, resource_a: i32, resource_b: i32) {
        if (p as usize) < MAX_PLAYERS {
            self.resource_a[p as usize] += resource_a;
            self.resource_b[p as usize] += resource_b;
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
        // Teammates share vision: a teammate's detectors reveal to the whole team, as in Dota. Shared vision is the
        // `vision` field, which is what detected() and the fog read; hostility is team/alliance, so this does not
        // make anyone an ally or an enemy that was not one already. The pair is cleared with players of other
        // teams, because every player starts on team 0 and would otherwise keep sharing with everyone.
        for q in 0..MAX_PLAYERS {
            if self.team[q] == team {
                self.vision[p as usize] |= 1 << q;
                self.vision[q] |= 1 << p as usize;
            } else {
                self.vision[p as usize] &= !(1 << q);
                self.vision[q] &= !(1 << p as usize);
            }
        }
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
    pub fn hostile(&self, a: u8, b: u8) -> bool {
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

    pub fn resource_a(&self, p: u8) -> i32 {
        if (p as usize) < MAX_PLAYERS { self.resource_a[p as usize] } else { 0 }
    }

    pub fn resource_b(&self, p: u8) -> i32 {
        if (p as usize) < MAX_PLAYERS { self.resource_b[p as usize] } else { 0 }
    }

    /// Spawns unit type `kind` (an index into the catalog) for player 0.
    pub fn spawn(&mut self, kind: u32, x: i32, y: i32) -> Option<usize> {
        self.spawn_for(0, kind, x, y)
    }

    /// Spawns unit type `kind` for `player` at a Q16.16 point. None when the catalog
    /// has no such type or no slot is free.
    pub fn spawn_for(&mut self, player: u8, kind: u32, x: i32, y: i32) -> Option<usize> {
        if self.head >= self.cap || !self.fits(kind, x, y) {
            return None;
        }
        let i = self.head;
        self.head += 1;
        self.n = self.head;
        self.init_slot(i, player, kind, x, y);
        Some(i)
    }

    /// Spawns unit type `kind` for `player` at exactly slot `index`, which keeps the index a mod
    /// holds. False when the slot is out of range or alive, the kind is unknown, or the point does
    /// not fit; nothing changes then.
    pub fn spawn_at(&mut self, index: usize, kind: u32, player: u8, x: i32, y: i32) -> bool {
        if index >= self.cap || self.alive(index) || !self.fits(kind, x, y) {
            return false;
        }
        if index >= self.head {
            self.head = index + 1;
            self.n = self.head;
        }
        self.init_slot(index, player, kind, x, y);
        true
    }

    fn fits(&mut self, kind: u32, x: i32, y: i32) -> bool {
        let Some(def) = self.cat.units.get(kind as usize) else {
            return false;
        };
        if !def.is_air() && !self.grid.walkable(fixed::to_int(x), fixed::to_int(y)) {
            return false;
        }
        if !self.cell_has_room(x, y) {
            self.refuse(Refusal::CellFull);
            return false;
        }
        true
    }

    fn init_slot(&mut self, i: usize, player: u8, kind: u32, x: i32, y: i32) {
        let ty = kind as usize;
        let def = self.cat.units[ty];
        self.pos_x[i] = x;
        self.pos_y[i] = y;
        self.tgt_x[i] = x;
        self.tgt_y[i] = y;
        self.groups.clear(i);
        self.aggro_clear(i);
        self.waypoint_clear(i);
        self.chase_clear(i);
        self.expiry.expires[i] = 0;
        self.order_memory.clear(i);
        self.spell.clear_unit(i);
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
        self.inventory_clear(i);
        self.stash_clear(i);
        self.xp_total[i] = 0;
        self.unit_kills[i] = 0;
        self.unit_deaths[i] = 0;
        self.unit_credits[i] = 0;
        self.strike_target[i] = -1;
        self.armour_scale[i] = fixed::ONE;
        self.speed_floor[i] = 0;
        self.speed_cap[i] = i32::MAX;
        self.miss_chance[i] = 0;
        self.magic_keep[i] = 100;
        self.damage_keep[i] = 100;
        self.response_keep[i] = 100;
        self.killer[i] = -1;
        self.hitter_id[i * crate::combat::HITTER_SLOTS..(i + 1) * crate::combat::HITTER_SLOTS].fill(-1);
        self.type_of[i] = ty as i32;
        self.filter[i] = def.filter;
        self.response[i] = def.response;
        self.flying[i] = u8::from(def.is_air());
        // The v11 format names no anti-air weapon, so every weapon is a ground weapon and refuses
        // air targets. A unit with no weapon targets nothing either way.
        self.targets[i] = LAYER_GROUND;
        self.kind[i] = self.classify(def);
        self.blocked[i] = 0;
        self.routed[i] = 0;
        self.order_kind[i] = ORDER_MOVE;
        self.patrol_x[i] = x;
        self.patrol_y[i] = y;
        self.amount[i] = match self.kind[i] {
            KIND_MINERAL => self.cat.econ.resource_a_per_patch,
            KIND_GEYSER => self.cat.econ.gas_per_geyser,
            _ => 0,
        };
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
        self.clear_statuses(i);
        for slot in 0..ABILITY_SLOTS {
            self.abil_idx[i * ABILITY_SLOTS + slot] = -1;
            self.abil_cd[i * ABILITY_SLOTS + slot] = 0;
            self.abil_rank[i * ABILITY_SLOTS + slot] = 0;
        }
        for slot in 0..(def.abil_count as usize).min(ABILITY_SLOTS) {
            if let Some(&ai) = self.cat.uabl.get(def.abil_first as usize + slot) {
                self.abil_idx[i * ABILITY_SLOTS + slot] = ai as i32;
                self.abil_cd[i * ABILITY_SLOTS + slot] = self.cat.abilities.get(ai as usize).map_or(0, |a| a.initial_cooldown);
                self.abil_on = true;
            }
        }
        self.energy[i] = 0;
        self.refresh_passives(i);
        if (player as usize) < MAX_PLAYERS {
            self.has_seer[player as usize] = true;
        }
        self.touch(i);
        self.bucket_add(i);
    }

    /// The engine kind of a unit type: the resource it holds (a mineral patch or a geyser), else an actor. Which
    /// units gather, train or are bases is the pack's, read from the unit data (game.unit_info).
    fn classify(&self, def: crate::loader::Unit) -> u8 {
        match def.resource_kind {
            1 => KIND_MINERAL,
            2 => KIND_GEYSER,
            _ => KIND_ACTOR,
        }
    }

    pub fn position_fixed(&self, i: usize) -> (i32, i32) {
        if i >= self.n { (0, 0) } else { (self.pos_x[i], self.pos_y[i]) }
    }

    /// Radius of unit `i` in Q16.16; zero for an unknown index.
    pub fn radius_fixed(&self, i: usize) -> i32 {
        self.radius.get(i).copied().unwrap_or(0)
    }

    /// Resource left in unit `i` (a mineral patch or a geyser); zero for an unknown index.
    pub fn resource_left(&self, i: usize) -> i32 {
        self.amount.get(i).copied().unwrap_or(0)
    }

    /// Takes up to `want` resource from live unit `i` and returns what it took. The unit is marked
    /// changed. A dead or unknown unit gives nothing.
    pub fn take_resource(&mut self, i: usize, want: i32) -> i32 {
        if !self.alive(i) || want <= 0 {
            return 0;
        }
        let taken = want.min(self.amount[i]).max(0);
        self.amount[i] -= taken;
        self.touch(i);
        taken
    }

    /// Harvest per trip that the data gives unit `i`'s type; zero when it gives none.
    pub fn harvest_per_trip(&self, i: usize) -> i32 {
        usize::try_from(self.unit_type(i))
            .ok()
            .and_then(|t| self.cat.units.get(t))
            .map_or(0, |def| def.harvest_amount)
    }

    /// The resource class of unit `i` as mods name it: mineral, geyser, or actor for any other unit.
    pub fn class_name(&self, i: usize) -> &'static str {
        match self.kind.get(i).copied() {
            Some(KIND_MINERAL) => "mineral",
            Some(KIND_GEYSER) => "geyser",
            _ => "actor",
        }
    }

    /// An economy setting by its data name (the saturation counts and the gas per trip); None for any other name.
    pub fn econ_value(&self, key: &str) -> Option<i64> {
        let e = &self.cat.econ;
        match key {
            "workers_per_mineral" => Some(i64::from(e.workers_per_mineral)),
            "workers_per_extractor" => Some(i64::from(e.workers_per_extractor)),
            "gas_per_trip" => Some(i64::from(e.gas_per_trip)),
            _ => None,
        }
    }

    /// Ticks left before unit `i` can strike again (its weapon cooldown); -1 for an unknown index.
    pub fn weapon_cooldown(&self, i: usize) -> i32 {
        self.cooldown.get(i).copied().unwrap_or(-1)
    }

    /// Kills credited to unit `i`; zero for an unknown index.
    pub fn kills_of(&self, i: usize) -> u32 {
        self.unit_kills.get(i).copied().unwrap_or(0)
    }

    /// Deaths unit `i` has suffered; zero for an unknown index.
    pub fn deaths_of(&self, i: usize) -> u32 {
        self.unit_deaths.get(i).copied().unwrap_or(0)
    }

    /// Ticks left on ability `slot` of unit `i` (0 when ready); -1 for an unknown unit or slot.
    pub fn ability_cooldown(&self, i: usize, slot: usize) -> i32 {
        if slot >= ABILITY_SLOTS || i >= self.cap {
            return -1;
        }
        self.abil_cd[i * ABILITY_SLOTS + slot]
    }

    /// Shield points left on unit `i` (Q16.16); zero for an unknown index.
    pub fn shield_of(&self, i: usize) -> i32 {
        self.shield.get(i).copied().unwrap_or(0)
    }

    pub fn hp(&self, i: usize) -> i32 {
        if i >= self.n { 0 } else { self.hp[i] }
    }

    pub fn hp_max(&self, i: usize) -> i32 {
        if i >= self.n { 0 } else { self.hp_max[i] }
    }

    /// Sets live unit `i`'s health and maximum (raw fixed-point). Returns false for a dead or out-of-range unit.
    pub fn set_health(&mut self, i: usize, hp: i32, hp_max: i32) -> bool {
        if !self.alive(i) {
            return false;
        }
        self.hp[i] = hp;
        self.hp_max[i] = hp_max;
        true
    }

    /// The catalog unit type index of unit `i` (the id catalog-unit-name takes), or -1 when unset.
    pub fn unit_type(&self, i: usize) -> i32 {
        self.type_of.get(i).copied().unwrap_or(-1)
    }

    pub fn player(&self, i: usize) -> u8 {
        if i >= self.n { 0 } else { self.player[i] }
    }

    /// The classification (a KIND_* value) of unit `i`, or 0 out of range.
    pub fn kind_of(&self, i: usize) -> u32 {
        self.kind.get(i).map_or(0, |&k| u32::from(k))
    }

    /// The catalog name of unit `i`'s type, or None when the unit has no type.
    pub fn kind_name(&self, i: usize) -> Option<&str> {
        let ty = usize::try_from(*self.type_of.get(i)?).ok()?;
        Some(self.cat.name(self.cat.units.get(ty)?.name))
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
    /// tick). Orders the tick itself issues are skipped: replaying the logged inputs re-derives them.
    /// A full log counts the order as dropped instead of storing it.
    pub(crate) fn record_order(&mut self, i: usize) {
        if self.in_tick {
            return;
        }
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
        self.bucket_push(cell, i);
    }

    /// Appends unit `i` to `cell`: an inline slot while one is free, else the cell's overflow list.
    /// Every unit is stored, so a crowded cell never hides a unit from the strike or separation scans.
    fn bucket_push(&mut self, cell: usize, i: usize) {
        let cnt = self.bucket_n[cell] as usize;
        if cnt < BUCKET_CAP {
            self.bucket_idx[cell * BUCKET_CAP + cnt] = i as u32;
            self.bucket_n[cell] += 1;
        } else {
            if self.bucket_over[cell].is_empty() {
                self.bucket_over_cells.push(cell as u32);
            }
            self.bucket_over[cell].push(i as u32);
        }
    }

    /// Units bucketed in `cell`, inline slots first then overflow, each in insertion order.
    pub(crate) fn bucket_members(&self, cell: usize) -> impl Iterator<Item = usize> + '_ {
        let inline = self.bucket_n[cell] as usize;
        let base = cell * BUCKET_CAP;
        self.bucket_idx[base..base + inline]
            .iter()
            .chain(self.bucket_over[cell].iter())
            .map(|&j| j as usize)
    }

    pub(crate) fn rebucket(&mut self) {
        self.bucket_n.fill(0);
        for cell in self.bucket_over_cells.drain(..) {
            self.bucket_over[cell as usize].clear();
        }
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
            self.bucket_push(cell, i);
        }
    }

    /// Moves live unit `i` to the Q16.16 point (x, y), clamped to the grid, and re-buckets it.
    /// Returns false for a dead or out-of-range unit, which changes nothing.
    pub fn set_position(&mut self, i: usize, x: i32, y: i32) -> bool {
        if !self.alive(i) {
            return false;
        }
        let (x, y) = self.clamp_to_grid(x, y);
        self.pos_x[i] = x;
        self.pos_y[i] = y;
        self.touch(i);
        self.rebucket();
        true
    }

    /// Kills live unit `i` and takes it off the buckets, so the slot no longer counts or is
    /// found. Returns false for a dead or out-of-range unit, which changes nothing.
    pub fn remove_unit(&mut self, i: usize) -> bool {
        if !self.alive(i) {
            return false;
        }
        self.alive[i] = 0;
        self.touch(i);
        self.rebucket();
        true
    }

    /// Brings dead slot `i` back to life at the point (x, y), clamped to the map: the unit is alive there, its
    /// position and target are the point, its velocity is zero, and its routed and blocked flags are clear. Returns
    /// false for an out-of-range slot or a slot that is alive, which changes nothing.
    pub fn revive_unit(&mut self, i: usize, x: i32, y: i32) -> bool {
        if i >= self.count() || self.alive(i) {
            return false;
        }
        let (x, y) = self.clamp_to_grid(x, y);
        self.alive[i] = 1;
        self.pos_x[i] = x;
        self.pos_y[i] = y;
        self.tgt_x[i] = x;
        self.tgt_y[i] = y;
        self.vel_x[i] = 0;
        self.vel_y[i] = 0;
        self.routed[i] = 0;
        self.blocked[i] = 0;
        self.touch(i);
        self.rebucket();
        true
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
