use std::cell::RefCell;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};

/// Positional reads over a file. Single-threaded: the seek and read happen under
/// one borrow.
pub(crate) struct Source {
    file: RefCell<File>,
    size: u64,
}

impl Source {
    pub(crate) fn new(file: File, size: u64) -> Source {
        Source {
            file: RefCell::new(file),
            size,
        }
    }

    pub(crate) fn size(&self) -> u64 {
        self.size
    }

    /// Reads up to `buf.len()` bytes at `off` and returns how many arrived.
    pub(crate) fn read_at(&self, off: u64, buf: &mut [u8]) -> io::Result<usize> {
        let mut f = self.file.borrow_mut();
        f.seek(SeekFrom::Start(off))?;
        let mut total = 0;
        while total < buf.len() {
            let n = f.read(&mut buf[total..])?;
            if n == 0 {
                break;
            }
            total += n;
        }
        Ok(total)
    }

    /// Fills `buf` completely or fails.
    pub(crate) fn read_exact_at(&self, off: u64, buf: &mut [u8]) -> io::Result<()> {
        if self.read_at(off, buf)? < buf.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "mpq: short read",
            ));
        }
        Ok(())
    }
}
