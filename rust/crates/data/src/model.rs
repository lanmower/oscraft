//! Records as Starlark builtins collect them (`*Raw`), and the validated
//! records the encoder writes (no suffix).

use std::cell::RefCell;

use starlark::any::ProvidesStaticType;

#[derive(Clone, Copy, Debug)]
pub struct EffectRaw {
    pub kind: usize,
    pub params: [i32; 4],
}

#[derive(Debug)]
pub struct WeaponRaw {
    pub damage: i32,
    pub range_milli: i32,
    pub period: i32,
}

#[derive(Debug)]
pub struct UnitRaw {
    pub name: String,
    pub health: i32,
    pub speed_milli: i32,
    pub radius_milli: i32,
    pub build_time: i32,
    pub minerals: i32,
    pub gas: i32,
    pub supply: i32,
    pub abilities: Vec<String>,
    pub filters: u32,
    pub product: Option<String>,
    pub weapon: Option<WeaponRaw>,
    pub sight_milli: i32,
    pub shield: i32,
    pub armor: i32,
    pub damage_response: usize,
    pub resource_kind: usize,
    pub harvest: i32,
    pub shield_regen: i32,
    pub regen_delay: i32,
    pub health_regen: i32,
}

#[derive(Debug)]
pub struct UnitExtRaw {
    pub name: String,
    pub supply_provided: i32,
    pub transport_capacity: i32,
    pub size: usize,
    pub damage_type: usize,
    pub cloak: u8,
    pub detector_radius: i32,
    pub ranged_miss: i32,
    pub burrow_ticks: i32,
    pub rally_dx: i32,
    pub rally_dy: i32,
}

#[derive(Debug)]
pub struct ChannelRaw {
    pub ability: String,
    pub duration: i32,
}

#[derive(Debug, Default)]
pub struct EconRaw {
    pub patch_saturation: i32,
    pub gas_saturation: i32,
    pub patches_per_base: i32,
    pub geysers_per_base: i32,
    pub extractor_yield: i32,
    pub opening_minerals: i32,
    pub opening_gas: i32,
    pub opening_workers: i32,
    pub supply_base: i32,
    pub supply_max: i32,
    pub patch_yield: i32,
    pub geyser_yield: i32,
}

#[derive(Debug)]
pub struct ResearchRaw {
    pub name: String,
    pub building: String,
    pub time: i32,
    pub minerals: i32,
    pub gas: i32,
    pub effect: usize,
    pub applies: u32,
}

#[derive(Debug)]
pub struct PrereqRaw {
    pub target_kind: usize,
    pub target: String,
    pub requires_kind: usize,
    pub requires: String,
}

#[derive(Debug)]
pub struct DamageFactorRaw {
    pub damage_type: usize,
    pub size: usize,
    pub factor: i32,
}

#[derive(Debug)]
pub struct AbilityRaw {
    pub name: String,
    pub cooldown: i32,
    pub energy: i32,
    pub target: usize,
    pub target_filters: u32,
    pub effects: Vec<usize>,
    pub projectile_speed: Option<i32>,
}

#[derive(Debug)]
pub struct BehaviourRaw {
    pub name: String,
    pub trigger: usize,
    pub action: usize,
    pub param: i32,
    pub ability: Option<String>,
}

#[derive(Debug)]
pub struct SpawnRaw {
    pub unit: Option<String>,
    pub player: i32,
    pub x: i32,
    pub y: i32,
    pub start: bool,
}

#[derive(Debug)]
pub struct UpgradeRaw {
    pub name: String,
    pub target_kind: usize,
    pub target: String,
    pub level: i32,
    pub apply: usize,
    pub stacking: usize,
    pub effect: usize,
}

/// Map grid, built by `grid()` and painted by `paint()`. Also the validated form.
/// Each cell byte is a walkability bitfield (see `format::CELL_*`).
#[derive(Debug, Default)]
pub struct Grid {
    pub width: u32,
    pub height: u32,
    pub cell_milli: u32,
    pub height_step: u32,
    pub cells: Vec<u8>,
    pub heights: Vec<u8>,
}

/// Lives in `Evaluator::extra`; builtins push into it while the .star files run.
/// Effect handles returned by `effect()` are indices into `effects`.
#[derive(Default, ProvidesStaticType)]
pub struct Collector {
    pub effects: RefCell<Vec<EffectRaw>>,
    pub units: RefCell<Vec<UnitRaw>>,
    pub abilities: RefCell<Vec<AbilityRaw>>,
    pub behaviours: RefCell<Vec<BehaviourRaw>>,
    pub spawns: RefCell<Vec<SpawnRaw>>,
    pub upgrades: RefCell<Vec<UpgradeRaw>>,
    pub cell_limit: RefCell<Option<i32>>,
    pub unit_ext: RefCell<Vec<UnitExtRaw>>,
    pub channels: RefCell<Vec<ChannelRaw>>,
    pub economy: RefCell<Option<EconRaw>>,
    pub researches: RefCell<Vec<ResearchRaw>>,
    pub prerequisites: RefCell<Vec<PrereqRaw>>,
    pub damage_factors: RefCell<Vec<DamageFactorRaw>>,
    pub grid: RefCell<Option<Grid>>,
}

#[derive(Clone, Copy, Debug)]
pub struct Effect {
    pub kind: u16,
    pub params: [i32; 4],
}

#[derive(Clone, Copy, Debug)]
pub struct Weapon {
    pub damage: i32,
    pub range: u32,
    pub period: u32,
}

#[derive(Debug)]
pub struct Unit {
    pub name: String,
    pub health: u32,
    pub speed_milli: u32,
    pub radius_milli: u32,
    pub build_time: u32,
    pub minerals: u32,
    pub gas: u32,
    pub supply: u16,
    pub ability_ids: Vec<u16>,
    pub filters: u32,
    pub product: u16,
    pub weapon: Option<Weapon>,
    pub sight: u32,
    pub shield: u32,
    pub armor: i32,
    pub damage_response: u8,
    pub resource_kind: u8,
    pub harvest: u32,
    pub shield_regen: u32,
    pub regen_delay: u32,
    pub health_regen: u32,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct UnitExt {
    pub supply_provided: u16,
    pub transport_capacity: u16,
    pub size: u8,
    pub damage_type: u8,
    pub cloak: u8,
    pub detector_radius: u32,
    pub ranged_miss: u32,
    pub burrow_ticks: u32,
    pub rally_dx: i32,
    pub rally_dy: i32,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Channel {
    pub flags: u16,
    pub duration: u32,
}

#[derive(Debug, Default)]
pub struct Econ {
    pub patch_saturation: u8,
    pub gas_saturation: u8,
    pub patches_per_base: u8,
    pub geysers_per_base: u8,
    pub extractor_yield: u32,
    pub opening_minerals: u32,
    pub opening_gas: u32,
    pub opening_workers: u16,
    pub supply_base: u16,
    pub supply_max: u16,
    pub patch_yield: u32,
    pub geyser_yield: u32,
}

#[derive(Debug)]
pub struct Research {
    pub name: String,
    pub building: u16,
    pub time: u32,
    pub minerals: u32,
    pub gas: u32,
    pub effect: Effect,
    pub applies: u32,
}

#[derive(Debug)]
pub struct Prereq {
    pub target_kind: u8,
    pub target: u16,
    pub requires_kind: u8,
    pub requires: u16,
}

#[derive(Debug)]
pub struct DamageFactor {
    pub damage_type: u8,
    pub size: u8,
    pub factor: u32,
}

#[derive(Debug)]
pub struct Ability {
    pub name: String,
    pub cooldown: u32,
    pub energy: u32,
    pub target: u8,
    pub target_filters: u16,
    pub effects: Vec<Effect>,
    pub projectile_speed: Option<u32>,
}

#[derive(Debug)]
pub struct Behaviour {
    pub name: String,
    pub trigger: u8,
    pub action: u8,
    pub param: i32,
    pub ability: u16,
}

#[derive(Debug)]
pub struct Spawn {
    pub unit: u16,
    pub player: u16,
    pub x: u32,
    pub y: u32,
    pub flags: u32,
}

#[derive(Debug)]
pub struct Upgrade {
    pub name: String,
    pub target_kind: u8,
    pub target: u16,
    pub level: u8,
    pub apply: u8,
    pub stacking: u8,
    pub effect: Effect,
}

#[derive(Debug, Default)]
pub struct Catalog {
    pub units: Vec<Unit>,
    pub unit_ext: Vec<UnitExt>,
    pub abilities: Vec<Ability>,
    pub channels: Vec<Channel>,
    pub behaviours: Vec<Behaviour>,
    pub spawns: Vec<Spawn>,
    pub upgrades: Vec<Upgrade>,
    pub cell_limit: u32,
    pub econ: Econ,
    pub researches: Vec<Research>,
    pub prereqs: Vec<Prereq>,
    pub damage_factors: Vec<DamageFactor>,
    pub grid: Grid,
}
