//! Layout constants and registries shared with the sim loader. See FORMAT.md.

pub const MAGIC: [u8; 4] = *b"OSDT";
pub const VERSION: u16 = 25;
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

// Effect kind names, stat names, filter names and doodad kinds are scenario data: see the *_names and
// doodad_kinds sections (SNAM, EKND, FNAM, DKND).
/// Status codes of the `status` effect (and the sim's status counters): the first five have their own effect kinds.
pub const STATUS_CODES: &[&str] = &[
    "stun", "slow", "silence", "root", "invisibility", "taunt", "hex", "disarm", "spell_immunity", "invulnerable", "break", "mute",
    "physical_immunity", "death_prevention",
];
/// Damage types of the `damage`, `dot` and `chain` effects: 0 is the legacy armour-free hit.
pub const SPELL_DAMAGE_TYPES: &[&str] = &["legacy", "physical", "magical", "pure"];
pub const ORDER_KINDS: &[&str] = &["stop", "attack_move", "hold"];
pub const ORDER_TARGETS: &[&str] = &["caster_relative", "point"];
pub const VITALS: &[&str] = &["life", "shield"];
pub const VITAL_MODES: &[&str] = &["drain", "set"];
pub const VITAL_TARGETS: &[&str] = &["current", "max"];
pub const Q16_ONE: i32 = 65536;

pub const TARGETS: &[&str] = &["none", "unit", "point"];
pub const TRIGGERS: &[&str] = &["idle", "attacked", "spawned", "ability_ready", "destroyed"];
// The sim queues each fired behaviour with its action code; the pack chooses what the action does.
pub const ACTIONS: &[&str] = &["flee", "use_ability"];
pub const ACTION_USE_ABILITY: usize = 1;
pub const FILTER_MASK_BITS: u32 = 15;
pub const FILTER_ALL: u32 = (1 << FILTER_MASK_BITS) - 1;
pub const DAMAGE_RESPONSES: &[&str] = &["normal", "reduced", "immune"];
pub const RESOURCE_KINDS: &[&str] = &["none", "resource_a", "gas"];
pub const DAMAGE_TYPES: &[&str] = &["normal", "concussive", "explosive"];
pub const SIZE_CATEGORIES: &[&str] = &["small", "medium", "large"];
pub const CLOAK_FLAGS: &[&str] = &["cloaked", "burrowable", "detector_required"];
pub const CLOAK_MASK: u8 = 0b111;
pub const REQ_KINDS: &[&str] = &["unit", "research"];
pub const CHANNEL_FLAG: u16 = 1;

pub const CELL_BLOCKED: u8 = 1;
pub const CELL_CLIFF: u8 = 2;
// A ramp cell is walkable and drawn with the slope it belongs to (6.88 W3E ramp flag, bit 0x10 of the ground byte).
pub const CELL_RAMP: u8 = 4;
pub const CELL_MASK: u8 = CELL_BLOCKED | CELL_CLIFF | CELL_RAMP;

pub const TAG_STRS: [u8; 4] = *b"STRS";
pub const TAG_EFCT: [u8; 4] = *b"EFCT";
pub const TAG_ABIL: [u8; 4] = *b"ABIL";
pub const TAG_UABL: [u8; 4] = *b"UABL";
pub const TAG_UNIT: [u8; 4] = *b"UNIT";
pub const TAG_BHVR: [u8; 4] = *b"BHVR";
pub const TAG_PLCE: [u8; 4] = *b"PLCE";
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
pub const TAG_AXTN: [u8; 4] = *b"AXTN";
pub const TAG_ROUT: [u8; 4] = *b"ROUT";
pub const TAG_TUNE: [u8; 4] = *b"TUNE";
/// Name-list sections (scenario data): stat slots, effect kinds, filter bits, mod conditions, doodad kinds.
pub const TAG_SNAM: [u8; 4] = *b"SNAM";
pub const TAG_EKND: [u8; 4] = *b"EKND";
pub const TAG_FNAM: [u8; 4] = *b"FNAM";
pub const TAG_MCND: [u8; 4] = *b"MCND";
pub const TAG_DKND: [u8; 4] = *b"DKND";
/// A name-list section is a byte stream: each name is one length byte (1..=255) then its UTF-8 bytes.
/// Its count is the byte length. Index of a name is its place in the list.
pub const NAME_RECORD: usize = 1;
/// Most names one list holds; each name index must fit a u16 in the image.
pub const NAME_LIST_MAX: usize = 1024;

pub const STRS_RECORD: usize = 1;
pub const EFCT_RECORD: usize = 20;
pub const ABIL_RECORD: usize = 32;
pub const UABL_RECORD: usize = 2;
pub const UNIT_RECORD: usize = 86;
pub const BHVR_RECORD: usize = 16;
pub const PLCE_RECORD: usize = 16;
pub const REGN_RECORD: usize = 12;
pub const LIMT_RECORD: usize = 4;
pub const ECON_RECORD: usize = 32;
pub const UEXT_RECORD: usize = 48;
pub const ACHN_RECORD: usize = 8;
pub const RSCH_RECORD: usize = 32;
pub const PREQ_RECORD: usize = 8;
pub const DMGF_RECORD: usize = 8;
pub const DODA_RECORD: usize = 20;
pub const GRID_RECORD: usize = 1;
pub const ITEM_RECORD: usize = 192;
/// Recipe components one item may name.
pub const ITEM_COMPONENTS: usize = 6;
/// Per-item stat bonuses, in record order. Armor, health regeneration and mana regeneration are hundredths,
/// percent stats are whole percents, move speed is milli-units per tick, ranges, distances and widths are
/// milli-units, durations are ticks, the rest are whole numbers.
pub const ITEM_STATS: &[&str] = &[
    "armor", "damage", "health", "mana", "strength", "agility", "intelligence", "move_speed", "attack_speed",
    "health_regen", "mana_regen", "magic_resist", "evasion", "spell_amp", "lifesteal", "spell_lifesteal",
    "cast_range", "attack_range", "status_resist", "cooldown_reduction", "gold_per_min", "xp_per_min",
    "crit_chance", "crit_multiplier", "cleave", "cleave_distance", "cleave_start_width", "cleave_end_width",
    "bash_chance", "bash_duration", "speed_pct", "slow_resist", "hp_regen_amp", "mana_regen_amp",
    "mana_cost_reduction", "night_vision", "scepter", "shard",
];
pub const ITEM_STAT_COUNT: usize = 38;
pub const ITEM_FLAG_CONSUMABLE: u16 = 1;
pub const ITEM_FLAG_NEUTRAL: u16 = 2;
pub const ITEM_FLAG_NO_BUY: u16 = 4;
pub const ITEM_FLAG_NO_SELL: u16 = 8;
pub const ITEM_FLAG_ENHANCEMENT: u16 = 16;
pub const ITEM_FLAG_MASK: u16 = 31;
/// Item tag names a scenario declares with `item_tags([...])`; a tag's bit is its place in that list.
/// The engine attaches no meaning to a tag: the mod reads the bits (scenarios/moba/store.lua).
pub const ITEM_TAG_LIMIT: usize = 8;
pub const NEUTRAL_TIERS: u8 = 5;
pub const SKIL_RECORD: usize = 72;
pub const AXTN_RECORD: usize = 256;
/// Waypoints one route holds.
pub const MAX_ROUTE_POINTS: usize = 64;
/// Route ids: 1 up to this value; the route id of a spawn effect rides the two top bits of its p3.
pub const MAX_ROUTE_ID: i32 = 3;
/// A route record: player u16, route id u16, count u16, reserved u16, then `MAX_ROUTE_POINTS` x/y pairs of u32 milli-units.
pub const ROUT_RECORD: usize = 8 + MAX_ROUTE_POINTS * 8;
/// Tuning record (TUNE, one record): structure scan cells u8, teleport ring points u8, teleport ring steps u8,
/// reserved u8, best-only stat mask u64 (bit k is item stat k of `ITEM_STATS`), then `XP_CURVE_LEN` cumulative xp
/// thresholds of u32, nondecreasing.
pub const TUNE_RECORD: usize = 12 + XP_CURVE_LEN * 4;
pub const XP_CURVE_LEN: usize = 29;
/// Most teleport landing points one ring holds: the eight compass directions.
pub const TELEPORT_RING_MAX_POINTS: u8 = 8;
/// Most cells a structure scan may reach past weapon range.
pub const STRUCTURE_SCAN_MAX_CELLS: u8 = 64;
/// Rank-scaled effect parameters and conditional modifiers one ability may carry.
pub const AXTN_SCALED: usize = 6;
pub const AXTN_MODS: usize = 8;
pub const AXTN_FLAG_MASK: u16 = 1023;
/// Modifier "effect" values from here up name an ability field: cooldown, energy, cast range, cast point.
pub const MOD_FIELD_FIRST: u8 = 250;
/// Ranks a skill tree may hold: an ability's max rank is `1..=SKILL_RANKS`.
pub const SKILL_RANKS: usize = 4;
/// Ultimates take at most this many ranks, each gated by its level in `ULTIMATE_GATES`.
pub const ULTIMATE_RANKS: usize = 3;
pub const ULTIMATE_GATES: [i32; ULTIMATE_RANKS] = [6, 12, 18];
/// Level gate of a non-ultimate rank: any level.
pub const NORMAL_GATE: i32 = 1;
/// Latest level gate of a non-ultimate rank (reference: 1, 3, 5, 7).
pub const MAX_NORMAL_GATE: i32 = 30;
/// Inventory slots each player-controlled unit carries.
pub const ITEM_SLOTS: usize = 6;
pub const GRID_HEADER_LEN: usize = 16;

/// Section tags and record sizes in the order FORMAT.md requires.
pub const SECTIONS: [([u8; 4], usize); 27] = [
    (TAG_STRS, STRS_RECORD),
    (TAG_EFCT, EFCT_RECORD),
    (TAG_ABIL, ABIL_RECORD),
    (TAG_UABL, UABL_RECORD),
    (TAG_UNIT, UNIT_RECORD),
    (TAG_BHVR, BHVR_RECORD),
    (TAG_PLCE, PLCE_RECORD),
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
    (TAG_SKIL, SKIL_RECORD),
    (TAG_AXTN, AXTN_RECORD),
    (TAG_ROUT, ROUT_RECORD),
    (TAG_TUNE, TUNE_RECORD),
    (TAG_SNAM, NAME_RECORD),
    (TAG_EKND, NAME_RECORD),
    (TAG_FNAM, NAME_RECORD),
    (TAG_MCND, NAME_RECORD),
    (TAG_DKND, NAME_RECORD),
];

/// Encodes one name list: each name as a length byte then its bytes.
pub fn encode_name_list(names: &[String]) -> Vec<u8> {
    let mut out = Vec::new();
    for name in names {
        out.push(name.len() as u8);
        out.extend_from_slice(name.as_bytes());
    }
    out
}

/// Decodes one name list. Refuses a length byte of 0, a name that runs past the end, non-UTF-8, or a
/// duplicate name, so an index always names one entry.
pub fn decode_name_list(bytes: &[u8]) -> Result<Vec<String>, String> {
    let mut names: Vec<String> = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let len = bytes[at] as usize;
        if len == 0 {
            return Err(format!("empty name at byte {at}"));
        }
        let text = bytes.get(at + 1..at + 1 + len).ok_or_else(|| format!("name at byte {at} runs past the list"))?;
        let name = std::str::from_utf8(text).map_err(|_| format!("name at byte {at} is not UTF-8"))?;
        if names.iter().any(|n| n == name) {
            return Err(format!("name {name:?} is listed twice"));
        }
        names.push(name.to_owned());
        at += 1 + len;
        if names.len() > NAME_LIST_MAX {
            return Err(format!("more than {NAME_LIST_MAX} names"));
        }
    }
    Ok(names)
}

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
        "damage" => p0 >= 0 && p1 >= 0 && (0..SPELL_DAMAGE_TYPES.len() as i32).contains(&p2),
        "heal" => p0 >= 0 && p1 >= 0 && (0..=2).contains(&p2),
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
        "stat" => p0 >= 0,
        "carry" => true,
        "dot" => p0 >= 0 && p1 >= 1 && p2 >= 1 && (0..SPELL_DAMAGE_TYPES.len() as i32).contains(&p3),
        "status" => (0..STATUS_CODES.len() as i32).contains(&p0) && p1 >= 1,
        "timed_stat" => p0 >= 0 && p2 >= 1 && (0..=1).contains(&p3),
        "aoe" => p0 >= 0 && (0..=31).contains(&p1) && (0..=16).contains(&p2) && p3 >= 0,
        "dispel" => (1..=7).contains(&p0),
        "chain" => p0 >= 0 && p1 >= 0 && p2 >= 0 && (0..SPELL_DAMAGE_TYPES.len() as i32).contains(&p3),
        "zone" => p0 >= 0 && p1 >= 1 && p2 >= 1 && p3 >= 0,
        "illusion" => (1..=8).contains(&p0) && p1 >= 1 && p2 >= 0 && p3 >= 0,
        "if_kill" => (1..=16).contains(&p0),
        "attack" => p0 >= 0,
        "refresh" => (1..=3).contains(&p0),
        "stun" | "silence" | "root" | "invisibility" => p0 >= 1,
        "slow" => p0 >= 1 && (0..=Q16_ONE).contains(&p1),
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
