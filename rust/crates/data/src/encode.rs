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

    let mut achn = Vec::new();
    for channel in &catalog.channels {
        u16_le(&mut achn, channel.flags);
        pad(&mut achn, 2);
        u32_le(&mut achn, channel.duration);
    }

    let mut uabl = Vec::new();
    let mut uabl_count: u32 = 0;
    let mut unit_bytes = Vec::new();
    let mut regn = Vec::new();
    let mut uext = Vec::new();
    for (index, unit) in catalog.units.iter().enumerate() {
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
        u32_le(&mut unit_bytes, unit.resource_a);
        u32_le(&mut unit_bytes, unit.gas);
        u32_le(&mut unit_bytes, unit.build_time);
        u32_le(&mut unit_bytes, ability_first);
        u16_le(&mut unit_bytes, unit.ability_ids.len() as u16);
        u16_le(&mut unit_bytes, unit.capacity);
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
        let (model_off, model_len) = strings.intern(&unit.model);
        u32_le(&mut unit_bytes, model_off);
        u16_le(&mut unit_bytes, model_len);
        pad(&mut unit_bytes, 4);
        debug_assert_eq!(unit_bytes.len() - start, UNIT_RECORD);

        let start = regn.len();
        u32_le(&mut regn, unit.shield_regen);
        u32_le(&mut regn, unit.regen_delay);
        u32_le(&mut regn, unit.health_regen);
        debug_assert_eq!(regn.len() - start, REGN_RECORD);

        let ext = catalog.unit_ext[index];
        let start = uext.len();
        u16_le(&mut uext, ext.capacity_provided);
        u16_le(&mut uext, ext.transport_capacity);
        uext.push(ext.size);
        uext.push(ext.damage_type);
        uext.push(ext.cloak);
        pad(&mut uext, 1);
        u32_le(&mut uext, ext.detector_radius);
        u32_le(&mut uext, ext.ranged_miss);
        u32_le(&mut uext, ext.burrow_ticks);
        i32_le(&mut uext, ext.rally_dx);
        i32_le(&mut uext, ext.rally_dy);
        u32_le(&mut uext, ext.bounty);
        debug_assert_eq!(uext.len() - start, UEXT_RECORD);
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
        u32_le(&mut plce, spawn.flags);
        debug_assert_eq!(plce.len() - start, PLCE_RECORD);
    }

    // Upgrade effects follow the ability effects in EFCT, one slot per upgrade.
    let mut upgr = Vec::new();
    for upgrade in &catalog.upgrades {
        let effect_index = efct_count;
        u16_le(&mut efct, upgrade.effect.kind);
        pad(&mut efct, 2);
        for param in upgrade.effect.params {
            i32_le(&mut efct, param);
        }
        efct_count += 1;

        let (name_off, name_len) = strings.intern(&upgrade.name);
        let start = upgr.len();
        u32_le(&mut upgr, name_off);
        u16_le(&mut upgr, name_len);
        upgr.push(upgrade.target_kind);
        upgr.push(upgrade.level);
        upgr.push(upgrade.apply);
        upgr.push(upgrade.stacking);
        u16_le(&mut upgr, upgrade.target);
        u32_le(&mut upgr, effect_index);
        u32_le(&mut upgr, upgrade.resource_a_per_level);
        u32_le(&mut upgr, upgrade.gas_per_level);
        debug_assert_eq!(upgr.len() - start, UPGR_RECORD);
    }

    // Research effects follow the upgrade effects, one slot per research.
    let mut rsch = Vec::new();
    for research in &catalog.researches {
        let effect_index = efct_count;
        u16_le(&mut efct, research.effect.kind);
        pad(&mut efct, 2);
        for param in research.effect.params {
            i32_le(&mut efct, param);
        }
        efct_count += 1;

        let (name_off, name_len) = strings.intern(&research.name);
        let start = rsch.len();
        u32_le(&mut rsch, name_off);
        u16_le(&mut rsch, name_len);
        u16_le(&mut rsch, research.building);
        u32_le(&mut rsch, research.time);
        u32_le(&mut rsch, research.resource_a);
        u32_le(&mut rsch, research.gas);
        u32_le(&mut rsch, effect_index);
        u32_le(&mut rsch, research.applies);
        pad(&mut rsch, 4);
        debug_assert_eq!(rsch.len() - start, RSCH_RECORD);
    }

    let mut preq = Vec::new();
    for prereq in &catalog.prereqs {
        preq.push(prereq.target_kind);
        preq.push(prereq.requires_kind);
        u16_le(&mut preq, prereq.target);
        u16_le(&mut preq, prereq.requires);
        pad(&mut preq, 2);
    }

    let mut dmgf = Vec::new();
    for factor in &catalog.damage_factors {
        dmgf.push(factor.damage_type);
        dmgf.push(factor.size);
        pad(&mut dmgf, 2);
        u32_le(&mut dmgf, factor.factor);
    }

    let econ = &catalog.econ;
    let mut econ_bytes = Vec::with_capacity(ECON_RECORD);
    econ_bytes.push(econ.patch_saturation);
    econ_bytes.push(econ.gas_saturation);
    econ_bytes.push(econ.patches_per_base);
    econ_bytes.push(econ.geysers_per_base);
    u32_le(&mut econ_bytes, econ.extractor_yield);
    u32_le(&mut econ_bytes, econ.opening_resource_a);
    u32_le(&mut econ_bytes, econ.opening_gas);
    u16_le(&mut econ_bytes, econ.opening_workers);
    u16_le(&mut econ_bytes, econ.capacity_base);
    u16_le(&mut econ_bytes, econ.capacity_max);
    pad(&mut econ_bytes, 2);
    u32_le(&mut econ_bytes, econ.patch_yield);
    u32_le(&mut econ_bytes, econ.geyser_yield);
    debug_assert_eq!(econ_bytes.len(), ECON_RECORD);

    let mut limt = Vec::new();
    u32_le(&mut limt, catalog.cell_limit);

    let mut doda = Vec::new();
    for doodad in &catalog.doodads {
        let start = doda.len();
        u32_le(&mut doda, doodad.id);
        u16_le(&mut doda, doodad.kind);
        pad(&mut doda, 2);
        u32_le(&mut doda, doodad.x);
        u32_le(&mut doda, doodad.y);
        u32_le(&mut doda, doodad.rotation);
        debug_assert_eq!(doda.len() - start, DODA_RECORD);
    }

    let mut grid = Vec::with_capacity(GRID_HEADER_LEN + 3 * catalog.grid.cells.len());
    u32_le(&mut grid, catalog.grid.width);
    u32_le(&mut grid, catalog.grid.height);
    u32_le(&mut grid, catalog.grid.cell_milli);
    u32_le(&mut grid, catalog.grid.height_step);
    grid.extend_from_slice(&catalog.grid.cells);
    grid.extend_from_slice(&catalog.grid.heights);
    grid.extend_from_slice(&catalog.grid.tiles);

    let strs_len = strings.bytes.len();
    let grid_len = grid.len();
    let sections: Vec<([u8; 4], usize, usize, Vec<u8>)> = vec![
        (TAG_STRS, strs_len, STRS_RECORD, strings.bytes),
        (TAG_EFCT, efct_count as usize, EFCT_RECORD, efct),
        (TAG_ABIL, catalog.abilities.len(), ABIL_RECORD, abil),
        (TAG_UABL, uabl_count as usize, UABL_RECORD, uabl),
        (TAG_UNIT, catalog.units.len(), UNIT_RECORD, unit_bytes),
        (TAG_BHVR, catalog.behaviours.len(), BHVR_RECORD, bhvr),
        (TAG_PLCE, catalog.spawns.len(), PLCE_RECORD, plce),
        (TAG_UPGR, catalog.upgrades.len(), UPGR_RECORD, upgr),
        (TAG_REGN, catalog.units.len(), REGN_RECORD, regn),
        (TAG_LIMT, 1, LIMT_RECORD, limt),
        (TAG_ECON, 1, ECON_RECORD, econ_bytes),
        (TAG_UEXT, catalog.units.len(), UEXT_RECORD, uext),
        (TAG_ACHN, catalog.abilities.len(), ACHN_RECORD, achn),
        (TAG_RSCH, catalog.researches.len(), RSCH_RECORD, rsch),
        (TAG_PREQ, catalog.prereqs.len(), PREQ_RECORD, preq),
        (TAG_DMGF, catalog.damage_factors.len(), DMGF_RECORD, dmgf),
        (TAG_DODA, catalog.doodads.len(), DODA_RECORD, doda),
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
