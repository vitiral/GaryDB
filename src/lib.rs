// TODO:
// - all K+V types should be Deserialize+Serialize

use std::cmp::Cmp;

type Offset = u64;
type Size = u32;

#[repr(packed)]
/// Location and Size Object
pub struct Block {
    /// Offset from the file start
    offset: Offset,

    /// Size of the data
    size: Size,
}

// FIXME: force to be a u16
pub enum DataTypeRaw {
    /// Raw data, no additional mappings
    Bytes,
    /// GenBTreeRaw
    GenBTree,
    /// Key for GenBTreeRaw
    GenKey,
}

#[repr(packed)]
/// Standard header for all blocks
pub struct BlockHeader {
    /// The parent of this block
    parent: Offset,

    /// Size of this block
    size: Size,

    /// Type of this block
    ty: DataTypeRaw,
}

#[repr(packed)]
pub struct GenBTreeRaw<K: Cmp, V> {
    header: BlockHeader,

    /// Keys: lookup keys for the map + locations of next bits of data
    ///
    /// Points to GenKey structs
    keys: [Block; 64],

    /// Additional leaves, 0 if they are not set.
    leaves: [Offset; 63],

    PhantomData<K>,
    PhantomData<V>,
}

pub struct GenKeyRaw<K: Cmp, V> {
    header: BlockHeader,

    /// Value of the generic key
    key: Vec<u8>,

    /// Location of the actual data
    data: Block,

    PhantomData<K>,
    PhantomData<V>,
}

pub struct DataBlockRaw<V> {
    header: BlockHeader,
    data: Vec<u8>,
    PhantomData<V>,
}
