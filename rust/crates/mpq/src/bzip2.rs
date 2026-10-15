//! bzip2 sector codec (mask 0x10), delegated to the bzip2 crate. The multi-stream
//! decoder accepts concatenated streams, as Storm writers may produce.

use std::io::Read;

use bzip2::read::MultiBzDecoder;

use crate::{Error, Result};

pub(crate) fn inflate(body: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    MultiBzDecoder::new(body)
        .read_to_end(&mut out)
        .map_err(|e| Error::Corrupt(format!("mpq: bzip2: {e}")))?;
    Ok(out)
}
