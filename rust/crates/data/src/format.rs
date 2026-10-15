//! Layout constants and registries shared with the sim loader. See FORMAT.md.

pub const MAGIC: [u8; 4] = *b"OSDT";
pub const VERSION: u16 = 3;
pub const HEADER_LEN: usize = 16;
pub const SECTION_ENTRY_LEN: usize = 16;
pub const NO_ABILITY: u16 = u16::MAX;
pub const NO_UNIT: u16 = u16::MAX;
pub const MAX_GRID_CELLS: usize = 1 << 24;

pub const EFFECT_KINDS: &[&str] = &["damage", "heal", "move", "spawn", "buff"];
pub const TARGETS: &[&str] = &["none", "unit", "point"];
pub const TRIGGERS: &[&str] = &["idle", "attacked", "spawned", "ability_ready"];
pub const ACTIONS: &[&str] = &["attack_nearest", "flee", "use_ability"];
pub const ACTION_USE_ABILITY: usize = 2;
pub const FILTERS: &[&str] = &[
    "ground", "air", "armored", "biological", "mechanical", "psionic", "structure", "light", "massive",
    "worker", "army", "building",
];
pub const FILTER_MASK_BITS: u32 = 12;
pub const DAMAGE_RESPONSES: &[&str] = &["normal", "reduced", "immune"];
pub const RESOURCE_KINDS: &[&str] = &["none", "minerals", "gas"];

pub const CELL_BLOCKED: u8 = 1;
pub const CELL_CLIFF: u8 = 2;
pub const CELL_MASK: u8 = CELL_BLOCKED | CELL_CLIFF;
/// `paint(walk=)` names, in bit order: entry `n` sets cell bit `1 << n`.
pub const WALK_FLAGS: &[&str] = &["blocked", "cliff"];

pub const TAG_STRS: [u8; 4] = *b"STRS";
pub const TAG_EFCT: [u8; 4] = *b"EFCT";
pub const TAG_ABIL: [u8; 4] = *b"ABIL";
pub const TAG_UABL: [u8; 4] = *b"UABL";
pub const TAG_UNIT: [u8; 4] = *b"UNIT";
pub const TAG_BHVR: [u8; 4] = *b"BHVR";
pub const TAG_GRID: [u8; 4] = *b"GRID";

pub const STRS_RECORD: usize = 1;
pub const EFCT_RECORD: usize = 16;
pub const ABIL_RECORD: usize = 32;
pub const UABL_RECORD: usize = 2;
pub const UNIT_RECORD: usize = 80;
pub const BHVR_RECORD: usize = 16;
pub const GRID_RECORD: usize = 1;
pub const GRID_HEADER_LEN: usize = 12;

pub fn index_of(list: &[&str], value: &str, what: &str) -> anyhow::Result<usize> {
    list.iter()
        .position(|v| *v == value)
        .ok_or_else(|| anyhow::anyhow!("unknown {what} {value:?}; expected one of {list:?}"))
}
