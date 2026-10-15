//! Map image persistence. Checks an image's version against the version this
//! loader accepts, and reports each mismatch as a named error.

use std::fmt;

use crate::format::{HEADER_LEN, MAGIC, VERSION};

/// The image version this loader accepts.
pub const ACCEPTED_VERSION: u16 = VERSION;

#[derive(Debug, PartialEq, Eq)]
pub enum ImageError {
    Truncated { needed: usize, have: usize },
    BadMagic,
    VersionMismatch { found: u16, accepted: u16 },
}

impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImageError::Truncated { needed, have } => {
                write!(f, "image is truncated: needs {needed} bytes, has {have}")
            }
            ImageError::BadMagic => write!(f, "image does not start with the OSDT magic"),
            ImageError::VersionMismatch { found, accepted } => {
                write!(f, "image is version {found}, this loader accepts version {accepted}")
            }
        }
    }
}

impl std::error::Error for ImageError {}

fn u16_at(image: &[u8], at: usize) -> Result<u16, ImageError> {
    let b = image
        .get(at..at + 2)
        .ok_or(ImageError::Truncated { needed: at + 2, have: image.len() })?;
    Ok(u16::from_le_bytes([b[0], b[1]]))
}

/// Accepts an image only when its magic and version match this loader.
pub fn check_version(image: &[u8]) -> Result<(), ImageError> {
    if image.len() < HEADER_LEN {
        return Err(ImageError::Truncated {
            needed: HEADER_LEN,
            have: image.len(),
        });
    }
    if image[0..4] != MAGIC {
        return Err(ImageError::BadMagic);
    }
    let found = u16_at(image, 4)?;
    if found != ACCEPTED_VERSION {
        return Err(ImageError::VersionMismatch {
            found,
            accepted: ACCEPTED_VERSION,
        });
    }
    Ok(())
}
