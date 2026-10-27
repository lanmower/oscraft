use crate::fixed;
use crate::loader::{Catalog, Prop};
use crate::pathfinding::{Grid, Pool, DEFAULT_MAX_FIELDS};
use crate::rules::{Refusal, REFUSAL_KINDS};
use crate::*;

pub(crate) const DEFAULT_PERCENT: u16 = 100;
pub(crate) const MAX_PERCENT: u16 = 1000;

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
    pub(crate) props: Vec<Prop>,
    pub(crate) ground: Vec<crate::ground::GroundItem>,
    pub(crate) ground_next: u32,
    pub(crate) obstacles: Vec<crate::obstacles::Obstacle>,
    pub(crate) tree_guard: i32,
    pub(crate) obstacle_cover: Vec<u8>,
    pub(crate) move_tree_half: i32,
    pub(crate) move_tree_regrow: u32,
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
    pub(crate) shield_timed: Vec<i32>,
    pub(crate) shield_until: Vec<u32>,
    pub(crate) shield_lock: Vec<u32>,
    pub(crate) heavy_hit: Vec<u32>,
    pub(crate) last_hit: Vec<u32>,
    pub(crate) bash_ready: Vec<u32>,
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
    pub(crate) heading: Vec<i32>,
    pub(crate) turn_step: Vec<i32>,
    pub(crate) hold_fire: Vec<u8>,
    pub(crate) unit_scale: Vec<u16>,
    pub(crate) xp_rate: [u16; MAX_PLAYERS],

    pub(crate) amount: Vec<i32>,
    pub(crate) resource_a: Vec<i32>,
    pub(crate) resource_b: Vec<i32>,

    pub(crate) abil_idx: Vec<i32>,
    pub(crate) abil_cd: Vec<i32>,
    pub(crate) abil_rank: Vec<u8>,
    pub(crate) item_cd: Vec<i32>,
    pub(crate) inv_owner: Vec<u8>,
    pub(crate) item_lock_until: Vec<u32>,
    pub(crate) item_lock_kind: Vec<u32>,
    pub(crate) block_until: Vec<u32>,
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
    pub(crate) inv_kind: Vec<u32>,
    pub(crate) inv_count: Vec<u32>,
    pub(crate) stash_kind: Vec<u32>,
    pub(crate) stash_count: Vec<u32>,
    pub(crate) item_gen: Vec<u32>,
    pub(crate) xp_total: Vec<u32>,
    pub(crate) xp_curve: Vec<u32>,
    pub(crate) unit_kills: Vec<u32>,
    pub(crate) unit_deaths: Vec<u32>,
    pub(crate) attributed: crate::attributes::AttributedState,
    pub(crate) counters: std::collections::BTreeMap<(usize, String), i64>,
    pub(crate) groups: crate::spawn_groups::GroupState,
    pub(crate) aggro: crate::aggro::AggroState,
    pub(crate) dests: crate::waypoints::DestState,
    pub(crate) cast_log: Vec<(u32, u16)>,
    pub(crate) block_log: Vec<(u32, u32, u16)>,
    pub(crate) last_cast: Vec<(i32, i32, i32)>,
    pub(crate) fx: Vec<crate::fx::FxEvent>,
    pub(crate) hits: Vec<crate::fx::Hit>,
    pub(crate) chase: crate::acquire::ChaseState,
    pub(crate) acquire: Vec<i32>,
    pub(crate) class_override: Vec<i8>,
    pub(crate) hold_override: Vec<i8>,
    pub(crate) strike_target: Vec<i32>,
    pub(crate) swing_target: Vec<i32>,
    pub(crate) swing_left: Vec<i32>,
    pub(crate) swing: crate::swing::Swing,
    pub(crate) armour_scale: Vec<i32>,
    pub(crate) speed_floor: Vec<i32>,
    pub(crate) speed_cap: Vec<i32>,
    pub(crate) miss_chance: Vec<i32>,
    pub(crate) magic_keep: Vec<i32>,
    pub(crate) damage_keep: Vec<i32>,
    pub(crate) response_keep: Vec<i32>,
    pub(crate) expiry: crate::timed_life::ExpiryState,
    pub(crate) order_memory: crate::order_memory::OrderMemory,
    pub(crate) order_queue: crate::order_queue::OrderQueue,
    pub(crate) item_requests: crate::store::ItemRequestState,
    pub(crate) spell: crate::spell::SpellState,
    pub(crate) unit_credits: Vec<u32>,
    pub(crate) hitter_id: Vec<i32>,
    pub(crate) hitter_tick: Vec<u32>,
    pub(crate) kill_queue: Vec<crate::combat::KillEvent>,
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
    pub(crate) in_tick: bool,

    pub(crate) bucket_n: Vec<u8>,
    pub(crate) bucket_idx: Vec<u32>,
    pub(crate) bucket_over: Vec<Vec<u32>>,
    pub(crate) bucket_over_cells: Vec<u32>,
    pub(crate) scratch: Vec<u32>,

    pub(crate) paths: Pool,
    pub(crate) cur_field: Option<usize>,

    pub(crate) vis: Vec<u8>,
    pub(crate) vw: i32,
    pub(crate) vh: i32,
    pub(crate) vcells: usize,
    pub(crate) seen: Vec<u8>,
    pub(crate) seen_x: Vec<i32>,
    pub(crate) seen_y: Vec<i32>,
    pub(crate) channel_left: Vec<i32>,
    pub(crate) channel_ai: Vec<i32>,
    pub(crate) channel_x: Vec<i32>,
    pub(crate) channel_y: Vec<i32>,
    pub(crate) channel_target: Vec<i32>,
    pub(crate) status_left: Vec<i32>,
    pub(crate) slow_mul: Vec<i32>,
    pub(crate) has_seer: [bool; MAX_PLAYERS],
    pub(crate) team: [u8; MAX_PLAYERS],
    pub(crate) alliance: [u16; MAX_PLAYERS],
    pub(crate) vision: [u16; MAX_PLAYERS],
    pub(crate) controller: [u8; MAX_PLAYERS],
    pub(crate) fog: bool,

    pub(crate) rng: u64,
    pub(crate) ticks: u32,
    pub(crate) night: bool,
    pub(crate) day_cycle: [u32; 3],
    pub stats: Stats,
}

impl Sim {
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
        let props_init = cat.props.clone();
        let cat_digest = crate::hash::catalog_digest(&cat, &grid);
        let acquire = crate::acquire::per_kind(&cat);
        let xp_curve = cat.tune.xp_curve.clone();
        let opening_resource_a = cat.econ.opening_resource_a;
        let opening_gas = cat.econ.opening_gas;

        let mut s = Sim {
            grid,
            props: props_init,
            ground: Vec::new(),
            ground_next: 0,
            obstacles: Vec::new(),
            tree_guard: 0,
            obstacle_cover: Vec::new(),
            move_tree_half: -1,
            move_tree_regrow: 0,
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
            shield_timed: vec![0; cap],
            shield_until: vec![0; cap],
            shield_lock: vec![0; cap],
            heavy_hit: vec![0; cap],
            last_hit: vec![0; cap],
            bash_ready: vec![0; cap],
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
            heading: vec![0; cap],
            turn_step: vec![0; cap],
            hold_fire: vec![0; cap],
            unit_scale: vec![DEFAULT_PERCENT; cap],
            xp_rate: [DEFAULT_PERCENT; MAX_PLAYERS],
            amount: vec![0; cap],
            resource_a: vec![0; MAX_PLAYERS],
            resource_b: vec![0; MAX_PLAYERS],
            abil_idx: vec![-1; cap * ABILITY_SLOTS],
            abil_cd: vec![0; cap * ABILITY_SLOTS],
            abil_rank: vec![0; cap * ABILITY_SLOTS],
            item_cd: vec![0; cap * crate::inventory::INV_SLOTS],
            inv_owner: vec![0; cap * crate::inventory::INV_SLOTS],
            item_lock_until: vec![0; cap * crate::inventory::INV_SLOTS],
            item_lock_kind: vec![0; cap * crate::inventory::INV_SLOTS],
            block_until: vec![0; cap],
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
            block_log: Vec::new(),
            last_cast: vec![(-1, 0, 0); cap],
            fx: Vec::new(),
            hits: Vec::new(),
            chase: crate::acquire::ChaseState::new(cap),
            acquire,
            class_override: vec![-1; cap],
            hold_override: vec![-1; cap],
            strike_target: vec![-1; cap],
            swing_target: vec![-1; cap],
            swing_left: vec![0; cap],
            swing: crate::swing::Swing::new(cap),
            armour_scale: vec![fixed::ONE; cap],
            speed_floor: vec![0; cap],
            speed_cap: vec![i32::MAX; cap],
            miss_chance: vec![0; cap],
            magic_keep: vec![100; cap],
            damage_keep: vec![100; cap],
            response_keep: vec![100; cap],
            expiry: crate::timed_life::ExpiryState::new(cap),
            order_memory: crate::order_memory::OrderMemory::new(cap),
            order_queue: crate::order_queue::OrderQueue::new(cap),
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
        for p in 0..MAX_PLAYERS {
            s.resource_a[p] = opening_resource_a;
            s.resource_b[p] = opening_gas;
        }
        s.rebucket();
        s
    }

    pub fn set_cell_blocked(&mut self, x: i32, y: i32, blocked: bool) -> Result<(), Refusal> {
        if !self.grid.set_blocked(x, y, blocked) {
            return Err(self.reject(Refusal::InvalidCell));
        }
        self.paths.invalidate();
        Ok(())
    }

    pub fn set_cell_cliff(&mut self, x: i32, y: i32, cliff: bool) -> Result<(), Refusal> {
        if !self.grid.set_cliff(x, y, cliff) {
            return Err(self.reject(Refusal::InvalidCell));
        }
        self.paths.invalidate();
        Ok(())
    }

    pub fn set_cell_tile(&mut self, x: i32, y: i32, tile: u8) -> Result<(), Refusal> {
        if !self.grid.set_tile(x, y, tile) {
            return Err(self.reject(Refusal::InvalidCell));
        }
        Ok(())
    }

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

    pub fn is_night(&self) -> bool {
        self.night
    }

    pub fn day_cycle(&self) -> [u32; 3] {
        self.day_cycle
    }

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

    pub fn set_team(&mut self, p: u8, team: u8) -> Result<(), Refusal> {
        if (p as usize) >= MAX_PLAYERS {
            return Err(self.reject(Refusal::InvalidPlayer));
        }
        if (team as usize) >= MAX_PLAYERS {
            return Err(self.reject(Refusal::InvalidTeam));
        }
        self.team[p as usize] = team;
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

    pub fn spawn(&mut self, kind: u32, x: i32, y: i32) -> Option<usize> {
        self.spawn_for(0, kind, x, y)
    }

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
        self.order_queue.clear(i);
        self.spell.clear_unit(i);
        self.vel_x[i] = 0;
        self.vel_y[i] = 0;
        self.hp[i] = fixed::from_int(def.health);
        self.hp_max[i] = self.hp[i];
        self.shield[i] = fixed::from_int(def.shield);
        self.shield_max[i] = self.shield[i];
        self.shield_timed[i] = 0;
        self.shield_until[i] = 0;
        self.shield_lock[i] = 0;
        self.heavy_hit[i] = 0;
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
        self.swing_target[i] = -1;
        self.swing_left[i] = 0;
        self.swing.clear(i);
        for u in 0..self.n {
            if self.swing_victim_is(u, i) {
                self.swing_target[u] = -1;
                self.swing_left[u] = 0;
                self.swing.point[u] = 0;
            }
        }
        self.armour_scale[i] = fixed::ONE;
        self.speed_floor[i] = 0;
        self.speed_cap[i] = i32::MAX;
        self.miss_chance[i] = 0;
        self.block_until[i] = 0;
        self.magic_keep[i] = 100;
        self.damage_keep[i] = 100;
        self.response_keep[i] = 100;
        self.killer[i] = -1;
        self.hitter_id[i * crate::combat::HITTER_SLOTS..(i + 1) * crate::combat::HITTER_SLOTS].fill(-1);
        self.type_of[i] = ty as i32;
        self.filter[i] = def.filter;
        self.response[i] = def.response;
        self.flying[i] = u8::from(def.is_air());
        self.targets[i] = LAYER_GROUND;
        self.kind[i] = self.classify(def);
        self.blocked[i] = 0;
        self.routed[i] = 0;
        self.order_kind[i] = ORDER_MOVE;
        self.patrol_x[i] = x;
        self.patrol_y[i] = y;
        self.heading[i] = 0;
        self.turn_step[i] = 0;
        self.hold_fire[i] = 0;
        self.unit_scale[i] = DEFAULT_PERCENT;
        self.amount[i] = match self.kind[i] {
            KIND_MINERAL => self.cat.econ.resource_a_per_patch,
            KIND_GEYSER => self.cat.econ.gas_per_geyser,
            _ => 0,
        };
        self.trig_hit[i] = 0;
        self.trig_spawn[i] = 1;
        self.last_hit[i] = self.ticks;
        self.bash_ready[i] = 0;
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

    pub(crate) fn classify(&self, def: crate::loader::Unit) -> u8 {
        match def.resource_kind {
            1 => KIND_MINERAL,
            2 => KIND_GEYSER,
            _ => KIND_ACTOR,
        }
    }

    pub fn position_fixed(&self, i: usize) -> (i32, i32) {
        if i >= self.n { (0, 0) } else { (self.pos_x[i], self.pos_y[i]) }
    }

    pub fn radius_fixed(&self, i: usize) -> i32 {
        self.radius.get(i).copied().unwrap_or(0)
    }

    pub fn resource_left(&self, i: usize) -> i32 {
        self.amount.get(i).copied().unwrap_or(0)
    }

    pub fn take_resource(&mut self, i: usize, want: i32) -> i32 {
        if !self.alive(i) || want <= 0 {
            return 0;
        }
        let taken = want.min(self.amount[i]).max(0);
        self.amount[i] -= taken;
        self.touch(i);
        taken
    }

    pub fn harvest_per_trip(&self, i: usize) -> i32 {
        usize::try_from(self.unit_type(i))
            .ok()
            .and_then(|t| self.cat.units.get(t))
            .map_or(0, |def| def.harvest_amount)
    }

    pub fn class_name(&self, i: usize) -> &'static str {
        match self.kind.get(i).copied() {
            Some(KIND_MINERAL) => "mineral",
            Some(KIND_GEYSER) => "geyser",
            _ => "actor",
        }
    }

    pub fn econ_value(&self, key: &str) -> Option<i64> {
        let e = &self.cat.econ;
        match key {
            "workers_per_mineral" => Some(i64::from(e.workers_per_mineral)),
            "workers_per_extractor" => Some(i64::from(e.workers_per_extractor)),
            "gas_per_trip" => Some(i64::from(e.gas_per_trip)),
            _ => None,
        }
    }

    pub fn weapon_cooldown(&self, i: usize) -> i32 {
        self.cooldown.get(i).copied().unwrap_or(-1)
    }

    pub fn kills_of(&self, i: usize) -> u32 {
        self.unit_kills.get(i).copied().unwrap_or(0)
    }

    pub fn deaths_of(&self, i: usize) -> u32 {
        self.unit_deaths.get(i).copied().unwrap_or(0)
    }

    pub fn ability_cooldown(&self, i: usize, slot: usize) -> i32 {
        if slot >= ABILITY_SLOTS || i >= self.cap {
            return -1;
        }
        self.abil_cd[i * ABILITY_SLOTS + slot]
    }

    pub fn set_ability_cooldown(&mut self, i: usize, slot: usize, ticks: i32) {
        if slot < ABILITY_SLOTS && i < self.cap {
            self.abil_cd[i * ABILITY_SLOTS + slot] = ticks;
            self.touch(i);
        }
    }

    pub fn shield_of(&self, i: usize) -> i32 {
        self.shield.get(i).copied().unwrap_or(0)
    }

    pub fn hp(&self, i: usize) -> i32 {
        if i >= self.n { 0 } else { self.hp[i] }
    }

    pub fn hp_max(&self, i: usize) -> i32 {
        if i >= self.n { 0 } else { self.hp_max[i] }
    }

    pub fn set_health(&mut self, i: usize, hp: i32, hp_max: i32) -> bool {
        if !self.alive(i) {
            return false;
        }
        self.hp[i] = hp;
        self.hp_max[i] = hp_max;
        true
    }

    pub fn unit_type(&self, i: usize) -> i32 {
        self.type_of.get(i).copied().unwrap_or(-1)
    }

    pub fn player(&self, i: usize) -> u8 {
        if i >= self.n { 0 } else { self.player[i] }
    }

    pub fn kind_of(&self, i: usize) -> u32 {
        self.kind.get(i).map_or(0, |&k| u32::from(k))
    }

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

    pub(crate) fn touch(&mut self, i: usize) {
        if let Some(c) = self.changed_at.get_mut(i) {
            *c = self.ticks.wrapping_add(1);
        }
    }

    pub fn changed_count_since(&self, since: u32) -> usize {
        (0..self.n).filter(|&i| self.changed_at[i] > since).count()
    }

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

    pub fn order_log_len(&self) -> usize {
        self.log_len
    }

    pub fn order_log_dropped(&self) -> u64 {
        self.log_dropped
    }

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

    pub fn remove_unit(&mut self, i: usize) -> bool {
        if !self.alive(i) {
            return false;
        }
        self.alive[i] = 0;
        self.touch(i);
        self.rebucket();
        true
    }

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

pub(crate) fn cell_of(world: i32) -> i32 {
    fixed::to_int(world)
}

pub(crate) fn dist2(ax: i32, ay: i32, bx: i32, by: i32) -> i64 {
    let dx = ax.wrapping_sub(bx) as i64;
    let dy = ay.wrapping_sub(by) as i64;
    dx * dx + dy * dy
}
