use crate::{Error, Result};

pub(crate) fn inflate_lzma(body: &[u8], want: u32) -> Result<Vec<u8>> {
    if body.first() == Some(&0xff) {
        if let Ok(out) = decode_lzma(&body[1..], want) {
            return Ok(out);
        }
    }
    if let Ok(out) = decode_lzma(body, want) {
        return Ok(out);
    }
    if body.len() > 5 {
        if let Ok(out) = decode_lzma(&body[5..], want) {
            return Ok(out);
        }
    }
    Err(Error::Unsupported(
        "mpq: lzma: no decoder accepted the stream".into(),
    ))
}

fn decode_lzma(body: &[u8], want: u32) -> Result<Vec<u8>> {
    let mut input = std::io::BufReader::new(body);
    let mut out = Vec::with_capacity((want as usize).min(1 << 24));
    lzma_rs::lzma_decompress(&mut input, &mut out)
        .map_err(|e| Error::Corrupt(format!("mpq: lzma: {e}")))?;
    Ok(out)
}
