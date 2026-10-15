//! zlib sector codec (mask 0x02), delegated to flate2. flate2 verifies the zlib
//! header and Adler-32 trailer itself.

use std::io::Read;

use flate2::read::ZlibDecoder;

use crate::{Error, Result};

pub(crate) fn zlib(body: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(body.len() * 2);
    ZlibDecoder::new(body)
        .read_to_end(&mut out)
        .map_err(|e| Error::Corrupt(format!("mpq: zlib: {e}")))?;
    Ok(out)
}
