//! HET/BET tables (format 3 and later). Both open with a 12-byte clear prefix;
//! MPQ encrypts the rest with the classic hash-table key (HET) or the
//! block-table key (BET). dwTableSize counts the bytes after that prefix.

use crate::crypt::{decrypt_block, hash_string};
use crate::jenkins::hash_jenkins;
use crate::source::Source;
use crate::{le_u32, Entry, Error, Result, FILE_KEY, FLAG_EXISTS, MAGIC_BET, MAGIC_HET};

const TABLE_EXT_HEADER: usize = 12;
const LIMIT_BYTES: u64 = 1 << 28;

pub(crate) struct HetTable {
    raw: Vec<u8>,
    hash_table_size: u32,
    hash_entry_size: u32,
    total_index_size: u32,
    index_size: u32,
    hash_off: usize,
    index_off: usize,
}

pub(crate) struct BetTable {
    raw: Vec<u8>,
    file_count: u32,
    table_entry_size: u32,
    bit_index_file_pos: u32,
    bit_index_file_size: u32,
    bit_index_cmp_size: u32,
    bit_index_flag_idx: u32,
    bit_count_file_pos: u32,
    bit_count_file_size: u32,
    bit_count_cmp_size: u32,
    bit_count_flag_idx: u32,
    total_bet_hash_size: u32,
    bet_hash_size: u32,
    flag_count: u32,
    flag_off: usize,
    entry_off: usize,
    hash_off: usize,
}

pub(crate) fn read_het(src: &Source, pos: u64, limit: u64) -> Result<HetTable> {
    if pos >= limit {
        return Err(Error::Corrupt(format!(
            "mpq: HET table at {pos} past end {limit}"
        )));
    }
    let mut probe = [0u8; 44];
    src.read_exact_at(pos, &mut probe)?;
    let sig = le_u32(&probe, 0);
    if sig != MAGIC_HET {
        return Err(Error::Corrupt(format!("mpq: HET signature {sig:08x}")));
    }
    // The geometry is itself encrypted, so it is decrypted from a probe first.
    decrypt_block(&mut probe[TABLE_EXT_HEADER..], hash_string(b"(hash table)", FILE_KEY));
    let table_size = le_u32(&probe, 12) as u64;
    let hash_table_size = le_u32(&probe, 20);
    let total_index_size = le_u32(&probe, 28);
    if hash_table_size == 0 || hash_table_size > 1 << 26 {
        return Err(Error::Corrupt(format!(
            "mpq: HET hashTableSize {hash_table_size}"
        )));
    }
    if total_index_size == 0 || total_index_size > 64 {
        return Err(Error::Corrupt(format!(
            "mpq: HET totalIndexSize {total_index_size}"
        )));
    }
    let want = 44
        + hash_table_size as u64
        + (hash_table_size as u64 * total_index_size as u64 + 7) / 8;
    let mut size = TABLE_EXT_HEADER as u64 + table_size;
    if size < want || size > LIMIT_BYTES {
        size = want;
    }
    if size > LIMIT_BYTES || size > limit - pos {
        return Err(Error::Unsupported(format!(
            "mpq: HET table size {size} of {} available",
            limit - pos
        )));
    }
    let mut raw = vec![0u8; size as usize];
    src.read_exact_at(pos, &mut raw)?;
    decrypt_block(&mut raw[TABLE_EXT_HEADER..], hash_string(b"(hash table)", FILE_KEY));

    let hash_table_size = le_u32(&raw, 20);
    let hash_entry_size = le_u32(&raw, 24);
    let total_index_size = le_u32(&raw, 28);
    let index_size = le_u32(&raw, 36);
    if hash_entry_size < 8 || hash_entry_size > 64 {
        return Err(Error::Unsupported(format!(
            "mpq: HET hashEntrySize {hash_entry_size} bits"
        )));
    }
    if total_index_size == 0 || total_index_size > 64 {
        return Err(Error::Corrupt(format!(
            "mpq: HET totalIndexSize {total_index_size} bits"
        )));
    }
    if index_size == 0 || index_size > total_index_size {
        return Err(Error::Corrupt(format!(
            "mpq: HET indexSize {index_size} of total {total_index_size}"
        )));
    }
    if hash_table_size == 0 || hash_table_size > 1 << 26 {
        return Err(Error::Corrupt(format!(
            "mpq: HET hashTableSize {hash_table_size}"
        )));
    }
    let need = 44
        + hash_table_size as u64
        + (hash_table_size as u64 * total_index_size as u64 + 7) / 8;
    if (raw.len() as u64) < need {
        return Err(Error::Corrupt(format!(
            "mpq: HET table {} bytes, needs {need}",
            raw.len()
        )));
    }
    Ok(HetTable {
        raw,
        hash_table_size,
        hash_entry_size,
        total_index_size,
        index_size,
        hash_off: 44,
        index_off: 44 + hash_table_size as usize,
    })
}

pub(crate) fn read_bet(src: &Source, pos: u64, limit: u64) -> Result<BetTable> {
    if pos >= limit {
        return Err(Error::Corrupt(format!(
            "mpq: BET table at {pos} past end {limit}"
        )));
    }
    let mut probe = [0u8; 88];
    src.read_exact_at(pos, &mut probe)?;
    let sig = le_u32(&probe, 0);
    if sig != MAGIC_BET {
        return Err(Error::Corrupt(format!("mpq: BET signature {sig:08x}")));
    }
    decrypt_block(&mut probe[TABLE_EXT_HEADER..], hash_string(b"(block table)", FILE_KEY));
    let table_size = le_u32(&probe, 12) as u64;
    let file_count = le_u32(&probe, 16);
    let table_entry_size = le_u32(&probe, 24);
    let flag_count = le_u32(&probe, 84);
    if file_count == 0 || file_count > 1 << 24 {
        return Err(Error::Corrupt(format!("mpq: BET fileCount {file_count}")));
    }
    if table_entry_size == 0 || table_entry_size > 256 {
        return Err(Error::Corrupt(format!(
            "mpq: BET tableEntrySize {table_entry_size}"
        )));
    }
    if flag_count > 1 << 16 {
        return Err(Error::Corrupt(format!("mpq: BET flagCount {flag_count}")));
    }
    let want = 88
        + flag_count as u64 * 4
        + (file_count as u64 * table_entry_size as u64 + 7) / 8;
    let mut size = TABLE_EXT_HEADER as u64 + table_size;
    if size < want || size > LIMIT_BYTES {
        size = want;
    }
    if size > LIMIT_BYTES || size > limit - pos {
        return Err(Error::Unsupported(format!(
            "mpq: BET table size {size} of {} available",
            limit - pos
        )));
    }
    let mut raw = vec![0u8; size as usize];
    src.read_exact_at(pos, &mut raw)?;
    decrypt_block(&mut raw[TABLE_EXT_HEADER..], hash_string(b"(block table)", FILE_KEY));

    let file_count = le_u32(&raw, 16);
    let table_entry_size = le_u32(&raw, 24);
    let total_bet_hash_size = le_u32(&raw, 68);
    let bet_hash_size = le_u32(&raw, 76);
    let bet_hash_array_size = le_u32(&raw, 80);
    let flag_count = le_u32(&raw, 84);
    if file_count == 0 || file_count > 1 << 24 {
        return Err(Error::Corrupt(format!("mpq: BET fileCount {file_count}")));
    }
    if table_entry_size == 0 || table_entry_size > 256 {
        return Err(Error::Corrupt(format!(
            "mpq: BET tableEntrySize {table_entry_size}"
        )));
    }
    if total_bet_hash_size > 64 || bet_hash_size > total_bet_hash_size {
        return Err(Error::Corrupt(format!(
            "mpq: BET hash sizes {total_bet_hash_size}/{bet_hash_size}"
        )));
    }
    let flag_off = 88usize;
    let entry_off = flag_off + flag_count as usize * 4;
    let hash_off = entry_off + ((file_count as u64 * table_entry_size as u64 + 7) / 8) as usize;
    let need = hash_off as u64 + bet_hash_array_size as u64;
    if (raw.len() as u64) < need {
        return Err(Error::Corrupt(format!(
            "mpq: BET table {} bytes, needs {need}",
            raw.len()
        )));
    }
    Ok(BetTable {
        bit_index_file_pos: le_u32(&raw, 28),
        bit_index_file_size: le_u32(&raw, 32),
        bit_index_cmp_size: le_u32(&raw, 36),
        bit_index_flag_idx: le_u32(&raw, 40),
        bit_count_file_pos: le_u32(&raw, 48),
        bit_count_file_size: le_u32(&raw, 52),
        bit_count_cmp_size: le_u32(&raw, 56),
        bit_count_flag_idx: le_u32(&raw, 60),
        raw,
        file_count,
        table_entry_size,
        total_bet_hash_size,
        bet_hash_size,
        flag_count,
        flag_off,
        entry_off,
        hash_off,
    })
}

/// Resolves `key` (already HET-normalised) to its BET entry.
pub(crate) fn lookup(het: &HetTable, bet: &BetTable, key: &[u8]) -> Option<Entry> {
    let nbs = het.hash_entry_size;
    let and_mask: u64 = if nbs < 64 { (1u64 << nbs) - 1 } else { u64::MAX };
    let or_mask: u64 = 1u64 << (nbs - 1);
    let full = (hash_jenkins(key) & and_mask) | or_mask;
    let het_hash = (full >> (nbs - 8)) as u8;
    let bet_hash = full & (and_mask >> 8);

    let table = het.hash_table_size as u64;
    let start = full % table;
    let idx = BitIndex {
        raw: &het.raw,
        base: het.index_off,
    };
    let bets = BitIndex {
        raw: &bet.raw,
        base: bet.hash_off,
    };
    for i in 0..table {
        let at = ((start + i) % table) as usize;
        let got = het.raw[het.hash_off + at];
        if got == 0 {
            break;
        }
        if got != het_hash {
            continue;
        }
        let file_idx = idx.field(at as u64 * het.total_index_size as u64, het.index_size);
        if file_idx >= bet.file_count as u64 {
            continue;
        }
        if bet.bet_hash_size != 0
            && bets.field(
                file_idx * bet.total_bet_hash_size as u64,
                bet.bet_hash_size,
            ) != bet_hash
        {
            continue;
        }
        return Some(bet.entry(file_idx as u32));
    }
    None
}

impl BetTable {
    fn entry(&self, i: u32) -> Entry {
        let r = BitIndex {
            raw: &self.raw,
            base: self.entry_off,
        };
        let off = i as u64 * self.table_entry_size as u64;
        let pos = r.field(off + self.bit_index_file_pos as u64, self.bit_count_file_pos);
        let size = r.field(off + self.bit_index_file_size as u64, self.bit_count_file_size);
        let cmp = r.field(off + self.bit_index_cmp_size as u64, self.bit_count_cmp_size);
        let flag_idx = r.field(off + self.bit_index_flag_idx as u64, self.bit_count_flag_idx);
        let mut flags = FLAG_EXISTS;
        if flag_idx < self.flag_count as u64 {
            let p = self.flag_off + flag_idx as usize * 4;
            if p + 4 <= self.raw.len() {
                flags = le_u32(&self.raw, p);
            }
        }
        Entry {
            name: String::new(),
            file_size: size as u32,
            comp_size: cmp as u32,
            flags,
            offset: pos,
        }
    }
}

/// Bit-packed array, least significant bit first. Bits past the end read as one,
/// matching the MPQ saturating reader.
struct BitIndex<'a> {
    raw: &'a [u8],
    base: usize,
}

impl BitIndex<'_> {
    fn field(&self, bit: u64, n: u32) -> u64 {
        let mut v = 0u64;
        for i in 0..n {
            let p = self.base as u64 * 8 + bit + i as u64;
            let by = (p >> 3) as usize;
            let set = if by >= self.raw.len() {
                true
            } else {
                (self.raw[by] >> (p & 7) as u32) & 1 != 0
            };
            if set {
                v |= 1u64.checked_shl(i).unwrap_or(0);
            }
        }
        v
    }
}
