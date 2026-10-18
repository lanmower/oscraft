//! Reads the OSDT data file, format version 2, exactly as
//! rust/crates/data/FORMAT.md states it. Every integer is little-endian;
//! reserved bytes are skipped on read; every index is bounds-checked.

use crate::arrays::Sim;
use crate::fixed;
use crate::inventory::{ITEM_COMPONENTS, ITEM_FLAG_MASK, ITEM_SHOP_MASK, ITEM_STAT_COUNT, NEUTRAL_TIERS};
use crate::pathfinding::Grid;
use std::fmt;
use std::path::Path;

const MAGIC: &[u8; 4] = b"OSDT";
const VERSION: u16 = 21;
const NEUTRAL_PLAYER: u8 = 0xFF;
const PLACE_START: u32 = 1;
const PLACE_TEAM: u32 = 2;
const PLACE_TEAM_SHIFT: u32 = 8;
const PLACE_TEAM_MASK: u32 = 0xFF << PLACE_TEAM_SHIFT;
const LIMT_COUNT: usize = 1;
const EFFECT_MOVE: u16 = 2;
const RESOURCE_KINDS: u8 = 3;
const ABILITY_FILTER_BITS: u16 = 15;
const CELL_BITS: u8 = 0b11;
const HEADER_LEN: usize = 16;
const SECTION_ENTRY: usize = 16;
const SECTION_TAGS: [&[u8; 4]; 22] = [b"STRS", b"EFCT", b"ABIL", b"UABL", b"UNIT", b"BHVR", b"PLCE", b"UPGR", b"REGN", b"LIMT", b"ECON", b"UEXT", b"ACHN", b"RSCH", b"PREQ", b"DMGF", b"DODA", b"GRID", b"ITEM", b"SKIL", b"AXTN", b"ROUT"];
const RECORD_SIZES: [usize; 22] = [1, 20, 32, 2, 86, 16, 16, 24, 12, 4, 32, 44, 8, 32, 8, 8, 20, 1, 184, 72, 256, 520];
const SKILL_RANKS: usize = 4;
const ULTIMATE_RANKS: u8 = 3;
const ULTIMATE_GATES: [i32; ULTIMATE_RANKS as usize] = [6, 12, 18];
const NORMAL_GATE: i32 = 1;
const MAX_NORMAL_GATE: i32 = 30;
const EFFECT_AMOUNT_KINDS: [u16; 2] = [0, 1];
const EFFECT_CREATE_HEALER: u16 = 7;
const EFFECT_VITAL_FRACTION: u16 = 9;
const EFFECT_DAMAGE: u16 = 0;
const EFFECT_HEAL: u16 = 1;

const EFFECT_KINDS: u16 = 30;
const EFFECT_STAT: u16 = 16;
const EFFECT_WARD: u16 = 17;
const EFFECT_GLYPH: u16 = 18;
const EFFECT_DELIVER: u16 = 19;
const EFFECT_SLOW: u16 = 12;
const EFFECT_STATUS_DURATION: [u16; 4] = [11, 13, 14, 15];
const EFFECT_ISSUE_ORDER: u16 = 5;
const EFFECT_REMOVE_BEHAVIOUR: u16 = 6;
const EFFECT_USE_CALLDOWN: u16 = 8;
const EFFECT_SHIELD_BONUS: u16 = 10;
const EFFECT_BUFF: u16 = 4;
const TARGET_CODES: u8 = 3;
const TRIGGER_CODES: u8 = 5;
const ACTION_CODES: u8 = 4;
const RESPONSE_CODES: u8 = 3;
const FILTER_BITS: u32 = 15;
pub(crate) const NO_INDEX: u16 = 0xFFFF;
const GRID_HEADER: usize = 16;
/// The cell size the simulation runs on: one world unit per cell. Other sizes are
/// refused, because every position-to-cell conversion assumes this.
pub const CELL_MILLI: i32 = 1000;

/// A name is a (offset, length) pair into the STRS section.
#[derive(Clone, Copy, Debug, Default)]
pub struct Name {
    pub offset: u32,
    pub len: u16,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Effect {
    pub kind: u16,
    /// Damage and heal: amount (Q16.16). Spawn: unit index. Buff: behaviour kind.
    pub p0: i32,
    /// Damage and heal: radius (Q16.16), until v5 defines one. Buff: duration, ticks.
    pub p1: i32,
    /// Create healer: period, ticks. Vital fraction: mode. Buff and others: unused.
    pub p2: i32,
    /// Create healer: lifetime, ticks. Vital fraction: target. Others: unused.
    pub p3: i32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Ability {
    pub name: Name,
    pub flags: u16,
    /// Game ticks.
    pub cooldown: i32,
    pub energy: i32,
    pub target: u8,
    pub effect_count: u8,
    pub effect_first: u32,
    /// Projectile speed in milli-units per tick; 0 when the ability is not a projectile.
    pub projectile_speed: i32,
    /// Ticks an instance of this ability starts on cooldown when its unit spawns; its ready trigger fires when they run out.
    pub initial_cooldown: i32,
    /// Unit filters a valid target must carry (12 bits); 0 means no filter.
    pub filter: u16,
    /// Skill tree, when the ability learns ranks; its cooldown and energy then depend on rank.
    pub skill: Option<Skill>,
}

/// Per-rank values of a skill tree. Entries past `max_rank` are unused.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Skill {
    pub ultimate: bool,
    pub max_rank: u8,
    pub gates: [i32; SKILL_RANKS],
    pub damage: [i32; SKILL_RANKS],
    pub cooldown: [i32; SKILL_RANKS],
    pub energy: [i32; SKILL_RANKS],
}

impl Ability {
    pub fn is_projectile(&self) -> bool {
        self.flags & 1 != 0
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Unit {
    pub name: Name,
    /// Model file the renderer draws the unit with. Empty for none.
    pub model: Name,
    pub flags: u16,
    /// Hit points, whole units.
    pub health: i32,
    /// Milli-units per tick.
    pub speed: i32,
    /// Milli-units.
    pub radius: i32,
    pub resource_a: i32,
    pub gas: i32,
    /// Game ticks.
    pub build_ticks: i32,
    pub abil_first: u32,
    pub abil_count: u16,
    pub capacity: u16,
    /// Bit n set when the unit has the filter in FORMAT.md's FILTERS list.
    pub filter: u32,
    /// Shield pool, whole units.
    pub shield: i32,
    /// Flat reduction per hit, whole units.
    pub armor: i32,
    /// Weapon damage, whole units. Valid only when the weapon flag is set.
    pub weapon_damage: i32,
    /// Weapon range, milli-units.
    pub weapon_range: i32,
    /// Game ticks.
    pub weapon_period: i32,
    /// Sight radius, milli-units.
    pub sight: i32,
    /// Index of the unit this one trains, or 0xFFFF.
    pub product: u16,
    /// 0 normal, 1 reduced (half), 2 immune.
    pub response: u8,
    /// 0 none, 1 resource_a, 2 gas.
    pub resource_kind: u8,
    /// Resource units a worker takes per trip from this resource; 0 when not a resource.
    pub harvest_amount: i32,
}

impl Unit {
    pub fn has_weapon(&self) -> bool {
        self.flags & 1 != 0
    }
    pub fn is_air(&self) -> bool {
        self.filter & FILTER_AIR != 0
    }
    pub fn is_worker(&self) -> bool {
        self.filter & FILTER_WORKER != 0
    }
    pub fn is_structure(&self) -> bool {
        self.filter & FILTER_STRUCTURE != 0
    }
    pub fn product_index(&self) -> Option<usize> {
        if self.product == NO_INDEX { None } else { Some(self.product as usize) }
    }
}

/// Filter bit positions, from FORMAT.md.
pub const FILTER_GROUND: u32 = 1 << 0;
pub const FILTER_AIR: u32 = 1 << 1;
pub const FILTER_ARMORED: u32 = 1 << 2;
pub const FILTER_BIOLOGICAL: u32 = 1 << 3;
pub const FILTER_MECHANICAL: u32 = 1 << 4;
pub const FILTER_PSIONIC: u32 = 1 << 5;
pub const FILTER_STRUCTURE: u32 = 1 << 6;
pub const FILTER_LIGHT: u32 = 1 << 7;
pub const FILTER_MASSIVE: u32 = 1 << 8;
pub const FILTER_WORKER: u32 = 1 << 9;
pub const FILTER_ARMY: u32 = 1 << 10;
pub const FILTER_BUILDING: u32 = 1 << 11;
pub const FILTER_BARRACKS: u32 = 1 << 12;
pub const FILTER_MELEE_RAX: u32 = 1 << 13;
pub const FILTER_RANGED_RAX: u32 = 1 << 14;

/// A starting unit from the PLCE section. Position is Q16.16, already converted from milli-units.
#[derive(Clone, Copy, Debug, Default)]
pub struct Placement {
    pub unit: usize,
    pub player: u8,
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Behaviour {
    pub name: Name,
    pub trigger: u8,
    pub action: u8,
    pub param: i32,
    pub ability: u16,
}

/// What makes a behaviour fire, by BHVR.trigger code (FORMAT.md registries).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trigger {
    Idle,
    Attacked,
    Spawned,
    AbilityReady,
    Destroyed,
}

impl Trigger {
    pub fn from_code(code: u8) -> Option<Trigger> {
        match code {
            0 => Some(Trigger::Idle),
            1 => Some(Trigger::Attacked),
            2 => Some(Trigger::Spawned),
            3 => Some(Trigger::AbilityReady),
            4 => Some(Trigger::Destroyed),
            _ => None,
        }
    }

    /// The name a mod declares the trigger by.
    pub fn name(self) -> &'static str {
        match self {
            Trigger::Idle => "idle",
            Trigger::Attacked => "attacked",
            Trigger::Spawned => "spawned",
            Trigger::AbilityReady => "ability_ready",
            Trigger::Destroyed => "destroyed",
        }
    }
}

impl Behaviour {
    pub fn trigger_kind(&self) -> Option<Trigger> {
        Trigger::from_code(self.trigger)
    }
}

/// An upgrade record: one level of an upgrade. Records with the same name share a group,
/// which is the index of the first record of that name.
#[derive(Clone, Copy, Debug, Default)]
pub struct Upgrade {
    pub name: Name,
    /// 0 unit, 1 ability.
    pub target_kind: u8,
    pub level: u8,
    /// 0 new units only, 1 existing units too.
    pub applies: u8,
    /// 0 replace, 1 add, 2 refuse if present.
    pub stacking: u8,
    /// UNIT index for kind 0, ABIL index for kind 1.
    pub target: u16,
    /// Index into EFCT.
    pub effect: u32,
    /// Resource A charged per level: reaching level n costs n times this.
    pub resource_a_per_level: i32,
    /// Gas charged per level: reaching level n costs n times this.
    pub gas_per_level: i32,
    pub group: u32,
}

/// Regeneration of one unit kind, per tick: shield and health, and the ticks to wait after a hit.
#[derive(Clone, Copy, Debug, Default)]
pub struct Regen {
    pub shield_per_tick: i32,
    pub delay: i32,
    pub hp_per_tick: i32,
}

/// The start location of one player (format v9 and later). Starts carry no unit.
#[derive(Clone, Copy, Debug, Default)]
pub struct Start {
    pub player: u8,
    pub team: u8,
    pub x: i32,
    pub y: i32,
}

/// Production economy for the map (ECON, one record).
#[derive(Clone, Copy, Debug, Default)]
pub struct Econ {
    pub workers_per_mineral: u8,
    pub workers_per_extractor: u8,
    pub patches_per_base: u8,
    pub extractors_per_base: u8,
    pub gas_per_trip: i32,
    pub opening_resource_a: i32,
    pub opening_gas: i32,
    pub opening_workers: u16,
    pub capacity_base: u16,
    pub capacity_max: u16,
    pub resource_a_per_patch: i32,
    pub gas_per_geyser: i32,
}

/// Per-unit extension fields (UEXT), one per unit kind in UNIT order.
#[derive(Clone, Copy, Debug, Default)]
pub struct UnitExt {
    pub capacity_provided: u16,
    pub transport: u16,
    /// 0 small, 1 medium, 2 large.
    pub size: u8,
    /// 0 normal, 1 concussive, 2 explosive.
    pub damage_type: u8,
    /// Bit 0 cloaked, bit 1 can burrow, bit 2 needs a detector to target.
    pub cloak: u8,
    pub detector_radius: i32,
    /// Ranged miss chance, Q16.
    pub miss_chance: i32,
    pub burrow_ticks: i32,
    pub rally_x: i32,
    pub rally_y: i32,
    /// Gold paid to the killer when this unit is destroyed by an enemy.
    pub bounty: i32,
    /// Health percent at or below which an allied unit may deny this unit.
    pub deny_pct: u8,
    /// Share of `xp_bounty` the enemies get when this unit is denied, percent.
    pub deny_xp_pct: u8,
    /// Player-controlled (a hero): shares assist and kill xp.
    pub player_controlled: bool,
    /// Xp shared by the killer and in-range heroes when this unit dies.
    pub xp_bounty: i32,
    /// Assist radius around this unit, fixed-point milli-units.
    pub assist_radius: i32,
}

/// Channelling of one ability (ACHN, one per ability).
#[derive(Clone, Copy, Debug, Default)]
pub struct Channel {
    pub channelled: bool,
    pub duration: i32,
}

/// A research record (RSCH). Its effect is the research's slot in EFCT.
#[derive(Clone, Copy, Debug, Default)]
pub struct Research {
    pub name: Name,
    pub building: u16,
    pub time: i32,
    pub resource_a: i32,
    pub gas: i32,
    pub effect: u32,
    pub applies_mask: u32,
    /// A mod-defined research also applies its effect to matching units built after it completes.
    pub to_new: bool,
}

/// A prerequisite (PREQ): the target needs the requirement complete.
#[derive(Clone, Copy, Debug, Default)]
pub struct Prereq {
    /// 0 unit, 1 research.
    pub target_kind: u8,
    /// 0 unit (a building), 1 research.
    pub requirement_kind: u8,
    pub target: u16,
    pub requirement: u16,
}

/// A damage factor (DMGF) for one damage type and size category; pairs without a record scale by 1.0.
#[derive(Clone, Copy, Debug, Default)]
pub struct DamageFactor {
    pub damage_type: u8,
    pub size: u8,
    /// Q16, 65536 = 1.0.
    pub factor: i32,
}

/// A decorative object (DODA). Doodads do not block movement.
#[derive(Clone, Copy, Debug, Default)]
pub struct Doodad {
    pub id: u32,
    /// Indexes DOODAD_KINDS: 0 rock, 1 tree, 2 crystal, 3 debris, 4 base, 5 shop (home), 6 fountain, 7 lane, 8 camp, 9 side shop, 10 secret shop.
    pub kind: u16,
    /// Milli-units, on the map.
    pub x: i32,
    pub y: i32,
    /// Milli-turns, 0..1000.
    pub rotation: i32,
}

/// An item (ITEM). The bonuses are added to the buyer's stats when the item enters the inventory;
/// `ability` is an index into the abilities, or `NO_INDEX` for a passive item. `components` are
/// the recipe parts, as item indices declared before this one, `NO_INDEX` for an empty slot.
#[derive(Clone, Copy, Debug)]
pub struct Item {
    pub name: Name,
    pub cost: i32,
    /// Stat bonuses in `crate::inventory::STAT_*` order.
    pub stats: [i32; ITEM_STAT_COUNT],
    pub ability: u16,
    pub components: [u16; ITEM_COMPONENTS],
    pub flags: u16,
    /// Neutral tier `1..=NEUTRAL_TIERS`, 0 for an item that is not a neutral drop.
    pub tier: u8,
    /// Shops that sell the item: `SHOP_HOME`, `SHOP_SIDE`, `SHOP_SECRET` bits.
    pub shops: u8,
    /// Charges one purchase grants, and the most one slot holds (0: a single unstackable item).
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
            name: Name::default(),
            cost: 0,
            stats: [0; ITEM_STAT_COUNT],
            ability: 0,
            components: [0; ITEM_COMPONENTS],
            flags: 0,
            tier: 0,
            shops: 0,
            charges: 0,
            stack_max: 0,
            stock_max: 0,
            stock_initial: 0,
            stock_time_s: 0,
            stock_start_s: 0,
        }
    }
}

/// The parsed data file. Read-only after load; the tick reads only numeric fields.
#[derive(Clone, Debug, Default)]
pub struct Catalog {
    pub strs: Vec<u8>,
    pub items: Vec<Item>,
    pub effects: Vec<Effect>,
    pub abilities: Vec<Ability>,
    pub uabl: Vec<u16>,
    pub units: Vec<Unit>,
    pub behaviours: Vec<Behaviour>,
    pub placements: Vec<Placement>,
    pub upgrades: Vec<Upgrade>,
    /// One per unit kind, in UNIT order.
    pub regen: Vec<Regen>,
    /// Most units one grid cell may hold at once; 0 = no limit.
    pub limit: u32,
    pub starts: Vec<Start>,
    pub econ: Econ,
    pub unit_ext: Vec<UnitExt>,
    pub channels: Vec<Channel>,
    pub ext: Vec<crate::spell::AbilityExt>,
    pub research: Vec<Research>,
    pub prereqs: Vec<Prereq>,
    pub damage_factors: Vec<DamageFactor>,
    pub doodads: Vec<Doodad>,
    pub routes: Vec<crate::route::Route>,
}

impl Catalog {
    pub fn name(&self, n: Name) -> &str {
        let a = n.offset as usize;
        let b = a + n.len as usize;
        std::str::from_utf8(&self.strs[a..b]).unwrap_or("")
    }

    pub fn unit_index(&self, name: &str) -> Option<usize> {
        self.units.iter().position(|u| self.name(u.name) == name)
    }

    pub fn item_index(&self, name: &str) -> Option<usize> {
        self.items.iter().position(|i| self.name(i.name) == name)
    }
}

#[derive(Debug)]
pub enum LoadError {
    Io(String),
    BadMagic,
    BadVersion(u16),
    BadLength { header: u32, file: usize },
    Section(&'static str),
    Truncated(&'static str),
    Range(&'static str),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Io(e) => write!(f, "read failed: {e}"),
            LoadError::BadMagic => write!(f, "magic is not OSDT"),
            LoadError::BadVersion(v) => write!(f, "unknown version {v}"),
            LoadError::BadLength { header, file } => {
                write!(f, "header length {header} does not match file length {file}")
            }
            LoadError::Section(s) => write!(f, "section table: {s}"),
            LoadError::Truncated(s) => write!(f, "truncated: {s}"),
            LoadError::Range(s) => write!(f, "invalid value: {s}"),
        }
    }
}

impl std::error::Error for LoadError {}

struct Reader<'a> {
    buf: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize, what: &'static str) -> Result<&'a [u8], LoadError> {
        let buf: &'a [u8] = self.buf;
        let end = self.at.checked_add(n).ok_or(LoadError::Truncated(what))?;
        if end > buf.len() {
            return Err(LoadError::Truncated(what));
        }
        let s = &buf[self.at..end];
        self.at = end;
        Ok(s)
    }
    fn skip(&mut self, n: usize, what: &'static str) -> Result<(), LoadError> {
        self.take(n, what).map(|_| ())
    }
    fn u8(&mut self, what: &'static str) -> Result<u8, LoadError> {
        Ok(self.take(1, what)?[0])
    }
    fn u16(&mut self, what: &'static str) -> Result<u16, LoadError> {
        let b = self.take(2, what)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }
    fn u32(&mut self, what: &'static str) -> Result<u32, LoadError> {
        let b = self.take(4, what)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn i32(&mut self, what: &'static str) -> Result<i32, LoadError> {
        Ok(self.u32(what)? as i32)
    }
    fn name(&mut self, what: &'static str) -> Result<Name, LoadError> {
        Ok(Name { offset: self.u32(what)?, len: self.u16(what)? })
    }
}

fn valid_name(s: &[u8]) -> bool {
    !s.is_empty()
        && s[0].is_ascii_lowercase()
        && s.iter().all(|&c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
}

/// A model is a file stem: lowercase letters, digits, hyphens and underscores. Empty means none.
fn check_model(strs: &[u8], n: Name) -> Result<(), LoadError> {
    if n.len == 0 {
        return Ok(());
    }
    let a = n.offset as usize;
    let b = a + n.len as usize;
    let ok = b <= strs.len() && strs[a..b].iter().all(|&c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-' || c == b'_');
    if !ok {
        return Err(LoadError::Range("model name"));
    }
    Ok(())
}

fn check_name(strs: &[u8], n: Name) -> Result<(), LoadError> {
    let a = n.offset as usize;
    let b = a + n.len as usize;
    if b > strs.len() || !valid_name(&strs[a..b]) {
        return Err(LoadError::Range("name"));
    }
    Ok(())
}

/// The parameter rule for one effect, from FORMAT.md v4. `None` when its parameters are valid.
fn effect_param_error(e: &Effect) -> Option<&'static str> {
    match e.kind {
        EFFECT_ISSUE_ORDER if !(0..=2).contains(&e.p0) || !(0..=1).contains(&e.p1) => {
            Some("issue_order order or point code")
        }
        EFFECT_REMOVE_BEHAVIOUR if e.p0 < 0 || e.p1 < 0 => Some("remove_behaviour kind or radius"),
        EFFECT_SHIELD_BONUS if e.p0 < 0 => Some("shield_bonus amount"),
        EFFECT_VITAL_FRACTION
            if !(0..=1).contains(&e.p0)
                || !(0..=65536).contains(&e.p1)
                || !(0..=1).contains(&e.p2)
                || !(0..=1).contains(&e.p3) =>
        {
            Some("vital_fraction vital, fraction, mode or target")
        }
        EFFECT_DAMAGE | EFFECT_HEAL if e.p0 < 0 || e.p1 < 0 || (e.kind == EFFECT_DAMAGE && !(0..=3).contains(&e.p2)) || (e.kind == EFFECT_HEAL && !(0..=1).contains(&e.p2)) => {
            Some("damage or heal amount, radius or type")
        }
        EFFECT_BUFF if e.p0 < 0 || e.p1 < 0 => Some("buff kind or duration"),
        EFFECT_SLOW if e.p0 < 1 || !(0..=65536).contains(&e.p1) => Some("slow duration or multiplier"),
        k if EFFECT_STATUS_DURATION.contains(&k) && e.p0 < 1 => Some("status duration"),
        EFFECT_STAT if !(0..crate::spell::STAT_COUNT as i32).contains(&e.p0) => Some("stat code"),
        k if k >= crate::spell::FIRST_SPELL_KIND => crate::spell::spell_effect_error(e),
        EFFECT_WARD if e.p0 < 0 || e.p1 < 1 => Some("ward unit or lifetime"),
        EFFECT_GLYPH if e.p0 < 1 || e.p1 < 0 || e.p2 < 1 || e.p3 < 0 => Some("glyph duration, cooldown or filters"),
        EFFECT_DELIVER if e.p0 < 0 => Some("deliver reach"),
        EFFECT_CREATE_HEALER if e.p0 < 0 || e.p1 < 0 || e.p2 < 1 || e.p3 < 1 => {
            Some("create_healer amount, radius, period or lifetime")
        }
        _ => None,
    }
}

/// Parses a version 2 OSDT image into its catalog and its grid.
/// Reads one SKIL record and checks it against the effects its ability names.
fn read_skill(sr: &mut Reader<'_>, effects: &[Effect], first: usize, count: usize) -> Result<Option<Skill>, LoadError> {
    let has = sr.u8("skill has")?;
    let max_rank = sr.u8("skill max rank")?;
    let ultimate = sr.u8("skill ultimate")?;
    sr.skip(5, "skill reserved")?;
    let mut skill = Skill::default();
    for table in [&mut skill.gates, &mut skill.damage, &mut skill.cooldown, &mut skill.energy] {
        for value in table.iter_mut() {
            *value = sr.i32("skill value")?;
        }
    }
    if has == 0 {
        if max_rank != 0 || ultimate != 0 || skill != Skill::default() {
            return Err(LoadError::Range("skill values without a skill tree"));
        }
        return Ok(None);
    }
    if has != 1 || max_rank == 0 || max_rank as usize > SKILL_RANKS || ultimate > 1 {
        return Err(LoadError::Range("skill header"));
    }
    if ultimate == 1 && max_rank > ULTIMATE_RANKS {
        return Err(LoadError::Range("ultimate rank count"));
    }
    let ranks = max_rank as usize;
    for k in 0..SKILL_RANKS {
        if k >= ranks {
            if skill.gates[k] != 0 || skill.damage[k] != 0 || skill.cooldown[k] != 0 || skill.energy[k] != 0 {
                return Err(LoadError::Range("skill entry past max rank"));
            }
            continue;
        }
        let gate_ok = if ultimate == 1 {
            skill.gates[k] == ULTIMATE_GATES[k]
        } else {
            (NORMAL_GATE..=MAX_NORMAL_GATE).contains(&skill.gates[k]) && (k == 0 || skill.gates[k] >= skill.gates[k - 1])
        };
        if !gate_ok {
            return Err(LoadError::Range("skill level gate"));
        }
        if skill.damage[k] < 0 || skill.cooldown[k] < 0 || skill.energy[k] < 0 {
            return Err(LoadError::Range("skill value negative"));
        }
    }
    let takes_amount = effects[first..first + count].iter().any(|e| EFFECT_AMOUNT_KINDS.contains(&e.kind));
    if !takes_amount && skill.damage.iter().any(|&d| d != 0) {
        return Err(LoadError::Range("skill damage without an amount effect"));
    }
    skill.ultimate = ultimate == 1;
    skill.max_rank = max_rank;
    Ok(Some(skill))
}

pub fn parse(bytes: &[u8]) -> Result<(Catalog, Grid), LoadError> {
    let mut r = Reader { buf: bytes, at: 0 };
    if r.take(4, "magic")? != &MAGIC[..] {
        return Err(LoadError::BadMagic);
    }
    let version = r.u16("version")?;
    if version != VERSION {
        return Err(LoadError::BadVersion(version));
    }
    let header_len = r.u16("header length")?;
    let section_count = r.u32("section count")?;
    let total = r.u32("total length")?;
    if header_len as usize != HEADER_LEN
        || section_count as usize != SECTION_TAGS.len()
        || total as usize != bytes.len()
    {
        return Err(LoadError::BadLength { header: total, file: bytes.len() });
    }

    let mut sections = [(0usize, 0usize); 22];
    let mut expect = HEADER_LEN + SECTION_TAGS.len() * SECTION_ENTRY;
    for (k, tag) in SECTION_TAGS.iter().enumerate() {
        if r.take(4, "section tag")? != &tag[..] {
            return Err(LoadError::Section("tag order"));
        }
        let off = r.u32("section offset")? as usize;
        let count = r.u32("section count")? as usize;
        let rec = r.u32("record size")? as usize;
        if off != expect || rec != RECORD_SIZES[k] {
            return Err(LoadError::Section("body offset or record size"));
        }
        let body = count.checked_mul(rec).ok_or(LoadError::Section("body size"))?;
        expect = off.checked_add(body).ok_or(LoadError::Section("body size"))?;
        sections[k] = (off, count);
    }
    if expect != bytes.len() {
        return Err(LoadError::Section("bodies do not end at total length"));
    }
    let body = |k: usize| sections[k];

    let (so, sl) = body(0);
    let strs = bytes[so..so + sl].to_vec();

    let (eo, ec) = body(1);
    let mut er = Reader { buf: bytes, at: eo };
    let mut effects = Vec::with_capacity(ec);
    for _ in 0..ec {
        let kind = er.u16("effect kind")?;
        if kind >= EFFECT_KINDS {
            return Err(LoadError::Range("effect kind"));
        }
        er.skip(2, "effect reserved")?;
        let eff = Effect {
            kind,
            p0: er.i32("p0")?,
            p1: er.i32("p1")?,
            p2: er.i32("p2")?,
            p3: er.i32("p3")?,
        };
        if let Some(why) = effect_param_error(&eff) {
            return Err(LoadError::Range(why));
        }
        effects.push(eff);
    }

    let (ao, ac) = body(2);
    let mut ar = Reader { buf: bytes, at: ao };
    let (so, sc) = body(19);
    if sc != ac {
        return Err(LoadError::Section("skill count"));
    }
    let mut sr = Reader { buf: bytes, at: so };
    let mut abilities = Vec::with_capacity(ac);
    for _ in 0..ac {
        let name = ar.name("ability name")?;
        let flags = ar.u16("ability flags")?;
        let cooldown = ar.i32("cooldown")?;
        let energy = ar.i32("energy")?;
        let target = ar.u8("target")?;
        let effect_count = ar.u8("effect count")?;
        let filter = ar.u16("target filter")?;
        let effect_first = ar.u32("effect first")?;
        let projectile_speed = ar.i32("projectile speed")?;
        let initial_cooldown = ar.i32("initial cooldown")?;
        if initial_cooldown < 0 {
            return Err(LoadError::Range("ability initial cooldown"));
        }
        check_name(&strs, name)?;
        if flags & !1 != 0 {
            return Err(LoadError::Range("ability flags reserved bit"));
        }
        if filter >> ABILITY_FILTER_BITS != 0 {
            return Err(LoadError::Range("ability filter bit out of range"));
        }
        if (flags & 1 != 0) != (projectile_speed > 0) {
            return Err(LoadError::Range("projectile flag disagrees with speed"));
        }
        if projectile_speed < 0 || target >= TARGET_CODES {
            return Err(LoadError::Range("ability speed or target code"));
        }
        if effect_first as usize + effect_count as usize > ec {
            return Err(LoadError::Range("effect run"));
        }
        let skill = read_skill(&mut sr, &effects, effect_first as usize, effect_count as usize)?;
        abilities.push(Ability { name, flags, cooldown, energy, target, effect_count, effect_first, projectile_speed, initial_cooldown, filter, skill });
    }

    if effects.iter().any(|e| e.kind == EFFECT_USE_CALLDOWN && (e.p0 < 0 || e.p0 as usize >= abilities.len())) {
        return Err(LoadError::Range("use_calldown ability index"));
    }

    let (uo, uc) = body(3);
    let mut ur = Reader { buf: bytes, at: uo };
    let mut uabl = Vec::with_capacity(uc);
    for _ in 0..uc {
        let ai = ur.u16("ability ref")?;
        if ai as usize >= ac {
            return Err(LoadError::Range("UABL ability index"));
        }
        uabl.push(ai);
    }

    let (no, nc) = body(4);
    let mut nr = Reader { buf: bytes, at: no };
    let mut units = Vec::with_capacity(nc);
    for _ in 0..nc {
        let name = nr.name("unit name")?;
        let flags = nr.u16("unit flags")?;
        let health = nr.i32("health")?;
        let speed = nr.i32("speed")?;
        let radius = nr.i32("radius")?;
        let resource_a = nr.i32("resource_a")?;
        let gas = nr.i32("gas")?;
        let build_ticks = nr.i32("build time")?;
        let abil_first = nr.u32("first ability")?;
        let abil_count = nr.u16("ability count")?;
        let capacity = nr.u16("capacity")?;
        let filter = nr.u32("filter mask")?;
        let shield = nr.i32("shield")?;
        let armor = nr.i32("armor")?;
        let weapon_damage = nr.i32("weapon damage")?;
        let weapon_range = nr.i32("weapon range")?;
        let weapon_period = nr.i32("weapon period")?;
        let sight = nr.i32("sight")?;
        let product = nr.u16("product")?;
        let response = nr.u8("damage response")?;
        let resource_kind = nr.u8("resource kind")?;
        let harvest_amount = nr.i32("harvest amount")?;
        let model = nr.name("unit model")?;
        nr.skip(4, "unit reserved")?;
        if resource_kind >= RESOURCE_KINDS {
            return Err(LoadError::Range("resource kind code"));
        }
        if harvest_amount < 0 {
            return Err(LoadError::Range("harvest amount must be >= 0"));
        }
        check_name(&strs, name)?;
        check_model(&strs, model)?;
        if health <= 0 {
            return Err(LoadError::Range("health must be > 0"));
        }
        if flags & !1 != 0 {
            return Err(LoadError::Range("unit flags reserved bit"));
        }
        if shield < 0 || sight < 0 || speed < 0 || radius < 0 {
            return Err(LoadError::Range("negative unit size, speed or shield"));
        }
        if filter >> FILTER_BITS != 0 {
            return Err(LoadError::Range("filter mask bit out of range"));
        }
        if response >= RESPONSE_CODES {
            return Err(LoadError::Range("damage response code"));
        }
        if product != NO_INDEX && product as usize >= nc {
            return Err(LoadError::Range("product index"));
        }
        if flags & 1 == 0 && (weapon_damage != 0 || weapon_range != 0 || weapon_period != 0) {
            return Err(LoadError::Range("weapon fields set without weapon flag"));
        }
        if flags & 1 != 0 && weapon_period <= 0 {
            return Err(LoadError::Range("weapon period must be > 0"));
        }
        if abil_first as usize + abil_count as usize > uabl.len() {
            return Err(LoadError::Range("UABL run"));
        }
        units.push(Unit {
            name,
            model,
            flags,
            health,
            speed,
            radius,
            resource_a,
            gas,
            build_ticks,
            abil_first,
            abil_count,
            capacity,
            filter,
            shield,
            armor,
            weapon_damage,
            weapon_range,
            weapon_period,
            sight,
            product,
            response,
            resource_kind,
            harvest_amount,
        });
    }

    let (bo, bc) = body(5);
    let mut br = Reader { buf: bytes, at: bo };
    let mut behaviours = Vec::with_capacity(bc);
    for _ in 0..bc {
        let name = br.name("behaviour name")?;
        let trigger = br.u8("trigger")?;
        let action = br.u8("action")?;
        let param = br.i32("param")?;
        let ability = br.u16("ability index")?;
        br.skip(2, "behaviour reserved")?;
        check_name(&strs, name)?;
        if trigger >= TRIGGER_CODES || action >= ACTION_CODES {
            return Err(LoadError::Range("trigger or action code"));
        }
        // The engine does not run flee yet. Refusing it at load is louder than a refusal at every trigger.
        if action == crate::ACTION_FLEE {
            return Err(LoadError::Range("behaviour action flee is not implemented"));
        }
        if action == 2 && ability == NO_INDEX {
            return Err(LoadError::Range("use_ability needs an ability"));
        }
        if ability != NO_INDEX && ability as usize >= ac {
            return Err(LoadError::Range("behaviour ability index"));
        }
        behaviours.push(Behaviour { name, trigger, action, param, ability });
    }

    let (go, gl) = body(17);
    if gl < GRID_HEADER {
        return Err(LoadError::Truncated("GRID header"));
    }
    let mut gr = Reader { buf: bytes, at: go };
    let w = gr.u32("grid width")? as usize;
    let h = gr.u32("grid height")? as usize;
    let cell_milli = gr.i32("cell size")?;
    let height_step = gr.i32("height step")?;
    if height_step < 1 {
        return Err(LoadError::Range("height step must be >= 1"));
    }
    if w < 1 || h < 1 {
        return Err(LoadError::Range("grid size must be >= 1"));
    }
    if cell_milli != CELL_MILLI {
        return Err(LoadError::Range("cell size must be 1000 milli-units"));
    }
    let cells = w.checked_mul(h).ok_or(LoadError::Range("grid size"))?;
    if cells.checked_mul(3).and_then(|n| n.checked_add(GRID_HEADER)) != Some(gl) {
        return Err(LoadError::Range("GRID body length is not 16 + 3*w*h"));
    }
    let flags = gr.take(w * h, "grid cells")?.to_vec();
    if flags.iter().any(|&c| c & !CELL_BITS != 0) {
        return Err(LoadError::Range("grid cell bits outside 0b11"));
    }
    let heights = gr.take(w * h, "grid heights")?.to_vec();
    let tiles = gr.take(w * h, "grid tiles")?.to_vec();
    let grid = Grid { w, h, flags, heights, tiles, height_step };

    let diagonal2 = (w as i64 * CELL_MILLI as i64).pow(2) + (h as i64 * CELL_MILLI as i64).pow(2);
    if effects.iter().any(|e| e.kind == EFFECT_MOVE && (e.p0 <= 0 || (e.p0 as i64).pow(2) > diagonal2)) {
        return Err(LoadError::Range("move cap outside 1..=grid diagonal"));
    }

    let (po, pc) = body(6);
    let mut pr = Reader { buf: bytes, at: po };
    let mut placements = Vec::with_capacity(pc);
    let mut starts: Vec<Start> = Vec::new();
    for _ in 0..pc {
        let unit = pr.u16("placement unit")? as usize;
        let player = pr.u16("placement player")?;
        let x = pr.i32("placement x")?;
        let y = pr.i32("placement y")?;
        let flags = pr.u32("placement flags")?;
        if flags & !(PLACE_START | PLACE_TEAM | PLACE_TEAM_MASK) != 0 {
            return Err(LoadError::Range("placement flags outside the start and team bits"));
        }
        if x < 0 || y < 0 || x as i64 >= w as i64 * CELL_MILLI as i64 || y as i64 >= h as i64 * CELL_MILLI as i64 {
            return Err(LoadError::Range("placement outside the grid"));
        }
        let (x, y) = (fixed::from_milli(x), fixed::from_milli(y));
        if flags & PLACE_START != 0 {
            if unit != NO_INDEX as usize || player > 15 {
                return Err(LoadError::Range("start record needs unit 0xFFFF and player 0..=15"));
            }
            if starts.iter().any(|s| s.player == player as u8) {
                return Err(LoadError::Range("two start records for one player"));
            }
            let team = if flags & PLACE_TEAM != 0 { ((flags & PLACE_TEAM_MASK) >> PLACE_TEAM_SHIFT) as usize } else { player as usize };
            if team >= crate::MAX_PLAYERS {
                return Err(LoadError::Range("start team outside 0..MAX_PLAYERS"));
            }
            starts.push(Start { player: player as u8, team: team as u8, x, y });
            continue;
        }
        if unit >= units.len() || (player > 15 && player != NO_INDEX) {
            return Err(LoadError::Range("placement unit or player"));
        }
        let owner = if player == NO_INDEX { NEUTRAL_PLAYER } else { player as u8 };
        placements.push(Placement { unit, player: owner, x, y });
    }

    let (uo, uc) = body(7);
    let mut ur = Reader { buf: bytes, at: uo };
    let mut upgrades: Vec<Upgrade> = Vec::with_capacity(uc);
    for _ in 0..uc {
        let name = ur.name("upgrade name")?;
        let target_kind = ur.u8("upgrade target kind")?;
        let level = ur.u8("upgrade level")?;
        let applies = ur.u8("upgrade applies to")?;
        let stacking = ur.u8("upgrade stacking")?;
        let target = ur.u16("upgrade target")?;
        let effect = ur.u32("upgrade effect")?;
        let resource_a_per_level = ur.i32("upgrade resource_a per level")?;
        let gas_per_level = ur.i32("upgrade gas per level")?;
        if resource_a_per_level < 0 || gas_per_level < 0 {
            return Err(LoadError::Range("upgrade cost below zero"));
        }
        if name.offset as usize + name.len as usize > strs.len() {
            return Err(LoadError::Range("upgrade name outside STRS"));
        }
        if target_kind > 1 || level == 0 || applies > 1 || stacking > 2 {
            return Err(LoadError::Range("upgrade code outside its table"));
        }
        let target_ok = if target_kind == 0 { (target as usize) < units.len() } else { (target as usize) < abilities.len() };
        if !target_ok || effect as usize >= effects.len() {
            return Err(LoadError::Range("upgrade target or effect index"));
        }
        upgrades.push(Upgrade {
            name,
            target_kind,
            level,
            applies,
            stacking,
            target,
            effect,
            resource_a_per_level,
            gas_per_level,
            group: 0,
        });
    }
    let groups: Vec<u32> = {
        let names: Vec<&[u8]> = upgrades
            .iter()
            .map(|u| &strs[u.name.offset as usize..u.name.offset as usize + u.name.len as usize])
            .collect();
        (0..names.len()).map(|k| (0..=k).find(|&j| names[j] == names[k]).unwrap_or(k) as u32).collect()
    };
    for (u, g) in upgrades.iter_mut().zip(groups) {
        u.group = g;
    }

    let (ro, rc) = body(8);
    if rc != units.len() {
        return Err(LoadError::Range("REGN count is not the UNIT count"));
    }
    let mut rr = Reader { buf: bytes, at: ro };
    let mut regen = Vec::with_capacity(rc);
    for _ in 0..rc {
        let shield_per_tick = rr.i32("regen shield")?;
        let delay = rr.i32("regen delay")?;
        let hp_per_tick = rr.i32("regen health")?;
        if shield_per_tick < 0 || delay < 0 || hp_per_tick < 0 {
            return Err(LoadError::Range("regeneration below zero"));
        }
        regen.push(Regen { shield_per_tick, delay, hp_per_tick });
    }

    let (lo, lc) = body(9);
    if lc != LIMT_COUNT {
        return Err(LoadError::Range("LIMT count is not 1"));
    }
    let limit = Reader { buf: bytes, at: lo }.u32("cell unit limit")?;

    let (eco, ecn) = body(10);
    if ecn != 1 {
        return Err(LoadError::Range("ECON count is not 1"));
    }
    let econ = {
        let mut er = Reader { buf: bytes, at: eco };
        let workers_per_mineral = er.u8("econ mineral saturation")?;
        let workers_per_extractor = er.u8("econ gas saturation")?;
        let patches_per_base = er.u8("econ mineral patches")?;
        let extractors_per_base = er.u8("econ extractors")?;
        let gas_per_trip = er.i32("econ gas per trip")?;
        let opening_resource_a = er.i32("econ opening resource_a")?;
        let opening_gas = er.i32("econ opening gas")?;
        let opening_workers = er.u16("econ opening workers")?;
        let capacity_base = er.u16("econ capacity base")?;
        let capacity_max = er.u16("econ capacity max")?;
        er.skip(2, "econ reserved")?;
        let resource_a_per_patch = er.i32("econ resource_a per patch")?;
        let gas_per_geyser = er.i32("econ gas per geyser")?;
        if workers_per_mineral < 1 || workers_per_extractor < 1 || gas_per_trip < 0 || opening_resource_a < 0
            || opening_gas < 0 || resource_a_per_patch < 0 || gas_per_geyser < 0
        {
            return Err(LoadError::Range("economy value outside its table"));
        }
        Econ {
            workers_per_mineral,
            workers_per_extractor,
            patches_per_base,
            extractors_per_base,
            gas_per_trip,
            opening_resource_a,
            opening_gas,
            opening_workers,
            capacity_base,
            capacity_max,
            resource_a_per_patch,
            gas_per_geyser,
        }
    };

    let (xo, xc) = body(11);
    if xc != units.len() {
        return Err(LoadError::Range("UEXT count is not the UNIT count"));
    }
    let mut xr = Reader { buf: bytes, at: xo };
    let mut unit_ext = Vec::with_capacity(xc);
    for _ in 0..xc {
        let capacity_provided = xr.u16("ext capacity")?;
        let transport = xr.u16("ext transport")?;
        let size = xr.u8("ext size")?;
        let damage_type = xr.u8("ext damage type")?;
        let cloak = xr.u8("ext cloak")?;
        xr.skip(1, "ext reserved")?;
        let detector_radius = xr.i32("ext detector radius")?;
        let miss_chance = xr.i32("ext miss chance")?;
        let burrow_ticks = xr.i32("ext burrow time")?;
        let rally_x = xr.i32("ext rally x")?;
        let rally_y = xr.i32("ext rally y")?;
        let bounty = xr.i32("ext bounty")?;
        let deny_pct = xr.u8("ext deny pct")?;
        let deny_xp_pct = xr.u8("ext deny xp pct")?;
        let flags = xr.u8("ext flags")?;
        xr.skip(1, "ext reserved 2")?;
        let xp_bounty = xr.i32("ext xp bounty")?;
        let assist_radius = xr.i32("ext assist radius")?;
        if size > 2 || damage_type > 2 || cloak > 0b111 || !(0..=65536).contains(&miss_chance) || burrow_ticks < 0 || bounty < 0 {
            return Err(LoadError::Range("unit extension outside its table"));
        }
        if deny_pct > 100 || deny_xp_pct > 100 || flags > 1 || xp_bounty < 0 || assist_radius < 0 {
            return Err(LoadError::Range("unit extension deny or xp outside its table"));
        }
        unit_ext.push(UnitExt {
            capacity_provided,
            transport,
            size,
            damage_type,
            cloak,
            detector_radius,
            miss_chance,
            burrow_ticks,
            rally_x,
            rally_y,
            bounty,
            deny_pct,
            deny_xp_pct,
            player_controlled: flags & 1 != 0,
            xp_bounty,
            assist_radius,
        });
    }

    let (ach, achn) = body(12);
    if achn != abilities.len() {
        return Err(LoadError::Range("ACHN count is not the ABIL count"));
    }
    let mut cr = Reader { buf: bytes, at: ach };
    let mut channels = Vec::with_capacity(achn);
    for _ in 0..achn {
        let flags = cr.u16("channel flags")?;
        cr.skip(2, "channel reserved")?;
        let duration = cr.i32("channel duration")?;
        let channelled = flags & 1 == 1;
        if flags & !1 != 0 || duration < 0 || (channelled && duration < 1) || (!channelled && duration != 0) {
            return Err(LoadError::Range("channel record outside its table"));
        }
        channels.push(Channel { channelled, duration });
    }

    let routes = crate::route::parse_routes(bytes, body(21), w, h, &grid.flags)?;
    let ext = crate::spell::parse_ext(bytes, body(20), &abilities, &effects)?;

    let (rso, rsc) = body(13);
    let mut rsr = Reader { buf: bytes, at: rso };
    let mut research = Vec::with_capacity(rsc);
    for _ in 0..rsc {
        let name = rsr.name("research name")?;
        let building = rsr.u16("research building")?;
        let time = rsr.i32("research time")?;
        let resource_a = rsr.i32("research resource_a")?;
        let gas = rsr.i32("research gas")?;
        let effect = rsr.u32("research effect")?;
        let applies_mask = rsr.u32("research applies-to")?;
        rsr.skip(4, "research reserved")?;
        if name.offset as usize + name.len as usize > strs.len()
            || building as usize >= units.len()
            || effect as usize >= effects.len()
        {
            return Err(LoadError::Range("research name, building or effect index"));
        }
        research.push(Research { name, building, time, resource_a, gas, effect, applies_mask, to_new: false });
    }

    let (pqo, pqc) = body(14);
    let mut pqr = Reader { buf: bytes, at: pqo };
    let mut prereqs = Vec::with_capacity(pqc);
    for _ in 0..pqc {
        let target_kind = pqr.u8("prereq target kind")?;
        let requirement_kind = pqr.u8("prereq requirement kind")?;
        let target = pqr.u16("prereq target")?;
        let requirement = pqr.u16("prereq requirement")?;
        pqr.skip(2, "prereq reserved")?;
        if target_kind > 1 || requirement_kind > 1 {
            return Err(LoadError::Range("prerequisite kind above 1"));
        }
        let target_ok = if target_kind == 0 { (target as usize) < units.len() } else { (target as usize) < research.len() };
        let req_ok = if requirement_kind == 0 {
            (requirement as usize) < units.len()
        } else {
            (requirement as usize) < research.len()
        };
        if !target_ok || !req_ok {
            return Err(LoadError::Range("prerequisite index past its section"));
        }
        prereqs.push(Prereq { target_kind, requirement_kind, target, requirement });
    }

    let (dmo, dmc) = body(15);
    let mut dmr = Reader { buf: bytes, at: dmo };
    let mut damage_factors = Vec::with_capacity(dmc);
    for _ in 0..dmc {
        let damage_type = dmr.u8("factor damage type")?;
        let size = dmr.u8("factor size")?;
        dmr.skip(2, "factor reserved")?;
        let factor = dmr.i32("factor")?;
        if damage_type > 2 || size > 2 || factor < 0 {
            return Err(LoadError::Range("damage factor outside its table"));
        }
        damage_factors.push(DamageFactor { damage_type, size, factor });
    }

    let (dd, ddc) = body(16);
    let mut ddr = Reader { buf: bytes, at: dd };
    let mut doodads: Vec<Doodad> = Vec::with_capacity(ddc);
    for _ in 0..ddc {
        let id = ddr.u32("doodad id")?;
        let kind = ddr.u16("doodad kind")?;
        ddr.skip(2, "doodad reserved")?;
        let x = ddr.i32("doodad x")?;
        let y = ddr.i32("doodad y")?;
        let rotation = ddr.i32("doodad rotation")?;
        if kind > 8
            || !(0..1000).contains(&rotation)
            || x < 0
            || y < 0
            || x as i64 >= w as i64 * CELL_MILLI as i64
            || y as i64 >= h as i64 * CELL_MILLI as i64
        {
            return Err(LoadError::Range("doodad outside its table"));
        }
        if doodads.iter().any(|d| d.id == id) {
            return Err(LoadError::Range("doodad id repeats"));
        }
        doodads.push(Doodad { id, kind, x, y, rotation });
    }

    let (io, ic) = body(18);
    let mut ir = Reader { buf: bytes, at: io };
    let mut items: Vec<Item> = Vec::with_capacity(ic);
    for _ in 0..ic {
        let name = ir.name("item name")?;
        let flags = ir.u16("item flags")?;
        let cost = ir.i32("item cost")?;
        let ability = ir.u16("item ability")?;
        let tier = ir.u8("item tier")?;
        let shops = ir.u8("item shops")?;
        let mut components = [NO_INDEX; ITEM_COMPONENTS];
        for c in components.iter_mut() {
            *c = ir.u16("item component")?;
        }
        let charges = ir.u16("item charges")?;
        let stack_max = ir.u16("item stack")?;
        let stock_max = ir.u8("item stock max")?;
        let stock_initial = ir.u8("item stock initial")?;
        let stock_time_s = ir.u16("item stock time")?;
        let stock_start_s = ir.u16("item stock start")?;
        ir.skip(2, "item reserved")?;
        let mut stats = [0i32; ITEM_STAT_COUNT];
        for v in stats.iter_mut() {
            *v = ir.i32("item stat")?;
        }
        let item = Item {
            name,
            cost,
            stats,
            ability,
            components,
            flags,
            tier,
            shops,
            charges,
            stack_max,
            stock_max,
            stock_initial,
            stock_time_s,
            stock_start_s,
        };
        let ability_ok = item.ability == NO_INDEX || (item.ability as usize) < abilities.len();
        // A component is an item declared before this one, and the components cost no more than
        // the item (the cost rule): a recipe never pays out gold.
        let mut component_cost = 0i64;
        let components_ok = item.components.iter().all(|&c| {
            if c == NO_INDEX {
                return true;
            }
            match items.get(c as usize) {
                Some(part) => {
                    component_cost += i64::from(part.cost);
                    true
                }
                None => false,
            }
        });
        if item.cost < 0
            || item.flags & !ITEM_FLAG_MASK != 0
            || item.tier > NEUTRAL_TIERS
            || item.shops > ITEM_SHOP_MASK
            || item.stock_initial > item.stock_max
            || !ability_ok
            || !components_ok
            || component_cost > i64::from(item.cost)
        {
            return Err(LoadError::Range("item outside its table"));
        }
        items.push(item);
    }

    Ok((
        Catalog {
            strs,
            items,
            effects,
            abilities,
            uabl,
            units,
            behaviours,
            placements,
            upgrades,
            regen,
            limit,
            starts,
            econ,
            unit_ext,
            channels,
            ext,
            research,
            prereqs,
            damage_factors,
            doodads,
            routes,
        },
        grid,
    ))
}

/// Loads a version 8 image into a simulation on the map's own grid, and places its
/// starting units. A placement the simulation refuses (no free slot, unknown unit, or a
/// blocked cell for a ground unit) fails the load.
pub fn load(bytes: &[u8], max_units: usize) -> Result<Sim, LoadError> {
    let (cat, grid) = parse(bytes)?;
    let mut sim = Sim::with_catalog(max_units, grid, cat);
    for k in 0..sim.cat.placements.len() {
        let p = sim.cat.placements[k];
        let Some(i) = sim.spawn_for(p.player, p.unit as u32, p.x, p.y) else {
            return Err(LoadError::Range("placement refused: no slot, unknown unit or blocked cell"));
        };
        if p.player == NEUTRAL_PLAYER {
            sim.mark_camp(i);
        }
        if sim.cat.unit_ext.get(p.unit as usize).is_some_and(|e| e.player_controlled) {
            sim.enroll_hero(i, p.x, p.y);
        }
    }
    sim.camps.boss_kind = sim.cat.unit_index(crate::camp::ROSHAN_NAME).map(|k| k as u32);
    sim.camps.rune_kind = sim.cat.unit_index(crate::camp::RUNE_NAME).map(|k| k as u32);
    sim.hold_first_camp_spawn();
    sim.camps.reward_kind = sim
        .cat
        .items
        .iter()
        .position(|it| sim.cat.name(it.name) == crate::camp::AEGIS_NAME)
        .map_or(0, |k| crate::inventory::item_kind(k));
    for k in 0..sim.cat.starts.len() {
        let s = sim.cat.starts[k];
        if sim.set_team(s.player, s.team).is_err() {
            return Err(LoadError::Range("start team refused"));
        }
        sim.open_player(s.player, s.x, s.y);
        sim.base_fields(s.x, s.y);
    }
    Ok(sim)
}

pub fn load_file(path: &Path, max_units: usize) -> Result<Sim, LoadError> {
    let bytes = std::fs::read(path).map_err(|e| LoadError::Io(e.to_string()))?;
    load(&bytes, max_units)
}
