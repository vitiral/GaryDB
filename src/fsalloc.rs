use packed_struct::prelude::*;
use std_prelude::*;
use std::io;

use raw::{AllocHeader, BlockHeader, BlockSize, Offset, ALLOC_HEADER_LEN, ALLOC_HEADER_RESERVED,
          ALLOC_INIITIAL_CAPACITY, ROOT_KEY_SIZE, ROOT_OFFSET};

/// An ultra simple "file allocator".
pub(crate) struct FsAlloc(Mutex<FsAllocInternal>);

struct FsAllocInternal {
    header: AllocHeader,

    /// The file where data is stored
    file: File,
}

fn invalid_data(reason: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, reason)
}

fn write_alloc_header(f: &mut File, header: &AllocHeader) -> io::Result<()> {
    f.seek(SeekFrom::Start(0));
    let mut writer = BufWriter::new(&mut file);
    writer.write_all(&header.pack())?;
    writer.flush()
}

fn write_block_header(f: &mut File, offset: Offset, header: &BlockHeaer) -> io::Result<()> {
    f.seek(SeekFrom::Start(offset))?;
    f.write_all(&header.pack())?;
}

impl FsAlloc {
    /// Open the Allocator. No validation is done for the root node
    /// (but it is created if the allocator is new).
    pub(crate) fn open<P: AsRef<Path>>(path: P) -> io::Result<FsAlloc> {
        assert_eq!(ALLOC_HEADER_LEN as usize, size_of::<AllocHeader>());
        let (header, file) = if path.as_ref().exists() {
            // The file exists, read the header
            let mut array = [0; ALLOC_HEADER_LEN as usize];
            let mut file = OpenOptions::new()
                .create(false)
                .write(true)
                .read(true)
                .open(path)?;
            if file.metadata()?.len() < ROOT_OFFSET {
                return Err(invalid_data("file size too small"));
            }
            file.seek(SeekFrom::Start(0));
            file.read_exact(&mut array)?;
            let header = AllocHeader::unpack(&array).expect("header unpack");
            (header, file)
        } else {
            // The file does not exist, create it and write the header
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .read(true)
                .open(path)?;
            file.set_size(ALLOC_INITIAL_SIZE)?;

            // Write the root node first
            let mut header = AllocHeader::default();
            let mut root = BlockHeader::new_sized_key_gen_value(ROOT_KEY_SIZE);
            // Nothing will have it open yet, so finished.
            root.finished = true;
            write_block_header(&mut file, header.root, &root);

            header.heap += size_of::<BlockHeader> + root.capacity;
            write_alloc_header(&mut file, &header)?;
            (header, file)
        };

        Ok(FsAlloc(Mutex::new(FsAllocInternal {
            header: header,
            file: file,
        })))
    }

    /// Allocate a block in the file equal to `capacity`
    pub(crate) fn alloc(&self, capacity: BlockSize) -> io::Result<Offset> {
        let mut lock = self.0.lock().expect("poisoned");
        let al = lock.deref_mut();
        let head = &mut al.header;
        let out = head.heap;
        {
            // Handle making the file larger if necessary
            let new_heap = head.heap + capacity as Offset;
            if new_heap >= head.capacity {
                let capacity = new_heap * 2;
                al.file.set_len(capacity)?;
                head.capacity = capacity;
            }
            head.heap = new_heap;
            write_alloc_header(&mut al.file, &head)?;
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
