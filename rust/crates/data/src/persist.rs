use std::fmt;

use crate::format::{HEADER_LEN, MAGIC, SECTIONS, SECTION_ENTRY_LEN, VERSION};

pub const ACCEPTED_VERSION: u16 = VERSION;

#[derive(Debug, PartialEq, Eq)]
pub enum ImageError {
    Truncated { needed: usize, have: usize },
    BadMagic,
    VersionMismatch { found: u16, accepted: u16 },
    BadHeaderLength { found: u16 },
    SectionCountMismatch { found: u32, expected: usize },
    LengthMismatch { declared: u32, actual: usize },
    SectionTagMismatch { index: usize, found: [u8; 4] },
    RecordSizeMismatch { index: usize, found: u32, expected: usize },
    SectionOutOfPlace { index: usize },
    SectionOverrun { index: usize },
    TrailingBytes,
    RoundTripMismatch,
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
            ImageError::BadHeaderLength { found } => write!(f, "header length is {found}, expected {HEADER_LEN}"),
            ImageError::SectionCountMismatch { found, expected } => {
                write!(f, "image declares {found} sections, expected {expected}")
            }
            ImageError::LengthMismatch { declared, actual } => {
                write!(f, "image declares {declared} bytes, has {actual}")
            }
            ImageError::SectionTagMismatch { index, found } => {
                write!(f, "section {index} has tag {:?}, expected another tag", String::from_utf8_lossy(found))
            }
            ImageError::RecordSizeMismatch { index, found, expected } => {
                write!(f, "section {index} has record size {found}, expected {expected}")
            }
            ImageError::SectionOutOfPlace { index } => write!(f, "section {index} does not follow the previous one"),
            ImageError::SectionOverrun { index } => write!(f, "section {index} runs past the end of the image"),
            ImageError::TrailingBytes => write!(f, "image has bytes after the last section"),
            ImageError::RoundTripMismatch => write!(f, "image does not reproduce byte-for-byte after a round trip"),
        }
    }
}

impl std::error::Error for ImageError {}

fn truncated(needed: usize, image: &[u8]) -> ImageError {
    ImageError::Truncated {
        needed,
        have: image.len(),
    }
}

fn u16_at(image: &[u8], at: usize) -> Result<u16, ImageError> {
    let b = image.get(at..at + 2).ok_or_else(|| truncated(at + 2, image))?;
    Ok(u16::from_le_bytes([b[0], b[1]]))
}

fn u32_at(image: &[u8], at: usize) -> Result<u32, ImageError> {
    let b = image.get(at..at + 4).ok_or_else(|| truncated(at + 4, image))?;
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

pub fn check_version(image: &[u8]) -> Result<(), ImageError> {
    if image.len() < HEADER_LEN {
        return Err(truncated(HEADER_LEN, image));
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

pub struct Section<'a> {
    pub tag: [u8; 4],
    pub count: u32,
    pub record: u32,
    pub body: &'a [u8],
}

pub fn split(image: &[u8]) -> Result<Vec<Section<'_>>, ImageError> {
    check_version(image)?;
    let header_len = u16_at(image, 6)?;
    if header_len as usize != HEADER_LEN {
        return Err(ImageError::BadHeaderLength { found: header_len });
    }
    let count = u32_at(image, 8)?;
    if count as usize != SECTIONS.len() {
        return Err(ImageError::SectionCountMismatch {
            found: count,
            expected: SECTIONS.len(),
        });
    }
    let declared = u32_at(image, 12)?;
    if declared as usize != image.len() {
        return Err(ImageError::LengthMismatch {
            declared,
            actual: image.len(),
        });
    }

    let mut sections = Vec::with_capacity(SECTIONS.len());
    let mut expected = HEADER_LEN + SECTIONS.len() * SECTION_ENTRY_LEN;
    for (index, (tag, record)) in SECTIONS.iter().enumerate() {
        let entry = HEADER_LEN + index * SECTION_ENTRY_LEN;
        let found = image.get(entry..entry + 4).ok_or_else(|| truncated(entry + 4, image))?;
        if found != &tag[..] {
            return Err(ImageError::SectionTagMismatch {
                index,
                found: [found[0], found[1], found[2], found[3]],
            });
        }
        let offset = u32_at(image, entry + 4)? as usize;
        let n = u32_at(image, entry + 8)?;
        let rec = u32_at(image, entry + 12)?;
        if rec as usize != *record {
            return Err(ImageError::RecordSizeMismatch {
                index,
                found: rec,
                expected: *record,
            });
        }
        if offset != expected {
            return Err(ImageError::SectionOutOfPlace { index });
        }
        let end = offset + n as usize * record;
        if end > image.len() {
            return Err(ImageError::SectionOverrun { index });
        }
        sections.push(Section {
            tag: *tag,
            count: n,
            record: rec,
            body: &image[offset..end],
        });
        expected = end;
    }
    if expected != image.len() {
        return Err(ImageError::TrailingBytes);
    }
    Ok(sections)
}

pub fn assemble(sections: &[Section]) -> Vec<u8> {
    let body_start = HEADER_LEN + sections.len() * SECTION_ENTRY_LEN;
    let mut table = Vec::new();
    let mut body = Vec::new();
    let mut offset = body_start;
    for section in sections {
        table.extend_from_slice(&section.tag);
        table.extend_from_slice(&(offset as u32).to_le_bytes());
        table.extend_from_slice(&section.count.to_le_bytes());
        table.extend_from_slice(&section.record.to_le_bytes());
        offset += section.body.len();
        body.extend_from_slice(section.body);
    }
    let mut out = Vec::with_capacity(offset);
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&(HEADER_LEN as u16).to_le_bytes());
    out.extend_from_slice(&(sections.len() as u32).to_le_bytes());
    out.extend_from_slice(&(offset as u32).to_le_bytes());
    out.extend_from_slice(&table);
    out.extend_from_slice(&body);
    out
}

pub fn round_trip(image: &[u8]) -> Result<(), ImageError> {
    let sections = split(image)?;
    if assemble(&sections) != image {
        return Err(ImageError::RoundTripMismatch);
    }
    Ok(())
}
