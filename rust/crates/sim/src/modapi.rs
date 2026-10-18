use crate::arrays::Sim;
use crate::loader::{Ability, Effect, Name};
use crate::MAX_PLAYERS;

/// The effect kinds a mod may name, in catalog kind order.
pub const MOD_EFFECTS: [&str; 5] = ["damage", "heal", "move", "spawn", "buff"];

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
    MOD_EFFECTS.iter().position(|k| *k == name).map(|k| k as u16)
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
        if !valid_ability_name(spec.name) {
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
            filter: spec.filter,
            skill: None,
        };
        match existing {
            Some(i) => {
                cat.abilities[i] = ability;
                Ok(i)
            }
            None => {
                cat.abilities.push(ability);
                Ok(cat.abilities.len() - 1)
            }
        }
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
    pub fn add_hero_gold(&mut self, unit: usize, amount: i32) -> bool {
        if !self.is_hero(unit) {
            return false;
        }
        self.heroes.gold[unit] = self.heroes.gold[unit].saturating_add(amount).max(0);
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
