pub const SHIFT: u32 = 16;
pub const ONE: i32 = 1 << SHIFT;
pub const HALF: i32 = ONE >> 1;

pub fn from_int(v: i32) -> i32 {
    v << SHIFT
}

pub fn to_int(v: i32) -> i32 {
    v >> SHIFT
}

pub fn round(v: i32) -> i32 {
    v.wrapping_add(HALF) >> SHIFT
}

pub fn mul(a: i32, b: i32) -> i32 {
    ((a as i64 * b as i64) >> SHIFT) as i32
}

pub fn to_float(v: i32) -> f64 {
    v as f64 / ONE as f64
}

pub fn from_milli(m: i32) -> i32 {
    ((m as i64 * ONE as i64) / 1000) as i32
}

fn bits(v: u64) -> u32 {
    64 - v.leading_zeros()
}

pub fn sqrt(a: i32) -> i32 {
    if a <= 0 {
        return 0;
    }
    let x = (a as i64) << SHIFT;
    let mut g: i64 = 1i64 << (bits(a as u64) / 2 + SHIFT / 2);
    for _ in 0..8 {
        if g == 0 {
            break;
        }
        let next = (g + x / g) >> 1;
        if next == g || next == g + 1 {
            g = next;
            break;
        }
        g = next;
    }
    g as i32
}

pub fn isqrt(n: i32) -> i32 {
    if n <= 1 {
        return n;
    }
    let mut r = n;
    loop {
        let q = n / r;
        if q >= r {
            break;
        }
        r = (r + q) >> 1;
    }
    r
}
