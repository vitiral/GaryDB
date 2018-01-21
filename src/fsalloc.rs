use packed_struct::prelude::*;
use std_prelude::*;
use std::io;

use path_abs::PathFile;

use raw::{
    AllocHeader, BlockHeader, BlockSize, Offset,
    ALLOC_HEADER_LEN, ROOT_OFFSET, ALLOC_HEADER_RESERVED
};

/// An ultra simple "file allocator".
pub(crate) struct FsAllocator(Mutex<FsAllocInternal>);

struct FsAllocInternal {
    header: AllocHeader,

    /// The file where data is stored
    file: File,

    /// "Heap" location, first byte of available space
    heap: Offset,

    /// Total capacity, if `heap > capacity` the file must be extended.
    capacity: Offset,
}

fn invalid_data(reason: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, reason)
}

impl FsAllocator {
    pub(crate) fn open<P: AsRef<Path>>(path: P) -> io::Result<FsAllocator> {
        assert_eq!(ALLOC_HEADER_LEN as usize, size_of::<AllocHeader>());
        let header = if path.as_ref().exists() {
            let mut array = [0; ALLOC_HEADER_LEN as usize];
            let mut file = OpenOptions::new()
                .read(true)
                .open(path)?;
            if file.metadata()?.len() < ROOT_OFFSET {
                return Err(invalid_data("file size too small"));
            }
            file.read_exact(&mut array)?;
            AllocHeader::unpack(&array)
                .expect("header unpack")
        } else {
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .open(path)?;
            let mut writer = BufWriter::new(file);
            let header = AllocHeader::default();
            writer.write_all(&header.pack())?;
            for _ in 0..ALLOC_HEADER_RESERVED {
                writer.write_all(&[0x00])?;
            }
            writer.flush()?;
            header
        };
        Err(io::Error::new(io::ErrorKind::Other, ""))
    }

    /// Allocate a block in the file equal to `capacity`
    pub(crate) fn alloc(&self, capacity: BlockSize) -> io::Result<Offset> {
        let mut lock = self.0.lock().expect("poisoned");
        let al = lock.deref_mut();
        let out = al.heap;
        {
            // Handle making the file larger if necessary
            let new_heap = al.heap + capacity as Offset;
            if new_heap >= al.capacity {
                let capacity = new_heap * 2;
                al.file.set_len(capacity)?;
                al.capacity = capacity;
            }
            al.heap = new_heap;
        }
        Ok(out)
    }
}

/// A Writer which only allows writes to happen within its allocated `capacity`
///
/// TODO: make this public
pub(crate) struct AllocedWriter {
    file: File,
    used: BlockSize,
    capacity: BlockSize,
}

impl IoWrite for AllocedWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.len() as u64 > (self.capacity - self.used) as u64 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "buf.len() > capacity - used",
            ));
        }

        let written = self.file.write(buf)?;
        self.used += written as u32;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}
