//! Sparse run-length coding (sector mask 0x20). Each control byte says
//! whether to copy literals, repeat a byte, zero-fill, or byte-fill.

use crate::{Error, Result};

pub(crate) fn inflate(body: &[u8], want: u32) -> Result<Vec<u8>> {
    let mut out: Vec<u8> = Vec::with_capacity(want as usize);
    let mut i = 0usize;
    while i < body.len() && (out.len() as u32) < want {
        let ctl = body[i];
        i += 1;
        if ctl < 0x80 {
            let n = ctl as usize + 1;
            if i + n > body.len() {
                return Err(Error::Corrupt("mpq: sparse: literal run overruns".into()));
            }
            out.extend_from_slice(&body[i..i + n]);
            i += n;
        } else if ctl < 0xc0 {
            if i >= body.len() {
                return Err(Error::Corrupt("mpq: sparse: repeat needs a byte".into()));
            }
            let n = (ctl & 0x3f) as usize + 3;
            let b = body[i];
            i += 1;
            out.extend(std::iter::repeat(b).take(n));
        } else if ctl < 0xe0 {
            let n = (ctl & 0x1f) as usize + 4;
            out.extend(std::iter::repeat(0u8).take(n));
        } else {
            if i >= body.len() {
                return Err(Error::Corrupt("mpq: sparse: fill needs a byte".into()));
            }
            let n = (ctl & 0x1f) as usize + 5;
            let b = body[i];
            i += 1;
            out.extend(std::iter::repeat(b).take(n));
        }
    }
    Ok(out)
}
