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
