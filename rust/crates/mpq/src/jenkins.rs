//! Bob Jenkins' lookup3 (hashlittle2, byte-at-a-time), the name hash HET/BET use.

use crate::le_u32;

/// The 64-bit HET/BET name hash: lookup3 seeded pc=2, pb=1, returned as b<<32|c.
pub(crate) fn hash_jenkins(key: &[u8]) -> u64 {
    let (c, b) = lookup3(key, 2, 1);
    ((b as u64) << 32) | c as u64
}

/// Returns (c, b). The main loop stops at length > 12, so a 12-byte multiple key
/// leaves a whole block for the tail, and the tail cascade runs 12 down to 1.
fn lookup3(key: &[u8], pc: u32, pb: u32) -> (u32, u32) {
    let mut length = key.len();
    let a0 = 0xdead_beefu32
        .wrapping_add(length as u32)
        .wrapping_add(pc);
    let mut a = a0;
    let mut b = a0;
    let mut c = a0.wrapping_add(pb);
    let mut i = 0usize;
    while length > 12 {
        a = a.wrapping_add(le_u32(key, i));
        b = b.wrapping_add(le_u32(key, i + 4));
        c = c.wrapping_add(le_u32(key, i + 8));
        let m = mix(a, b, c);
        a = m.0;
        b = m.1;
        c = m.2;
        i += 12;
        length -= 12;
    }
    if length != 0 {
        let k = |j: usize| key[i + j] as u32;
        if length >= 12 {
            c = c.wrapping_add(k(11) << 24);
        }
        if length >= 11 {
            c = c.wrapping_add(k(10) << 16);
        }
        if length >= 10 {
            c = c.wrapping_add(k(9) << 8);
        }
        if length >= 9 {
            c = c.wrapping_add(k(8));
        }
        if length >= 8 {
            b = b.wrapping_add(k(7) << 24);
        }
        if length >= 7 {
            b = b.wrapping_add(k(6) << 16);
        }
        if length >= 6 {
            b = b.wrapping_add(k(5) << 8);
        }
        if length >= 5 {
            b = b.wrapping_add(k(4));
        }
        if length >= 4 {
            a = a.wrapping_add(k(3) << 24);
        }
        if length >= 3 {
            a = a.wrapping_add(k(2) << 16);
        }
        if length >= 2 {
            a = a.wrapping_add(k(1) << 8);
        }
        if length >= 1 {
            a = a.wrapping_add(k(0));
        }
        let f = final_mix(a, b, c);
        c = f.2;
        b = f.1;
    }
    (c, b)
}

fn mix(mut a: u32, mut b: u32, mut c: u32) -> (u32, u32, u32) {
    a = a.wrapping_sub(c);
    a ^= c.rotate_left(4);
    c = c.wrapping_add(b);
    b = b.wrapping_sub(a);
    b ^= a.rotate_left(6);
    a = a.wrapping_add(c);
    c = c.wrapping_sub(b);
    c ^= b.rotate_left(8);
    b = b.wrapping_add(a);
    a = a.wrapping_sub(c);
    a ^= c.rotate_left(16);
    c = c.wrapping_add(b);
    b = b.wrapping_sub(a);
    b ^= a.rotate_left(19);
    a = a.wrapping_add(c);
    c = c.wrapping_sub(b);
    c ^= b.rotate_left(4);
    b = b.wrapping_add(a);
    (a, b, c)
}

fn final_mix(mut a: u32, mut b: u32, mut c: u32) -> (u32, u32, u32) {
    c ^= b;
    c = c.wrapping_sub(b.rotate_left(14));
    a ^= c;
    a = a.wrapping_sub(c.rotate_left(11));
    b ^= a;
    b = b.wrapping_sub(a.rotate_left(25));
    c ^= b;
    c = c.wrapping_sub(b.rotate_left(16));
    a ^= c;
    a = a.wrapping_sub(c.rotate_left(4));
    b ^= a;
    b = b.wrapping_sub(a.rotate_left(14));
    c ^= b;
    c = c.wrapping_sub(b.rotate_left(24));
    (a, b, c)
}
