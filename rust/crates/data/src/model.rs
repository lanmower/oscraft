//! Records as Starlark builtins collect them (`*Raw`), and the validated
//! records the encoder writes (no suffix).

use std::cell::RefCell;

use starlark::any::ProvidesStaticType;

#[derive(Clone, Copy, Debug)]
pub struct EffectRaw {
    pub kind: usize,
    pub params: [i32; 3],
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

/// Map grid, built by `grid()` and painted by `paint()`. Also the validated form.
/// Each cell byte is a walkability bitfield (see `format::CELL_*`).
#[derive(Debug, Default)]
pub struct Grid {
    pub width: u32,
    pub height: u32,
    pub cell_milli: u32,
    pub cells: Vec<u8>,
}

/// Lives in `Evaluator::extra`; builtins push into it while the .star files run.
/// Effect handles returned by `effect()` are indices into `effects`.
#[derive(Default, ProvidesStaticType)]
pub struct Collector {
    pub effects: RefCell<Vec<EffectRaw>>,
    pub units: RefCell<Vec<UnitRaw>>,
    pub abilities: RefCell<Vec<AbilityRaw>>,
    pub behaviours: RefCell<Vec<BehaviourRaw>>,
    pub grid: RefCell<Option<Grid>>,
}

#[derive(Clone, Copy, Debug)]
pub struct Effect {
    pub kind: u16,
    pub params: [i32; 3],
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

#[derive(Debug, Default)]
pub struct Catalog {
    pub units: Vec<Unit>,
    pub abilities: Vec<Ability>,
    pub behaviours: Vec<Behaviour>,
    pub grid: Grid,
}
