use core::mem::{size_of, transmute};

use p3_derive::AlignedBorrow;
use p3_util::indices_arr;

#[cfg(feature = "debug-trace")]
use p3_derive::Headers;

#[derive(Default, AlignedBorrow)]
#[cfg_attr(feature = "debug-trace", derive(Headers))]
pub struct RangeCols<T> {
    pub mult: T,
}

#[derive(Default, AlignedBorrow)]
#[cfg_attr(feature = "debug-trace", derive(Headers))]
pub struct RangePreprocessedCols<T> {
    pub counter: T,
}

pub const NUM_RANGE_COLS: usize = size_of::<RangeCols<u8>>();
pub const RANGE_COL_MAP: RangeCols<usize> = make_col_map();

pub const NUM_RANGE_PREPROCESSED_COLS: usize = size_of::<RangePreprocessedCols<u8>>();
pub const RANGE_PREPROCESSED_COL_MAP: RangePreprocessedCols<usize> = make_preprocessed_col_map();

const fn make_col_map() -> RangeCols<usize> {
    let indices_arr = indices_arr::<NUM_RANGE_COLS>();
    unsafe { transmute::<[usize; NUM_RANGE_COLS], RangeCols<usize>>(indices_arr) }
}

const fn make_preprocessed_col_map() -> RangePreprocessedCols<usize> {
    let indices_arr = indices_arr::<NUM_RANGE_PREPROCESSED_COLS>();
    unsafe {
        transmute::<[usize; NUM_RANGE_PREPROCESSED_COLS], RangePreprocessedCols<usize>>(indices_arr)
    }
}
