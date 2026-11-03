use std::cell::RefCell;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};

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
