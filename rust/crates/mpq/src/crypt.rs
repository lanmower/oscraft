//! Storm's string hash and stream cipher.

const fn build_crypt_table() -> [u32; 0x500] {
    let mut t = [0u32; 0x500];
    let mut seed: u32 = 0x0010_0001;
    let mut i = 0;
    while i < 0x100 {
        let mut j = 0;
        while j < 5 {
            seed = seed.wrapping_mul(125).wrapping_add(3) % 0x2aaaab;
            let hi = (seed & 0xffff) << 0x10;
            seed = seed.wrapping_mul(125).wrapping_add(3) % 0x2aaaab;
            let lo = seed & 0xffff;
            t[i + j * 0x100] = hi | lo;
            j += 1;
        }
        i += 1;
    }
    t
}

static CRYPT_TABLE: [u32; 0x500] = build_crypt_table();

/// Index of the key block the stream cipher reads from.
const KEY_BLOCK: usize = 0x400;

/// Storm's HashString. It upper-cases as it goes; a lower-cased input decrypts
/// to noise.
pub(crate) fn hash_string(s: &[u8], typ: u32) -> u32 {
    let mut seed1: u32 = 0x7fed_7fed;
    let mut seed2: u32 = 0xeeee_eeee;
    for &c in s {
        let ch = if c.is_ascii_lowercase() {
            (c - 32) as u32
        } else {
            c as u32
        };
        seed1 = CRYPT_TABLE[((typ << 8) + ch) as usize] ^ seed1.wrapping_add(seed2);
        seed2 = ch
            .wrapping_add(seed1)
            .wrapping_add(seed2)
            .wrapping_add(seed2 << 5)
            .wrapping_add(3);
    }
    seed1
}

/// Storm's stream cipher, in place, over whole dwords. The seed advances on the
/// plaintext word, not the ciphertext.
pub(crate) fn decrypt_block(p: &mut [u8], key: u32) {
    let mut key = key;
    let mut seed2: u32 = 0xeeee_eeee;
    let mut i = 0;
    while i + 4 <= p.len() {
        seed2 = seed2.wrapping_add(CRYPT_TABLE[KEY_BLOCK + (key & 0xff) as usize]);
        let word = u32::from_le_bytes([p[i], p[i + 1], p[i + 2], p[i + 3]]) ^ key.wrapping_add(seed2);
        p[i..i + 4].copy_from_slice(&word.to_le_bytes());
        key = ((!key) << 0x15).wrapping_add(0x1111_1111) | (key >> 0x0b);
        seed2 = word
            .wrapping_add(seed2)
            .wrapping_add(seed2 << 5)
            .wrapping_add(3);
        i += 4;
    }
}
