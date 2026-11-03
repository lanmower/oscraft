use std::cell::RefCell;

use starlark::any::ProvidesStaticType;

use crate::format::{ITEM_COMPONENTS, ITEM_STAT_COUNT, NO_ABILITY};

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
    pub resource_a: i32,
    pub gas: i32,
    pub capacity: i32,
    pub abilities: Vec<String>,
    pub filters: u32,
    pub product: Option<String>,
    pub weapon: Option<WeaponRaw>,
    pub sight_milli: i32,
    pub shield: i32,
    pub defense: i32,
    pub damage_response: usize,
    pub resource_kind: usize,
    pub harvest: i32,
    pub shield_regen: i32,
    pub regen_delay: i32,
    pub health_regen: i32,
    pub model: String,
    pub deny_pct: i32,
}

#[derive(Debug)]
pub struct UnitExtRaw {
    pub name: String,
    pub capacity_provided: i32,
    pub transport_capacity: i32,
    pub size: usize,
    pub damage_type: usize,
    pub cloak: u8,
    pub detector_radius: i32,
    pub ranged_miss: i32,
    pub burrow_ticks: i32,
    pub rally_dx: i32,
    pub rally_dy: i32,
    pub reward: i32,
    pub deny_xp_pct: i32,
    pub player_controlled: bool,
    pub melee: bool,
    pub acquire: i32,
    pub xp_reward: i32,
    pub xp_share_radius_milli: i32,
    pub night_sight_milli: i32,
    pub attack_point_ms: i32,
    pub missile_speed: i32,
    pub backswing_ms: i32,
    pub missile_homing: bool,
    pub missile_arc: i32,
    pub reward_min: i32,
    pub reward_max: i32,
}

#[derive(Debug)]
pub struct ChannelRaw {
    pub ability: String,
    pub duration: i32,
}

#[derive(Debug, Default)]
pub struct TuneRaw {
    pub structure_scan_cells: i32,
    pub teleport_ring_points: i32,
    pub teleport_ring_steps: i32,
    pub best_only: u64,
    pub xp_curve: Vec<i32>,
}

#[derive(Debug, Default)]
pub struct EconRaw {
    pub patch_saturation: i32,
    pub gas_saturation: i32,
    pub patches_per_base: i32,
    pub geysers_per_base: i32,
    pub extractor_yield: i32,
    pub opening_resource_a: i32,
    pub opening_gas: i32,
    pub opening_workers: i32,
    pub capacity_base: i32,
    pub capacity_max: i32,
    pub patch_yield: i32,
    pub geyser_yield: i32,
}

#[derive(Debug)]
pub struct ResearchRaw {
    pub name: String,
    pub building: String,
    pub time: i32,
    pub resource_a: i32,
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
pub struct PropRaw {
    pub id: i32,
    pub kind: usize,
    pub x: i32,
    pub y: i32,
    pub rotation: i32,
    pub scale: i32,
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
    pub initial_cooldown: i32,
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
pub struct RouteRaw {
    pub player: i32,
    pub id: i32,
    pub points: Vec<(i32, i32)>,
}

#[derive(Debug)]
pub struct SpawnRaw {
    pub unit: Option<String>,
    pub player: i32,
    pub x: i32,
    pub y: i32,
    pub start: bool,
    pub team: Option<i32>,
}

#[derive(Debug)]
pub struct ItemRaw {
    pub name: String,
    pub cost: i32,
    pub stats: [i32; ITEM_STAT_COUNT],
    pub ability: Option<String>,
    pub aura: Option<String>,
    pub components: Vec<String>,
    pub flags: u16,
    pub tier: i32,
    pub tags: i32,
    pub charges: i32,
    pub stack_max: i32,
    pub stock_max: i32,
    pub stock_initial: i32,
    pub stock_time_s: i32,
    pub stock_start_s: i32,
}

impl Default for ItemRaw {
    fn default() -> Self {
        ItemRaw {
            name: String::new(),
            cost: 0,
            stats: [0; ITEM_STAT_COUNT],
            ability: None,
            aura: None,
            components: Vec::new(),
            flags: 0,
            tier: 0,
            tags: 0,
            charges: 0,
            stack_max: 0,
            stock_max: 0,
            stock_initial: 0,
            stock_time_s: 0,
            stock_start_s: 0,
        }
    }
}

#[derive(Debug, Default)]
pub struct Grid {
    pub width: u32,
    pub height: u32,
    pub cell_milli: u32,
    pub height_step: u32,
    pub cells: Vec<u8>,
    pub heights: Vec<u8>,
    pub tiles: Vec<u8>,
}

#[derive(Debug, Default, Clone)]
pub struct Names {
    pub stat_names: Vec<String>,
    pub effect_kinds: Vec<String>,
    pub filter_names: Vec<String>,
    pub mod_conds: Vec<String>,
    pub prop_kinds: Vec<String>,
}

#[derive(Default, ProvidesStaticType)]
pub struct Collector {
    pub names: RefCell<Names>,
    pub effects: RefCell<Vec<EffectRaw>>,
    pub units: RefCell<Vec<UnitRaw>>,
    pub abilities: RefCell<Vec<AbilityRaw>>,
    pub behaviours: RefCell<Vec<BehaviourRaw>>,
    pub spawns: RefCell<Vec<SpawnRaw>>,
    pub cell_limit: RefCell<Option<i32>>,
    pub unit_ext: RefCell<Vec<UnitExtRaw>>,
    pub channels: RefCell<Vec<ChannelRaw>>,
    pub economy: RefCell<Option<EconRaw>>,
    pub tuning: RefCell<Option<TuneRaw>>,
    pub researches: RefCell<Vec<ResearchRaw>>,
    pub prerequisites: RefCell<Vec<PrereqRaw>>,
    pub damage_factors: RefCell<Vec<DamageFactorRaw>>,
    pub props: RefCell<Vec<PropRaw>>,
    pub items: RefCell<Vec<ItemRaw>>,
    pub item_tags: RefCell<Vec<String>>,
    pub skills: RefCell<Vec<SkillRaw>>,
    pub ext: RefCell<crate::abilityext::ExtRaw>,
    pub grid: RefCell<Option<Grid>>,
    pub routes: RefCell<Vec<RouteRaw>>,
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
    pub resource_a: u32,
    pub gas: u32,
    pub capacity: u16,
    pub ability_ids: Vec<u16>,
    pub filters: u32,
    pub product: u16,
    pub weapon: Option<Weapon>,
    pub sight: u32,
    pub shield: u32,
    pub defense: i32,
    pub damage_response: u8,
    pub resource_kind: u8,
    pub harvest: u32,
    pub shield_regen: u32,
    pub regen_delay: u32,
    pub health_regen: u32,
    pub model: String,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct UnitExt {
    pub capacity_provided: u16,
    pub transport_capacity: u16,
    pub size: u8,
    pub damage_type: u8,
    pub cloak: u8,
    pub detector_radius: u32,
    pub ranged_miss: u32,
    pub burrow_ticks: u32,
    pub rally_dx: i32,
    pub rally_dy: i32,
    pub reward: u32,
    pub deny_pct: u8,
    pub deny_xp_pct: u8,
    pub player_controlled: bool,
    pub melee: bool,
    pub xp_reward: u32,
    pub xp_share_radius: u32,
    pub acquire: u32,
    pub night_sight: u32,
    pub attack_point: u16,
    pub missile_speed: u16,
    pub bounty_min: u32,
    pub bounty_max: u32,
    pub attack_backswing: u16,
    pub missile_homing: bool,
    pub missile_arc: u8,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Channel {
    pub flags: u16,
    pub duration: u32,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Tune {
    pub structure_scan_cells: u8,
    pub teleport_ring_points: u8,
    pub teleport_ring_steps: u8,
    pub best_only: u64,
    pub xp_curve: Vec<u32>,
}

#[derive(Debug, Default)]
pub struct Econ {
    pub patch_saturation: u8,
    pub gas_saturation: u8,
    pub patches_per_base: u8,
    pub geysers_per_base: u8,
    pub extractor_yield: u32,
    pub opening_resource_a: u32,
    pub opening_gas: u32,
    pub opening_workers: u16,
    pub capacity_base: u16,
    pub capacity_max: u16,
    pub patch_yield: u32,
    pub geyser_yield: u32,
}

#[derive(Debug)]
pub struct Research {
    pub name: String,
    pub building: u16,
    pub time: u32,
    pub resource_a: u32,
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
    pub initial_cooldown: u32,
    pub skill: Option<Skill>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Skill {
    pub ultimate: bool,
    pub max_rank: u8,
    pub gates: [i32; crate::format::SKILL_RANKS],
    pub damage: [i32; crate::format::SKILL_RANKS],
    pub cooldown: [i32; crate::format::SKILL_RANKS],
    pub energy: [i32; crate::format::SKILL_RANKS],
}

#[derive(Debug)]
pub struct SkillRaw {
    pub ability: String,
    pub ultimate: bool,
    pub gates: Vec<i32>,
    pub damage: Vec<i32>,
    pub cooldown: Vec<i32>,
    pub energy: Vec<i32>,
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

#[derive(Debug, Clone)]
pub struct Route {
    pub player: u16,
    pub id: u16,
    pub points: Vec<(u32, u32)>,
}

#[derive(Debug, Default)]
pub struct Prop {
    pub id: u32,
    pub kind: u16,
    pub x: u32,
    pub y: u32,
    pub rotation: u32,
    pub scale: u32,
}

#[derive(Debug)]
pub struct Item {
    pub name: String,
    pub cost: u32,
    pub stats: [i32; ITEM_STAT_COUNT],
    pub ability: u16,
    pub aura: u16,
    pub components: [u16; ITEM_COMPONENTS],
    pub flags: u16,
    pub tier: u8,
    pub tags: u8,
    pub charges: u16,
    pub stack_max: u16,
    pub stock_max: u8,
    pub stock_initial: u8,
    pub stock_time_s: u16,
    pub stock_start_s: u16,
}

impl Default for Item {
    fn default() -> Self {
        Item {
            name: String::new(),
            cost: 0,
            stats: [0; ITEM_STAT_COUNT],
            ability: 0,
            aura: u16::MAX,
            components: [0; ITEM_COMPONENTS],
            flags: 0,
            tier: 0,
            tags: 0,
            charges: 0,
            stack_max: 0,
            stock_max: 0,
            stock_initial: 0,
            stock_time_s: 0,
            stock_start_s: 0,
        }
    }
}

#[derive(Debug, Default)]
pub struct Catalog {
    pub items: Vec<Item>,
    pub units: Vec<Unit>,
    pub unit_ext: Vec<UnitExt>,
    pub abilities: Vec<Ability>,
    pub channels: Vec<Channel>,
    pub ext: Vec<crate::abilityext::AbilityExt>,
    pub behaviours: Vec<Behaviour>,
    pub spawns: Vec<Spawn>,
    pub props: Vec<Prop>,
    pub cell_limit: u32,
    pub econ: Econ,
    pub tune: Tune,
    pub researches: Vec<Research>,
    pub prereqs: Vec<Prereq>,
    pub damage_factors: Vec<DamageFactor>,
    pub grid: Grid,
    pub routes: Vec<Route>,
    pub names: Names,
}
