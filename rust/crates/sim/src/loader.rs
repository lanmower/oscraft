//! Reads the OSDT data file, format version 2, exactly as
//! rust/crates/data/FORMAT.md states it. Every integer is little-endian;
//! reserved bytes are skipped on read; every index is bounds-checked.

use crate::arrays::Sim;
use crate::pathfinding::Grid;
use std::fmt;
use std::path::Path;

const MAGIC: &[u8; 4] = b"OSDT";
const VERSION: u16 = 4;
const RESOURCE_KINDS: u8 = 3;
const ABILITY_FILTER_BITS: u16 = 12;
const CELL_BITS: u8 = 0b11;
const HEADER_LEN: usize = 16;
const SECTION_ENTRY: usize = 16;
const SECTION_TAGS: [&[u8; 4]; 7] = [b"STRS", b"EFCT", b"ABIL", b"UABL", b"UNIT", b"BHVR", b"GRID"];
const RECORD_SIZES: [usize; 7] = [1, 16, 32, 2, 80, 16, 1];

const EFFECT_KINDS: u16 = 11;
const EFFECT_ISSUE_ORDER: u16 = 5;
const EFFECT_REMOVE_BEHAVIOUR: u16 = 6;
const TARGET_CODES: u8 = 3;
const TRIGGER_CODES: u8 = 4;
const ACTION_CODES: u8 = 3;
const RESPONSE_CODES: u8 = 3;
const FILTER_BITS: u32 = 12;
const NO_INDEX: u16 = 0xFFFF;
const GRID_HEADER: usize = 12;
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
    /// Damage and heal: radius (Q16.16). Buff: damage per application (Q16.16).
    pub p1: i32,
    /// Buff: ticks between applications.
    pub p2: i32,
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
    /// Unit filters a valid target must carry (12 bits); 0 means no filter.
    pub filter: u16,
}

impl Ability {
    pub fn is_projectile(&self) -> bool {
        self.flags & 1 != 0
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Unit {
    pub name: Name,
    pub flags: u16,
    /// Hit points, whole units.
    pub health: i32,
    /// Milli-units per tick.
    pub speed: i32,
    /// Milli-units.
    pub radius: i32,
    pub minerals: i32,
    pub gas: i32,
    /// Game ticks.
    pub build_ticks: i32,
    pub abil_first: u32,
    pub abil_count: u16,
    pub supply: u16,
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
    /// 0 none, 1 minerals, 2 gas.
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

#[derive(Clone, Copy, Debug, Default)]
pub struct Behaviour {
    pub name: Name,
    pub trigger: u8,
    pub action: u8,
    pub param: i32,
    pub ability: u16,
}

/// The parsed data file. Read-only after load; the tick reads only numeric fields.
#[derive(Clone, Debug, Default)]
pub struct Catalog {
    pub strs: Vec<u8>,
    pub effects: Vec<Effect>,
    pub abilities: Vec<Ability>,
    pub uabl: Vec<u16>,
    pub units: Vec<Unit>,
    pub behaviours: Vec<Behaviour>,
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
        _ => None,
    }
}

/// Parses a version 2 OSDT image into its catalog and its grid.
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

    let mut sections = [(0usize, 0usize); 7];
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
        let eff = Effect { kind, p0: er.i32("p0")?, p1: er.i32("p1")?, p2: er.i32("p2")? };
        if let Some(why) = effect_param_error(&eff) {
            return Err(LoadError::Range(why));
        }
        effects.push(eff);
    }

    let (ao, ac) = body(2);
    let mut ar = Reader { buf: bytes, at: ao };
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
        ar.skip(4, "ability reserved")?;
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
        abilities.push(Ability { name, flags, cooldown, energy, target, effect_count, effect_first, projectile_speed, filter });
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
        let minerals = nr.i32("minerals")?;
        let gas = nr.i32("gas")?;
        let build_ticks = nr.i32("build time")?;
        let abil_first = nr.u32("first ability")?;
        let abil_count = nr.u16("ability count")?;
        let supply = nr.u16("supply")?;
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
        nr.skip(4, "unit reserved")?;
        if resource_kind >= RESOURCE_KINDS {
            return Err(LoadError::Range("resource kind code"));
        }
        if harvest_amount < 0 {
            return Err(LoadError::Range("harvest amount must be >= 0"));
        }
        check_name(&strs, name)?;
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
            flags,
            health,
            speed,
            radius,
            minerals,
            gas,
            build_ticks,
            abil_first,
            abil_count,
            supply,
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
        if action == 2 && ability == NO_INDEX {
            return Err(LoadError::Range("use_ability needs an ability"));
        }
        if ability != NO_INDEX && ability as usize >= ac {
            return Err(LoadError::Range("behaviour ability index"));
        }
        behaviours.push(Behaviour { name, trigger, action, param, ability });
    }

    let (go, gl) = body(6);
    if gl < GRID_HEADER {
        return Err(LoadError::Truncated("GRID header"));
    }
    let mut gr = Reader { buf: bytes, at: go };
    let w = gr.u32("grid width")? as usize;
    let h = gr.u32("grid height")? as usize;
    let cell_milli = gr.i32("cell size")?;
    if w < 1 || h < 1 {
        return Err(LoadError::Range("grid size must be >= 1"));
    }
    if cell_milli != CELL_MILLI {
        return Err(LoadError::Range("cell size must be 1000 milli-units"));
    }
    if GRID_HEADER.checked_add(w.checked_mul(h).ok_or(LoadError::Range("grid size"))?) != Some(gl) {
        return Err(LoadError::Range("GRID body length is not 12 + w*h"));
    }
    let flags = gr.take(w * h, "grid cells")?.to_vec();
    if flags.iter().any(|&c| c & !CELL_BITS != 0) {
        return Err(LoadError::Range("grid cell bits outside 0b11"));
    }
    let grid = Grid { w, h, flags };

    Ok((Catalog { strs, effects, abilities, uabl, units, behaviours }, grid))
}

/// Loads a version 2 image into a simulation on the map's own grid.
pub fn load(bytes: &[u8], max_units: usize) -> Result<Sim, LoadError> {
    let (cat, grid) = parse(bytes)?;
    Ok(Sim::with_catalog(max_units, grid, cat))
}

pub fn load_file(path: &Path, max_units: usize) -> Result<Sim, LoadError> {
    let bytes = std::fs::read(path).map_err(|e| LoadError::Io(e.to_string()))?;
    load(&bytes, max_units)
}
