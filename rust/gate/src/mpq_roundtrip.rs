//! Round-trip check for the mpq reader. Each codec's archive is written here with
//! the same crate the reader delegates to, then read back through `mpq::open`.
//! Only single-unit files are written; tables are encrypted the way Storm does.

use std::fs;
use std::io::Write;
use std::path::Path;

const HEADER_SIZE: usize = 32;
const HASH_SLOTS: usize = 16;
const SECTOR_SHIFT: u16 = 3;
const MAGIC: u32 = 0x1a51_504d;
const EMPTY: u32 = 0xffff_ffff;
const FILE_KEY_TYPE: u32 = 3;

const FLAG_EXISTS: u32 = 0x8000_0000;
const FLAG_SINGLE_UNIT: u32 = 0x0100_0000;
const FLAG_COMPRESS: u32 = 0x0000_0200;

const MASK_ZLIB: u8 = 0x02;
const MASK_BZIP2: u8 = 0x10;
const MASK_LZMA: u8 = 0x12;

const FILE_NAME: &str = "data\\roundtrip.txt";

#[derive(Clone, Copy)]
enum Codec {
    Stored,
    Zlib,
    Bzip2,
    Lzma,
}

impl Codec {
    const ALL: [Codec; 4] = [Codec::Stored, Codec::Zlib, Codec::Bzip2, Codec::Lzma];

    fn label(self) -> &'static str {
        match self {
            Codec::Stored => "stored",
            Codec::Zlib => "zlib",
            Codec::Bzip2 => "bzip2",
            Codec::Lzma => "lzma",
        }
    }
}

/// Runs every codec, prints one line each, and returns false if any failed.
pub fn run() -> bool {
    let data = payload();
    let mut ok = true;
    for codec in Codec::ALL {
        match round_trip(codec, &data) {
            Ok(()) => println!("mpq roundtrip {} PASS", codec.label()),
            Err(reason) => {
                ok = false;
                println!("mpq roundtrip {} FAIL {}", codec.label(), reason.replace('\n', " "));
            }
        }
    }
    println!(
        "mpq unchecked: PKWARE implode, Huffman and sparse have no independent reference in this repo and are not checked"
    );
    ok
}

fn payload() -> Vec<u8> {
    let mut v = Vec::with_capacity(4096 + 9600);
    let mut x: u32 = 0x1234_5678;
    for i in 0..4096u32 {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        v.push(((x >> 24) as u8) ^ (i as u8));
    }
    v.extend(b"mpq roundtrip payload, repeated so the codecs have something to compress. ".repeat(120));
    v
}

fn round_trip(codec: Codec, data: &[u8]) -> Result<(), String> {
    let (body, flags) = encode(codec, data)?;
    let archive = build_archive(&body, data.len() as u32, flags);
    let path = std::env::temp_dir().join(format!("oscraft-gate-mpq-{}.mpq", codec.label()));
    fs::write(&path, &archive).map_err(|e| format!("write {}: {e}", path.display()))?;
    let result = read_back(&path, data);
    let _ = fs::remove_file(&path);
    result
}

fn read_back(path: &Path, expected: &[u8]) -> Result<(), String> {
    let archive = mpq::open(path).map_err(|e| format!("open: {e}"))?;
    if !archive.warnings().is_empty() {
        return Err(format!("reader warnings: {}", archive.warnings().join("; ")));
    }
    let got = archive.read(FILE_NAME).map_err(|e| format!("read: {e}"))?;
    if got.len() != expected.len() {
        return Err(format!("length {} want {}", got.len(), expected.len()));
    }
    if let Some(at) = got.iter().zip(expected).position(|(a, b)| a != b) {
        return Err(format!("bytes differ at offset {at}"));
    }
    Ok(())
}

/// Returns the block payload and its block-table flags. Stored data is written
/// raw; compressed data gets the one-byte codec mask the reader strips.
fn encode(codec: Codec, data: &[u8]) -> Result<(Vec<u8>, u32), String> {
    let (mask, stream) = match codec {
        Codec::Stored => return Ok((data.to_vec(), FLAG_EXISTS | FLAG_SINGLE_UNIT)),
        Codec::Zlib => {
            let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
            enc.write_all(data).map_err(|e| format!("zlib encode: {e}"))?;
            (MASK_ZLIB, enc.finish().map_err(|e| format!("zlib finish: {e}"))?)
        }
        Codec::Bzip2 => {
            let mut enc = bzip2::write::BzEncoder::new(Vec::new(), bzip2::Compression::default());
            enc.write_all(data).map_err(|e| format!("bzip2 encode: {e}"))?;
            (MASK_BZIP2, enc.finish().map_err(|e| format!("bzip2 finish: {e}"))?)
        }
        Codec::Lzma => {
            let mut input: &[u8] = data;
            let mut out = Vec::new();
            lzma_rs::lzma_compress(&mut input, &mut out).map_err(|e| format!("lzma encode: {e}"))?;
            (MASK_LZMA, out)
        }
    };
    let mut body = Vec::with_capacity(stream.len() + 1);
    body.push(mask);
    body.extend_from_slice(&stream);
    if body.len() == data.len() {
        return Err("compressed block is the same length as the plaintext, so the reader would not inflate it".into());
    }
    Ok((body, FLAG_EXISTS | FLAG_SINGLE_UNIT | FLAG_COMPRESS))
}

fn build_archive(body: &[u8], file_size: u32, flags: u32) -> Vec<u8> {
    let crypt = crypt_table();
    let mut out = vec![0u8; HEADER_SIZE];
    let data_offset = out.len() as u32;
    out.extend_from_slice(body);

    let hash_pos = out.len() as u32;
    let mut hash = hash_table_bytes(&crypt);
    encrypt_block(&mut hash, &crypt, hash_string(&crypt, b"(hash table)", FILE_KEY_TYPE));
    out.extend_from_slice(&hash);

    let block_pos = out.len() as u32;
    let mut block = Vec::with_capacity(16);
    for word in [data_offset, body.len() as u32, file_size, flags] {
        block.extend_from_slice(&word.to_le_bytes());
    }
    encrypt_block(&mut block, &crypt, hash_string(&crypt, b"(block table)", FILE_KEY_TYPE));
    out.extend_from_slice(&block);

    let total = out.len() as u32;
    out[0..4].copy_from_slice(&MAGIC.to_le_bytes());
    out[4..8].copy_from_slice(&(HEADER_SIZE as u32).to_le_bytes());
    out[8..12].copy_from_slice(&total.to_le_bytes());
    out[14..16].copy_from_slice(&SECTOR_SHIFT.to_le_bytes());
    out[16..20].copy_from_slice(&hash_pos.to_le_bytes());
    out[20..24].copy_from_slice(&block_pos.to_le_bytes());
    out[24..28].copy_from_slice(&(HASH_SLOTS as u32).to_le_bytes());
    out[28..32].copy_from_slice(&1u32.to_le_bytes());
    out
}

/// Hash table with one occupied slot for FILE_NAME (block index 0); every other
/// slot is marked empty.
fn hash_table_bytes(crypt: &[u32]) -> Vec<u8> {
    let mut slots = vec![[EMPTY; 4]; HASH_SLOTS];
    let bytes = FILE_NAME.as_bytes();
    let index = hash_string(crypt, bytes, 0) as usize % HASH_SLOTS;
    slots[index] = [hash_string(crypt, bytes, 1), hash_string(crypt, bytes, 2), 0, 0];
    slots
        .iter()
        .flat_map(|slot| slot.iter().flat_map(|w| w.to_le_bytes()))
        .collect()
}

fn crypt_table() -> Vec<u32> {
    let mut t = vec![0u32; 0x500];
    let mut seed: u32 = 0x0010_0001;
    for i in 0..0x100 {
        for j in 0..5 {
            seed = seed.wrapping_mul(125).wrapping_add(3) % 0x2aaaab;
            let hi = (seed & 0xffff) << 0x10;
            seed = seed.wrapping_mul(125).wrapping_add(3) % 0x2aaaab;
            let lo = seed & 0xffff;
            t[i + j * 0x100] = hi | lo;
        }
    }
    t
}

fn hash_string(crypt: &[u32], s: &[u8], typ: u32) -> u32 {
    let mut seed1: u32 = 0x7fed_7fed;
    let mut seed2: u32 = 0xeeee_eeee;
    for &c in s {
        let ch = c.to_ascii_uppercase() as u32;
        seed1 = crypt[((typ << 8) + ch) as usize] ^ seed1.wrapping_add(seed2);
        seed2 = ch
            .wrapping_add(seed1)
            .wrapping_add(seed2)
            .wrapping_add(seed2 << 5)
            .wrapping_add(3);
    }
    seed1
}

/// Inverse of the reader's decrypt_block: the seed advances on the plaintext word.
fn encrypt_block(p: &mut [u8], crypt: &[u32], key: u32) {
    let mut key = key;
    let mut seed2: u32 = 0xeeee_eeee;
    let mut i = 0;
    while i + 4 <= p.len() {
        seed2 = seed2.wrapping_add(crypt[0x400 + (key & 0xff) as usize]);
        let plain = u32::from_le_bytes([p[i], p[i + 1], p[i + 2], p[i + 3]]);
        let cipher = plain ^ key.wrapping_add(seed2);
        p[i..i + 4].copy_from_slice(&cipher.to_le_bytes());
        key = ((!key) << 0x15).wrapping_add(0x1111_1111) | (key >> 0x0b);
        seed2 = plain.wrapping_add(seed2).wrapping_add(seed2 << 5).wrapping_add(3);
        i += 4;
    }
}
