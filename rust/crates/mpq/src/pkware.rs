use crate::{Error, Result};

const MAX_BITS: usize = 13;
const MAX_WIN: usize = 4096;

const LIT_REP: &[u8] = &[
    11, 124, 8, 7, 28, 7, 188, 13, 76, 4, 10, 8, 12, 10, 12, 10, 8, 23, 8, 9, 7, 6, 7, 8, 7, 6,
    55, 8, 23, 24, 12, 11, 7, 9, 11, 12, 6, 7, 22, 5, 7, 24, 6, 11, 9, 6, 7, 22, 7, 11, 38, 7, 9,
    8, 25, 11, 8, 11, 9, 12, 8, 12, 5, 38, 5, 38, 5, 11, 7, 5, 6, 21, 6, 10, 53, 8, 7, 24, 10,
    27, 44, 253, 253, 253, 252, 252, 252, 13, 12, 45, 12, 45, 12, 61, 12, 45, 44, 173,
];
const LEN_REP: [u8; 6] = [2, 35, 36, 53, 38, 23];
const DIST_REP: [u8; 7] = [2, 20, 53, 230, 247, 151, 248];

const BASE: [usize; 16] = [3, 2, 4, 5, 6, 7, 8, 9, 10, 12, 16, 24, 40, 72, 136, 264];
const EXTRA: [usize; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8];

struct Huff {
    count: [i16; MAX_BITS + 1],
    symbol: Vec<i16>,
}

impl Huff {
    fn with_symbols(n: usize) -> Huff {
        Huff {
            count: [0; MAX_BITS + 1],
            symbol: vec![0; n],
        }
    }
}

fn construct(h: &mut Huff, rep: &[u8]) -> i64 {
    let mut length = [0i16; 256];
    let mut n = 0usize;
    for &r in rep {
        let count = (r >> 4) as usize + 1;
        let len = (r & 15) as i16;
        for _ in 0..count {
            if n >= 256 {
                return -1;
            }
            length[n] = len;
            n += 1;
        }
    }
    h.count = [0; MAX_BITS + 1];
    for &l in length.iter().take(n) {
        if l as usize > MAX_BITS {
            return -1;
        }
        h.count[l as usize] += 1;
    }
    if h.count[0] as usize == n {
        return 0;
    }
    let mut left: i64 = 1;
    for l in 1..=MAX_BITS {
        left <<= 1;
        left -= h.count[l] as i64;
        if left < 0 {
            return left;
        }
    }
    let mut offs = [0i16; MAX_BITS + 1];
    for l in 1..MAX_BITS {
        offs[l + 1] = offs[l] + h.count[l];
    }
    for sym in 0..n {
        let l = length[sym] as usize;
        if l != 0 {
            let at = offs[l] as usize;
            if at >= h.symbol.len() {
                return -1;
            }
            h.symbol[at] = sym as i16;
            offs[l] += 1;
        }
    }
    left
}

fn tables() -> (Huff, Huff, Huff) {
    let mut lit = Huff::with_symbols(256);
    let mut len = Huff::with_symbols(16);
    let mut dist = Huff::with_symbols(64);
    construct(&mut lit, &LIT_REP);
    construct(&mut len, &LEN_REP);
    construct(&mut dist, &DIST_REP);
    (lit, len, dist)
}

struct PkBits<'a> {
    input: &'a [u8],
    pos: usize,
    bitbuf: i64,
    bitcnt: i64,
}

impl PkBits<'_> {
    fn bits(&mut self, need: usize) -> Result<usize> {
        let mut val = self.bitbuf;
        while self.bitcnt < need as i64 {
            if self.pos >= self.input.len() {
                return Err(Error::Corrupt("mpq: pkware: out of input".into()));
            }
            val |= (self.input[self.pos] as i64) << self.bitcnt;
            self.pos += 1;
            self.bitcnt += 8;
        }
        self.bitbuf = val >> need;
        self.bitcnt -= need as i64;
        Ok((val & ((1i64 << need) - 1)) as usize)
    }

    fn decode(&mut self, h: &Huff) -> Result<usize> {
        let mut bitbuf = self.bitbuf;
        let mut left = self.bitcnt;
        let mut code: i64 = 0;
        let mut first: i64 = 0;
        let mut index: i64 = 0;
        let mut n: i64 = 1;
        let mut at: usize = 1;
        loop {
            while left > 0 {
                code |= (bitbuf & 1) ^ 1;
                bitbuf >>= 1;
                if at >= h.count.len() {
                    return Err(Error::Corrupt("mpq: pkware: code longer than 13 bits".into()));
                }
                let count = h.count[at] as i64;
                at += 1;
                if code < first + count {
                    let idx = index + (code - first);
                    if idx < 0 || idx as usize >= h.symbol.len() {
                        return Err(Error::Corrupt("mpq: pkware: symbol out of table".into()));
                    }
                    self.bitbuf = bitbuf;
                    self.bitcnt = (self.bitcnt - n) & 7;
                    return Ok(h.symbol[idx as usize] as usize);
                }
                index += count;
                first += count;
                first <<= 1;
                code <<= 1;
                n += 1;
                left -= 1;
            }
            left = (MAX_BITS as i64 + 1) - n;
            if left == 0 {
                break;
            }
            if self.pos >= self.input.len() {
                return Err(Error::Corrupt("mpq: pkware: out of input".into()));
            }
            bitbuf = self.input[self.pos] as i64;
            self.pos += 1;
            if left > 8 {
                left = 8;
            }
        }
        Err(Error::Corrupt("mpq: pkware: ran out of codes".into()))
    }
}

pub(crate) fn inflate(body: &[u8], want: u32) -> Result<Vec<u8>> {
    if body.len() < 2 {
        return Err(Error::Corrupt(format!(
            "mpq: pkware: {}-byte block",
            body.len()
        )));
    }
    let (lit_code, len_code, dist_code) = tables();
    let mut s = PkBits {
        input: body,
        pos: 0,
        bitbuf: 0,
        bitcnt: 0,
    };
    let lit = s.bits(8)?;
    if lit > 1 {
        return Err(Error::Corrupt(format!(
            "mpq: pkware: literal coding flag {lit}"
        )));
    }
    let dict = s.bits(8)?;
    if !(4..=6).contains(&dict) {
        return Err(Error::Corrupt(format!(
            "mpq: pkware: dictionary exponent {dict}"
        )));
    }

    let mut win = [0u8; MAX_WIN];
    let mut next = 0usize;
    let mut first = true;
    let mut out: Vec<u8> = Vec::with_capacity(want as usize);

    loop {
        if s.bits(1)? == 0 {
            let sym = if lit != 0 {
                s.decode(&lit_code)?
            } else {
                s.bits(8)?
            };
            win[next] = sym as u8;
            next += 1;
            if next == MAX_WIN {
                out.extend_from_slice(&win);
                next = 0;
                first = false;
            }
            continue;
        }

        let sym = s.decode(&len_code)?;
        if sym >= BASE.len() {
            return Err(Error::Corrupt(format!("mpq: pkware: length code {sym}")));
        }
        let extra = s.bits(EXTRA[sym])?;
        let mut length = BASE[sym] + extra;
        if length == 519 {
            break;
        }

        let bits = if length == 2 { 2 } else { dict };
        let dist_sym = s.decode(&dist_code)?;
        let low = s.bits(bits)?;
        let dist = (dist_sym << bits) + low + 1;
        if first && dist > next {
            return Err(Error::Corrupt(format!(
                "mpq: pkware: distance {dist} past {next} written"
            )));
        }

        while length > 0 {
            let to = next;
            let mut from = next as i64 - dist as i64;
            let mut n = MAX_WIN;
            if next < dist {
                from += n as i64;
                n = dist;
            }
            n -= next;
            if n > length {
                n = length;
            }
            length -= n;
            next += n;
            let mut t = to;
            let mut f = from as usize;
            for _ in 0..n {
                win[t] = win[f];
                t += 1;
                f += 1;
            }
            if next == MAX_WIN {
                out.extend_from_slice(&win);
                next = 0;
                first = false;
            }
        }
    }
    out.extend_from_slice(&win[..next]);
    Ok(out)
}
