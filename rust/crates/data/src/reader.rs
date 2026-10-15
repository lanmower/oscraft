//! Reference reader: checks an image against FORMAT.md v11 before it is written.
//! The sim crate has its own loader; this one guards the generator's output.

use anyhow::{anyhow, ensure, Result};

use crate::format::*;

fn u16_at(image: &[u8], at: usize) -> Result<u16> {
    let b = image.get(at..at + 2).ok_or_else(|| anyhow!("truncated at byte {at}"))?;
    Ok(u16::from_le_bytes([b[0], b[1]]))
}

fn u32_at(image: &[u8], at: usize) -> Result<u32> {
    let b = image.get(at..at + 4).ok_or_else(|| anyhow!("truncated at byte {at}"))?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn i32_at(image: &[u8], at: usize) -> Result<i32> {
    Ok(u32_at(image, at)? as i32)
}

struct Section {
    offset: usize,
    count: usize,
}

pub fn verify(image: &[u8]) -> Result<()> {
    ensure!(image.len() >= HEADER_LEN, "image shorter than the header");
    ensure!(image[0..4] == MAGIC, "bad magic");
    let version = u16_at(image, 4)?;
    ensure!(version == VERSION, "unsupported version {version}, expected {VERSION}");
    ensure!(u16_at(image, 6)? as usize == HEADER_LEN, "bad header length");
    ensure!(u32_at(image, 8)? as usize == SECTIONS.len(), "bad section count");
    ensure!(u32_at(image, 12)? as usize == image.len(), "file length does not match");

    let mut sections = Vec::with_capacity(SECTIONS.len());
    let mut expected = HEADER_LEN + SECTIONS.len() * SECTION_ENTRY_LEN;
    for (i, (tag, record)) in SECTIONS.iter().enumerate() {
        let entry = HEADER_LEN + i * SECTION_ENTRY_LEN;
        ensure!(image.get(entry..entry + 4) == Some(&tag[..]), "section {i} tag mismatch");
        let offset = u32_at(image, entry + 4)? as usize;
        let count = u32_at(image, entry + 8)? as usize;
        ensure!(u32_at(image, entry + 12)? as usize == *record, "section {i} record size mismatch");
        ensure!(offset == expected, "section {i} is not contiguous");
        expected = offset + count * record;
        ensure!(expected <= image.len(), "section {i} runs past the end of the file");
        sections.push(Section { offset, count });
    }
    ensure!(expected == image.len(), "trailing bytes after the last section");

    let efct = &sections[1];
    let abil = &sections[2];
    let uabl = &sections[3];
    let unit = &sections[4];
    let plce = &sections[6];
    let upgr = &sections[7];
    let regn = &sections[8];
    let limt = &sections[9];
    let econ = &sections[10];
    let uext = &sections[11];
    let achn = &sections[12];
    let rsch = &sections[13];
    let preq = &sections[14];
    let dmgf = &sections[15];
    let grid = &sections[16];

    let width = u32_at(image, grid.offset)?;
    let height = u32_at(image, grid.offset + 4)?;
    let cell_milli = u32_at(image, grid.offset + 8)?;
    ensure!(
        width >= 1 && height >= 1 && GRID_HEADER_LEN + 2 * width as usize * height as usize == grid.count,
        "grid size does not match its header"
    );
    ensure!(u32_at(image, grid.offset + 12)? >= 1, "grid height step is 0");
    ensure!(limt.count == 1, "LIMT count must be 1");
    ensure!(econ.count == 1, "ECON count must be 1");
    ensure!(regn.count == unit.count, "REGN count differs from UNIT count");
    ensure!(uext.count == unit.count, "UEXT count differs from UNIT count");
    ensure!(achn.count == abil.count, "ACHN count differs from ABIL count");

    for i in 0..efct.count {
        let at = efct.offset + i * EFCT_RECORD;
        let kind = u16_at(image, at)? as usize;
        ensure!(kind <= MAX_EFFECT_KIND, "effect {i} code {kind} is above {MAX_EFFECT_KIND}");
        let name = EFFECT_KINDS[kind];
        let params = [
            i32_at(image, at + 4)?,
            i32_at(image, at + 8)?,
            i32_at(image, at + 12)?,
            i32_at(image, at + 16)?,
        ];
        ensure!(effect_params_valid(name, params), "effect {i} ({name}) params {params:?} out of range");
        if name == "move" {
            ensure!(
                move_cap_fits_grid(params[0], width, height, cell_milli),
                "effect {i} move cap {} is outside 1..=grid diagonal",
                params[0]
            );
        }
        if name == "use_calldown" {
            ensure!((params[0] as usize) < abil.count, "effect {i} names an ability past the end of ABIL");
        }
        if name == "spawn" {
            ensure!((params[0] as usize) < unit.count, "effect {i} names a unit past the end of UNIT");
        }
    }

    for i in 0..abil.count {
        let at = abil.offset + i * ABIL_RECORD;
        let first = u32_at(image, at + 20)? as usize;
        let effects = image[at + 17] as usize;
        ensure!(first + effects <= efct.count, "ability {i} effects run past EFCT");
        ensure!(image[at + 16] as usize <= TARGETS.len() - 1, "ability {i} target out of range");
        ensure!(u16_at(image, at + 18)? as u32 & !FILTER_ALL == 0, "ability {i} filter mask uses reserved bits");
    }

    for i in 0..achn.count {
        let at = achn.offset + i * ACHN_RECORD;
        let flags = u16_at(image, at)?;
        let duration = u32_at(image, at + 4)?;
        ensure!(flags & !CHANNEL_FLAG == 0, "channel {i} sets reserved flags");
        if flags & CHANNEL_FLAG != 0 {
            ensure!(duration >= 1, "channel {i} has no duration");
        } else {
            ensure!(duration == 0, "ability {i} has a duration without the channel flag");
        }
    }

    for i in 0..uabl.count {
        let id = u16_at(image, uabl.offset + i * UABL_RECORD)? as usize;
        ensure!(id < abil.count, "UABL entry {i} names an ability past the end of ABIL");
    }

    for i in 0..unit.count {
        let at = unit.offset + i * UNIT_RECORD;
        let first = u32_at(image, at + 32)? as usize;
        let count = u16_at(image, at + 36)? as usize;
        ensure!(first + count <= uabl.count, "unit {i} abilities run past UABL");
        let product = u16_at(image, at + 68)?;
        ensure!(product == NO_UNIT || (product as usize) < unit.count, "unit {i} product out of range");
        ensure!(u32_at(image, at + 40)? & !FILTER_ALL == 0, "unit {i} filter mask uses reserved bits");
        ensure!((image[at + 70] as usize) < DAMAGE_RESPONSES.len(), "unit {i} damage response out of range");
        ensure!((image[at + 71] as usize) < RESOURCE_KINDS.len(), "unit {i} resource kind out of range");
    }

    for i in 0..uext.count {
        let at = uext.offset + i * UEXT_RECORD;
        let size = image[at + 4] as usize;
        let damage_type = image[at + 5] as usize;
        let cloak = image[at + 6];
        let ranged_miss = u32_at(image, at + 12)?;
        ensure!(size < SIZE_CATEGORIES.len(), "unit {i} size category out of range");
        ensure!(damage_type < DAMAGE_TYPES.len(), "unit {i} damage type out of range");
        ensure!(cloak & !CLOAK_MASK == 0, "unit {i} cloak flags exceed the registry");
        ensure!(ranged_miss <= Q16_ONE as u32, "unit {i} miss chance above 1.0");
    }

    let mut starts = std::collections::HashSet::new();
    for i in 0..plce.count {
        let at = plce.offset + i * PLCE_RECORD;
        let unit_index = u16_at(image, at)?;
        let player = u16_at(image, at + 2)? as i32;
        let x = u32_at(image, at + 4)? as i64;
        let y = u32_at(image, at + 8)? as i64;
        let flags = u32_at(image, at + 12)?;
        ensure!(flags & !PLCE_FLAG_START == 0, "placement {i} sets reserved flags");
        ensure!(
            player == NEUTRAL_PLAYER as i32 || (0..MAX_PLAYERS).contains(&player),
            "placement {i} player {player} out of range"
        );
        if flags & PLCE_FLAG_START != 0 {
            ensure!(unit_index == NO_UNIT, "start {i} names a unit");
            ensure!(player != NEUTRAL_PLAYER as i32, "start {i} is neutral");
            ensure!(starts.insert(player), "player {player} has two start locations");
        } else {
            ensure!((unit_index as usize) < unit.count, "placement {i} names a unit past the end of UNIT");
        }
        ensure!(
            position_on_grid(x, y, width, height, cell_milli),
            "placement {i} at ({x}, {y}) is off the grid"
        );
    }

    for i in 0..upgr.count {
        let at = upgr.offset + i * UPGR_RECORD;
        let target_kind = image[at + 6] as usize;
        let level = image[at + 7];
        let apply = image[at + 8] as usize;
        let stacking = image[at + 9] as usize;
        let target = u16_at(image, at + 10)? as usize;
        let effect = u32_at(image, at + 12)? as usize;
        ensure!(target_kind < UPGRADE_TARGET_KINDS.len(), "upgrade {i} target kind out of range");
        ensure!(level >= 1, "upgrade {i} level is 0");
        ensure!(apply < UPGRADE_APPLY.len(), "upgrade {i} applies-to out of range");
        ensure!(stacking < UPGRADE_STACKING.len(), "upgrade {i} stacking rule out of range");
        let target_count = if target_kind == 0 { unit.count } else { abil.count };
        ensure!(target < target_count, "upgrade {i} target past its section");
        ensure!(effect < efct.count, "upgrade {i} effect past EFCT");
    }

    for i in 0..rsch.count {
        let at = rsch.offset + i * RSCH_RECORD;
        let building = u16_at(image, at + 6)? as usize;
        let effect = u32_at(image, at + 20)? as usize;
        let applies = u32_at(image, at + 24)?;
        ensure!(building < unit.count, "research {i} runs at a unit past UNIT");
        ensure!(effect < efct.count, "research {i} effect past EFCT");
        ensure!(applies & !FILTER_ALL == 0, "research {i} applies-to mask uses reserved bits");
    }

    for i in 0..preq.count {
        let at = preq.offset + i * PREQ_RECORD;
        let target_kind = image[at] as usize;
        let requires_kind = image[at + 1] as usize;
        let target = u16_at(image, at + 2)? as usize;
        let requires = u16_at(image, at + 4)? as usize;
        ensure!(target_kind < REQ_KINDS.len(), "prerequisite {i} target kind out of range");
        ensure!(requires_kind < REQ_KINDS.len(), "prerequisite {i} requirement kind out of range");
        let target_count = if target_kind == 0 { unit.count } else { rsch.count };
        let requires_count = if requires_kind == 0 { unit.count } else { rsch.count };
        ensure!(target < target_count, "prerequisite {i} target past its section");
        ensure!(requires < requires_count, "prerequisite {i} requirement past its section");
    }

    for i in 0..dmgf.count {
        let at = dmgf.offset + i * DMGF_RECORD;
        let damage_type = image[at] as usize;
        let size = image[at + 1] as usize;
        ensure!(damage_type < DAMAGE_TYPES.len(), "damage factor {i} type out of range");
        ensure!(size < SIZE_CATEGORIES.len(), "damage factor {i} size out of range");
    }

    let walkability_end = grid.offset + GRID_HEADER_LEN + width as usize * height as usize;
    for &cell in &image[grid.offset + GRID_HEADER_LEN..walkability_end] {
        ensure!(cell & !CELL_MASK == 0, "grid cell {cell:#04x} has bits outside the walkability mask");
    }
    Ok(())
}
