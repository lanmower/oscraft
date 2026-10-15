//! Layout constants and registries shared with the sim loader. See FORMAT.md.

pub const MAGIC: [u8; 4] = *b"OSDT";
pub const VERSION: u16 = 5;
pub const HEADER_LEN: usize = 16;
pub const SECTION_ENTRY_LEN: usize = 16;
pub const NO_ABILITY: u16 = u16::MAX;
pub const NO_UNIT: u16 = u16::MAX;
pub const MAX_GRID_CELLS: usize = 1 << 24;

pub const EFFECT_KINDS: &[&str] = &[
    "damage",
    "heal",
    "move",
    "spawn",
    "buff",
    "issue_order",
    "remove_behaviour",
    "create_healer",
    "use_calldown",
    "vital_fraction",
    "shield_bonus",
];
pub const MAX_EFFECT_KIND: usize = 10;
pub const ORDER_KINDS: &[&str] = &["stop", "attack_move", "hold"];
pub const ORDER_TARGETS: &[&str] = &["caster_relative", "point"];
pub const VITALS: &[&str] = &["life", "shield"];
pub const VITAL_MODES: &[&str] = &["drain", "set"];
pub const VITAL_TARGETS: &[&str] = &["current", "max"];
pub const Q16_ONE: i32 = 65536;

pub const TARGETS: &[&str] = &["none", "unit", "point"];
pub const TRIGGERS: &[&str] = &["idle", "attacked", "spawned", "ability_ready"];
pub const ACTIONS: &[&str] = &["attack_nearest", "flee", "use_ability"];
pub const ACTION_USE_ABILITY: usize = 2;
pub const FILTERS: &[&str] = &[
    "ground", "air", "armored", "biological", "mechanical", "psionic", "structure", "light", "massive",
    "worker", "army", "building",
];
pub const FILTER_MASK_BITS: u32 = 12;
pub const FILTER_ALL: u32 = (1 << FILTER_MASK_BITS) - 1;
pub const DAMAGE_RESPONSES: &[&str] = &["normal", "reduced", "immune"];
pub const RESOURCE_KINDS: &[&str] = &["none", "minerals", "gas"];

pub const CELL_BLOCKED: u8 = 1;
pub const CELL_CLIFF: u8 = 2;
pub const CELL_MASK: u8 = CELL_BLOCKED | CELL_CLIFF;

pub const TAG_STRS: [u8; 4] = *b"STRS";
pub const TAG_EFCT: [u8; 4] = *b"EFCT";
pub const TAG_ABIL: [u8; 4] = *b"ABIL";
pub const TAG_UABL: [u8; 4] = *b"UABL";
pub const TAG_UNIT: [u8; 4] = *b"UNIT";
pub const TAG_BHVR: [u8; 4] = *b"BHVR";
pub const TAG_GRID: [u8; 4] = *b"GRID";

pub const STRS_RECORD: usize = 1;
pub const EFCT_RECORD: usize = 20;
pub const ABIL_RECORD: usize = 32;
pub const UABL_RECORD: usize = 2;
pub const UNIT_RECORD: usize = 80;
pub const BHVR_RECORD: usize = 16;
pub const GRID_RECORD: usize = 1;
pub const GRID_HEADER_LEN: usize = 12;

/// Section tags and record sizes in the order FORMAT.md requires.
pub const SECTIONS: [([u8; 4], usize); 7] = [
    (TAG_STRS, STRS_RECORD),
    (TAG_EFCT, EFCT_RECORD),
    (TAG_ABIL, ABIL_RECORD),
    (TAG_UABL, UABL_RECORD),
    (TAG_UNIT, UNIT_RECORD),
    (TAG_BHVR, BHVR_RECORD),
    (TAG_GRID, GRID_RECORD),
];

pub fn index_of(list: &[&str], value: &str, what: &str) -> anyhow::Result<usize> {
    list.iter()
        .position(|v| *v == value)
        .ok_or_else(|| anyhow::anyhow!("unknown {what} {value:?}; expected one of {list:?}"))
}

/// Per-kind parameter ranges from FORMAT.md's effect table. Shared by the
/// Starlark builtins and the reference reader.
pub fn effect_params_valid(kind: &str, p: [i32; 4]) -> bool {
    let [p0, p1, p2, p3] = p;
    match kind {
        "damage" | "heal" => p0 >= 0 && p1 >= 0,
        "buff" => p0 >= 0 && p1 >= 0,
        "spawn" => p0 >= 0,
        "issue_order" => (0..ORDER_KINDS.len() as i32).contains(&p0) && (0..ORDER_TARGETS.len() as i32).contains(&p1),
        "remove_behaviour" => p0 >= 0 && p1 >= 0,
        "create_healer" => p0 >= 0 && p1 >= 0 && p2 >= 1 && p3 >= 1,
        "use_calldown" => p0 >= 0,
        "vital_fraction" => {
            (0..VITALS.len() as i32).contains(&p0)
                && (0..=Q16_ONE).contains(&p1)
                && (0..VITAL_MODES.len() as i32).contains(&p2)
                && (0..VITAL_TARGETS.len() as i32).contains(&p3)
        }
        "shield_bonus" => p0 >= 0,
        _ => true,
    }
}
