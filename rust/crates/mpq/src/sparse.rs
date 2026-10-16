//! Sparse coding (sector mask 0x20), as StormLib's sparse.cpp writes and reads it: a 4-byte
//! big-endian output length, then chunks. A control byte with the top bit set copies the next
//! (control & 0x7f) + 1 literal bytes; a clear top bit is a zero run of (control & 0x7f) + 3 bytes.

use crate::{Error, Result};

pub(crate) fn inflate(body: &[u8], want: u32) -> Result<Vec<u8>> {
    if body.len() < 4 {
        return Err(Error::Corrupt("mpq: sparse: stream shorter than its length header".into()));
    }
    let len = u32::from_be_bytes([body[0], body[1], body[2], body[3]]);
    if len != want {
        return Err(Error::Corrupt(format!("mpq: sparse: header says {len} bytes, block expects {want}")));
    }
    let mut out: Vec<u8> = Vec::with_capacity(want as usize);
    let mut i = 4usize;
    while i < body.len() && (out.len() as u32) < len {
        let ctl = body[i];
        i += 1;
        if ctl & 0x80 != 0 {
            let n = (ctl & 0x7f) as usize + 1;
            if i + n > body.len() {
                return Err(Error::Corrupt("mpq: sparse: literal run overruns".into()));
            }
            out.extend_from_slice(&body[i..i + n]);
            i += n;
        } else {
            let n = (ctl & 0x7f) as usize + 3;
            out.extend(std::iter::repeat(0u8).take(n));
        }
    }
    out.truncate(len as usize);
    if out.len() as u32 != len {
        return Err(Error::Corrupt("mpq: sparse: stream ends before its length".into()));
    }
    Ok(out)
}
