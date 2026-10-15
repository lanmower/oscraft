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

pub fn encode(catalog: &Catalog) -> Vec<u8> {
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

    let mut grid = Vec::with_capacity(GRID_HEADER_LEN + catalog.grid.cells.len());
    u32_le(&mut grid, catalog.grid.width);
    u32_le(&mut grid, catalog.grid.height);
    u32_le(&mut grid, catalog.grid.cell_milli);
    grid.extend_from_slice(&catalog.grid.cells);

    let strs_len = strings.bytes.len();
    let grid_len = grid.len();
    let sections: Vec<([u8; 4], usize, usize, Vec<u8>)> = vec![
        (TAG_STRS, strs_len, STRS_RECORD, strings.bytes),
        (TAG_EFCT, efct_count as usize, EFCT_RECORD, efct),
        (TAG_ABIL, catalog.abilities.len(), ABIL_RECORD, abil),
        (TAG_UABL, uabl_count as usize, UABL_RECORD, uabl),
        (TAG_UNIT, catalog.units.len(), UNIT_RECORD, unit_bytes),
        (TAG_BHVR, catalog.behaviours.len(), BHVR_RECORD, bhvr),
        (TAG_GRID, grid_len, GRID_RECORD, grid),
    ];

    let body_start = HEADER_LEN + sections.len() * SECTION_ENTRY_LEN;
    let mut table = Vec::new();
    let mut body = Vec::new();
    let mut offset = body_start;
    for (tag, count, record, bytes) in &sections {
        table.extend_from_slice(tag);
        u32_le(&mut table, offset as u32);
        u32_le(&mut table, *count as u32);
        u32_le(&mut table, *record as u32);
        offset += bytes.len();
        body.extend_from_slice(bytes);
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
