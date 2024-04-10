use core::mem::{size_of, transmute};
use p3_test_derive::AlignedBorrow;
use p3_util::indices_arr;

#[cfg(feature = "debug-trace")]
use p3_test_derive::Headers;

#[repr(C)]
#[derive(AlignedBorrow)]
#[cfg_attr(feature = "debug-trace", derive(Headers))]
pub struct XorCols<T> {
    pub is_real: T,

    pub input1: [T; 4],

    pub input2: [T; 4],

    /// Bit decomposition of input_1 bytes
    pub bits1: [[T; 8]; 4],

    /// Bit decomposition of input_2 bytes
    pub bits2: [[T; 8]; 4],

    /// Aggregated output
    pub output: [T; 4],
}

impl<T: Copy> XorCols<T> {}

pub(crate) const NUM_XOR_COLS: usize = size_of::<XorCols<u8>>();
pub(crate) const XOR_COL_MAP: XorCols<usize> = make_col_map();

const fn make_col_map() -> XorCols<usize> {
    let indices_arr = indices_arr::<NUM_XOR_COLS>();
    unsafe { transmute::<[usize; NUM_XOR_COLS], XorCols<usize>>(indices_arr) }
}
