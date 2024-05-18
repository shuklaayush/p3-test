use core::mem::{size_of, transmute};

use p3_derive::AlignedBorrow;
use p3_keccak_air::U64_LIMBS;
use p3_util::indices_arr;

#[cfg(feature = "debug-trace")]
use p3_derive::Headers;

use crate::chips::keccak_permute::NUM_U64_HASH_ELEMS;

#[repr(C)]
#[derive(AlignedBorrow)]
#[cfg_attr(feature = "debug-trace", derive(Headers))]
pub struct MerkleTreeCols<T> {
    pub is_real: T,

    pub is_final_step: T,

    pub node: [[T; U64_LIMBS]; NUM_U64_HASH_ELEMS],

    pub sibling: [[T; U64_LIMBS]; NUM_U64_HASH_ELEMS],

    pub parity_selector: T,

    pub left_node: [[T; U64_LIMBS]; NUM_U64_HASH_ELEMS],

    pub right_node: [[T; U64_LIMBS]; NUM_U64_HASH_ELEMS],

    pub output: [[T; U64_LIMBS]; NUM_U64_HASH_ELEMS],
}

impl<T: Copy> MerkleTreeCols<T> {}

pub(crate) const NUM_MERKLE_TREE_COLS: usize = size_of::<MerkleTreeCols<u8>>();
pub(crate) const MERKLE_TREE_COL_MAP: MerkleTreeCols<usize> = make_col_map();

const fn make_col_map() -> MerkleTreeCols<usize> {
    let indices_arr = indices_arr::<NUM_MERKLE_TREE_COLS>();
    unsafe { transmute::<[usize; NUM_MERKLE_TREE_COLS], MerkleTreeCols<usize>>(indices_arr) }
}
