//! Writes a validated `Catalog` in the layout FORMAT.md describes.

use std::collections::HashMap;

use crate::format::*;
use crate::model::Catalog;

#[derive(Default)]
struct Strings {
    bytes: Vec<u8>,
    seen: HashMap<String, (u32, u16)>,
}

impl Strings {
    fn intern(&mut self, text: &str) -> (u32, u16) {
        if let Some(found) = self.seen.get(text) {
            return *found;
        }
        let entry = (self.bytes.len() as u32, text.len() as u16);
        self.bytes.extend_from_slice(text.as_bytes());
        self.seen.insert(text.to_owned(), entry);
        entry
    }
}

fn u16_le(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn u32_le(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn i32_le(out: &mut Vec<u8>, value: i32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn pad(out: &mut Vec<u8>, count: usize) {
    out.resize(out.len() + count, 0);
}

/// One section as the encoder wrote it: its tag, the record count and record
/// size FORMAT.md declares, and the body bytes.
pub struct Section {
    pub tag: [u8; 4],
    pub count: usize,
    pub record: usize,
    pub body: Vec<u8>,
}

pub fn encode(catalog: &Catalog) -> Vec<u8> {
    assemble(&sections(catalog))
}

pub fn sections(catalog: &Catalog) -> Vec<Section> {
    let mut strings = Strings::default();

    let mut efct = Vec::new();
    let mut efct_count: u32 = 0;
    let mut abil = Vec::new();
    for ability in &catalog.abilities {
        let effect_first = efct_count;
        for effect in &ability.effects {
            u16_le(&mut efct, effect.kind);
            pad(&mut efct, 2);
            for param in effect.params {
                i32_le(&mut efct, param);
            }
            efct_count += 1;
        }
        let (name_off, name_len) = strings.intern(&ability.name);
        let flags: u16 = if ability.projectile_speed.is_some() { 1 } else { 0 };
        let start = abil.len();
        u32_le(&mut abil, name_off);
        u16_le(&mut abil, name_len);
        u16_le(&mut abil, flags);
        u32_le(&mut abil, ability.cooldown);
        u32_le(&mut abil, ability.energy);
        abil.push(ability.target);
        abil.push(ability.effects.len() as u8);
        u16_le(&mut abil, ability.target_filters);
        u32_le(&mut abil, effect_first);
        u32_le(&mut abil, ability.projectile_speed.unwrap_or(0));
        pad(&mut abil, 4);
        debug_assert_eq!(abil.len() - start, ABIL_RECORD);
    }

    let mut uabl = Vec::new();
    let mut uabl_count: u32 = 0;
    let mut unit_bytes = Vec::new();
    for unit in &catalog.units {
        let ability_first = uabl_count;
        for &id in &unit.ability_ids {
            u16_le(&mut uabl, id);
            uabl_count += 1;
        }
        let (name_off, name_len) = strings.intern(&unit.name);
        let (weapon_damage, weapon_range, weapon_period, flags) = match unit.weapon {
            Some(w) => (w.damage, w.range, w.period, 1u16),
            None => (0, 0, 0, 0u16),
        };
        let start = unit_bytes.len();
        u32_le(&mut unit_bytes, name_off);
        u16_le(&mut unit_bytes, name_len);
        u16_le(&mut unit_bytes, flags);
        u32_le(&mut unit_bytes, unit.health);
        u32_le(&mut unit_bytes, unit.speed_milli);
        u32_le(&mut unit_bytes, unit.radius_milli);
        u32_le(&mut unit_bytes, unit.minerals);
        u32_le(&mut unit_bytes, unit.gas);
        u32_le(&mut unit_bytes, unit.build_time);
        u32_le(&mut unit_bytes, ability_first);
        u16_le(&mut unit_bytes, unit.ability_ids.len() as u16);
        u16_le(&mut unit_bytes, unit.supply);
        u32_le(&mut unit_bytes, unit.filters);
        u32_le(&mut unit_bytes, unit.shield);
        i32_le(&mut unit_bytes, unit.armor);
        i32_le(&mut unit_bytes, weapon_damage);
        u32_le(&mut unit_bytes, weapon_range);
        u32_le(&mut unit_bytes, weapon_period);
        u32_le(&mut unit_bytes, unit.sight);
        u16_le(&mut unit_bytes, unit.product);
        unit_bytes.push(unit.damage_response);
        unit_bytes.push(unit.resource_kind);
        u32_le(&mut unit_bytes, unit.harvest);
        pad(&mut unit_bytes, 4);
        debug_assert_eq!(unit_bytes.len() - start, UNIT_RECORD);
    }

    let mut bhvr = Vec::new();
    for behaviour in &catalog.behaviours {
        let (name_off, name_len) = strings.intern(&behaviour.name);
        let start = bhvr.len();
        u32_le(&mut bhvr, name_off);
        u16_le(&mut bhvr, name_len);
        bhvr.push(behaviour.trigger);
        bhvr.push(behaviour.action);
        i32_le(&mut bhvr, behaviour.param);
        u16_le(&mut bhvr, behaviour.ability);
        pad(&mut bhvr, 2);
        debug_assert_eq!(bhvr.len() - start, BHVR_RECORD);
    }

    let mut plce = Vec::new();
    for spawn in &catalog.spawns {
        let start = plce.len();
        u16_le(&mut plce, spawn.unit);
        u16_le(&mut plce, spawn.player);
        u32_le(&mut plce, spawn.x);
        u32_le(&mut plce, spawn.y);
        pad(&mut plce, 4);
        debug_assert_eq!(plce.len() - start, PLCE_RECORD);
    }

    let mut grid = Vec::with_capacity(GRID_HEADER_LEN + catalog.grid.cells.len());
    u32_le(&mut grid, catalog.grid.width);
    u32_le(&mut grid, catalog.grid.height);
    u32_le(&mut grid, catalog.grid.cell_milli);
    grid.extend_from_slice(&catalog.grid.cells);

    let strs_len = strings.bytes.len();
    let grid_len = grid.len();
    vec![
        Section { tag: TAG_STRS, count: strs_len, record: STRS_RECORD, body: strings.bytes },
        Section { tag: TAG_EFCT, count: efct_count as usize, record: EFCT_RECORD, body: efct },
        Section { tag: TAG_ABIL, count: catalog.abilities.len(), record: ABIL_RECORD, body: abil },
        Section { tag: TAG_UABL, count: uabl_count as usize, record: UABL_RECORD, body: uabl },
        Section { tag: TAG_UNIT, count: catalog.units.len(), record: UNIT_RECORD, body: unit_bytes },
        Section { tag: TAG_BHVR, count: catalog.behaviours.len(), record: BHVR_RECORD, body: bhvr },
        Section { tag: TAG_PLCE, count: catalog.spawns.len(), record: PLCE_RECORD, body: plce },
        Section { tag: TAG_GRID, count: grid_len, record: GRID_RECORD, body: grid },
    ]
}

/// Lays sections out as the header, the section table and the bodies, in order.
pub fn assemble(sections: &[Section]) -> Vec<u8> {
    let body_start = HEADER_LEN + sections.len() * SECTION_ENTRY_LEN;
    let mut table = Vec::new();
    let mut body = Vec::new();
    let mut offset = body_start;
    for section in sections {
        table.extend_from_slice(&section.tag);
        u32_le(&mut table, offset as u32);
        u32_le(&mut table, section.count as u32);
        u32_le(&mut table, section.record as u32);
        offset += section.body.len();
        body.extend_from_slice(&section.body);
    }

    let mut out = Vec::with_capacity(offset);
    out.extend_from_slice(&MAGIC);
    u16_le(&mut out, VERSION);
    u16_le(&mut out, HEADER_LEN as u16);
    u32_le(&mut out, sections.len() as u32);
    u32_le(&mut out, offset as u32);
    out.extend_from_slice(&table);
    out.extend_from_slice(&body);
    out
}
