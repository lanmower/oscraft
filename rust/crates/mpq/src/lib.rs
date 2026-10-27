#![forbid(unsafe_code)]

mod bzip2;
mod checksum;
mod crypt;
mod het;
mod huffman;
mod huffman_tables;
mod inflate;
mod jenkins;
mod lzma;
mod pkware;
mod source;
mod sparse;

use std::cell::Cell;
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

use crypt::{decrypt_block, hash_string};
use source::Source;

const MAGIC_MPQ: u32 = 0x1a51_504d;
const MAGIC_MPQ_USER_DATA: u32 = 0x1b51_504d;
const MAGIC_HET: u32 = 0x1a54_4548;
const MAGIC_BET: u32 = 0x1a54_4542;

const HASH_NAME_IDX: u32 = 0;
const HASH_NAME_A: u32 = 1;
const HASH_NAME_B: u32 = 2;
const FILE_KEY: u32 = 3;

const FLAG_EXISTS: u32 = 0x8000_0000;
const FLAG_SECTOR_CRC: u32 = 0x0400_0000;
const FLAG_DELETED: u32 = 0x0200_0000;
const FLAG_SINGLE_UNIT: u32 = 0x0100_0000;
const FLAG_FIX_KEY: u32 = 0x0002_0000;
const FLAG_ENCRYPTED: u32 = 0x0001_0000;
const FLAG_COMPRESS: u32 = 0x0000_0200;
const FLAG_IMPLODE: u32 = 0x0000_0100;
const FLAG_COMPRESSED: u32 = FLAG_COMPRESS | FLAG_IMPLODE;

const COMP_HUFFMAN: u8 = 0x01;
const COMP_ZLIB: u8 = 0x02;
const COMP_PKWARE: u8 = 0x08;
const COMP_BZIP2: u8 = 0x10;
const COMP_SPARSE: u8 = 0x20;
const COMP_ADPCM_M: u8 = 0x40;
const COMP_ADPCM_S: u8 = 0x80;
const COMP_LZMA: u8 = 0x12;

#[derive(Debug)]
pub enum Error {
    Magic,
    NoFile(String),
    Corrupt(String),
    Unsupported(String),
    Io(io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Magic => write!(f, "mpq: no MPQ header found"),
            Error::NoFile(name) => write!(f, "mpq: file not in archive: {name}"),
            Error::Corrupt(msg) | Error::Unsupported(msg) => write!(f, "{msg}"),
            Error::Io(e) => write!(f, "mpq: io: {e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub file_size: u32,
    pub comp_size: u32,
    pub flags: u32,
    pub offset: u64,
}

impl Entry {
    pub fn is_encrypted(&self) -> bool {
        self.flags & FLAG_ENCRYPTED != 0
    }

    pub fn is_compressed(&self) -> bool {
        self.flags & FLAG_COMPRESSED != 0
    }
}

#[derive(Clone, Copy)]
struct HashEntry {
    name_a: u32,
    name_b: u32,
    block: u32,
}

pub struct Archive {
    src: Source,
    size: u64,
    sector_size: u32,
    format: u16,
    header_offset: u64,
    hi_block_pos: u64,
    hash: Option<Vec<HashEntry>>,
    block: Vec<Entry>,
    het: Option<het::HetTable>,
    bet: Option<het::BetTable>,
    warn: Vec<String>,
    bytes_read: Cell<u64>,
}

pub fn open(path: impl AsRef<Path>) -> Result<Archive> {
    let file = fs::File::open(path)?;
    let len = file.metadata()?.len();
    Archive::parse(Source::new(file, len))
}

impl Archive {
    fn parse(src: Source) -> Result<Archive> {
        let size = src.size();
        let mut a = Archive {
            src,
            size,
            sector_size: 0,
            format: 0,
            header_offset: 0,
            hi_block_pos: 0,
            hash: None,
            block: Vec::new(),
            het: None,
            bet: None,
            warn: Vec::new(),
            bytes_read: Cell::new(0),
        };
        let mut hdr = [0u8; 32];
        let mut found = false;
        let mut off: i64 = 0;
        while off + 32 <= size as i64 && off < 0x0400_0000 {
            match a.src.read_at(off as u64, &mut hdr) {
                Ok(32) => {}
                _ => break,
            }
            match le_u32(&hdr, 0) {
                MAGIC_MPQ => {
                    a.header_offset = off as u64;
                    found = true;
                }
                MAGIC_MPQ_USER_DATA => {
                    let next = le_u32(&hdr, 8) as i64;
                    if next > 0 {
                        off += next - 512;
                    }
                }
                _ => {}
            }
            if found {
                break;
            }
            off += 512;
        }
        if !found {
            return Err(Error::Magic);
        }
        a.add_read(32);
        a.read_header()?;
        Ok(a)
    }

    fn add_read(&self, n: usize) {
        self.bytes_read.set(self.bytes_read.get() + n as u64);
    }

    fn read_header(&mut self) -> Result<()> {
        let mut raw = [0u8; 208];
        let n = self.src.read_at(self.header_offset, &mut raw)?;
        let buf = &raw[..n];
        if n < 32 || le_u32(buf, 0) != MAGIC_MPQ {
            return Err(Error::Corrupt(format!(
                "mpq: header at {} is not an MPQ header",
                self.header_offset
            )));
        }
        self.format = le_u16(buf, 12);
        let shift = le_u16(buf, 14);
        self.sector_size = 512u32.checked_shl(shift as u32).unwrap_or(0);
        if self.sector_size == 0 || self.sector_size > 1 << 24 {
            return Err(Error::Corrupt(format!(
                "mpq: implausible sector size {} (header wBlockSize={shift})",
                self.sector_size
            )));
        }
        let mut hash_pos = le_u32(buf, 16) as u64;
        let mut block_pos = le_u32(buf, 20) as u64;
        let hash_size = le_u32(buf, 24);
        let block_size = le_u32(buf, 28);
        let mut het_pos = 0u64;
        let mut bet_pos = 0u64;
        if self.format >= 1 && n >= 0x2c {
            self.hi_block_pos = le_u64(buf, 0x20);
            hash_pos |= (le_u16(buf, 0x28) as u64) << 32;
            block_pos |= (le_u16(buf, 0x2a) as u64) << 32;
        }
        if self.format >= 2 && n >= 0x48 {
            bet_pos = le_u64(buf, 0x34);
            het_pos = le_u64(buf, 0x3c);
        }

        if block_size != 0 && block_pos != 0 && block_size < (1 << 24) {
            if let Err(e) = self.read_block_table(block_pos, block_size) {
                self.warn.push(e.to_string());
            }
        }
        if hash_size != 0 && hash_pos != 0 && hash_size < (1 << 24) {
            if let Err(e) = self.read_hash_table(hash_pos, hash_size) {
                self.warn.push(e.to_string());
            }
        }
        if het_pos != 0 && bet_pos != 0 {
            match het::read_het(&self.src, het_pos, self.size) {
                Err(e) => {
                    self.warn.push(e.to_string());
                    if self.hash.is_none() {
                        return Err(e);
                    }
                }
                Ok(het) => match het::read_bet(&self.src, bet_pos, self.size) {
                    Err(e) => {
                        self.warn.push(e.to_string());
                        if self.hash.is_none() {
                            return Err(e);
                        }
                    }
                    Ok(bet) => {
                        self.het = Some(het);
                        self.bet = Some(bet);
                    }
                },
            }
        }
        if self.het.is_none() && self.hash.is_none() {
            return Err(Error::Corrupt(
                "mpq: archive has neither a hash table nor a usable HET/BET pair".into(),
            ));
        }
        Ok(())
    }

    fn read_hash_table(&mut self, pos: u64, count: u32) -> Result<()> {
        let n = count as usize;
        let mut raw = vec![0u8; n * 16];
        self.src.read_exact_at(pos, &mut raw)?;
        self.add_read(raw.len());
        let block_count = self.block.len() as u32;
        let score = pick_key(&mut raw, "(hash table)", |b| {
            (0..n)
                .filter(|&i| {
                    let e = le_u32(b, i * 16 + 12);
                    e == 0xffff_ffff || e == 0xffff_fffe || block_count == 0 || e < block_count
                })
                .count()
        });
        if score == 0 {
            return Err(Error::Corrupt(format!(
                "mpq: hash table at {pos} decrypts to nothing plausible"
            )));
        }
        self.hash = Some(
            (0..n)
                .map(|i| {
                    let e = &raw[i * 16..];
                    HashEntry {
                        name_a: le_u32(e, 0),
                        name_b: le_u32(e, 4),
                        block: le_u32(e, 12),
                    }
                })
                .collect(),
        );
        Ok(())
    }

    fn read_block_table(&mut self, pos: u64, count: u32) -> Result<()> {
        let n = count as usize;
        let mut raw = vec![0u8; n * 16];
        self.src.read_exact_at(pos, &mut raw)?;
        self.add_read(raw.len());
        let size = self.size;
        let score = pick_key(&mut raw, "(block table)", |b| {
            (0..n)
                .filter(|&i| {
                    if le_u32(b, i * 16 + 12) & FLAG_EXISTS == 0 {
                        return false;
                    }
                    let off = le_u32(b, i * 16) as u64;
                    let csize = le_u32(b, i * 16 + 4) as u64;
                    off + csize <= size
                })
                .count()
        });
        if score == 0 {
            return Err(Error::Corrupt(format!(
                "mpq: block table at {pos} decrypts to nothing plausible"
            )));
        }

        let mut hi: Option<Vec<u16>> = None;
        if self.hi_block_pos != 0 {
            let mut hib = vec![0u8; n * 2];
            if self.src.read_exact_at(self.hi_block_pos, &mut hib).is_ok() {
                self.add_read(hib.len());
                hi = Some((0..n).map(|i| le_u16(&hib, i * 2)).collect());
            }
        }

        self.block = (0..n)
            .map(|i| {
                let e = &raw[i * 16..];
                let mut off = le_u32(e, 0) as u64;
                if let Some(h) = &hi {
                    off |= (h[i] as u64) << 32;
                }
                Entry {
                    name: String::new(),
                    offset: off,
                    comp_size: le_u32(e, 4),
                    file_size: le_u32(e, 8),
                    flags: le_u32(e, 12),
                }
            })
            .collect();
        Ok(())
    }

    pub fn has(&self, name: &str) -> bool {
        self.lookup(name).is_ok()
    }

    pub fn lookup(&self, name: &str) -> Result<Entry> {
        let key = norm_name(name);
        if let (Some(het), Some(bet)) = (&self.het, &self.bet) {
            if let Some(mut f) = het::lookup(het, bet, &het_key(name)) {
                f.name = key;
                return Ok(f);
            }
            if self.hash.is_some() {
                if let Ok(f) = self.lookup_hash(&key) {
                    return Ok(f);
                }
            }
            return Err(Error::NoFile(name.to_string()));
        }
        self.lookup_hash(&key)
    }

    fn lookup_hash(&self, key: &str) -> Result<Entry> {
        let hash = self
            .hash
            .as_ref()
            .ok_or_else(|| Error::Unsupported(format!("mpq: {key}: no hash table")))?;
        let bytes = key.as_bytes();
        let idx = hash_string(bytes, HASH_NAME_IDX);
        let h1 = hash_string(bytes, HASH_NAME_A);
        let h2 = hash_string(bytes, HASH_NAME_B);
        let mask = (hash.len() - 1) as u32;
        let start = idx & mask;
        for i in 0..=mask {
            let e = hash[((start + i) & mask) as usize];
            match e.block {
                0xffff_ffff => return Err(Error::NoFile(key.to_string())),
                0xffff_fffe => continue,
                _ => {}
            }
            if e.name_a == h1 && e.name_b == h2 && (e.block as usize) < self.block.len() {
                let f = &self.block[e.block as usize];
                if f.flags & FLAG_EXISTS == 0 || f.flags & FLAG_DELETED != 0 {
                    continue;
                }
                let mut out = f.clone();
                out.name = key.to_string();
                return Ok(out);
            }
        }
        Err(Error::NoFile(key.to_string()))
    }

    pub fn read(&self, name: &str) -> Result<Vec<u8>> {
        let f = self.lookup(name)?;
        self.read_entry(&f)
    }

    pub fn read_entry(&self, f: &Entry) -> Result<Vec<u8>> {
        if f.file_size == 0 {
            return Ok(Vec::new());
        }
        if f.offset >= self.size {
            return Err(Error::Corrupt(format!(
                "mpq: {:?} offset {} past end {}",
                f.name, f.offset, self.size
            )));
        }
        let mut key = 0u32;
        if f.is_encrypted() {
            key = hash_string(file_base(&f.name).as_bytes(), FILE_KEY);
            if f.flags & FLAG_FIX_KEY != 0 {
                key = key.wrapping_add(f.offset as u32) ^ f.file_size;
            }
        }

        if f.flags & FLAG_SINGLE_UNIT != 0 {
            let mut raw = vec![0u8; f.comp_size as usize];
            self.src.read_exact_at(f.offset, &mut raw)?;
            self.add_read(raw.len());
            if f.is_encrypted() {
                decrypt_block(&mut raw, key);
            }
            let out = self.inflate_unit(raw, f, f.file_size)?;
            if out.len() != f.file_size as usize {
                return Err(Error::Corrupt(format!(
                    "mpq: {:?}: {} bytes, header says {}",
                    f.name,
                    out.len(),
                    f.file_size
                )));
            }
            return Ok(out);
        }

        let sector_count =
            ((f.file_size as u64 + self.sector_size as u64 - 1) / self.sector_size as u64) as usize;
        let mut table_entries = sector_count + 1;
        if f.flags & FLAG_SECTOR_CRC != 0 {
            table_entries += 1;
        }
        let mut raw_table = vec![0u8; table_entries * 4];
        self.src.read_exact_at(f.offset, &mut raw_table)?;
        self.add_read(raw_table.len());
        if f.is_encrypted() {
            decrypt_block(&mut raw_table, key.wrapping_sub(1));
        }
        let offs: Vec<u32> = (0..table_entries).map(|i| le_u32(&raw_table, i * 4)).collect();

        let mut crc_table: Option<Vec<u32>> = None;
        if f.flags & FLAG_SECTOR_CRC != 0 && f.is_compressed() {
            let start = offs[sector_count + 1] as u64;
            if start < f.comp_size as u64 && f.comp_size as u64 - start >= sector_count as u64 * 4 {
                let mut crc_raw = vec![0u8; sector_count * 4];
                if self.src.read_exact_at(f.offset + start, &mut crc_raw).is_ok() {
                    self.add_read(crc_raw.len());
                    crc_table = Some((0..sector_count).map(|i| le_u32(&crc_raw, i * 4)).collect());
                }
            }
        }

        let mut out: Vec<u8> = Vec::with_capacity(f.file_size as usize);
        for s in 0..sector_count {
            let start = offs[s];
            let end = offs[s + 1];
            if end < start || end > f.comp_size {
                return Err(Error::Corrupt(format!(
                    "mpq: {:?} sector {s} bounds {start}..{end} (block {})",
                    f.name, f.comp_size
                )));
            }
            let mut raw = vec![0u8; (end - start) as usize];
            self.src.read_exact_at(f.offset + start as u64, &mut raw)?;
            self.add_read(raw.len());
            if f.is_encrypted() {
                decrypt_block(&mut raw, key.wrapping_add(s as u32));
            }
            let rem = f.file_size - out.len() as u32;
            let want = self.sector_size.min(rem);
            let chunk = self.inflate_unit(raw, f, want)?;
            if chunk.len() != want as usize {
                return Err(Error::Corrupt(format!(
                    "mpq: {:?} sector {s}: {} bytes, want {want}",
                    f.name,
                    chunk.len()
                )));
            }
            if let Some(t) = &crc_table {
                let got = checksum::crc32(&chunk);
                if got != t[s] {
                    return Err(Error::Corrupt(format!(
                        "mpq: {:?} sector {s} crc {got:08x} want {:08x}",
                        f.name, t[s]
                    )));
                }
            }
            out.extend_from_slice(&chunk);
        }
        if out.len() != f.file_size as usize {
            return Err(Error::Corrupt(format!(
                "mpq: {:?}: {} bytes, header says {}",
                f.name,
                out.len(),
                f.file_size
            )));
        }
        Ok(out)
    }

    fn inflate_unit(&self, raw: Vec<u8>, f: &Entry, want: u32) -> Result<Vec<u8>> {
        if !f.is_compressed() || raw.is_empty() || raw.len() as u64 == want as u64 {
            return Ok(raw);
        }
        let mask = raw[0];
        let body = &raw[1..];
        if mask == 0 {
            return Ok(body.to_vec());
        }
        if mask & (COMP_ADPCM_M | COMP_ADPCM_S) != 0 {
            return Err(Error::Unsupported(format!(
                "mpq: compression mask {mask:02x} (ADPCM) not implemented"
            )));
        }
        if mask == COMP_LZMA {
            return lzma::inflate_lzma(body, want);
        }
        if mask == COMP_HUFFMAN {
            return huffman::inflate(body, want);
        }
        if mask & COMP_ZLIB != 0 {
            return inflate::zlib(body);
        }
        if mask & COMP_BZIP2 != 0 {
            return bzip2::inflate(body);
        }
        if mask & COMP_PKWARE != 0 {
            return pkware::inflate(body, want);
        }
        if mask & COMP_SPARSE != 0 {
            return sparse::inflate(body, want);
        }
        Err(Error::Unsupported(format!("mpq: compression mask {mask:02x}")))
    }

    pub fn list(&self) -> &[Entry] {
        &self.block
    }

    pub fn format(&self) -> u16 {
        self.format
    }

    pub fn uses_het_bet(&self) -> bool {
        self.het.is_some()
    }

    pub fn warnings(&self) -> &[String] {
        &self.warn
    }

    pub fn bytes_read(&self) -> u64 {
        self.bytes_read.get()
    }
}

fn pick_key(raw: &mut [u8], name: &str, score: impl Fn(&[u8]) -> usize) -> usize {
    let mut buf = vec![0u8; raw.len()];
    for typ in [FILE_KEY, HASH_NAME_IDX, HASH_NAME_A, HASH_NAME_B, 4] {
        buf.copy_from_slice(raw);
        decrypt_block(&mut buf, hash_string(name.as_bytes(), typ));
        let s = score(&buf[..]);
        if s > 0 {
            raw.copy_from_slice(&buf);
            return s;
        }
    }
    0
}

fn norm_name(name: &str) -> String {
    let bytes: Vec<u8> = name
        .bytes()
        .map(|b| match b {
            b'a'..=b'z' => b - 32,
            b'/' => b'\\',
            _ => b,
        })
        .collect();
    String::from_utf8(bytes).expect("ASCII-only substitution keeps UTF-8 valid")
}

fn het_key(name: &str) -> Vec<u8> {
    name.bytes()
        .map(|b| match b {
            b'A'..=b'Z' => b + 32,
            b'/' => b'\\',
            _ => b,
        })
        .collect()
}

fn file_base(name: &str) -> &str {
    match name.rfind(|c| c == '\\' || c == '/') {
        Some(i) => &name[i + 1..],
        None => name,
    }
}

pub(crate) fn le_u16(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

pub(crate) fn le_u32(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

pub(crate) fn le_u64(b: &[u8], at: usize) -> u64 {
    let mut a = [0u8; 8];
    a.copy_from_slice(&b[at..at + 8]);
    u64::from_le_bytes(a)
}
