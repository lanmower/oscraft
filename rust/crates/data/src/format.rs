//! Layout constants and registries shared with the sim loader. See FORMAT.md.

pub const MAGIC: [u8; 4] = *b"OSDT";
pub const VERSION: u16 = 18;
pub const PLCE_FLAG_START: u32 = 1;
/// A start record that names its team: the team index sits in bits 8..16 of the flags.
pub const PLCE_FLAG_TEAM: u32 = 2;
pub const PLCE_TEAM_SHIFT: u32 = 8;
pub const PLCE_TEAM_MASK: u32 = 0xFF << PLCE_TEAM_SHIFT;
pub const NEUTRAL_PLAYER: u16 = u16::MAX;
pub const HEADER_LEN: usize = 16;
pub const SECTION_ENTRY_LEN: usize = 16;
pub const NO_ABILITY: u16 = u16::MAX;
pub const NO_UNIT: u16 = u16::MAX;
pub const MAX_GRID_CELLS: usize = 1 << 24;
pub const MAX_PLAYERS: i32 = 16;

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
pub const TRIGGERS: &[&str] = &["idle", "attacked", "spawned", "ability_ready", "destroyed"];
pub const ACTIONS: &[&str] = &["attack_nearest", "flee", "use_ability", "declare_winner"];
pub const ACTION_USE_ABILITY: usize = 2;
pub const ACTION_DECLARE_WINNER: usize = 3;
pub const FILTERS: &[&str] = &[
    "ground", "air", "armored", "biological", "mechanical", "psionic", "structure", "light", "massive",
    "worker", "army", "building",
];
pub const FILTER_MASK_BITS: u32 = 12;
pub const FILTER_ALL: u32 = (1 << FILTER_MASK_BITS) - 1;
pub const DAMAGE_RESPONSES: &[&str] = &["normal", "reduced", "immune"];
pub const RESOURCE_KINDS: &[&str] = &["none", "resource_a", "gas"];
pub const UPGRADE_TARGET_KINDS: &[&str] = &["unit", "ability"];
pub const UPGRADE_APPLY: &[&str] = &["new_only", "existing"];
pub const UPGRADE_STACKING: &[&str] = &["replace", "add", "refuse"];
pub const DAMAGE_TYPES: &[&str] = &["normal", "concussive", "explosive"];
pub const SIZE_CATEGORIES: &[&str] = &["small", "medium", "large"];
pub const CLOAK_FLAGS: &[&str] = &["cloaked", "burrowable", "detector_required"];
pub const CLOAK_MASK: u8 = 0b111;
pub const REQ_KINDS: &[&str] = &["unit", "research"];
pub const CHANNEL_FLAG: u16 = 1;

pub const CELL_BLOCKED: u8 = 1;
pub const CELL_CLIFF: u8 = 2;
pub const CELL_MASK: u8 = CELL_BLOCKED | CELL_CLIFF;

pub const TAG_STRS: [u8; 4] = *b"STRS";
pub const TAG_EFCT: [u8; 4] = *b"EFCT";
pub const TAG_ABIL: [u8; 4] = *b"ABIL";
pub const TAG_UABL: [u8; 4] = *b"UABL";
pub const TAG_UNIT: [u8; 4] = *b"UNIT";
pub const TAG_BHVR: [u8; 4] = *b"BHVR";
pub const TAG_PLCE: [u8; 4] = *b"PLCE";
pub const TAG_UPGR: [u8; 4] = *b"UPGR";
pub const TAG_REGN: [u8; 4] = *b"REGN";
pub const TAG_LIMT: [u8; 4] = *b"LIMT";
pub const TAG_ECON: [u8; 4] = *b"ECON";
pub const TAG_UEXT: [u8; 4] = *b"UEXT";
pub const TAG_ACHN: [u8; 4] = *b"ACHN";
pub const TAG_RSCH: [u8; 4] = *b"RSCH";
pub const TAG_PREQ: [u8; 4] = *b"PREQ";
pub const TAG_DMGF: [u8; 4] = *b"DMGF";
pub const TAG_DODA: [u8; 4] = *b"DODA";
pub const TAG_GRID: [u8; 4] = *b"GRID";
pub const TAG_ITEM: [u8; 4] = *b"ITEM";
pub const TAG_SKIL: [u8; 4] = *b"SKIL";

pub const STRS_RECORD: usize = 1;
pub const EFCT_RECORD: usize = 20;
pub const ABIL_RECORD: usize = 32;
pub const UABL_RECORD: usize = 2;
pub const UNIT_RECORD: usize = 86;
pub const BHVR_RECORD: usize = 16;
pub const PLCE_RECORD: usize = 16;
pub const UPGR_RECORD: usize = 24;
pub const REGN_RECORD: usize = 12;
pub const LIMT_RECORD: usize = 4;
pub const ECON_RECORD: usize = 32;
pub const UEXT_RECORD: usize = 44;
pub const ACHN_RECORD: usize = 8;
pub const RSCH_RECORD: usize = 32;
pub const PREQ_RECORD: usize = 8;
pub const DMGF_RECORD: usize = 8;
pub const DODA_RECORD: usize = 20;
pub const GRID_RECORD: usize = 1;
pub const ITEM_RECORD: usize = 32;
pub const SKIL_RECORD: usize = 72;
/// Ranks a skill tree may hold: an ability's max rank is `1..=SKILL_RANKS`.
pub const SKILL_RANKS: usize = 4;
/// Ultimates take at most this many ranks, each gated by its level in `ULTIMATE_GATES`.
pub const ULTIMATE_RANKS: usize = 3;
pub const ULTIMATE_GATES: [i32; ULTIMATE_RANKS] = [6, 12, 18];
/// Level gate of a non-ultimate rank: any level.
pub const NORMAL_GATE: i32 = 1;
/// Inventory slots each hero carries.
pub const ITEM_SLOTS: usize = 6;
pub const DOODAD_KINDS: &[&str] = &["rock", "tree", "crystal", "debris", "base", "shop", "fountain", "lane", "camp"];
pub const GRID_HEADER_LEN: usize = 16;

/// Section tags and record sizes in the order FORMAT.md requires.
pub const SECTIONS: [([u8; 4], usize); 20] = [
    (TAG_STRS, STRS_RECORD),
    (TAG_EFCT, EFCT_RECORD),
    (TAG_ABIL, ABIL_RECORD),
    (TAG_UABL, UABL_RECORD),
    (TAG_UNIT, UNIT_RECORD),
    (TAG_BHVR, BHVR_RECORD),
    (TAG_PLCE, PLCE_RECORD),
    (TAG_UPGR, UPGR_RECORD),
    (TAG_REGN, REGN_RECORD),
    (TAG_LIMT, LIMT_RECORD),
    (TAG_ECON, ECON_RECORD),
    (TAG_UEXT, UEXT_RECORD),
    (TAG_ACHN, ACHN_RECORD),
    (TAG_RSCH, RSCH_RECORD),
    (TAG_PREQ, PREQ_RECORD),
    (TAG_DMGF, DMGF_RECORD),
    (TAG_DODA, DODA_RECORD),
    (TAG_GRID, GRID_RECORD),
    (TAG_ITEM, ITEM_RECORD),
];

pub fn index_of(list: &[&str], value: &str, what: &str) -> anyhow::Result<usize> {
    list.iter()
        .position(|v| *v == value)
        .ok_or_else(|| anyhow::anyhow!("unknown {what} {value:?}; expected one of {list:?}"))
}

/// Per-kind parameter ranges from FORMAT.md's effect table. Shared by the
/// Starlark builtins and the reference reader. The `move` cap is bounded by
/// the grid, so it is checked by `move_cap_fits_grid` instead.
pub fn effect_params_valid(kind: &str, p: [i32; 4]) -> bool {
    let [p0, p1, p2, p3] = p;
    match kind {
        "damage" | "heal" => p0 >= 0 && p1 >= 0,
        "move" => p0 > 0,
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

/// True when a `move` cap is in `1..=diagonal`, where the diagonal is
/// `sqrt((width * cell)^2 + (height * cell)^2)` in milli-units. Compares squares
/// in integers, so no float is read.
pub fn move_cap_fits_grid(cap: i32, width: u32, height: u32, cell_milli: u32) -> bool {
    if cap <= 0 {
        return false;
    }
    let cap = cap as u128;
    let w = width as u128 * cell_milli as u128;
    let h = height as u128 * cell_milli as u128;
    cap * cap <= w * w + h * h
}

/// True when a placement lies on the map: `0 <= x < width * cell` and `0 <= y < height * cell`.
pub fn position_on_grid(x: i64, y: i64, width: u32, height: u32, cell_milli: u32) -> bool {
    let extent_x = width as i64 * cell_milli as i64;
    let extent_y = height as i64 * cell_milli as i64;
    (0..extent_x).contains(&x) && (0..extent_y).contains(&y)
}
