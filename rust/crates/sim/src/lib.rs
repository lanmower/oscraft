pub mod fixed;

mod arrays;
mod buffs;
mod combat;
mod filters;
mod acquire;
mod fog;
mod fx;
mod healers;
mod hash;
mod loader;
mod movement;
mod persist;
mod pathfinding;
mod astar;
mod projectiles;
mod rules;
mod ranks;
mod status;
mod shields;
mod tick;
mod behaviours;
mod props;
mod limits;
mod inventory;
mod store;
mod point_move;
mod refresh;
mod displace;
mod wall;
mod damage_by;
mod item_form;
mod spell_block;
mod execute;
mod xp;
mod attributes;
mod modapi;
mod diplomacy;
mod spawn_groups;
mod carry;
mod ground;
mod obstacles;
mod facing;
mod aggro;
mod timed_life;
mod morph;
mod order_memory;
mod order_queue;
mod swing;
mod spell;
mod waypoints;

pub use arrays::{Sim, Stats};
pub use carry::carry_from;
pub use ground::{GroundItem, MAX_GROUND_ITEMS};
pub use facing::{angle_diff, angle_of, FACE_TOLERANCE, FULL_TURN};
pub use obstacles::{Obstacle, MAX_OBSTACLES, MAX_OBSTACLE_SIDE};
pub use fx::{FxEvent, Hit, FX_CAP};
pub use loader::{load, load_file, Catalog, Prop, Effect, LoadError, Trigger};
pub use modapi::{effect_kind, stat_code, target_code, AbilitySpec, ItemSpec, UnitSpec, UpgradeSpec, MOD_EFFECTS, STAT_NAMES};
pub use diplomacy::{controller_code, CONTROLLER_COMPUTER, CONTROLLER_HUMAN, CONTROLLER_OPEN, MOD_ALLIANCES, MOD_CONTROLLERS};
pub use pathfinding::Grid;
pub use persist::StateError;
pub use rules::{Refusal, ALL_REFUSALS};
pub use store::{ItemRequest, ItemVerdict, NO_COMPONENT, NO_TAKE, VERDICT_TAKES};
pub use status::STATUS_NAMES;
pub use spell::{spell_type_valid, AbilityExt, SPELL_KIND_MASK, STAT_COUNT};
pub use inventory::ITEM_STATS;

pub const TICKS_PER_SECOND: i32 = 16;

#[cfg(test)]
pub(crate) fn test_pack_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("..").join("scenarios").join(TEST_PACK)
}
#[cfg(test)]
const TEST_PACK: &str = "moba";
pub(crate) const BUCKET_CAP: usize = 8;
pub(crate) const VIS_SCALE: i32 = 2;
pub(crate) const VIS_PERIOD: u32 = 8;
pub const MAX_PLAYERS: usize = 16;
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

pub(crate) const LAYER_GROUND: u8 = 1;
pub(crate) const LAYER_AIR: u8 = 2;

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
pub(crate) const ACTION_FLEE: u8 = 0;
pub(crate) const ACTION_USE_ABILITY: u8 = 1;
