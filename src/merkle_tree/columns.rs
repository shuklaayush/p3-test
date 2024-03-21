use core::borrow::{Borrow, BorrowMut};
use core::mem::{size_of, transmute};

use p3_util::indices_arr;

pub(crate) const U64_LIMBS: usize = 4;
pub(crate) const NUM_U64_HASH_ELEMS: usize = 4;

#[repr(C)]
#[derive(Debug)]
pub struct MerkleTreeCols<T> {
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

impl<T> Borrow<MerkleTreeCols<T>> for [T] {
    fn borrow(&self) -> &MerkleTreeCols<T> {
        debug_assert_eq!(self.len(), NUM_MERKLE_TREE_COLS);
        let (prefix, shorts, suffix) = unsafe { self.align_to::<MerkleTreeCols<T>>() };
        debug_assert!(prefix.is_empty(), "Alignment should match");
        debug_assert!(suffix.is_empty(), "Alignment should match");
        debug_assert_eq!(shorts.len(), 1);
        &shorts[0]
    }
}

impl<T> BorrowMut<MerkleTreeCols<T>> for [T] {
    fn borrow_mut(&mut self) -> &mut MerkleTreeCols<T> {
        debug_assert_eq!(self.len(), NUM_MERKLE_TREE_COLS);
        let (prefix, shorts, suffix) = unsafe { self.align_to_mut::<MerkleTreeCols<T>>() };
        debug_assert!(prefix.is_empty(), "Alignment should match");
        debug_assert!(suffix.is_empty(), "Alignment should match");
        debug_assert_eq!(shorts.len(), 1);
        &mut shorts[0]
    }
}
