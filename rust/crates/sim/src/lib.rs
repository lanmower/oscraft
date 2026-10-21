//! Deterministic headless simulation. Port of internal/sim (Go).
//!
//! Q16.16 fixed point in i32, struct-of-arrays, and no heap allocation inside
//! `Sim::tick`: every buffer is sized in `Sim::new` or `load`. No floats, no
//! maps, no wall clock. The data file is the OSDT layout in
//! rust/crates/data/FORMAT.md, read by `load`.

pub mod fixed;

mod arrays;
mod buffs;
mod combat;
mod filters;
mod acquire;
mod fog;
mod fx;
mod harvest;
mod healers;
mod hash;
mod loader;
mod movement;
mod persist;
mod pathfinding;
mod astar;
mod production;
mod projectiles;
mod rules;
mod skills;
mod status;
mod shields;
mod tick;
mod transport;
mod triggers;
mod doodads;
mod limits;
mod upgrades;
mod inventory;
mod store;
mod teleport;
mod refresh;
mod grant;
mod xp;
mod attributes;
mod modapi;
mod diplomacy;
mod groups;
mod carry;
mod aggro;
mod wards;
mod carrier;
mod spell;
mod waves;
mod route;

pub use arrays::{Sim, Stats};
pub use carry::carry_from;
pub use fx::{FxEvent, Hit, FX_CAP};
pub use loader::{load, load_file, Catalog, Doodad, Effect, LoadError, Trigger};
pub use modapi::{effect_kind, stat_code, target_code, AbilitySpec, ItemSpec, UnitSpec, UpgradeSpec, MOD_EFFECTS, STAT_NAMES};
pub use diplomacy::{controller_code, CONTROLLER_COMPUTER, CONTROLLER_HUMAN, CONTROLLER_OPEN, MOD_ALLIANCES, MOD_CONTROLLERS};
pub use pathfinding::Grid;
pub use persist::StateError;
pub use combat::{armour_reduced, Payout};
pub use rules::{Refusal, ALL_REFUSALS};
pub use store::{ItemRequest, ItemVerdict, NO_COMPONENT, NO_TAKE, VERDICT_TAKES};
pub use status::STATUS_NAMES;
pub use spell::{AbilityExt, STAT_COUNT};
pub use inventory::ITEM_STATS;

pub const TICKS_PER_SECOND: i32 = 16;
pub(crate) const BUCKET_CAP: usize = 8;
pub(crate) const VIS_SCALE: i32 = 2;
pub(crate) const VIS_PERIOD: u32 = 8;
/// Day and night lengths in ticks. UNVERIFIED: 5 minutes each is the recalled reference ruleset cycle, not a sourced value.
pub const DAY_TICKS: u32 = 300 * TICKS_PER_SECOND as u32;
pub const NIGHT_TICKS: u32 = 300 * TICKS_PER_SECOND as u32;
pub const MAX_PLAYERS: usize = 16;
pub(crate) const REVEAL_CAP: usize = 16;
pub(crate) const MISSILE_CAP: usize = 128;
pub(crate) const BUFF_SLOTS: usize = 4;
pub(crate) const HEALER_SLOTS: usize = 16;
pub const ABILITY_SLOTS: usize = 8;
pub const ITEM_SLOTS: usize = inventory::INV_SLOTS;
pub(crate) const AREA_BUF: usize = 289 * BUCKET_CAP;
pub(crate) const ORDER_LOG_CAP: usize = 4096;

pub(crate) const KIND_ACTOR: u8 = 0;
pub(crate) const KIND_MINERAL: u8 = 1;
pub(crate) const KIND_GEYSER: u8 = 2;
pub(crate) const KIND_WORKER: u8 = 3;
pub(crate) const KIND_BASE: u8 = 4;
pub(crate) const KIND_PRODUCER: u8 = 5;

pub(crate) const LAYER_GROUND: u8 = 1;
pub(crate) const LAYER_AIR: u8 = 2;

pub(crate) const W_SEEK: u8 = 0;
pub(crate) const W_GATHER: u8 = 1;
pub(crate) const W_RETURN: u8 = 2;
pub(crate) const W_DEPOSIT: u8 = 3;

pub(crate) const GATHER_TICKS: i32 = 32;
pub(crate) const DEPOSIT_TICKS: i32 = 8;
pub(crate) const ARRIVE_SLACK: i32 = fixed::ONE / 2;
pub(crate) const RETARGET_BUDGET: i32 = 64;
pub(crate) const MELEE_RANGE: i32 = fixed::ONE / 2;

pub(crate) const ORDER_MOVE: u8 = 0;
pub(crate) const ORDER_STOP: u8 = 1;
pub const ORDER_ATTACK_MOVE: u8 = 2;
pub(crate) const ORDER_PATROL: u8 = 3;
pub const ORDER_HOLD: u8 = 4;

pub(crate) const TRIGGER_IDLE: u8 = 0;
pub(crate) const TRIGGER_ATTACKED: u8 = 1;
pub(crate) const TRIGGER_SPAWNED: u8 = 2;
pub(crate) const TRIGGER_ABILITY_READY: u8 = 3;
pub(crate) const TRIGGER_DESTROYED: u8 = 4;
pub(crate) const ACTION_ATTACK_NEAREST: u8 = 0;
pub(crate) const ACTION_FLEE: u8 = 1;
pub(crate) const ACTION_USE_ABILITY: u8 = 2;
