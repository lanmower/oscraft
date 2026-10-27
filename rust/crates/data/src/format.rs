pub const MAGIC: [u8; 4] = *b"OSDT";
pub const VERSION: u16 = 37;
pub const PLCE_FLAG_START: u32 = 1;
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

pub const STATUS_CODES: &[&str] = &[
    "stun", "slow", "silence", "root", "invisibility", "taunt", "hex", "disarm", "spell_immunity", "invulnerable", "break", "mute",
    "physical_immunity", "death_prevention", "blind", "revealed", "sleep",
];
pub const SPELL_DAMAGE_TYPES: &[&str] = &["legacy", "physical", "magical", "pure"];
pub const SPELL_KIND_BITS: u32 = 2;

pub fn spell_damage_type_valid(value: i32) -> bool {
    value >= 0 && ((value >> SPELL_KIND_BITS) as usize) <= DAMAGE_TYPES.len()
}
pub const ORDER_KINDS: &[&str] = &["stop", "attack_move", "hold"];
pub const ORDER_TARGETS: &[&str] = &["caster_relative", "point"];
pub const VITALS: &[&str] = &["life", "shield"];
pub const VITAL_MODES: &[&str] = &["drain", "set"];
pub const VITAL_TARGETS: &[&str] = &["current", "max"];
pub const Q16_ONE: i32 = 65536;

pub const TARGETS: &[&str] = &["none", "unit", "point", "unit_or_point"];
pub const TRIGGERS: &[&str] = &["idle", "attacked", "spawned", "ability_ready", "destroyed"];
pub const ACTIONS: &[&str] = &["flee", "use_ability"];
pub const ACTION_USE_ABILITY: usize = 1;
pub const FILTER_MASK_BITS: u32 = 15;
pub const FILTER_ALL: u32 = (1 << FILTER_MASK_BITS) - 1;
pub const DAMAGE_RESPONSES: &[&str] = &["normal", "reduced", "immune"];
pub const RESOURCE_KINDS: &[&str] = &["none", "resource_a", "gas"];
pub const DAMAGE_TYPES: &[&str] = &["normal", "concussive", "explosive", "pierce", "siege", "magic", "chaos", "hero", "spells"];
pub const SIZE_CATEGORIES: &[&str] = &["small", "medium", "large", "fort", "normal", "hero", "divine", "unarmored"];
pub const CLOAK_FLAGS: &[&str] = &["cloaked", "burrowable", "detector_required"];
pub const CLOAK_MASK: u8 = 0b111;
pub const REQ_KINDS: &[&str] = &["unit", "research"];
pub const CHANNEL_FLAG: u16 = 1;

pub const CELL_BLOCKED: u8 = 1;
pub const CELL_CLIFF: u8 = 2;
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
pub const TAG_SNAM: [u8; 4] = *b"SNAM";
pub const TAG_EKND: [u8; 4] = *b"EKND";
pub const TAG_FNAM: [u8; 4] = *b"FNAM";
pub const TAG_MCND: [u8; 4] = *b"MCND";
pub const TAG_DKND: [u8; 4] = *b"DKND";
pub const NAME_RECORD: usize = 1;
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
pub const UEXT_RECORD: usize = 68;
pub const ACHN_RECORD: usize = 8;
pub const RSCH_RECORD: usize = 32;
pub const PREQ_RECORD: usize = 8;
pub const DMGF_RECORD: usize = 8;
pub const DODA_RECORD: usize = 24;
pub const GRID_RECORD: usize = 1;
pub const ITEM_RECORD: usize = 40 + ITEM_STAT_COUNT * 4;
pub const ITEM_COMPONENTS: usize = 6;
pub const ITEM_STATS: &[&str] = &[
    "armor", "damage", "health", "mana", "strength", "agility", "intelligence", "move_speed", "attack_speed",
    "health_regen", "mana_regen", "magic_resist", "evasion", "spell_amp", "lifesteal", "spell_lifesteal",
    "cast_range", "attack_range", "status_resist", "cooldown_reduction", "gold_per_min", "xp_per_min",
    "crit_chance", "crit_multiplier", "cleave", "cleave_distance", "cleave_start_width", "cleave_end_width",
    "bash_chance", "bash_duration", "speed_pct", "slow_resist", "hp_regen_amp", "mana_regen_amp",
    "mana_cost_reduction", "night_vision", "scepter", "shard", "bash_damage", "bash_chance_ranged", "bash_cooldown",
    "block_chance", "block_amount", "spell_block", "true_strike", "block_chance_nonhero",
    "creep_damage_pct", "true_sight",
];
pub const ITEM_STAT_COUNT: usize = 48;
pub const ITEM_FLAG_CONSUMABLE: u16 = 1;
pub const ITEM_FLAG_NEUTRAL: u16 = 2;
pub const ITEM_FLAG_NO_BUY: u16 = 4;
pub const ITEM_FLAG_NO_SELL: u16 = 8;
pub const ITEM_FLAG_ENHANCEMENT: u16 = 16;
pub const ITEM_FLAG_NO_DROP: u16 = 32;
pub const ITEM_FLAG_MASK: u16 = 63;
pub const ITEM_TAG_LIMIT: usize = 8;
pub const NEUTRAL_TIERS: u8 = 5;
pub const SKIL_RECORD: usize = 72;
pub const AXTN_RECORD: usize = 256;
pub const MAX_ROUTE_POINTS: usize = 64;
pub const MAX_ROUTE_ID: i32 = 3;
pub const ROUT_RECORD: usize = 8 + MAX_ROUTE_POINTS * 8;
pub const TUNE_RECORD: usize = 12 + XP_CURVE_LEN * 4;
pub const XP_CURVE_LEN: usize = 24;
pub const TELEPORT_RING_MAX_POINTS: u8 = 8;
pub const STRUCTURE_SCAN_MAX_CELLS: u8 = 64;
pub const AXTN_SCALED: usize = 6;
pub const AXTN_MODS: usize = 8;
pub const AXTN_FLAG_MASK: u16 = 1023;
pub const MOD_FIELD_FIRST: u8 = 250;
pub const SKILL_RANKS: usize = 4;
pub const ULTIMATE_RANKS: usize = 3;
pub const NORMAL_GATE: i32 = 1;
pub const MAX_NORMAL_GATE: i32 = 30;
pub const ITEM_SLOTS: usize = 6;
pub const GRID_HEADER_LEN: usize = 16;

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

pub fn encode_name_list(names: &[String]) -> Vec<u8> {
    let mut out = Vec::new();
    for name in names {
        out.push(name.len() as u8);
        out.extend_from_slice(name.as_bytes());
    }
    out
}

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

pub fn effect_params_valid(kind: &str, p: [i32; 4]) -> bool {
    let [p0, p1, p2, p3] = p;
    match kind {
        "damage" => p0 >= 0 && p1 >= 0 && spell_damage_type_valid(p2),
        "heal" => p0 >= 0 && p1 >= 0 && (0..=2).contains(&p2),
        "move" => p0 > 0 && p1 >= 0 && (0..=3).contains(&p2) && p3 >= 0,
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
        "shield_bonus" => p0 >= 0 && p1 >= 0 && p2 >= 0,
        "stat" => p0 >= 0,
        "carry" => true,
        "dot" => p0 >= 0 && p1 >= 1 && p2 >= 1 && spell_damage_type_valid(p3),
        "status" => (0..STATUS_CODES.len() as i32).contains(&p0) && p1 >= 1,
        "timed_stat" => p0 >= 0 && p2 >= 1 && (0..=1).contains(&p3),
        "aoe" => p0 >= 0 && (0..=31).contains(&p1) && (0..=16).contains(&p2) && p3 >= 0,
        "dispel" => (1..=7).contains(&p0),
        "chain" => p0 >= 0 && p1 >= 0 && p2 >= 0 && p3 >= 0 && spell_damage_type_valid(p3 & 0x00FF_FFFF),
        "zone" => p0 >= 0 && p1 >= 1 && p2 >= 1 && p3 >= 0,
        "illusion" => (1..=8).contains(&p0) && p1 >= 1 && p2 >= 0 && p3 >= 0,
        "if_kill" => (1..=16).contains(&p0),
        "attack" => p0 >= 0 && p1 >= 0,
        "refresh" => (1..=3).contains(&p0),
        "displace" => p0 >= 0 && p2 >= 0 && p3 >= 0 && ((0..=4).contains(&p1) || ((8..=12).contains(&p1) && p3 > 0)),
        "wall" => p0 > 0 && p1 >= 1 && (0..=1).contains(&p3),
        "item_form" => p0 >= 0 && (0..=1).contains(&p1) && p2 >= 0 && p3 >= 0,
        "damage_by" => p0 >= 0 && (1..=9).contains(&p1) && p2 & 3 != 0 && spell_damage_type_valid(p2) && p3 >= 0,
        "execute" => p0 >= 0 && p1 >= 0 && p2 & 3 != 0 && spell_damage_type_valid(p2) && (0..=1).contains(&p3),
        "stun" | "silence" | "root" | "invisibility" => p0 >= 1 && p2 >= 0,
        "slow" => p0 >= 1 && (0..=Q16_ONE).contains(&p1) && p2 >= 0,
        _ => true,
    }
}

pub fn move_cap_fits_grid(cap: i32, width: u32, height: u32, cell_milli: u32) -> bool {
    if cap <= 0 {
        return false;
    }
    let cap = cap as u128;
    let w = width as u128 * cell_milli as u128;
    let h = height as u128 * cell_milli as u128;
    cap * cap <= w * w + h * h
}

pub fn position_on_grid(x: i64, y: i64, width: u32, height: u32, cell_milli: u32) -> bool {
    let extent_x = width as i64 * cell_milli as i64;
    let extent_y = height as i64 * cell_milli as i64;
    (0..extent_x).contains(&x) && (0..extent_y).contains(&y)
}
