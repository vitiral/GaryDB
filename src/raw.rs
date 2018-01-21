//! Raw types and methods
//! Note: Big Endian == MSB (most significant bit) == Network Endian

use packed_struct::prelude::*;

const MAGIC_NUMBER: u32 = 0x01A1_A1AF;
const VERSION_MAJOR: u32 = 0x0000;
const VERSION_MINOR: u32 = 0x0001;
const VERSION_PATCH: u64 = 0x0000;

/// Type which defines the size of a block
pub type BlockSize = u32;

/// Offset location within the file. Think of this as a pointer.
pub type Offset = u64;

// ALLOC CONSTANTS
pub const ALLOC_HEADER_LEN: u64 = 44;
pub type AllocHeaderArray = [u8; ALLOC_HEADER_LEN as usize];
pub const ROOT_OFFSET: u64 = 512;
pub const ALLOC_HEADER_RESERVED: u64 = ROOT_OFFSET - ALLOC_HEADER_LEN;
/// Initial size = 1MiB == 2^20
pub const ALLOC_INITIAL_SIZE: u64 = 1_048_576;
pub const ROOT_KEY_SIZE: u16 = 32;

// NODE CONSTANTS
pub const NODE_NUM_ITEMS: usize = 64;

#[derive(Clone, Debug, PartialEq, PackedStruct)]
#[packed_struct(bit_numbering = "msb0")]
/// The header for the "allocator" -- i.e. the header for the whole file.
pub struct AllocHeader {
    #[packed_field(bits = "0..31", endian = "msb")]
    /// [Magic Number][1] at the beginning of every file
    ///
    /// [1]: https://en.wikipedia.org/wiki/List_of_file_signatures
    pub(crate) magic: u32,

    #[packed_field(bits = "32..63", endian = "msb")]
    /// Major version. Differing major versions are never compatible.
    pub(crate) version_major: u32,

    #[packed_field(bits = "64..95", endian = "msb")]
    /// Minor version. Newer minor versions can read older minor versions but not vice versa.
    pub(crate) version_minor: u32,

    #[packed_field(bits = "96..159", endian = "msb")]
    /// Patch version, not currently used.
    pub(crate) version_patch: u64,

    #[packed_field(bits = "160..223", endian = "msb")]
    /// Location of the root node
    pub(crate) root: u64,

    #[packed_field(bits = "224..287", endian = "msb")]
    /// Location of unused data.
    pub(crate) heap: u64,

    #[packed_field(bits = "288..351", endian = "msb")]
    /// Location of total capacity of the file. Should be equal to `file.size()`
    pub(crate) capacity: u64,
}

#[derive(Clone, Debug, PartialEq, PackedStruct)]
#[packed_struct(bit_numbering = "msb0")]
/// Packed raw header object at the beginning of every Block.
///
/// Currently is 24 bytes in size.
pub struct BlockHeader {
    #[packed_field(bits = "0..63", endian = "msb")]
    /// Parent Offset of this block.
    pub(crate) parent: u64,

    // CAPACITY AND USED SPACE
    #[packed_field(bits = "64..95", endian = "msb")]
    /// Capacity (in bytes) of the block
    ///
    /// > Next block is at `self.offset + self.capacity`
    pub(crate) capacity: u32,

    #[packed_field(bits = "96..127", endian = "msb")]
    /// Used bytes of the block
    pub(crate) used: u32,

    // TYPE ENUM + EXTRA
    #[packed_field(bits = "128..143", endian = "msb")]
    /// Extra data for the type
    pub(crate) ty_keysize: u16,

    #[packed_field(bits = "144..159", endian = "msb")]
    /// Extra data for the type
    pub(crate) ty_valuesize: u16,

    #[packed_field(bits = "160..175", endian = "msb")]
    /// Reserved bits, probably for extended types
    _ty_reserved: u16,

    #[packed_field(bits = "176..183", ty = "enum", endian = "msb")]
    /// Type of the block
    pub(crate) ty: BlockType,

    // STATUS BYTE
    // This must always be updated last
    #[packed_field(bits = "184")]
    /// Whether this block has been deleted
    pub(crate) deleted: bool,

    #[packed_field(bits = "185")]
    /// If true, no data can be "in route" to this block.
    pub(crate) finished: bool,

    #[packed_field(bits = "186")]
    /// Whether this block is the "root" of a tree
    ///
    /// Data blocks are always root (they have no children either).
    pub(crate) is_root: bool,

    #[packed_field(bits = "187..191")]
    /// Reserved bits
    _status_reserved: Integer<u8, packed_bits::Bits5>,
}

#[derive(Clone, Copy, Debug, PartialEq, PrimitiveEnum_u8)]
pub enum BlockType {
    DataBlock = 0b0000_0000,
    GenKeyGenValue = 0b0001_0001,
    GenKeySizedValue = 0b0001_0011,
    SizedKeyGenValue = 0b0011_0001,
    SizedKeySizedValue = 0b0011_0011,
}

impl Default for AllocHeader {
    fn default() -> AllocHeader {
        let heap = ROOT_OFFSET;
        AllocHeader {
            magic: MAGIC_NUMBER,
            version_major: VERSION_MAJOR,
            version_minor: VERSION_MINOR,
            version_patch: VERSION_PATCH,
            root: ROOT_OFFSET,
            heap: heap,
            capacity: ALLOC_INITIAL_SIZE - heap,
        }
    }
}

/// Given the key size calculate how much capacity a node needs
fn calc_capacity_node_sizekey_genvalue(key_size: u16) -> usize {
    // It stores an array of _values_, which are just pointers
    let values = NODE_NUM_ITEMS * size_of::<Offset>();
    // It stores an array of _leaves_, which are just pointers
    let leaves = (NODE_NUM_ITEMS - 1) * size_of::<Offset>();
    // It stores an array of keys
    let keys = NODE_NUM_ITEMS * key_size as usize;
    values + leaves + keys
}

impl BlockHeader {
    /// New SizedKeyGenValue, i.e. the default root
    fn new_sized_key_gen_value(parent: Offset, is_root: bool, key_size: u16) -> BlockHeader {
        let capacity = calc_capacity_node_sizekey_genvalue(key_size);
        BlockHeader {
            parent: parent,
            deleted: false,
            finished: false,
            is_root: is_root,
            _status_reserved: 0.into(),
            ty: BlockType::SizedKeyGenValue,
            _ty_reserved: 0x00,
            ty_keysize: key_size,
            ty_valuesize: 0x00,
            capacity: capacity,
            used: 0,
        }
    }
}

#[test]
fn sanity_header() {
    let expected = BlockHeader {
        parent: 0x10,
        deleted: false,
        finished: true,
        is_root: true,
        _status_reserved: 0.into(),
        ty: BlockType::GenKeyGenValue,
        _ty_reserved: 0x00,
        ty_keysize: 0x00,
        ty_valuesize: 0x00,
        capacity: 64,
        used: 64,
    };

    let packed = expected.pack();
    let result = BlockHeader::unpack(&packed).unwrap();
    assert_eq!(expected, result);

    // let test = TestPack {
    //     tiny_int: 5.into(),
    //     mode: SelfTestMode::DebugMode,
    //     enabled: true
    // };

    // let packed = test.pack();
    // assert_eq!([0b10111001], packed);

    // let unpacked = TestPack::unpack(&packed).unwrap();
    // assert_eq!(*unpacked.tiny_int, 5);
    // assert_eq!(unpacked.mode, SelfTestMode::DebugMode);
    // assert_eq!(unpacked.enabled, true);
}
