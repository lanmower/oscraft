use crate::arrays::Sim;
use crate::loader::{Ability, Effect, Item, Name, Regen, Research, Unit, UnitExt, Prereq, NO_INDEX};
use crate::rules::EFFECT_STAT;
use crate::{fixed, ABILITY_SLOTS, MAX_PLAYERS};

/// The effect kinds a mod may name.
pub const MOD_EFFECTS: [&str; 7] = ["damage", "heal", "move", "spawn", "buff", "shield", "stat"];

/// The stats an effect of kind "stat" may raise, by the code in its first parameter.
pub const STAT_NAMES: [&str; 5] = ["damage", "armor", "health", "speed", "range"];

const MOD_TARGET_NONE: u8 = 0;
const MOD_TARGET_UNIT: u8 = 1;
const MOD_TARGET_POINT: u8 = 2;
const MAX_EFFECTS: usize = u8::MAX as usize;

/// A mod-defined ability: the fields of a catalog ability before it is appended.
pub struct AbilitySpec<'a> {
    pub name: &'a str,
    pub cooldown: i32,
    pub energy: i32,
    pub target: u8,
    pub filter: u16,
    pub effects: Vec<Effect>,
}

pub fn effect_kind(name: &str) -> Option<u16> {
    match name {
        "shield" => Some(10),
        "stat" => Some(EFFECT_STAT),
        _ => MOD_EFFECTS[..5].iter().position(|k| *k == name).map(|k| k as u16),
    }
}

pub fn stat_code(name: &str) -> Option<i32> {
    STAT_NAMES.iter().position(|k| *k == name).map(|k| k as i32)
}

/// A mod-defined unit: the catalog fields a mod may set. Speed, radius, range and sight are milli-units.
#[derive(Clone)]
pub struct UnitSpec {
    pub name: String,
    pub model: String,
    pub health: i32,
    pub speed: i32,
    pub radius: i32,
    pub armor: i32,
    pub shield: i32,
    pub sight: i32,
    pub filter: u32,
    pub cost: i32,
    pub bounty: i32,
    pub weapon: Option<(i32, i32, i32)>,
    pub abilities: Vec<String>,
}

/// A mod-defined item. `ability` names the active ability the item casts.
#[derive(Clone)]
pub struct ItemSpec {
    pub name: String,
    pub cost: i32,
    pub armor: i32,
    pub damage: i32,
    pub health: i32,
    pub ability: Option<String>,
}

/// A mod-defined upgrade (a research record). `requires` names researches that must be complete
/// first, `unlocks` names unit kinds the research opens for production.
#[derive(Clone)]
pub struct UpgradeSpec {
    pub name: String,
    pub building: String,
    pub time: i32,
    pub cost: i32,
    pub gas: i32,
    pub effect: Effect,
    pub filter: u32,
    pub requires: Vec<String>,
    pub unlocks: Vec<String>,
}

pub fn target_code(name: &str) -> Option<u8> {
    match name {
        "none" => Some(MOD_TARGET_NONE),
        "unit" => Some(MOD_TARGET_UNIT),
        "point" => Some(MOD_TARGET_POINT),
        _ => None,
    }
}

fn valid_ability_name(s: &str) -> bool {
    s.as_bytes().first().is_some_and(|c| c.is_ascii_lowercase())
        && s.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
}

impl Sim {
    /// Appends a mod-defined ability to the catalog, or redefines the ability of that name. Returns its
    /// catalog index. The name must be a lowercase identifier and an ability holds at most 255 effects.
    pub fn define_ability(&mut self, spec: AbilitySpec) -> Result<usize, &'static str> {
        if !valid_ability_name(&spec.name) {
            return Err("invalid_name");
        }
        if spec.effects.len() > MAX_EFFECTS {
            return Err("too_many_effects");
        }
        if spec.cooldown < 0 || spec.energy < 0 {
            return Err("negative_cost");
        }
        let cat = &mut self.cat;
        let existing = cat.abilities.iter().position(|a| cat.name(a.name) == spec.name);
        let name = match existing {
            Some(i) => cat.abilities[i].name,
            None => {
                let offset = cat.strs.len() as u32;
                cat.strs.extend_from_slice(spec.name.as_bytes());
                Name { offset, len: spec.name.len() as u16 }
            }
        };
        let effect_first = cat.effects.len() as u32;
        cat.effects.extend_from_slice(&spec.effects);
        let ability = Ability {
            name,
            flags: 0,
            cooldown: spec.cooldown,
            energy: spec.energy,
            target: spec.target,
            effect_count: spec.effects.len() as u8,
            effect_first,
            projectile_speed: 0,
            initial_cooldown: 0,
            filter: spec.filter,
            skill: None,
        };
        match existing {
            Some(i) => {
                cat.abilities[i] = ability;
                cat.ext[i] = Default::default();
                Ok(i)
            }
            None => {
                cat.abilities.push(ability);
                cat.ext.push(Default::default());
                Ok(cat.abilities.len() - 1)
            }
        }
    }

    fn intern(&mut self, text: &str) -> Name {
        let offset = self.cat.strs.len() as u32;
        self.cat.strs.extend_from_slice(text.as_bytes());
        Name { offset, len: text.len() as u16 }
    }

    /// Appends a mod-defined unit kind to the catalog, or redefines the kind of that name in place
    /// (units already alive keep the stats they spawned with). Returns its catalog index.
    pub fn define_unit(&mut self, spec: &UnitSpec) -> Result<usize, &'static str> {
        if !valid_ability_name(&spec.name) {
            return Err("invalid_name");
        }
        if spec.health <= 0 || spec.speed < 0 || spec.radius < 0 || spec.armor < 0 || spec.shield < 0 || spec.sight < 0 || spec.cost < 0 || spec.bounty < 0 {
            return Err("negative_stat");
        }
        if spec.abilities.len() > ABILITY_SLOTS {
            return Err("too_many_abilities");
        }
        let mut abilities = Vec::new();
        for name in &spec.abilities {
            let at = self.cat.abilities.iter().position(|a| self.cat.name(a.name) == name.as_str()).ok_or("unknown_ability")?;
            abilities.push(at as u16);
        }
        let existing = self.cat.unit_index(&spec.name);
        let name = match existing {
            Some(i) => self.cat.units[i].name,
            None => self.intern(&spec.name),
        };
        let model = self.intern(&spec.model);
        let abil_first = self.cat.uabl.len() as u32;
        let abil_count = abilities.len() as u16;
        self.cat.uabl.extend(abilities);
        let (weapon_damage, weapon_range, weapon_period) = spec.weapon.unwrap_or((0, 0, 0));
        let unit = Unit {
            name,
            model,
            flags: u16::from(spec.weapon.is_some()),
            health: spec.health,
            speed: spec.speed,
            radius: spec.radius,
            resource_a: spec.cost,
            abil_first,
            abil_count,
            filter: spec.filter,
            shield: spec.shield,
            armor: spec.armor,
            weapon_damage,
            weapon_range,
            weapon_period,
            sight: spec.sight,
            product: NO_INDEX,
            ..Unit::default()
        };
        let ext = UnitExt { bounty: spec.bounty, ..UnitExt::default() };
        match existing {
            Some(i) => {
                self.cat.units[i] = unit;
                self.cat.unit_ext[i] = ext;
                Ok(i)
            }
            None => {
                self.cat.units.push(unit);
                self.cat.unit_ext.push(ext);
                self.cat.regen.push(Regen::default());
                Ok(self.cat.units.len() - 1)
            }
        }
    }

    /// Appends a mod-defined item to the catalog, or redefines the item of that name in place.
    /// Returns its catalog index, the number `buy_item` takes.
    pub fn define_item(&mut self, spec: &ItemSpec) -> Result<usize, &'static str> {
        if !valid_ability_name(&spec.name) {
            return Err("invalid_name");
        }
        if spec.cost < 0 {
            return Err("negative_cost");
        }
        let ability = match spec.ability.as_deref() {
            None => NO_INDEX,
            Some(a) => self.cat.abilities.iter().position(|x| self.cat.name(x.name) == a).ok_or("unknown_ability")? as u16,
        };
        let existing = self.cat.items.iter().position(|i| self.cat.name(i.name) == spec.name.as_str());
        let name = match existing {
            Some(i) => self.cat.items[i].name,
            None => self.intern(&spec.name),
        };
        let mut stats = [0; crate::inventory::ITEM_STAT_COUNT];
        stats[crate::inventory::STAT_SLOT_0] = spec.armor;
        stats[crate::inventory::STAT_DAMAGE] = spec.damage;
        stats[crate::inventory::STAT_HEALTH] = spec.health;
        let item = Item {
            name,
            cost: spec.cost,
            stats,
            ability,
            tags: u8::MAX,
            components: [NO_INDEX; crate::inventory::ITEM_COMPONENTS],
            ..Item::default()
        };
        match existing {
            Some(i) => {
                self.cat.items[i] = item;
                Ok(i)
            }
            None => {
                self.cat.items.push(item);
                Ok(self.cat.items.len() - 1)
            }
        }
    }

    /// Appends a mod-defined upgrade to the catalog as a research record, with its prerequisites,
    /// or redefines the upgrade of that name (its prerequisites are added, never removed). The effect
    /// also reaches units built after the research completes. Returns its research index.
    pub fn define_upgrade(&mut self, spec: &UpgradeSpec) -> Result<usize, &'static str> {
        if !valid_ability_name(&spec.name) {
            return Err("invalid_name");
        }
        if spec.time < 0 || spec.cost < 0 || spec.gas < 0 {
            return Err("negative_cost");
        }
        let building = self.cat.unit_index(&spec.building).ok_or("unknown_unit")? as u16;
        let mut requires = Vec::new();
        for r in &spec.requires {
            requires.push(self.research_index(r).ok_or("unknown_upgrade")?);
        }
        let mut unlocks = Vec::new();
        for u in &spec.unlocks {
            unlocks.push(self.cat.unit_index(u).ok_or("unknown_unit")?);
        }
        let existing = self.research_index(&spec.name);
        let name = match existing {
            Some(r) => self.cat.research[r].name,
            None => self.intern(&spec.name),
        };
        let effect = self.cat.effects.len() as u32;
        self.cat.effects.push(spec.effect);
        let rec = Research { name, building, time: spec.time, resource_a: spec.cost, gas: spec.gas, effect, applies_mask: spec.filter, to_new: true };
        let r = match existing {
            Some(r) => {
                self.cat.research[r] = rec;
                r
            }
            None => {
                let old = self.cat.research.len();
                self.cat.research.push(rec);
                let mut grown = vec![0u8; (old + 1) * MAX_PLAYERS];
                for p in 0..MAX_PLAYERS {
                    grown[p * (old + 1)..p * (old + 1) + old].copy_from_slice(&self.researched[p * old..(p + 1) * old]);
                }
                self.researched = grown;
                old
            }
        };
        for q in requires {
            self.cat.prereqs.push(Prereq { target_kind: 1, requirement_kind: 1, target: r as u16, requirement: q as u16 });
        }
        for u in unlocks {
            self.cat.prereqs.push(Prereq { target_kind: 0, requirement_kind: 1, target: u as u16, requirement: r as u16 });
        }
        Ok(r)
    }

    /// The catalog research index of the research named `name`.
    pub fn research_index(&self, name: &str) -> Option<usize> {
        self.cat.research.iter().position(|r| self.cat.name(r.name) == name)
    }

    /// Completes research `r` for `player` at no cost and time: marks it done and applies its effect
    /// to the player's living units it matches. False for an unknown research or player, or one
    /// the player already holds.
    pub fn grant_research(&mut self, player: u8, r: usize) -> bool {
        let rc = self.cat.research.len();
        if r >= rc || (player as usize) >= MAX_PLAYERS || self.researched[player as usize * rc + r] != 0 {
            return false;
        }
        self.researched[player as usize * rc + r] = 1;
        let rec = self.cat.research[r];
        if let Some(eff) = self.cat.effects.get(rec.effect as usize).copied() {
            for j in 0..self.n {
                if self.alive[j] != 0 && self.player[j] == player && self.meets_filter(j, rec.applies_mask) {
                    self.apply_effect(j, 0, eff, self.pos_x[j], self.pos_y[j], j as i32, true);
                }
            }
        }
        true
    }

    /// Applies the effect of every to_new research `player` holds to unit `i`, just built.
    pub(crate) fn apply_research_to_new(&mut self, i: usize, player: u8) {
        let rc = self.cat.research.len();
        if (player as usize) >= MAX_PLAYERS {
            return;
        }
        for r in 0..rc {
            let rec = self.cat.research[r];
            if rec.to_new && self.researched[player as usize * rc + r] != 0 && self.meets_filter(i, rec.applies_mask) {
                if let Some(eff) = self.cat.effects.get(rec.effect as usize).copied() {
                    self.apply_effect(i, 0, eff, self.pos_x[i], self.pos_y[i], i as i32, true);
                }
            }
        }
    }

    /// Adds a flat bonus to one stat of the receiver (the cast target unit, or the caster when the
    /// cast names none). `stat` is a `STAT_NAMES` code; damage, armor and health take whole units,
    /// speed and range milli-units.
    pub(crate) fn stat_bonus(&mut self, caster: usize, target: i32, stat: i32, amount: i32) {
        let r = if target >= 0 && (target as usize) < self.n { target as usize } else { caster };
        if r >= self.n || self.alive[r] == 0 {
            self.refuse(crate::rules::Refusal::NoTarget);
            return;
        }
        match stat {
            0 => self.damage[r] = self.damage[r].saturating_add(fixed::from_int(amount)),
            1 => self.armor[r] = self.armor[r].saturating_add(fixed::from_int(amount)),
            2 => {
                let by = fixed::from_int(amount);
                self.hp_max[r] = self.hp_max[r].saturating_add(by);
                self.hp[r] = self.hp[r].saturating_add(by).clamp(1, self.hp_max[r].max(1));
            }
            3 => self.speed[r] = self.speed[r].saturating_add(fixed::from_milli(amount)),
            4 => {
                self.range_q[r] = self.range_q[r].saturating_add(fixed::from_milli(amount));
                self.range_sq[r] = fixed::mul(self.range_q[r], self.range_q[r]);
            }
            _ => self.refuse(crate::rules::Refusal::EffectUnsupported),
        }
        self.touch(r);
    }

    /// The unit credited with unit `i`'s latest death, or None when nothing is credited.
    pub fn killer_of(&self, i: usize) -> Option<usize> {
        self.killer.get(i).and_then(|&k| usize::try_from(k).ok())
    }

    /// The catalog ability index in ability slot `slot` of unit `i`, or None when the slot is empty.
    pub fn ability_in_slot(&self, i: usize, slot: usize) -> Option<usize> {
        if i >= self.cap || slot >= ABILITY_SLOTS {
            return None;
        }
        usize::try_from(self.abil_idx[i * ABILITY_SLOTS + slot]).ok()
    }

    /// Ticks left on the active ability of the item in inventory slot `slot` of unit `i`; 0 for none.
    pub fn item_cooldown(&self, i: usize, slot: usize) -> i32 {
        if i >= self.cap || slot >= crate::inventory::INV_SLOTS {
            return 0;
        }
        self.item_cd[i * crate::inventory::INV_SLOTS + slot]
    }

    /// Puts the catalog ability `name` into ability slot `slot` of a living unit. False when the unit
    /// is dead, the slot is out of range, or no ability has that name.
    pub fn give_ability(&mut self, unit: usize, slot: usize, name: &str) -> bool {
        if unit >= self.n || self.alive[unit] == 0 || slot >= crate::ABILITY_SLOTS {
            return false;
        }
        let Some(ai) = self.cat.abilities.iter().position(|a| self.cat.name(a.name) == name) else { return false };
        let at = unit * crate::ABILITY_SLOTS + slot;
        self.abil_idx[at] = ai as i32;
        self.abil_cd[at] = 0;
        self.abil_rank[at] = 0;
        self.abil_on = true;
        true
    }

    /// Adds `amount` (negative to take) to the gold of a hero unit. False when the unit is not a hero.
    pub fn add_unit_gold(&mut self, unit: usize, amount: i32) -> bool {
        if !self.is_hero(unit) {
            return false;
        }
        self.attributed.gold[unit] = self.attributed.gold[unit].saturating_add(amount).max(0);
        true
    }

    /// Declares the team of `player` the winner. The first winner stands.
    pub fn declare_victory(&mut self, player: u8) {
        if self.winner < 0 && (player as usize) < MAX_PLAYERS {
            self.winner = self.team[player as usize] as i32;
        }
    }

    /// Declares `player` defeated: the team of the first player hostile to it wins. The first winner stands.
    pub fn declare_defeat(&mut self, player: u8) {
        if self.winner >= 0 || (player as usize) >= MAX_PLAYERS {
            return;
        }
        if let Some(rival) = self.rival_of(player) {
            self.winner = self.team[rival as usize] as i32;
        }
    }
}
