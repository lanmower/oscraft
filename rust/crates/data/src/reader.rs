//! Reference reader: checks an image against FORMAT.md v6 before it is written.
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

    let (abil, uabl, unit, efct, grid) = (&sections[2], &sections[3], &sections[4], &sections[1], &sections[6]);

    let width = u32_at(image, grid.offset)?;
    let height = u32_at(image, grid.offset + 4)?;
    let cell_milli = u32_at(image, grid.offset + 8)?;
    ensure!(
        width >= 1 && height >= 1 && GRID_HEADER_LEN + width as usize * height as usize == grid.count,
        "grid size does not match its header"
    );

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

    for &cell in &image[grid.offset + GRID_HEADER_LEN..grid.offset + grid.count] {
        ensure!(cell & !CELL_MASK == 0, "grid cell {cell:#04x} has bits outside the walkability mask");
    }
    Ok(())
}
